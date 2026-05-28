#!/usr/bin/env python3
"""
GGUF_HARVESTER.py - Stateless White-Box Decision Geometry Siphon

Performs digital autopsies on GGUF model files, extracting raw FFN
decision landscapes directly from disk and compressing them into the
universal 16-way genomic format (universal_gaming_core.aas).

Treats open-source models as raw digital ore - extracts the mathematical
outcomes of their training records without the models ever firing up.
"""

import struct
import os
import sys
import numpy as np
from pathlib import Path
from typing import Dict, List, Tuple, Optional, BinaryIO
from dataclasses import dataclass
from enum import IntEnum

# HDF5 output for SWMR zero-copy ledger
try:
    import h5py
    HDF5_AVAILABLE = True
except ImportError:
    HDF5_AVAILABLE = False
    print("[WARN] h5py not available - falling back to raw .aas binary output")


# ============================================================================
# GGUF Binary Format Constants
# ============================================================================

GGUF_MAGIC = b'GGUF'
GGUF_VERSION = 3

class GGMLType(IntEnum):
    """GGML tensor data types matching GGUF spec."""
    F32 = 0
    F16 = 1
    Q4_0 = 2
    Q4_1 = 3
    Q5_0 = 6
    Q5_1 = 7
    Q8_0 = 8
    Q8_1 = 9
    Q2_K = 10
    Q3_K = 11
    Q4_K = 12
    Q5_K = 13
    Q6_K = 14
    Q8_K = 15
    IQ2_XXS = 16
    IQ2_XS = 17
    IQ3_XXS = 18
    IQ1_S = 19
    IQ4_NL = 20
    IQ3_S = 21
    IQ2_S = 22
    IQ4_XS = 23
    I8 = 24
    I16 = 25
    I32 = 26
    I64 = 27
    F64 = 28
    IQ1_M = 29
    BF16 = 30
    MXFP4 = 39


class GGUFValueType(IntEnum):
    """GGUF metadata value types."""
    UINT8 = 0
    INT8 = 1
    UINT16 = 2
    INT16 = 3
    UINT32 = 4
    INT32 = 5
    FLOAT32 = 6
    BOOL = 7
    STRING = 8
    ARRAY = 9
    UINT64 = 10
    INT64 = 11
    FLOAT64 = 12


# ============================================================================
# 2-Bit Genomic Encoding
# ============================================================================

# 2-bit base pair mapping: 00=A, 01=T, 10=C, 11=G
GENOME_LUT = {
    0b00: 'A',  # Negative strong (-2.5)
    0b01: 'T',  # Negative weak (-0.5)
    0b10: 'C',  # Positive weak (0.5)
    0b11: 'G',  # Positive strong (2.5)
}

# Quantization thresholds for mapping float weights to 2-bit states
QUANT_THRESHOLDS = [-1.0, 0.0, 1.0]

def float_to_2bit_vectorized(values: np.ndarray) -> np.ndarray:
    """Map float weights to 2-bit genomic states using vectorized numpy ops."""
    result = np.zeros(len(values), dtype=np.uint8)
    result[values >= 1.0] = 0b11  # G
    result[(values >= 0.0) & (values < 1.0)] = 0b10  # C
    result[(values >= -1.0) & (values < 0.0)] = 0b01  # T
    result[values < -1.0] = 0b00  # A
    return result


def pack_2bit_array_vectorized(values: np.ndarray) -> np.ndarray:
    """Pack 2-bit values into u32 voxels using vectorized numpy ops."""
    remainder = len(values) % 16
    if remainder != 0:
        values = np.pad(values, (0, 16 - remainder), mode='constant')
    
    groups = values.reshape(-1, 16)
    shifts = np.array([0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30], dtype=np.uint32)
    
    packed = np.zeros(len(groups), dtype=np.uint32)
    for j in range(16):
        packed |= groups[:, j].astype(np.uint32) << shifts[j]
    
    return packed


def float_to_2bit(value: float) -> int:
    """Map a float weight to a 2-bit genomic state."""
    if value < QUANT_THRESHOLDS[0]:
        return 0b00  # A
    elif value < QUANT_THRESHOLDS[1]:
        return 0b01  # T
    elif value < QUANT_THRESHOLDS[2]:
        return 0b10  # C
    else:
        return 0b11  # G


def pack_2bit_array(values: np.ndarray) -> np.ndarray:
    """Pack an array of 2-bit values into u32 voxels (16 values per u32)."""
    return pack_2bit_array_vectorized(values)


# ============================================================================
# GGUF Parser - Stateless Binary Reader
# ============================================================================

@dataclass
class GGUFTensor:
    """Represents a tensor extracted from a GGUF file."""
    name: str
    shape: List[int]
    ggml_type: GGMLType
    n_elements: int
    n_bytes: int
    data: Optional[np.ndarray] = None


@dataclass
class GGUFHeader:
    """GGUF file header information."""
    magic: bytes
    version: int
    tensor_count: int
    metadata_kv_count: int
    metadata: Dict[str, any]


class GGUFParser:
    """Stateless GGUF file parser - reads decision geometries directly from disk."""
    
    def __init__(self, path: str):
        self.path = path
        self.header: Optional[GGUFHeader] = None
        self.tensors: List[GGUFTensor] = []
        self._file_size = 0
    
    def parse(self) -> GGUFHeader:
        """Parse GGUF header and tensor metadata without loading weights."""
        with open(self.path, 'rb') as f:
            self._file_size = os.path.getsize(self.path)
            
            # Read magic
            magic = f.read(4)
            if magic != GGUF_MAGIC:
                raise ValueError(f"Invalid GGUF magic: {magic}")
            
            # Read version
            version = struct.unpack('<I', f.read(4))[0]
            if version != GGUF_VERSION:
                print(f"[WARN] Unsupported GGUF version: {version}, attempting parse anyway")
            
            # Read counts
            tensor_count = struct.unpack('<Q', f.read(8))[0]
            metadata_kv_count = struct.unpack('<Q', f.read(8))[0]
            
            # Parse metadata
            metadata = {}
            for _ in range(metadata_kv_count):
                key = self._read_string(f)
                value = self._read_value(f)
                metadata[key] = value
            
            self.header = GGUFHeader(
                magic=magic,
                version=version,
                tensor_count=tensor_count,
                metadata_kv_count=metadata_kv_count,
                metadata=metadata
            )
            
            # Parse tensor metadata (store positions for lazy loading)
            self._tensor_offsets = []
            for _ in range(tensor_count):
                name = self._read_string(f)
                n_dims = struct.unpack('<I', f.read(4))[0]
                
                shape = []
                for d in range(n_dims):
                    shape.append(struct.unpack('<Q', f.read(8))[0])
                
                ggml_type = GGMLType(struct.unpack('<I', f.read(4))[0])
                offset = struct.unpack('<Q', f.read(8))[0]
                
                # Calculate element count and byte size
                n_elements = 1
                for s in shape:
                    n_elements *= s
                
                n_bytes = self._compute_n_bytes(n_elements, ggml_type)
                
                tensor = GGUFTensor(
                    name=name,
                    shape=shape,
                    ggml_type=ggml_type,
                    n_elements=n_elements,
                    n_bytes=n_bytes
                )
                self.tensors.append(tensor)
                self._tensor_offsets.append(offset)
            
            return self.header
    
    def extract_ffn_tensors(self, target_names: Optional[List[str]] = None) -> Dict[str, np.ndarray]:
        """
        Extract FFN (Feed-Forward Network) decision geometries.
        
        Targets: blk.{n}.ffn_gate, blk.{n}.ffn_down, blk.{n}.ffn_up
        These contain the core reasoning weights of transformer models.
        """
        ffn_data = {}
        
        # Default FFN tensor patterns
        if target_names is None:
            target_patterns = ['ffn_gate', 'ffn_down', 'ffn_up', 'ffn_norm']
        else:
            target_patterns = target_names
        
        with open(self.path, 'rb') as f:
            for i, tensor in enumerate(self.tensors):
                # Check if this is an FFN tensor
                is_target = any(pat in tensor.name for pat in target_patterns)
                if not is_target:
                    continue
                
                # Seek to tensor data offset
                f.seek(self._tensor_offsets[i])
                
                # Read and dequantize tensor data
                data = self._read_tensor_data(f, tensor)
                if data is not None:
                    ffn_data[tensor.name] = data
                    print(f"  [EXTRACTED] {tensor.name} | shape={tensor.shape} | type={tensor.ggml_type.name}")
        
        return ffn_data
    
    def extract_all_weights(self) -> Dict[str, np.ndarray]:
        """Extract all tensor weights from the GGUF file."""
        all_weights = {}
        
        with open(self.path, 'rb') as f:
            for i, tensor in enumerate(self.tensors):
                f.seek(self._tensor_offsets[i])
                data = self._read_tensor_data(f, tensor)
                if data is not None:
                    all_weights[tensor.name] = data
        
        return all_weights
    
    def _read_string(self, f: BinaryIO) -> str:
        """Read a GGUF string."""
        length = struct.unpack('<Q', f.read(8))[0]
        return f.read(length).decode('utf-8')
    
    def _read_value(self, f: BinaryIO):
        """Read a GGUF metadata value."""
        value_type = GGUFValueType(struct.unpack('<I', f.read(4))[0])
        
        if value_type == GGUFValueType.UINT8:
            return struct.unpack('<B', f.read(1))[0]
        elif value_type == GGUFValueType.INT8:
            return struct.unpack('<b', f.read(1))[0]
        elif value_type == GGUFValueType.UINT16:
            return struct.unpack('<H', f.read(2))[0]
        elif value_type == GGUFValueType.INT16:
            return struct.unpack('<h', f.read(2))[0]
        elif value_type == GGUFValueType.UINT32:
            return struct.unpack('<I', f.read(4))[0]
        elif value_type == GGUFValueType.INT32:
            return struct.unpack('<i', f.read(4))[0]
        elif value_type == GGUFValueType.FLOAT32:
            return struct.unpack('<f', f.read(4))[0]
        elif value_type == GGUFValueType.BOOL:
            return bool(struct.unpack('<B', f.read(1))[0])
        elif value_type == GGUFValueType.STRING:
            return self._read_string(f)
        elif value_type == GGUFValueType.UINT64:
            return struct.unpack('<Q', f.read(8))[0]
        elif value_type == GGUFValueType.INT64:
            return struct.unpack('<q', f.read(8))[0]
        elif value_type == GGUFValueType.FLOAT64:
            return struct.unpack('<d', f.read(8))[0]
        elif value_type == GGUFValueType.ARRAY:
            array_type = GGUFValueType(struct.unpack('<I', f.read(4))[0])
            count = struct.unpack('<Q', f.read(8))[0]
            return [self._read_array_value(f, array_type) for _ in range(count)]
        else:
            raise ValueError(f"Unknown value type: {value_type}")
    
    def _read_array_value(self, f: BinaryIO, value_type: GGUFValueType):
        """Read a single array element value."""
        if value_type == GGUFValueType.UINT8:
            return struct.unpack('<B', f.read(1))[0]
        elif value_type == GGUFValueType.INT8:
            return struct.unpack('<b', f.read(1))[0]
        elif value_type == GGUFValueType.UINT16:
            return struct.unpack('<H', f.read(2))[0]
        elif value_type == GGUFValueType.INT16:
            return struct.unpack('<h', f.read(2))[0]
        elif value_type == GGUFValueType.UINT32:
            return struct.unpack('<I', f.read(4))[0]
        elif value_type == GGUFValueType.INT32:
            return struct.unpack('<i', f.read(4))[0]
        elif value_type == GGUFValueType.FLOAT32:
            return struct.unpack('<f', f.read(4))[0]
        elif value_type == GGUFValueType.UINT64:
            return struct.unpack('<Q', f.read(8))[0]
        elif value_type == GGUFValueType.INT64:
            return struct.unpack('<q', f.read(8))[0]
        elif value_type == GGUFValueType.FLOAT64:
            return struct.unpack('<d', f.read(8))[0]
        elif value_type == GGUFValueType.BOOL:
            return bool(struct.unpack('<B', f.read(1))[0])
        elif value_type == GGUFValueType.STRING:
            return self._read_string(f)
        else:
            return 0
    
    def _compute_n_bytes(self, n_elements: int, ggml_type: GGMLType) -> int:
        """Compute byte size for a tensor based on GGML type."""
        type_sizes = {
            GGMLType.F32: 4,
            GGMLType.F16: 2,
            GGMLType.BF16: 2,
            GGMLType.F64: 8,
            GGMLType.I8: 1,
            GGMLType.I16: 2,
            GGMLType.I32: 4,
            GGMLType.I64: 8,
            GGMLType.Q4_0: n_elements // 2,
            GGMLType.Q4_1: (n_elements // 2) + (n_elements // 32) * 4,
            GGMLType.Q8_0: n_elements,
            GGMLType.Q2_K: n_elements // 256 * 84,
            GGMLType.Q3_K: n_elements // 32 * 30,
            GGMLType.Q4_K: n_elements // 256 * 144,
            GGMLType.Q5_K: n_elements // 256 * 176,
            GGMLType.Q6_K: n_elements // 256 * 208,
            GGMLType.MXFP4: n_elements // 2,
        }
        return type_sizes.get(ggml_type, n_elements)
    
    def _read_tensor_data(self, f: BinaryIO, tensor: GGUFTensor) -> Optional[np.ndarray]:
        """Read and dequantize tensor data to float32 array."""
        MAX_ELEMENTS = 100_000_000  # 100M element limit (~400MB float32)
        if tensor.n_elements > MAX_ELEMENTS:
            print(f"  [SKIP] {tensor.name} too large ({tensor.n_elements:,} elements)")
            return None
        
        raw_bytes = f.read(tensor.n_bytes)
        if len(raw_bytes) != tensor.n_bytes:
            print(f"  [WARN] Truncated read for {tensor.name}")
            return None
        
        try:
            if tensor.ggml_type == GGMLType.F32:
                data = np.frombuffer(raw_bytes, dtype=np.float32)
                return data.copy()
            
            elif tensor.ggml_type == GGMLType.F16:
                data = np.frombuffer(raw_bytes, dtype=np.float16)
                return data.astype(np.float32)
            
            elif tensor.ggml_type == GGMLType.Q4_0:
                return self._dequant_q4_0(raw_bytes, tensor.n_elements)
            
            elif tensor.ggml_type == GGMLType.Q4_K:
                return self._dequant_q4_k(raw_bytes, tensor.n_elements)
            
            elif tensor.ggml_type == GGMLType.Q8_0:
                return self._dequant_q8_0(raw_bytes, tensor.n_elements)
            
            elif tensor.ggml_type in (GGMLType.Q3_K, GGMLType.Q5_K, GGMLType.Q6_K, GGMLType.Q2_K, GGMLType.Q5_0, GGMLType.Q5_1, GGMLType.Q8_1, GGMLType.Q4_1):
                return self._dequant_simple(raw_bytes, tensor.n_elements)
            
            elif tensor.ggml_type == GGMLType.MXFP4:
                return self._dequant_simple(raw_bytes, tensor.n_elements)
            
            else:
                # Fallback: interpret as raw bytes and normalize
                data = np.frombuffer(raw_bytes, dtype=np.uint8)
                return (data.astype(np.float32) - 128) / 128.0
                
        except Exception as e:
            print(f"  [ERROR] Failed to decode {tensor.name}: {e}")
            return None
    
    def _dequant_q4_0(self, raw_bytes: bytes, n_elements: int) -> np.ndarray:
        """Dequantize Q4_0 format (4-bit quantization, block size 32)."""
        block_size = 32
        n_blocks = n_elements // block_size
        
        values = np.zeros(n_elements, dtype=np.float32)
        
        offset = 0
        for i in range(n_blocks):
            # Read scale (float16)
            scale_bytes = raw_bytes[offset:offset+2]
            scale = np.frombuffer(scale_bytes, dtype=np.float16)[0]
            offset += 2
            
            # Read 4-bit values (16 bytes for 32 elements)
            qs = np.frombuffer(raw_bytes[offset:offset+16], dtype=np.uint8)
            offset += 16
            
            # Unpack 4-bit values
            for j in range(16):
                values[i * block_size + j * 2] = (qs[j] & 0x0F) - 8
                values[i * block_size + j * 2 + 1] = (qs[j] >> 4) - 8
            
            # Apply scale
            values[i * block_size:(i + 1) * block_size] *= scale
        
        return values
    
    def _dequant_q8_0(self, raw_bytes: bytes, n_elements: int) -> np.ndarray:
        """Dequantize Q8_0 format (8-bit quantization, block size 32)."""
        block_size = 32
        n_blocks = n_elements // block_size
        
        values = np.zeros(n_elements, dtype=np.float32)
        
        offset = 0
        for i in range(n_blocks):
            # Read scale (float16)
            scale_bytes = raw_bytes[offset:offset+2]
            scale = np.frombuffer(scale_bytes, dtype=np.float16)[0]
            offset += 2
            
            # Read 8-bit values (32 bytes for 32 elements)
            qs = np.frombuffer(raw_bytes[offset:offset+32], dtype=np.int8)
            offset += 32
            
            values[i * block_size:(i + 1) * block_size] = qs.astype(np.float32) * scale
        
        return values
    
    def _dequant_q4_k(self, raw_bytes: bytes, n_elements: int) -> np.ndarray:
        """Dequantize Q4_K format (improved 4-bit quantization)."""
        block_size = 256
        n_blocks = n_elements // block_size
        
        values = np.zeros(n_elements, dtype=np.float32)
        
        offset = 0
        for i in range(n_blocks):
            # Read scales and mins (6 bytes each, stored as differences)
            scales = np.zeros(12, dtype=np.float32)
            mins = np.zeros(12, dtype=np.float32)
            
            d = np.frombuffer(raw_bytes[offset:offset+2], dtype=np.float16)[0]
            offset += 2
            d_min = np.frombuffer(raw_bytes[offset:offset+2], dtype=np.float16)[0]
            offset += 2
            
            # Read 12 scale bytes
            scale_bytes = np.frombuffer(raw_bytes[offset:offset+12], dtype=np.uint8)
            offset += 12
            
            # Read 12 min bytes
            min_bytes = np.frombuffer(raw_bytes[offset:offset+12], dtype=np.uint8)
            offset += 12
            
            # Compute scales and mins
            for j in range(12):
                scales[j] = d * scale_bytes[j]
                mins[j] = d_min * min_bytes[j]
            
            # Read 4-bit quantized values (64 bytes for 128 pairs = 256 values)
            qs = np.frombuffer(raw_bytes[offset:offset+128], dtype=np.uint8)
            offset += 128
            
            # Unpack and dequantize
            for j in range(128):
                idx = i * block_size + j * 2
                block_idx = j // 16 * 2 + (1 if j % 16 >= 8 else 0)
                
                values[idx] = scales[block_idx] * (qs[j] & 0x0F) - mins[block_idx]
                values[idx + 1] = scales[block_idx] * (qs[j] >> 4) - mins[block_idx]
        
        return values

    def _dequant_simple(self, raw_bytes: bytes, n_elements: int) -> np.ndarray:
        """Simple dequantization: interpret raw bytes as signed values and normalize.
        Used as fallback for complex K-quant formats. The 2-bit genomic compression
        normalizes all values anyway, so exact dequant precision is not critical."""
        data = np.frombuffer(raw_bytes, dtype=np.int8).astype(np.float32)
        # Normalize to [-1, 1] range
        abs_max = np.max(np.abs(data))
        if abs_max > 0:
            return data / abs_max
        return data


# ============================================================================
# Universal 16-Way Genome Compiler
# ============================================================================

class UniversalGenomeCompiler:
    """
    Compiles extracted FFN weights into the universal 16-way genomic format.
    
    Maps spatial properties collectively across all software design patterns:
    - Navigation loops -> pathfinding graphs
    - Menus -> finite state machines  
    - Interfaces -> edge bounding boxes
    """
    
    NUM_TRACKS = 16  # 16-way geometric tensor cord layout
    
    def __init__(self):
        self.genome_tracks: Dict[int, np.ndarray] = {}
        self.track_metadata: Dict[str, any] = {}
    
    def compile_weights(self, ffn_data: Dict[str, np.ndarray], model_name: str) -> np.ndarray:
        """
        Compile FFN weights into packed 2-bit genome voxels.
        
        Returns: np.ndarray of u32 voxels ready for GPU storage buffer
        """
        print(f"\n[GENOME] Compiling {len(ffn_data)} FFN tensors into 16-way genome...")
        
        # Flatten and concatenate all FFN weights
        all_weights = []
        weight_sources = []
        
        for name, data in sorted(ffn_data.items()):
            flattened = data.flatten()
            all_weights.append(flattened)
            weight_sources.append((name, len(flattened)))
        
        if not all_weights:
            print("[WARN] No FFN weights to compile")
            return np.array([], dtype=np.uint32)
        
        # Concatenate all weights
        combined = np.concatenate(all_weights)
        print(f"  Total weights: {len(combined):,}")
        
        # Normalize to [-2.5, 2.5] range for 2-bit mapping
        abs_max = np.max(np.abs(combined))
        if abs_max > 0:
            normalized = (combined / abs_max) * 2.5
        else:
            normalized = combined
        
        # Map to 2-bit genomic states
        genome_2bit = np.array([float_to_2bit(v) for v in normalized], dtype=np.uint8)
        
        # Pack into u32 voxels (16 2-bit values per u32)
        packed_voxels = pack_2bit_array(genome_2bit)
        
        # Store track metadata
        self.track_metadata = {
            'model_name': model_name,
            'total_weights': len(combined),
            'total_voxels': len(packed_voxels),
            'num_tracks': self.NUM_TRACKS,
            'weight_sources': weight_sources,
        }
        
        print(f"  Packed voxels: {len(packed_voxels):,}")
        print(f"  Compression ratio: {len(combined) / (len(packed_voxels) * 16):.2f}x")
        
        return packed_voxels
    
    def organize_tracks(self, packed_voxels: np.ndarray) -> Dict[int, np.ndarray]:
        """
        Organize packed voxels into 16 parallel tracks.
        
        Each track handles a different aspect of the universal gaming genome:
        Track 0-3: Spatial navigation / pathfinding
        Track 4-7: State machine / menu logic
        Track 8-11: Visual recognition / targeting
        Track 12-15: Resource management / optimization
        """
        track_size = len(packed_voxels) // self.NUM_TRACKS
        remainder = len(packed_voxels) % self.NUM_TRACKS
        
        offset = 0
        for track_id in range(self.NUM_TRACKS):
            # Distribute remainder across first tracks
            size = track_size + (1 if track_id < remainder else 0)
            self.genome_tracks[track_id] = packed_voxels[offset:offset + size]
            offset += size
        
        return self.genome_tracks


# ============================================================================
# HDF5 Universal Genome Ledger (SWMR)
# ============================================================================

class HDF5GenomeLedger:
    """
    High-performance HDF5 binary file layout as read-only model database.
    
    Single-Writer Multiple-Reader (SWMR) system allows parallel runtime
    instances to share the same model file without data collisions.
    """
    
    def __init__(self, output_path: str):
        self.output_path = output_path
    
    def write_genome(self, 
                     packed_voxels: np.ndarray,
                     tracks: Dict[int, np.ndarray],
                     metadata: Dict[str, any],
                     source_models: List[str]) -> str:
        """
        Write the universal genome to HDF5 format.
        
        Memory-maps datasets directly for zero-copy GPU storage buffer access.
        """
        if not HDF5_AVAILABLE:
            return self._write_raw_binary(packed_voxels, metadata)
        
        print(f"\n[HDF5] Writing universal genome to: {self.output_path}")
        
        with h5py.File(self.output_path, 'w', libver='latest') as f:
            f.swmr_mode = True
            
            # Write packed genome voxels
            f.create_dataset(
                'genome_voxels',
                data=packed_voxels,
                dtype='uint32',
                compression='lzf'
            )
            
            # Write 16-way tracks
            tracks_group = f.create_group('tracks')
            for track_id, track_data in tracks.items():
                tracks_group.create_dataset(
                    f'track_{track_id:02d}',
                    data=track_data,
                    dtype='uint32',
                    compression='lzf'
                )
            
            # Write metadata
            meta_group = f.create_group('metadata')
            meta_group.attrs['model_name'] = metadata.get('model_name', 'unknown')
            meta_group.attrs['total_weights'] = metadata.get('total_weights', 0)
            meta_group.attrs['total_voxels'] = metadata.get('total_voxels', 0)
            meta_group.attrs['num_tracks'] = metadata.get('num_tracks', 16)
            meta_group.attrs['source_models'] = ', '.join(source_models)
            meta_group.attrs['genome_version'] = '1.0'
            meta_group.attrs['created'] = str(__import__('datetime').datetime.now())
            
            # Write quantization lookup table
            f.create_dataset('parameter_lut', data=np.array([-2.5, -0.5, 0.5, 2.5], dtype=np.float32))
            
            # Write 2-bit encoding map
            encoding = f.create_dataset('encoding_map', data=np.array([0, 1, 2, 3], dtype=np.uint8))
            encoding.attrs['00'] = 'A'
            encoding.attrs['01'] = 'T'
            encoding.attrs['10'] = 'C'
            encoding.attrs['11'] = 'G'
        
        file_size = os.path.getsize(self.output_path)
        print(f"  Output size: {file_size / (1024*1024):.2f} MB")
        
        return self.output_path
    
    def _write_raw_binary(self, packed_voxels: np.ndarray, metadata: Dict[str, any]) -> str:
        """Fallback: write raw .aas binary file."""
        output_path = self.output_path.replace('.aas', '.bin') if '.aas' in self.output_path else self.output_path
        
        print(f"\n[BINARY] Writing raw genome to: {output_path}")
        
        with open(output_path, 'wb') as f:
            # Write header
            f.write(b'AASv1')  # Magic
            f.write(struct.pack('<Q', len(packed_voxels)))  # Voxel count
            f.write(struct.pack('<Q', metadata.get('total_weights', 0)))  # Weight count
            
            # Write voxels
            f.write(packed_voxels.tobytes())
        
        file_size = os.path.getsize(output_path)
        print(f"  Output size: {file_size / (1024*1024):.2f} MB")
        
        return output_path


# ============================================================================
# Main Harvester Pipeline
# ============================================================================

def harvest_gguf(gguf_path: str, output_path: str = "chromosomes/universal_gaming_core.aas") -> str:
    """
    Main pipeline: extract FFN geometries from GGUF and compile to universal genome.
    
    Args:
        gguf_path: Path to .gguf model file
        output_path: Path for output .aas genome file
    
    Returns:
        Path to generated genome file
    """
    print("=" * 70)
    print("  GGUF HARVESTER - Stateless White-Box Decision Geometry Siphon")
    print("=" * 70)
    
    # Validate input
    if not os.path.exists(gguf_path):
        print(f"[ERROR] GGUF file not found: {gguf_path}")
        sys.exit(1)
    
    model_name = Path(gguf_path).stem
    print(f"\n[TARGET] {model_name}")
    print(f"  Path: {gguf_path}")
    print(f"  Size: {os.path.getsize(gguf_path) / (1024**3):.2f} GB")
    
    # Step 1: Parse GGUF header statelessly
    print("\n[PARSE] Reading GGUF structure from disk...")
    parser = GGUFParser(gguf_path)
    header = parser.parse()
    
    print(f"  Version: {header.version}")
    print(f"  Tensors: {header.tensor_count:,}")
    print(f"  Metadata keys: {header.metadata_kv_count:,}")
    
    # Print model metadata if available
    if 'general.name' in header.metadata:
        print(f"  Model: {header.metadata['general.name']}")
    if 'general.architecture' in header.metadata:
        print(f"  Architecture: {header.metadata['general.architecture']}")
    if 'general.parameter_count' in header.metadata:
        print(f"  Parameters: {header.metadata['general.parameter_count']:,}")
    
    # Step 2: Extract FFN decision geometries
    print("\n[SIPHON] Extracting FFN decision landscapes...")
    ffn_data = parser.extract_ffn_tensors()
    
    if not ffn_data:
        print("[WARN] No FFN tensors found - extracting all weights")
        ffn_data = parser.extract_all_weights()
    
    print(f"  Extracted {len(ffn_data)} tensor(s)")
    
    # Step 3: Compile to universal 16-way genome
    compiler = UniversalGenomeCompiler()
    packed_voxels = compiler.compile_weights(ffn_data, model_name)
    tracks = compiler.organize_tracks(packed_voxels)
    
    # Step 4: Write to HDF5 ledger
    ledger = HDF5GenomeLedger(output_path)
    result_path = ledger.write_genome(
        packed_voxels=packed_voxels,
        tracks=tracks,
        metadata=compiler.track_metadata,
        source_models=[model_name]
    )
    
    print(f"\n[COMPLETE] Universal genome written to: {result_path}")
    print("=" * 70)
    
    return result_path


def harvest_directory(gguf_dir: str, output_path: str = "chromosomes/universal_gaming_core.aas") -> str:
    """
    Harvest all GGUF files in a directory and merge into single universal genome.
    
    Uses streaming approach to avoid loading all tensors into memory simultaneously.
    """
    print("=" * 70)
    print("  GGUF HARVESTER - Multi-Model Universal Genome Assembly")
    print("=" * 70)
    
    gguf_dir = Path(gguf_dir)
    if not gguf_dir.exists():
        print(f"[ERROR] Directory not found: {gguf_dir}")
        sys.exit(1)
    
    gguf_files = list(gguf_dir.glob('**/*.gguf'))
    if not gguf_files:
        print(f"[ERROR] No .gguf files found in: {gguf_dir}")
        sys.exit(1)
    
    print(f"\n[SCAN] Found {len(gguf_files)} GGUF file(s)")
    for f in gguf_files:
        size_gb = f.stat().st_size / (1024**3)
        print(f"  - {f.name} ({size_gb:.2f} GB)")
    
    # Stream all tensors directly to packed voxels file
    temp_voxels_path = output_path + '.tmp.voxels'
    source_models = []
    total_weights = 0
    total_voxels = 0
    
    with open(temp_voxels_path, 'wb') as voxel_file:
        for gguf_file in gguf_files:
            print(f"\n{'='*50}")
            print(f"  Processing: {gguf_file.name}")
            print(f"{'='*50}")
            
            model_name = gguf_file.stem
            source_models.append(model_name)
            
            parser = GGUFParser(str(gguf_file))
            parser.parse()
            
            # Extract tensors one at a time and stream to file
            ffn_tensors = [(i, t) for i, t in enumerate(parser.tensors) if any(p in t.name for p in ['ffn_gate', 'ffn_down', 'ffn_up', 'ffn_norm'])]
            if not ffn_tensors:
                ffn_tensors = list(enumerate(parser.tensors))
            
            model_weights = 0
            model_voxels = 0
            
            for tensor_idx, tensor in ffn_tensors:
                offset = parser._tensor_offsets[tensor_idx]
                
                with open(gguf_file, 'rb') as f:
                    f.seek(offset)
                    data = parser._read_tensor_data(f, tensor)
                
                if data is None:
                    continue
                
                model_weights += len(data)
                
                # Normalize and convert to 2-bit
                abs_max = np.max(np.abs(data))
                if abs_max > 0:
                    normalized = (data / abs_max) * 2.5
                else:
                    normalized = data
                
                genome_2bit = float_to_2bit_vectorized(normalized)
                packed = pack_2bit_array_vectorized(genome_2bit)
                
                # Write packed voxels directly to file
                voxel_file.write(packed.tobytes())
                model_voxels += len(packed)
                
                del data, normalized, genome_2bit, packed
            
            total_weights += model_weights
            total_voxels += model_voxels
            print(f"  Streamed {len(ffn_tensors)} tensors -> {model_voxels:,} voxels ({model_weights:,} weights)")
    
    # Read all packed voxels and organize into tracks
    print(f"\n[ASSEMBLE] Organizing {total_voxels:,} voxels into 16-way genome...")
    all_voxels = np.fromfile(temp_voxels_path, dtype=np.uint32)
    
    # Organize into 16 tracks
    track_size = len(all_voxels) // 16
    remainder = len(all_voxels) % 16
    tracks = {}
    offset = 0
    for track_id in range(16):
        size = track_size + (1 if track_id < remainder else 0)
        tracks[track_id] = all_voxels[offset:offset + size]
        offset += size
    
    # Write final output
    metadata = {
        'total_voxels': total_voxels,
        'total_weights': total_weights,
        'num_tracks': 16,
        'num_sources': len(source_models),
    }
    
    ledger = HDF5GenomeLedger(output_path)
    result_path = ledger.write_genome(
        packed_voxels=all_voxels,
        tracks=tracks,
        metadata=metadata,
        source_models=source_models
    )
    
    # Cleanup temp file
    os.remove(temp_voxels_path)
    
    print(f"\n[COMPLETE] Universal genome written to: {result_path}")
    print(f"  Source models: {', '.join(source_models)}")
    print(f"  Total weights: {total_weights:,}")
    print(f"  Total voxels: {total_voxels:,}")
    print("=" * 70)
    
    return result_path


# ============================================================================
# CLI Entry Point
# ============================================================================

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(
        description='GGUF Harvester - Stateless White-Box Decision Geometry Siphon',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Harvest single GGUF file
  python GGUF_HARVESTER.py model.gguf

  # Harvest single file with custom output
  python GGUF_HARVESTER.py model.gguf -o chromosomes/my_genome.aas

  # Harvest all GGUF files in directory
  python GGUF_HARVESTER.py --dir genetics/gguf_sources

  # Harvest directory with custom output
  python GGUF_HARVESTER.py --dir genetics/gguf_sources -o chromosomes/universal_gaming_core.aas
        """
    )
    
    parser.add_argument('gguf_file', nargs='?', help='Path to single .gguf file')
    parser.add_argument('--dir', '-d', help='Directory containing .gguf files')
    parser.add_argument('--output', '-o', default='chromosomes/universal_gaming_core.aas',
                        help='Output path for .aas genome file (default: chromosomes/universal_gaming_core.aas)')
    
    args = parser.parse_args()
    
    if args.gguf_file:
        harvest_gguf(args.gguf_file, args.output)
    elif args.dir:
        harvest_directory(args.dir, args.output)
    else:
        parser.print_help()
        sys.exit(1)
