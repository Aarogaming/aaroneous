#!/usr/bin/env python3
"""
Spatial-Kinetic Orchestrator - End-to-End Pipeline

Unifies the complete spatial-kinetic execution loop:
1. Win32 screen capture → 128x128 float grid
2. Epigenetic visual gating → zero-compute skip on static regions
3. CPU reflex kernel → bitwise genome processing (2-bit A/T/C/G)
4. Motor intent computation → delta coordinates + action flags
5. HID output bridge → SendInput hardware events

This is the Python reference implementation that mirrors the Rust/WGPU pipeline.
"""

import sys
import os
import time
import struct
import mmap
import numpy as np
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

# Add project paths
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'extensions'))
sys.path.insert(0, os.path.dirname(__file__))

from win32_intercept import Win32ScreenCapture, HIDOutputBridge, MotorIntent
from win32_intercept import (
    ACTION_MOUSE_MOVE, ACTION_MOUSE_LEFT_DOWN, ACTION_MOUSE_LEFT_UP,
    ACTION_MOUSE_RIGHT_DOWN, ACTION_MOUSE_RIGHT_UP, ACTION_MOUSE_WHEEL,
    ACTION_CLICK, ACTION_DOUBLE_CLICK, ACTION_DRAG_START, ACTION_DRAG_END,
)


# ============================================================================
# Constants
# ============================================================================

GRID_WIDTH = 128
GRID_HEIGHT = 128
GRID_SIZE = GRID_WIDTH * GRID_HEIGHT

SECTOR_SIZE = 8
SECTORS_PER_ROW = 16
SECTORS_PER_COL = 16
TOTAL_SECTORS = SECTORS_PER_ROW * SECTORS_PER_COL  # 256

DELTA_THRESHOLD = 0.02
HYSTERESIS_FRAMES = 3

# 2-bit genomic encoding thresholds
QUANT_THRESHOLDS = [-1.0, 0.0, 1.0]

# Parameter lookup table (matches reflex_kernel.wgsl)
PARAMETER_LUT = np.array([-2.5, -0.5, 0.5, 2.5], dtype=np.float32)


# ============================================================================
# Epigenetic Gate Matrix
# ============================================================================

class EpigeneticGateMatrix:
    """Binary bitmask overlay for zero-compute skipping on static screen regions."""

    def __init__(self):
        self.sector_means = np.zeros(TOTAL_SECTORS, dtype=np.float32)
        self.sector_active = np.ones(TOTAL_SECTORS, dtype=bool)
        self.sector_static_count = np.zeros(TOTAL_SECTORS, dtype=np.int32)
        self.active_count = TOTAL_SECTORS
        self.frame_id = 0

    def update(self, frame: np.ndarray) -> int:
        """Update gating matrix based on new frame. Returns active sector count."""
        self.frame_id += 1

        for sy in range(SECTORS_PER_COL):
            for sx in range(SECTORS_PER_ROW):
                idx = sy * SECTORS_PER_ROW + sx
                y_start = sy * SECTOR_SIZE
                x_start = sx * SECTOR_SIZE

                sector = frame[y_start:y_start+SECTOR_SIZE, x_start:x_start+SECTOR_SIZE]
                mean = sector.mean()
                delta = abs(mean - self.sector_means[idx])

                if delta > DELTA_THRESHOLD:
                    self.sector_active[idx] = True
                    self.sector_static_count[idx] = 0
                else:
                    self.sector_static_count[idx] += 1
                    if self.sector_static_count[idx] >= HYSTERESIS_FRAMES:
                        self.sector_active[idx] = False

                self.sector_means[idx] = mean

        self.active_count = int(self.sector_active.sum())
        return self.active_count

    def get_packed_mask(self) -> np.ndarray:
        """Get 256-bit packed mask as 4x uint64 for GPU transfer."""
        mask = np.zeros(4, dtype=np.uint64)
        for i in range(TOTAL_SECTORS):
            if self.sector_active[i]:
                word = i // 64
                bit = i % 64
                mask[word] |= (1 << bit)
        return mask

    def skip_ratio(self) -> float:
        """Get percentage of sectors gated off."""
        return 1.0 - (self.active_count / TOTAL_SECTORS)

    def is_pixel_active(self, x: int, y: int) -> bool:
        """Check if a pixel coordinate is in an active sector."""
        if x >= GRID_WIDTH or y >= GRID_HEIGHT:
            return False
        sector_x = x // SECTOR_SIZE
        sector_y = y // SECTOR_SIZE
        idx = sector_y * SECTORS_PER_ROW + sector_x
        return bool(self.sector_active[idx])


# ============================================================================
# Genome Loader
# ============================================================================

class GenomeLoader:
    """Loads the universal gaming genome from binary file."""

    def __init__(self, genome_path: str):
        self.genome_path = genome_path
        self.voxels = None
        self.voxel_count = 0

    def load(self) -> bool:
        """Load genome from .bin file."""
        if not os.path.exists(self.genome_path):
            print(f"[GenomeLoader] Genome not found: {self.genome_path}")
            return False

        file_size = os.path.getsize(self.genome_path)
        print(f"[GenomeLoader] Loading genome: {self.genome_path}")
        print(f"[GenomeLoader] File size: {file_size / (1024**3):.2f} GB")

        with open(self.genome_path, 'rb') as f:
            # Check for AASv1 header
            magic = f.read(5)
            if magic == b'AASv1':
                # Parse header
                self.voxel_count = struct.unpack('<Q', f.read(8))[0]
                weight_count = struct.unpack('<Q', f.read(8))[0]
                num_tracks = struct.unpack('<I', f.read(4))[0]

                print(f"[GenomeLoader] Header: {self.voxel_count:,} voxels, "
                      f"{weight_count:,} weights, {num_tracks} tracks")

                # Read track sizes
                for i in range(num_tracks):
                    track_size = struct.unpack('<Q', f.read(8))[0]

                # Read genome data
                data = f.read(self.voxel_count * 4)
                self.voxels = np.frombuffer(data, dtype=np.uint32)
            else:
                # Raw binary format (no header)
                f.seek(0)
                data = f.read()
                self.voxels = np.frombuffer(data, dtype=np.uint32)
                self.voxel_count = len(self.voxels)
                print(f"[GenomeLoader] Raw binary: {self.voxel_count:,} voxels")

        print(f"[GenomeLoader] Genome loaded: {self.voxel_count:,} voxels "
              f"({self.voxel_count * 16:,} equivalent weights)")
        return True

    def get_voxel(self, index: int) -> np.uint32:
        """Get a single u32 voxel."""
        if 0 <= index < self.voxel_count:
            return self.voxels[index]
        return np.uint32(0)

    def get_voxels_slice(self, start: int, end: int) -> np.ndarray:
        """Get a slice of voxels."""
        return self.voxels[start:end]


# ============================================================================
# CPU Reflex Kernel
# ============================================================================

class CPUReflexKernel:
    """
    CPU implementation of the reflex kernel compute shader.

    Processes the 2-bit genome using bitwise operations and LUT lookup,
    matching the WGSL shader behavior exactly.
    """

    def __init__(self, genome: GenomeLoader):
        self.genome = genome
        self.parameter_lut = PARAMETER_LUT

    def execute(self, pixels: np.ndarray, gate_matrix: EpigeneticGateMatrix) -> np.ndarray:
        """
        Execute reflex kernel on a frame.

        Args:
            pixels: 128x128 normalized float grid
            gate_matrix: epigenetic gating state

        Returns:
            Computed intent values array
        """
        flat_pixels = pixels.flatten()
        num_voxels = min(self.genome.voxel_count, len(flat_pixels) // 16)
        intents = np.zeros(num_voxels, dtype=np.float32)

        for i in range(num_voxels):
            voxel = int(self.genome.get_voxel(i))
            accumulated_force = 0.0

            for j in range(16):
                pixel_idx = i * 16 + j
                if pixel_idx >= len(flat_pixels):
                    break

                # Check if this pixel's sector is active
                px = pixel_idx % GRID_WIDTH
                py = pixel_idx // GRID_WIDTH
                if not gate_matrix.is_pixel_active(px, py):
                    continue

                # Extract 2-bit parameter via right-shift and mask
                extracted_base = (voxel >> (j * 2)) & 0x03

                # LUT lookup and multiply
                accumulated_force += flat_pixels[pixel_idx] * self.parameter_lut[extracted_base]

            intents[i] = accumulated_force

        return intents

    def execute_vectorized(self, pixels: np.ndarray, gate_matrix: EpigeneticGateMatrix) -> np.ndarray:
        """
        Vectorized CPU reflex kernel using numpy operations.

        Much faster than the loop-based version for large genomes.
        """
        flat_pixels = pixels.flatten()
        num_voxels = min(self.genome.voxel_count, len(flat_pixels) // 16)

        if num_voxels == 0:
            return np.array([], dtype=np.float32)

        # Get voxel batch
        voxels = self.genome.get_voxels_slice(0, num_voxels)

        # Extract all 16 2-bit parameters at once using vectorized bit operations
        intents = np.zeros(num_voxels, dtype=np.float32)

        for j in range(16):
            pixel_idx = np.arange(num_voxels) * 16 + j
            valid_mask = pixel_idx < len(flat_pixels)

            # Check sector activity
            px = pixel_idx % GRID_WIDTH
            py = pixel_idx // GRID_WIDTH
            active_mask = np.array([gate_matrix.is_pixel_active(int(x), int(y))
                                   for x, y in zip(px, py)], dtype=bool)

            combined_mask = valid_mask & active_mask

            # Extract 2-bit parameters
            extracted = (voxels >> (j * 2)) & 0x03

            # LUT lookup
            lut_values = self.parameter_lut[extracted]

            # Multiply and accumulate
            pixel_values = np.where(combined_mask, flat_pixels[pixel_idx], 0.0)
            intents += pixel_values * lut_values

        return intents


# ============================================================================
# Motor Intent Computer
# ============================================================================

class MotorIntentComputer:
    """Converts reflex kernel output to motor intent for HID execution."""

    def __init__(self, sensitivity: float = 1.0):
        self.sensitivity = sensitivity

    def compute(self, intents: np.ndarray, gate_matrix: EpigeneticGateMatrix) -> MotorIntent:
        """
        Compute motor intent from reflex kernel output.

        Aggregates intent values weighted by position to produce
        delta_x, delta_y, and action flags.
        """
        if len(intents) == 0:
            return MotorIntent(0.0, 0.0, 0)

        sum_x = 0.0
        sum_y = 0.0
        max_magnitude = 0.0
        active_count = 0

        for i, intent in enumerate(intents):
            # Map voxel index to screen position
            pixel_idx = i * 16
            if pixel_idx >= GRID_SIZE:
                break

            x = (pixel_idx % GRID_WIDTH) / GRID_WIDTH
            y = (pixel_idx // GRID_WIDTH) / GRID_HEIGHT

            if gate_matrix.is_pixel_active(pixel_idx % GRID_WIDTH, pixel_idx // GRID_WIDTH):
                sum_x += intent * (x - 0.5)
                sum_y += intent * (y - 0.5)
                max_magnitude = max(max_magnitude, abs(intent))
                active_count += 1

        if active_count == 0:
            return MotorIntent(0.0, 0.0, 0)

        # Normalize
        scale = 1.0 / max_magnitude if max_magnitude > 0 else 1.0
        dx = sum_x * scale / active_count * 100.0 * self.sensitivity
        dy = sum_y * scale / active_count * 100.0 * self.sensitivity

        # Determine action flags
        actions = ACTION_MOUSE_MOVE
        if max_magnitude > 2.0:
            actions |= ACTION_CLICK
        if max_magnitude > 3.5:
            actions |= ACTION_DOUBLE_CLICK

        return MotorIntent(float(dx), float(dy), actions)


# ============================================================================
# Spatial-Kinetic Orchestrator
# ============================================================================

@dataclass
class OrchestratorConfig:
    genome_path: str = "chromosomes/universal_gaming_core.bin"
    target_fps: float = 10.0
    mouse_sensitivity: float = 1.0
    enable_hid: bool = False
    enable_gating: bool = True
    use_vectorized: bool = True
    max_frames: int = 0  # 0 = unlimited


class SpatialKineticOrchestrator:
    """
    Main orchestrator for the spatial-kinetic pipeline.

    Ties together screen capture, epigenetic gating, genome processing,
    and HID output into a single execution loop.
    """

    def __init__(self, config: OrchestratorConfig = OrchestratorConfig()):
        self.config = config
        self.capture = Win32ScreenCapture()
        self.gate_matrix = EpigeneticGateMatrix()
        self.genome = GenomeLoader(config.genome_path)
        self.kernel = None
        self.intent_computer = MotorIntentComputer(config.mouse_sensitivity)
        self.hid_bridge = HIDOutputBridge() if config.enable_hid else None

        self._running = False
        self._frame_id = 0
        self._fps_counter = 0
        self._fps_timer = time.time()
        self._current_fps = 0.0
        self._prev_frame = None

    def initialize(self) -> bool:
        """Initialize all pipeline components."""
        print("="*60)
        print("  Spatial-Kinetic Orchestrator")
        print("="*60)

        # Load genome
        if not self.genome.load():
            print("[Orchestrator] Genome loading failed")
            return False

        # Create reflex kernel
        self.kernel = CPUReflexKernel(self.genome)

        # Initialize capture
        self.capture.initialize()

        print(f"\n  Configuration:")
        print(f"    Genome: {self.config.genome_path}")
        print(f"    Voxels: {self.genome.voxel_count:,}")
        print(f"    FPS: {self.config.target_fps}")
        print(f"    HID: {'enabled' if self.config.enable_hid else 'disabled'}")
        print(f"    Gating: {'enabled' if self.config.enable_gating else 'disabled'}")
        print(f"    Vectorized: {self.config.use_vectorized}")
        print()

        return True

    def run(self):
        """Execute the main reflex loop."""
        if not self.initialize():
            return

        self._running = True
        frame_interval = 1.0 / self.config.target_fps

        print("  Starting reflex loop...")
        print("  Press Ctrl+C to stop.\n")

        try:
            while self._running:
                loop_start = time.time()

                # Execute one frame
                self._execute_frame()

                self._frame_id += 1

                # Check frame limit
                if self.config.max_frames > 0 and self._frame_id >= self.config.max_frames:
                    break

                # FPS tracking
                self._fps_counter += 1
                elapsed = time.time() - self._fps_timer
                if elapsed >= 1.0:
                    self._current_fps = self._fps_counter / elapsed
                    self._fps_counter = 0
                    self._fps_timer = time.time()

                # Status output every 30 frames
                if self._frame_id % 30 == 0:
                    active = self.gate_matrix.active_count
                    skip = self.gate_matrix.skip_ratio() * 100
                    print(f"  Frame {self._frame_id:6d} | FPS: {self._current_fps:5.1f} | "
                          f"Active: {active:3d}/256 | Skip: {skip:5.1f}%")

                # Frame rate limiting
                frame_elapsed = time.time() - loop_start
                sleep_time = frame_interval - frame_elapsed
                if sleep_time > 0:
                    time.sleep(sleep_time)

        except KeyboardInterrupt:
            print("\n  Received shutdown signal...")
        finally:
            self._running = False
            self.capture.cleanup()
            print(f"\n  Reflex loop stopped after {self._frame_id} frames")

    def _execute_frame(self):
        """Execute a single frame of the spatial-kinetic pipeline."""
        # Step 1: Capture screen
        frame = self.capture.capture_frame()

        # Step 2: Update epigenetic gate matrix
        if self.config.enable_gating:
            active_sectors = self.gate_matrix.update(frame)
        else:
            self.gate_matrix.force_all_active()
            active_sectors = TOTAL_SECTORS

        # Step 3: Execute reflex kernel
        if self.config.use_vectorized:
            intents = self.kernel.execute_vectorized(frame, self.gate_matrix)
        else:
            intents = self.kernel.execute(frame, self.gate_matrix)

        # Step 4: Compute motor intent
        intent = self.intent_computer.compute(intents, self.gate_matrix)

        # Step 5: Execute HID output
        if self.hid_bridge and intent.binary_action_register != 0:
            self.hid_bridge.execute_intent(intent)

        # Store frame for next delta computation
        self._prev_frame = frame.copy()

    def stop(self):
        """Stop the reflex loop."""
        self._running = False


def main():
    import argparse

    parser = argparse.ArgumentParser(description='Spatial-Kinetic Orchestrator')
    parser.add_argument('--genome', '-g', default='chromosomes/universal_gaming_core.bin',
                        help='Path to genome binary file')
    parser.add_argument('--fps', type=float, default=10.0, help='Target FPS')
    parser.add_argument('--sensitivity', '-s', type=float, default=1.0, help='Mouse sensitivity')
    parser.add_argument('--hid', action='store_true', help='Enable HID output')
    parser.add_argument('--no-gating', action='store_true', help='Disable epigenetic gating')
    parser.add_argument('--no-vectorized', action='store_true', help='Use loop-based kernel')
    parser.add_argument('--frames', type=int, default=0, help='Max frames (0 = unlimited)')

    args = parser.parse_args()

    config = OrchestratorConfig(
        genome_path=args.genome,
        target_fps=args.fps,
        mouse_sensitivity=args.sensitivity,
        enable_hid=args.hid,
        enable_gating=not args.no_gating,
        use_vectorized=not args.no_vectorized,
        max_frames=args.frames,
    )

    orchestrator = SpatialKineticOrchestrator(config)
    orchestrator.run()


if __name__ == '__main__':
    main()
