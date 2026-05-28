# Aaroneous — System Specification & Ingestion Pipeline

## Architecture Overview

A localized, deterministic, solid-state synthetic intelligence program in Rust + WASM.
Completely rejects autoregressive token inference, chat loops, and cloud dependencies.
Treats all inputs as static multi-dimensional geometric lookup maps.

### Core Principles
- **Zero-copy**: Raw pixel arrays stream through shared RAM blocks. SIMD XOR-delta isolates changes.
- **No hallucinations**: Actions locked within flat ECS arrays + FSM tables. 100% mathematical.
- **Architecture-agnostic distillation**: Digest features from GGUF models via SVD → 8192-bit VSA.

---

## Module Map

### `native_ingestion/` — Hardware Ingestion Substrate
| Module | Function | Status |
|--------|----------|--------|
| `shmem_capture` | Zero-copy framebuffer capture (Win32 GDI + DXGI) | ✅ implemented |
| `simd_xor_delta` | AVX2/SSE4/NEON/scalar XOR byte-level change detection | ✅ implemented |
| `svd_feature_select` | Randomized power-iteration truncated SVD | ✅ implemented |
| `fractional_normalizer` | Resolution-agnostic [0.0, 1.0] normalization | ✅ implemented |
| `hardware_governor` | sysinfo CPU/mem profiling → execution throttle | ✅ implemented |
| `fs_crawl` | Zero-allocation directory walker, SeaHash VSA per file | ✅ implemented |
| `chebyshev_trajectory` | Degree-2 polynomial least-squares mouse path fitting (6 coeffs) | ✅ implemented |
| `video_stream` | Frame-by-frame BGRA ingestion with SIMD delta + normalization | ✅ implemented |

### `substrate.rs` — Core Memory Structs
- `IngestionSourceType` — DesktopVideoRecord / ProgramDirectory / DocumentRawBytes
- `UnifiedLayerType` — AttentionQuery/Key/Value, FeedForwardUp
- `ScreenCoordinate` — f32 x,y in [0.0, 1.0]
- `SystemInstructionNode` — 64-byte-aligned HDF5 row (8192-bit VSA + metadata)
- `IngestionDataChunk` — Source-tracked data block with spatial VSA signature
- `NetworkDataStream` — SeaHash-based XOR VSA fusion for sandboxed telemetry
- `GgufTensorBlock` — Raw GGUF tensor data after binary seek (no model runtime)
- `seek_gguf_tensor_blocks()` — Pure file read, parses GGUF v3 headers + raw weights
- `classify_layer_name()` — Maps proprietary GGUF naming to `UnifiedLayerType`

### `sandboxed_network/` — Telemetry Network Link
- `SandboxedNetworkProcessor` — WASM-sandboxed HTTPS stream processing
- SeaHash per 64-byte chunk, popcount similarity gate, selective VSA fusion

### Upcoming
- `pipeline_distillation/` — Multi-model VSA space superposition
- GGUF forge integration (`federation/forge`) → SVD reduction → VSA projection
- HDF5 persistence layer for `SystemInstructionNode` and `IngestionDataChunk`

---

## Build & Test Status
- `cargo build -p a_run --bin a_run` — compiles clean
- `cargo test -p a_run --lib` — **655 pass, 0 fail, 3 ignored**

## Resource Constraints
- **Never** `cargo test --workspace` — OOMs (rocksdb, wasmtime, candle-core, wgpu, tokenizers)
- Always `cargo test -p <crate> --lib`
- `--jobs N` if single crate spikes memory
