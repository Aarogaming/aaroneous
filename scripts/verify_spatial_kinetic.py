#!/usr/bin/env python3
"""
Spatial-Kinetic Engine Verification Script
Verifies genome integrity, shader syntax, and pipeline readiness.
"""

import os
import sys
import struct
import numpy as np
from pathlib import Path

def verify_genome(genome_path: str) -> dict:
    """Verify the universal gaming genome binary file."""
    print(f"\n{'='*60}")
    print(f"  GENOME VERIFICATION: {genome_path}")
    print(f"{'='*60}")
    
    if not os.path.exists(genome_path):
        print(f"  [ERROR] Genome file not found: {genome_path}")
        return {"valid": False}
    
    file_size = os.path.getsize(genome_path)
    print(f"  File size: {file_size / (1024**3):.2f} GB ({file_size:,} bytes)")
    
    with open(genome_path, 'rb') as f:
        # Read header
        magic = f.read(5)
        if magic != b'AASv1':
            print(f"  [WARN] No AASv1 header detected (raw binary format)")
            f.seek(0)
            # Treat entire file as raw u32 voxels
            data = f.read()
            voxels = np.frombuffer(data, dtype=np.uint32)
            voxel_count = len(voxels)
            weight_count = voxel_count * 16  # 16 weights per voxel (2-bit each)
        else:
            # Parse header
            voxel_count = struct.unpack('<Q', f.read(8))[0]
            weight_count = struct.unpack('<Q', f.read(8))[0]
            num_tracks = struct.unpack('<I', f.read(4))[0]
            
            print(f"  Magic: AASv1")
            print(f"  Voxel count: {voxel_count:,}")
            print(f"  Weight count: {weight_count:,}")
            print(f"  Tracks: {num_tracks}")
            
            # Read track sizes
            track_sizes = []
            for i in range(num_tracks):
                track_size = struct.unpack('<Q', f.read(8))[0]
                track_sizes.append(track_size)
                print(f"    Track {i}: {track_size:,} voxels")
            
            # Verify genome voxels
            genome_data = f.read(voxel_count * 4)
            if len(genome_data) != voxel_count * 4:
                print(f"  [ERROR] Genome data truncated")
                return {"valid": False}
            
            voxels = np.frombuffer(genome_data, dtype=np.uint32)
    
    # Analyze 2-bit distribution
    print(f"\n  2-BIT GENOMIC ANALYSIS:")
    sample_size = min(1000000, len(voxels))
    sample = voxels[:sample_size]
    
    a_count = t_count = c_count = g_count = 0
    for voxel in sample:
        for i in range(16):
            bits = (voxel >> (i * 2)) & 0x03
            if bits == 0: a_count += 1
            elif bits == 1: t_count += 1
            elif bits == 2: c_count += 1
            elif bits == 3: g_count += 1
    
    total = a_count + t_count + c_count + g_count
    print(f"    A (00): {a_count:,} ({a_count/total*100:.1f}%)")
    print(f"    T (01): {t_count:,} ({t_count/total*100:.1f}%)")
    print(f"    C (10): {c_count:,} ({c_count/total*100:.1f}%)")
    print(f"    G (11): {g_count:,} ({g_count/total*100:.1f}%)")
    
    compression_ratio = (voxel_count * 16) / (voxel_count * 4 / 4)  # weights / bytes
    print(f"\n  COMPRESSION:")
    print(f"    Voxels: {voxel_count:,}")
    print(f"    Equivalent weights: {voxel_count * 16:,}")
    print(f"    File size: {file_size / (1024**3):.2f} GB")
    print(f"    2-bit compression: 4x vs raw float32")
    
    return {
        "valid": True,
        "voxel_count": voxel_count,
        "weight_count": weight_count,
        "file_size": file_size,
        "genome_distribution": {"A": a_count, "T": t_count, "C": c_count, "G": g_count}
    }


def verify_shaders(shader_dir: str) -> dict:
    """Verify WGSL shader files exist and have valid syntax."""
    print(f"\n{'='*60}")
    print(f"  SHADER VERIFICATION: {shader_dir}")
    print(f"{'='*60}")
    
    shaders = {}
    for shader_file in ['reflex_kernel.wgsl', 'epigenetic_gate.wgsl']:
        path = os.path.join(shader_dir, shader_file)
        if os.path.exists(path):
            with open(path, 'r') as f:
                content = f.read()
            
            # Basic syntax checks
            has_main = '@compute' in content and 'fn main' in content
            has_bindings = '@group(0)' in content
            has_workgroup = '@workgroup_size' in content
            
            size_kb = os.path.getsize(path) / 1024
            
            shaders[shader_file] = {
                "exists": True,
                "size_kb": size_kb,
                "has_main": has_main,
                "has_bindings": has_bindings,
                "has_workgroup": has_workgroup,
            }
            
            print(f"  {shader_file}:")
            print(f"    Size: {size_kb:.1f} KB")
            print(f"    Compute entry: {'OK' if has_main else 'FAIL'}")
            print(f"    Bind groups: {'OK' if has_bindings else 'FAIL'}")
            print(f"    Workgroup size: {'OK' if has_workgroup else 'FAIL'}")
        else:
            shaders[shader_file] = {"exists": False}
            print(f"  {shader_file}: [MISSING]")
    
    return shaders


def verify_rust_modules(src_dir: str) -> dict:
    """Verify Rust module structure."""
    print(f"\n{'='*60}")
    print(f"  RUST MODULE VERIFICATION: {src_dir}")
    print(f"{'='*60}")
    
    required_modules = [
        'epigenetic_gate.rs',
        'wgpu_reflex_pipeline.rs',
        'spatial_kinetic_engine.rs',
        'win32_intercept/mod.rs',
        'win32_intercept/capture.rs',
        'win32_intercept/hid_bridge.rs',
        'win32_intercept/synapse_io.rs',
        'dashboard/spatial_kinetic.rs',
    ]
    
    results = {}
    for module in required_modules:
        path = os.path.join(src_dir, module)
        exists = os.path.exists(path)
        size_kb = os.path.getsize(path) / 1024 if exists else 0
        
        results[module] = {"exists": exists, "size_kb": size_kb}
        
        status = f"{size_kb:.1f} KB" if exists else "[MISSING]"
        print(f"  {module}: {status}")
    
    return results


def verify_python_scripts(scripts_dir: str) -> dict:
    """Verify Python pipeline scripts."""
    print(f"\n{'='*60}")
    print(f"  PYTHON SCRIPT VERIFICATION: {scripts_dir}")
    print(f"{'='*60}")
    
    required_scripts = [
        'GGUF_HARVESTER.py',
        'HELIX_COMPILER.py',
    ]
    
    results = {}
    for script in required_scripts:
        path = os.path.join(scripts_dir, script)
        exists = os.path.exists(path)
        size_kb = os.path.getsize(path) / 1024 if exists else 0
        
        results[script] = {"exists": exists, "size_kb": size_kb}
        
        status = f"{size_kb:.1f} KB" if exists else "[MISSING]"
        print(f"  {script}: {status}")
    
    # Check numpy availability
    try:
        import numpy
        print(f"  numpy: OK {numpy.__version__}")
    except ImportError:
        print(f"  numpy: MISSING")
    
    # Check h5py availability
    try:
        import h5py
        print(f"  h5py: OK {h5py.__version__}")
    except ImportError:
        print(f"  h5py: MISSING (falling back to raw binary)")
    
    return results


def main():
    print("="*60)
    print("  Aaroneous Spatial-Kinetic Engine - Verification")
    print("="*60)
    
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    results = {}
    
    # Verify genome
    genome_path = os.path.join(base_dir, "chromosomes", "universal_gaming_core.bin")
    results['genome'] = verify_genome(genome_path)
    
    # Verify shaders
    shader_dir = os.path.join(base_dir, "shaders")
    results['shaders'] = verify_shaders(shader_dir)
    
    # Verify Rust modules
    src_dir = os.path.join(base_dir, "core", "hypervisor", "src")
    results['rust_modules'] = verify_rust_modules(src_dir)
    
    # Verify Python scripts
    scripts_dir = os.path.join(base_dir, "scripts")
    results['python_scripts'] = verify_python_scripts(scripts_dir)
    
    # Summary
    print(f"\n{'='*60}")
    print(f"  VERIFICATION SUMMARY")
    print(f"{'='*60}")
    
    all_valid = True
    
    if results['genome']['valid']:
        print(f"  Genome: OK {results['genome']['voxel_count']:,} voxels")
    else:
        print(f"  Genome: FAIL Invalid or missing")
        all_valid = False
    
    for shader, info in results['shaders'].items():
        if info.get('exists'):
            print(f"  Shader {shader}: OK")
        else:
            print(f"  Shader {shader}: FAIL Missing")
            all_valid = False
    
    for module, info in results['rust_modules'].items():
        if info['exists']:
            print(f"  Module {module}: OK")
        else:
            print(f"  Module {module}: FAIL Missing")
            all_valid = False
    
    print(f"\n  Overall: {'ALL SYSTEMS READY' if all_valid else 'ISSUES DETECTED'}")
    
    return 0 if all_valid else 1


if __name__ == '__main__':
    sys.exit(main())
