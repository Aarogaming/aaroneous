# Stability Audit Report

## Risky Crates

| Crate | Version | Security Risk | Notes |
|-------|---------|---------------|-------|
| `wasmtime` | 18.0.0 | High | Used in WASM execution |
| `tokio` | 1.0.0 | Medium | Used in async runtime |
| `serde` | 1.0.0 | Low | Used for serialization |
| `anyhow` | 1.0.0 | Low | Error handling |

## Unsafe Code Gaps

| File | Line | Unsafe Block | Missing Documentation |
|------|------|--------------|----------------------|
| `core/hypervisor/src/native_ingestion/simd_xor_delta.rs` | 109 | `avx2_xor_bytecount` | Missing safety comment |
| `core/hypervisor/src/native_ingestion/simd_xor_delta.rs` | 112 | `sse4_xor_bytecount` | Missing safety comment |
| `core/hypervisor/src/native_ingestion/simd_xor_delta.rs` | 115 | `sse2_xor_bytecount` | Missing safety comment |
| `core/hypervisor/src/native_ingestion/simd_xor_delta.rs` | 122 | `neon_xor_bytecount` | Missing safety comment |
| `core/hypervisor/src/autonomic_loop.rs` | 34 | Memory mapping | Missing safety comment |
| `core/hypervisor/src/autonomic_loop.rs` | 43 | Memory mapping | Missing safety comment |
| `core/hypervisor/src/autonomic_loop.rs` | 191 | Memory mapping | Missing safety comment |
| `core/hypervisor/src/autonomic_loop.rs` | 256 | Memory mapping | Missing safety comment |
| `core/hypervisor/src/synapse.rs` | 110 | Memory mapping | Missing safety comment |
| `core/hypervisor/src/retina_module.rs` | 76 | Memory access | Missing safety comment |
| `core/hypervisor/src/substrate.rs` | 99 | Memory access | Missing safety comment |
| `core/hypervisor/src/wgpu_reflex_pipeline.rs` | 130 | Memory access | Missing safety comment |
| `core/hypervisor/src/win32_intercept/capture.rs` | 43 | Memory access | Missing safety comment |
| `core/hypervisor/src/win32_intercept/capture.rs` | 68 | Memory access | Missing safety comment |
| `core/hypervisor/src/win32_intercept/capture.rs` | 167 | Memory access | Missing safety comment |
| `core/hypervisor/src/shmem_capture.rs` | 85 | Memory mapping | Missing safety comment |
| `core/hypervisor/src/shmem_capture.rs` | 133 | Memory access | Missing safety comment |
| `core/hypervisor/src/shmem_capture.rs` | 151 | Memory access | Missing safety comment |
| `core/hypervisor/src/shmem_capture.rs` | 159 | Memory access | Missing safety comment |
| `core/hypervisor/src/shmem_capture.rs` | 186 | Memory access | Missing safety comment |
| `core/hypervisor/src/hardened_env.rs` | 21 | Security check | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 26 | FFI declaration | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 145 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 159 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 176 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 187 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 209 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 220 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 236 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 252 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 282 | Memory access | Missing safety comment |
| `core/hypervisor/src/hid_driver/platform.rs` | 349 | Memory access | Missing safety comment |