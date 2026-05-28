#!/usr/bin/env python3
"""
HELIX_COMPILER.py - Universal 16-Way Genome Compiler

Compiles harvested GGUF decision geometries into a single monolithic
HDF5 binary package (universal_gaming_core.aas) with 16-way geometric
tensor cord layout.

Eliminates feature fragmentation by mapping spatial properties collectively
across all software design patterns into a unified pool of cross-genre intelligence.
"""

import os
import sys
import struct
import numpy as np
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass

try:
    import h5py
    HDF5_AVAILABLE = True
except ImportError:
    HDF5_AVAILABLE = False
    print("[WARN] h5py not available - output will be raw binary format")


# ============================================================================
# 16-Way Universal Track Configuration
# ============================================================================

TRACK_DEFINITIONS = {
    # Spatial Navigation / Pathfinding (Tracks 0-3)
    0: {'name': 'nav_pathfinding', 'domain': 'spatial', 'description': 'A* graph traversal, waypoint interpolation'},
    1: {'name': 'nav_collision', 'domain': 'spatial', 'description': 'Bounding box detection, obstacle avoidance'},
    2: {'name': 'nav_terrain', 'domain': 'spatial', 'description': 'Elevation mapping, surface classification'},
    3: {'name': 'nav_flowfield', 'domain': 'spatial', 'description': 'Vector field navigation, crowd movement'},
    
    # State Machine / Menu Logic (Tracks 4-7)
    4: {'name': 'state_transitions', 'domain': 'logic', 'description': 'FSM state transitions, menu navigation'},
    5: {'name': 'state_hierarchy', 'domain': 'logic', 'description': 'Nested state management, sub-menu depth'},
    6: {'name': 'state_rewards', 'domain': 'logic', 'description': 'Action reward prediction, utility scoring'},
    7: {'name': 'state_memory', 'domain': 'logic', 'description': 'History tracking, context persistence'},
    
    # Visual Recognition / Targeting (Tracks 8-11)
    8: {'name': 'vis_object_detect', 'domain': 'visual', 'description': 'Object bounding, entity classification'},
    9: {'name': 'vis_motion_track', 'domain': 'visual', 'description': 'Velocity estimation, trajectory prediction'},
    10: {'name': 'vis_threat_assess', 'domain': 'visual', 'description': 'Danger vector analysis, priority targeting'},
    11: {'name': 'vis_hud_parse', 'domain': 'visual', 'description': 'UI element extraction, health/ammo reading'},
    
    # Resource Management / Optimization (Tracks 12-15)
    12: {'name': 'res_budget_alloc', 'domain': 'resource', 'description': 'Resource distribution, economy balancing'},
    13: {'name': 'res_timing_opt', 'domain': 'resource', 'description': 'Cooldown management, action sequencing'},
    14: {'name': 'res_risk_eval', 'domain': 'resource', 'description': 'Risk/reward calculation, survival heuristics'},
    15: {'name': 'res_adapt_rate', 'domain': 'resource', 'description': 'Learning rate modulation, plasticity control'},
}

NUM_TRACKS = 16

# 2-bit genomic encoding
GENOME_LUT = {
    0b00: 'A',  # Strong negative (-2.5)
    0b01: 'T',  # Weak negative (-0.5)
    0b10: 'C',  # Weak positive (0.5)
    0b11: 'G',  # Strong positive (2.5)
}

QUANT_THRESHOLDS = [-1.0, 0.0, 1.0]


# ============================================================================
# Genomic Encoding Utilities
# ============================================================================

def float_to_2bit(value: float) -> int:
    """Map a float weight to a 2-bit genomic state."""
    if value < QUANT_THRESHOLDS[0]:
        return 0b00
    elif value < QUANT_THRESHOLDS[1]:
        return 0b01
    elif value < QUANT_THRESHOLDS[2]:
        return 0b10
    else:
        return 0b11


def pack_2bit_to_u32(values: np.ndarray) -> np.ndarray:
    """Pack 2-bit values into u32 voxels (16 values per u32)."""
    # Pad to multiple of 16
    remainder = len(values) % 16
    if remainder != 0:
        values = np.pad(values, (0, 16 - remainder), mode='constant')
    
    # Reshape into groups of 16
    groups = values.reshape(-1, 16)
    
    # Pack each group into u32 using vectorized operations
    shifts = np.array([0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30], dtype=np.uint32)
    masks = np.array([0x03] * 16, dtype=np.uint32)
    
    packed = np.zeros(len(groups), dtype=np.uint32)
    for i, group in enumerate(groups):
        voxel = np.uint32(0)
        for j in range(16):
            voxel |= np.uint32(int(group[j])) << int(shifts[j])
        packed[i] = voxel
    
    return packed


def unpack_u32_to_2bit(voxels: np.ndarray) -> np.ndarray:
    """Unpack u32 voxels back to 2-bit values."""
    shifts = np.array([0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30], dtype=np.uint32)
    
    unpacked = np.zeros(len(voxels) * 16, dtype=np.uint8)
    for i, voxel in enumerate(voxels):
        for j in range(16):
            unpacked[i * 16 + j] = (int(voxel) >> int(shifts[j])) & 0x03
    
    return unpacked


# ============================================================================
# Input Source Readers
# ============================================================================

class GenomeSourceReader:
    """Reads genome data from various input formats."""
    
    @staticmethod
    def read_hdf5(path: str) -> Tuple[np.ndarray, Dict[str, any]]:
        """Read genome data from HDF5 file."""
        if not HDF5_AVAILABLE:
            raise ImportError("h5py required for HDF5 reading")
        
        with h5py.File(path, 'r') as f:
            voxels = f['genome_voxels'][:]
            metadata = dict(f['metadata'].attrs)
            tracks = {}
            for key in f['tracks'].keys():
                tracks[key] = f['tracks'][key][:]
        
        return voxels, {'metadata': metadata, 'tracks': tracks}
    
    @staticmethod
    def read_raw_binary(path: str) -> Tuple[np.ndarray, Dict[str, any]]:
        """Read genome data from raw .bin/.aas file."""
        with open(path, 'rb') as f:
            magic = f.read(5)
            if magic != b'AASv1':
                raise ValueError(f"Invalid magic: {magic}")
            
            voxel_count = struct.unpack('<Q', f.read(8))[0]
            weight_count = struct.unpack('<Q', f.read(8))[0]
            
            voxels = np.frombuffer(f.read(voxel_count * 4), dtype=np.uint32)
        
        return voxels, {'weight_count': weight_count}
    
    @staticmethod
    def read_numpy(path: str) -> Tuple[np.ndarray, Dict[str, any]]:
        """Read genome data from .npy file."""
        data = np.load(path, allow_pickle=True)
        if isinstance(data, np.ndarray) and data.dtype == np.uint32:
            return data, {}
        raise ValueError("Expected uint32 numpy array")


# ============================================================================
# Helix Compiler Core
# ============================================================================

@dataclass
class HelixOutput:
    """Compiled helix output structure."""
    genome_voxels: np.ndarray
    track_voxels: Dict[int, np.ndarray]
    metadata: Dict[str, any]
    track_assignments: Dict[int, str]


class HelixCompiler:
    """
    Compiles multiple genome sources into a single 16-way universal genome.
    
    Merges cross-genre intelligence into a unified pool where switching
    between applications requires no code change - just reading from the
    same massive intelligence pool with different track weightings.
    """
    
    def __init__(self, output_path: str = "chromosomes/universal_gaming_core.aas"):
        self.output_path = output_path
        self.sources: List[Tuple[str, np.ndarray, Dict]] = []
        self.compiled: Optional[HelixOutput] = None
    
    def add_source(self, path: str, source_type: str = 'auto') -> 'HelixCompiler':
        """
        Add a genome source file.
        
        Args:
            path: Path to genome file (.aas, .h5, .npy, .bin)
            source_type: 'auto', 'hdf5', 'raw', or 'numpy'
        """
        if not os.path.exists(path):
            print(f"[WARN] Source not found: {path}")
            return self
        
        # Auto-detect format
        if source_type == 'auto':
            ext = Path(path).suffix.lower()
            if ext in ['.h5', '.hdf5']:
                source_type = 'hdf5'
            elif ext in ['.npy']:
                source_type = 'numpy'
            else:
                source_type = 'raw'
        
        # Read source
        if source_type == 'hdf5':
            voxels, extra = GenomeSourceReader.read_hdf5(path)
        elif source_type == 'raw':
            voxels, extra = GenomeSourceReader.read_raw_binary(path)
        elif source_type == 'numpy':
            voxels, extra = GenomeSourceReader.read_numpy(path)
        else:
            raise ValueError(f"Unknown source type: {source_type}")
        
        self.sources.append((path, voxels, extra))
        print(f"  [ADDED] {path} ({len(voxels):,} voxels)")
        
        return self
    
    def compile(self) -> HelixOutput:
        """
        Compile all sources into unified 16-way genome.
        
        Merges multiple model genomes by concatenating their voxel data
        and organizing into the 16-track geometric layout.
        """
        print(f"\n[HELIX] Compiling {len(self.sources)} source(s)...")
        
        if not self.sources:
            raise ValueError("No sources added to compiler")
        
        # Concatenate all source voxels
        all_voxels = []
        total_weights = 0
        
        for path, voxels, extra in self.sources:
            all_voxels.append(voxels)
            if 'weight_count' in extra:
                total_weights += extra['weight_count']
            elif 'metadata' in extra:
                total_weights += extra['metadata'].get('total_weights', 0)
        
        combined = np.concatenate(all_voxels)
        print(f"  Combined voxels: {len(combined):,}")
        
        # Organize into 16 tracks
        track_voxels = self._distribute_tracks(combined)
        
        # Build metadata
        metadata = {
            'total_voxels': len(combined),
            'total_weights': total_weights,
            'num_tracks': NUM_TRACKS,
            'num_sources': len(self.sources),
            'source_files': [s[0] for s in self.sources],
            'genome_version': '1.0',
            'created': str(__import__('datetime').datetime.now()),
        }
        
        # Build track assignments
        track_assignments = {
            track_id: track_def['name']
            for track_id, track_def in TRACK_DEFINITIONS.items()
        }
        
        self.compiled = HelixOutput(
            genome_voxels=combined,
            track_voxels=track_voxels,
            metadata=metadata,
            track_assignments=track_assignments
        )
        
        print(f"  Tracks organized: {NUM_TRACKS}")
        for track_id, voxels in track_voxels.items():
            track_name = TRACK_DEFINITIONS[track_id]['name']
            print(f"    Track {track_id:2d} [{track_name}]: {len(voxels):,} voxels")
        
        return self.compiled
    
    def _distribute_tracks(self, voxels: np.ndarray) -> Dict[int, np.ndarray]:
        """Distribute voxels evenly across 16 tracks."""
        track_size = len(voxels) // NUM_TRACKS
        remainder = len(voxels) % NUM_TRACKS
        
        track_voxels = {}
        offset = 0
        
        for track_id in range(NUM_TRACKS):
            size = track_size + (1 if track_id < remainder else 0)
            track_voxels[track_id] = voxels[offset:offset + size]
            offset += size
        
        return track_voxels
    
    def write_output(self) -> str:
        """Write compiled genome to output file."""
        if self.compiled is None:
            raise ValueError("Must call compile() before write_output()")
        
        print(f"\n[OUTPUT] Writing to: {self.output_path}")
        
        ext = Path(self.output_path).suffix.lower()
        
        if ext in ['.h5', '.hdf5'] or (ext == '.aas' and HDF5_AVAILABLE):
            return self._write_hdf5()
        else:
            return self._write_raw_binary()
    
    def _write_hdf5(self) -> str:
        """Write to HDF5 format with SWMR support."""
        output_path = self.output_path
        if not output_path.endswith('.h5'):
            output_path = self.output_path.replace('.aas', '.h5')
        
        with h5py.File(output_path, 'w', libver='latest') as f:
            f.swmr_mode = True
            
            # Write combined genome
            f.create_dataset(
                'genome_voxels',
                data=self.compiled.genome_voxels,
                dtype='uint32',
                compression='lzf'
            )
            
            # Write 16 tracks
            tracks_group = f.create_group('tracks')
            for track_id, voxels in self.compiled.track_voxels.items():
                track_name = TRACK_DEFINITIONS[track_id]['name']
                tracks_group.create_dataset(
                    f'track_{track_id:02d}_{track_name}',
                    data=voxels,
                    dtype='uint32',
                    compression='lzf'
                )
            
            # Write metadata
            meta_group = f.create_group('metadata')
            for key, value in self.compiled.metadata.items():
                if isinstance(value, list):
                    value = ', '.join(str(v) for v in value)
                meta_group.attrs[key] = value
            
            # Write track definitions
            track_def_group = f.create_group('track_definitions')
            for track_id, defn in TRACK_DEFINITIONS.items():
                track_group = track_def_group.create_group(f'track_{track_id:02d}')
                track_group.attrs['name'] = defn['name']
                track_group.attrs['domain'] = defn['domain']
                track_group.attrs['description'] = defn['description']
            
            # Write parameter lookup table
            f.create_dataset('parameter_lut', data=np.array([-2.5, -0.5, 0.5, 2.5], dtype=np.float32))
            
            # Write encoding map
            f.create_dataset('encoding_map', data=np.array([0, 1, 2, 3], dtype=np.uint8))
        
        file_size = os.path.getsize(output_path)
        print(f"  Output size: {file_size / (1024*1024):.2f} MB")
        
        return output_path
    
    def _write_raw_binary(self) -> str:
        """Write to raw binary .aas format."""
        with open(self.output_path, 'wb') as f:
            # Header
            f.write(b'AASv1')
            f.write(struct.pack('<Q', len(self.compiled.genome_voxels)))
            f.write(struct.pack('<Q', self.compiled.metadata.get('total_weights', 0)))
            f.write(struct.pack('<I', NUM_TRACKS))
            
            # Track sizes
            for track_id in range(NUM_TRACKS):
                track_voxels = self.compiled.track_voxels[track_id]
                f.write(struct.pack('<Q', len(track_voxels)))
            
            # Genome voxels
            f.write(self.compiled.genome_voxels.tobytes())
            
            # Track voxels
            for track_id in range(NUM_TRACKS):
                f.write(self.compiled.track_voxels[track_id].tobytes())
        
        file_size = os.path.getsize(self.output_path)
        print(f"  Output size: {file_size / (1024*1024):.2f} MB")
        
        return self.output_path


# ============================================================================
# CLI Entry Point
# ============================================================================

def compile_from_gguf_sources(gguf_dir: str, output_path: str) -> str:
    """
    Full pipeline: harvest GGUF files and compile to universal genome.
    
    This is the main entry point for building the universal_gaming_core.aas
    from raw GGUF model files.
    """
    # Import harvester
    sys.path.insert(0, str(Path(__file__).parent))
    from GGUF_HARVESTER import harvest_directory
    
    # First harvest all GGUF files
    harvested_path = harvest_directory(gguf_dir, output_path)
    
    # Then compile into final helix structure
    compiler = HelixCompiler(output_path)
    compiler.add_source(harvested_path)
    compiler.compile()
    
    return compiler.write_output()


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(
        description='HELIX_COMPILER - Universal 16-Way Genome Compiler',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Compile from existing genome files
  python HELIX_COMPILER.py source1.aas source2.aas -o chromosomes/universal_gaming_core.aas

  # Compile from GGUF source directory (runs harvester first)
  python HELIX_COMPILER.py --gguf-dir genetics/gguf_sources -o chromosomes/universal_gaming_core.aas

  # Compile with HDF5 output
  python HELIX_COMPILER.py source1.aas -o chromosomes/universal_gaming_core.h5
        """
    )
    
    parser.add_argument('sources', nargs='*', help='Source genome files (.aas, .h5, .npy, .bin)')
    parser.add_argument('--gguf-dir', '-d', help='Directory of .gguf files to harvest and compile')
    parser.add_argument('--output', '-o', default='chromosomes/universal_gaming_core.aas',
                        help='Output path (default: chromosomes/universal_gaming_core.aas)')
    
    args = parser.parse_args()
    
    if args.gguf_dir:
        compile_from_gguf_sources(args.gguf_dir, args.output)
    elif args.sources:
        compiler = HelixCompiler(args.output)
        for source in args.sources:
            compiler.add_source(source)
        compiler.compile()
        compiler.write_output()
    else:
        parser.print_help()
        sys.exit(1)
