#!/usr/bin/env python3
"""
Live Telemetry Bridge - Connects Win32 Intercept to Dashboard via shared memory.

Streams real-time capture metrics, epigenetic gate states, and motor intents
to the egui dashboard through a memory-mapped telemetry channel.
"""

import sys
import os
import time
import struct
import mmap
import numpy as np
from dataclasses import dataclass, asdict
from typing import Optional

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'extensions'))
from win32_intercept import Win32ScreenCapture, MotorIntent

# Telemetry shared memory layout
TELEMETRY_MAGIC = b'TEL1'
TELEMETRY_SIZE = 64 * 1024  # 64KB telemetry buffer

@dataclass
class TelemetryFrame:
    """Single frame of telemetry data."""
    frame_id: int
    fps: float
    capture_latency_ms: float
    active_sectors: int
    total_sectors: int
    skip_ratio: float
    delta_mean: float
    delta_max: float
    intent_dx: float
    intent_dy: float
    intent_actions: int
    genome_voxels: int
    vram_mb: float

class TelemetryWriter:
    """Writes telemetry data to shared memory for dashboard consumption."""

    def __init__(self, name: str = "SAB_TELEMETRY"):
        self.name = name
        self.path = os.path.join(
            os.environ.get("LOCALAPPDATA", ""), "Temp", f"{name}.telemetry"
        )
        self._mm = None
        self._f = None
        self._frame_id = 0

    def open(self, size: int = TELEMETRY_SIZE):
        """Open or create telemetry shared memory."""
        if not os.path.exists(self.path):
            with open(self.path, "wb") as f:
                f.write(b'\x00' * size)

        self._f = open(self.path, "r+b")
        self._mm = mmap.mmap(self._f.fileno(), size)

    def write_frame(self, telemetry: TelemetryFrame):
        """Write a telemetry frame to shared memory."""
        if self._mm is None:
            self.open()

        # Write magic + frame ID
        self._mm.seek(0)
        self._mm.write(TELEMETRY_MAGIC)
        self._mm.write(struct.pack('<Q', telemetry.frame_id))

        # Write telemetry data as packed struct
        data = struct.pack(
            '<f f i i f f f f f I I f',
            telemetry.fps,
            telemetry.capture_latency_ms,
            telemetry.active_sectors,
            telemetry.total_sectors,
            telemetry.skip_ratio,
            telemetry.delta_mean,
            telemetry.delta_max,
            telemetry.intent_dx,
            telemetry.intent_dy,
            telemetry.intent_actions,
            telemetry.genome_voxels,
            telemetry.vram_mb,
        )
        self._mm.seek(12)
        self._mm.write(data)

        self._frame_id += 1

    def close(self):
        if self._mm:
            self._mm.close()
        if self._f:
            self._f.close()


class LiveTelemetryBridge:
    """
    Bridges the Win32 intercept perimeter to the dashboard.
    
    Captures frames, computes epigenetic gating, generates motor intents,
    and streams all metrics to shared memory for real-time dashboard display.
    """

    def __init__(self, target_fps: float = 30.0, enable_hid: bool = False):
        self.target_fps = target_fps
        self.enable_hid = enable_hid
        self.capture = Win32ScreenCapture()
        self.telemetry = TelemetryWriter()
        self._running = False
        self._frame_history = []
        self._prev_frame = None

        # Epigenetic gating config
        self.sector_size = 8
        self.sectors = 16
        self.delta_threshold = 0.02
        self.hysteresis_frames = 3
        self._sector_static_count = np.zeros(256, dtype=np.int32)
        self._sector_active = np.ones(256, dtype=bool)

        # Genome info
        self.genome_voxels = 1_289_158_774
        self.vram_mb = 4917.75 * 1024 / 1024  # Approximate

    def run(self):
        """Main telemetry loop."""
        print("="*60)
        print("  Live Telemetry Bridge")
        print("="*60)
        print(f"  Target FPS: {self.target_fps}")
        print(f"  HID Output: {'enabled' if self.enable_hid else 'disabled'}")
        print(f"  Genome: {self.genome_voxels:,} voxels")
        print()

        self.capture.initialize()
        self.telemetry.open()
        self._running = True

        frame_interval = 1.0 / self.target_fps
        frame_id = 0
        fps_counter = 0
        fps_timer = time.time()
        current_fps = 0.0

        try:
            while self._running:
                loop_start = time.time()

                # Capture frame
                capture_start = time.time()
                frame = self.capture.capture_frame()
                capture_latency = (time.time() - capture_start) * 1000

                # Compute epigenetic gating
                active_sectors, skip_ratio, delta_mean, delta_max = self._compute_gating(frame)

                # Simulate motor intent from frame delta
                intent = self._compute_intent(frame)

                # Compute FPS
                fps_counter += 1
                if time.time() - fps_timer >= 1.0:
                    current_fps = fps_counter / (time.time() - fps_timer)
                    fps_counter = 0
                    fps_timer = time.time()

                # Write telemetry
                telemetry = TelemetryFrame(
                    frame_id=frame_id,
                    fps=current_fps,
                    capture_latency_ms=capture_latency,
                    active_sectors=int(active_sectors),
                    total_sectors=256,
                    skip_ratio=skip_ratio,
                    delta_mean=float(delta_mean),
                    delta_max=float(delta_max),
                    intent_dx=intent.delta_x,
                    intent_dy=intent.delta_y,
                    intent_actions=intent.binary_action_register,
                    genome_voxels=self.genome_voxels,
                    vram_mb=self.vram_mb,
                )
                self.telemetry.write_frame(telemetry)

                frame_id += 1

                # Frame rate limiting
                elapsed = time.time() - loop_start
                sleep_time = frame_interval - elapsed
                if sleep_time > 0:
                    time.sleep(sleep_time)

                # Print status every 60 frames
                if frame_id % 60 == 0:
                    print(f"  Frame {frame_id:6d} | FPS: {current_fps:5.1f} | "
                          f"Active: {active_sectors:3d}/256 | "
                          f"Skip: {skip_ratio*100:5.1f}% | "
                          f"Latency: {capture_latency:5.1f}ms")

        except KeyboardInterrupt:
            print("\n  Shutting down...")
        finally:
            self._running = False
            self.capture.cleanup()
            self.telemetry.close()

    def _compute_gating(self, frame: np.ndarray):
        """Compute epigenetic gating for current frame."""
        if self._prev_frame is None:
            self._prev_frame = frame.copy()
            return 256, 0.0, 0.0, 0.0

        delta = np.abs(frame - self._prev_frame)
        delta_mean = delta.mean()
        delta_max = delta.max()

        active_count = 0
        for sy in range(self.sectors):
            for sx in range(self.sectors):
                idx = sy * self.sectors + sx
                sector_delta = delta[
                    sy*self.sector_size:(sy+1)*self.sector_size,
                    sx*self.sector_size:(sx+1)*self.sector_size
                ]
                sector_mean = sector_delta.mean()

                if sector_mean > self.delta_threshold:
                    self._sector_static_count[idx] = 0
                    self._sector_active[idx] = True
                else:
                    self._sector_static_count[idx] += 1
                    if self._sector_static_count[idx] >= self.hysteresis_frames:
                        self._sector_active[idx] = False

                if self._sector_active[idx]:
                    active_count += 1

        skip_ratio = 1.0 - active_count / 256.0
        self._prev_frame = frame.copy()

        return active_count, skip_ratio, delta_mean, delta_max

    def _compute_intent(self, frame: np.ndarray) -> MotorIntent:
        """Compute motor intent from frame analysis."""
        if self._prev_frame is None:
            return MotorIntent(0.0, 0.0, 0)

        delta = np.abs(frame - self._prev_frame)

        # Compute weighted center of motion
        y_coords, x_coords = np.mgrid[0:128, 0:128]
        total_delta = delta.sum()

        if total_delta > 0.01:
            center_x = (x_coords * delta).sum() / total_delta
            center_y = (y_coords * delta).sum() / total_delta

            # Convert to delta from center
            dx = (center_x - 64.0) * 0.5
            dy = (center_y - 64.0) * 0.5
        else:
            dx = 0.0
            dy = 0.0

        # Determine actions based on motion magnitude
        actions = 0
        if total_delta > 0.001:
            actions |= 1  # ACTION_MOUSE_MOVE
        if total_delta > 5.0:
            actions |= 256  # ACTION_CLICK

        return MotorIntent(float(dx), float(dy), actions)


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Live Telemetry Bridge')
    parser.add_argument('--fps', type=float, default=30.0, help='Target FPS')
    parser.add_argument('--hid', action='store_true', help='Enable HID output')

    args = parser.parse_args()
    bridge = LiveTelemetryBridge(args.fps, args.hid)
    bridge.run()
