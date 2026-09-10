# 05: WASM Phase-Out & Deprecation Plan

## Strategic Decision: Elimination of WebAssembly (WASM)

A strategic decision has been made to **phase out WebAssembly (WASM), WIT (WASM Interface Type) bindings, and Wasmtime runtime virtualization** across the entire Aaroneous ecosystem.

---

## Why WASM is Being Phased Out

1. **Virtualization & Boundary Overhead**: Marshalling data (especially 128x128 float sensory frames and high-dimensional embeddings) across the WASM boundary introduces substantial memory copying, serialization latency, and CPU thrashing.
2. **Impedance Mismatch with Native Hardware**: Direct access to Win32 GDI/DirectX/Vulkan graphics devices, raw hardware peripherals, GPU reflex compute pipelines (WGPU/WGSL), and low-level shared memory (`memmap2`) requires awkward host-import bridges that negate WASM's sandbox benefits.
3. **Complexity of WIT Toolchains**: Managing `wasmtime`, `wit-bindgen`, and multi-target compilation added friction and fragility to the build pipeline without measurable real-world security gains in this single-node or federated architecture.
4. **Machine-Native Synthetic Intelligence**: Machine-native binaries (`.dll` on Windows, `.so` on Linux) executing directly on the host CPU/GPU provide zero-overhead execution, direct SIMD vectorization, and true microsecond response times.

---

## 🔍 WASM Artifacts & Modules Targeted for Removal

The following files, dependencies, and modules will be systematically deprecated, archived, and removed:

| Path / Dependency | Current Role | Migration Action |
| :--- | :--- | :--- |
| `core/hypervisor/Cargo.toml` (`wasmtime = "45.0"`, `wasmtime-wasi`) | Wasmtime engine dependencies | Remove dependencies from `Cargo.toml`. |
| `core/hypervisor/src/wasm_loader.rs` | Loads `.wasm` enzyme binaries | Replace with native dynamic library loader (`libloading`). |
| `core/hypervisor/src/wasm_splicer.rs` | Bytecode splicing for WASM modules | Replace with native AST / binary patching in Chimera. |
| `core/hypervisor/src/wasm_validator.rs` | WASM bytecode validation | Deprecate and remove. |
| `core/hypervisor/src/wasm_discovery.rs` | Scans workspace for `.wasm` files | Update to discover native `.sovereign` / `.dll` modules. |
| `core/hypervisor/src/wasm_ebus_bridge/` | Multi-file WASM event bus & WIT bridge | Deprecate and replace with native IPC / Linking Protocol. |
| `templates/universal_sab/` | WASM starter template crate | Replace with `templates/universal_native`. |
| `chromosomes/wasm_enzyme.wasm` | Pre-compiled WASM enzyme | Removed (WASM phaseout complete). |

---

## 📋 Phased Deprecation Schedule

```
Phase 1: Freeze WASM Development
  ├── Tag and document all WASM interfaces in dev/docs/
  └── Cease creation of new .wasm enzymes or WIT definitions

Phase 2: Introduce Native Module Trait Abstractions
  ├── Deploy libloading-based NativeEnzyme / SovereignModule traits
  └── Validate zero-copy shared memory access for native modules

Phase 3: Sever Wasmtime from Core Hypervisor
  ├── Remove wasmtime and wasmtime-wasi from core/hypervisor/Cargo.toml
  ├── Refactor WasmSplicingEngine to NativeSplicingEngine
  └── Remove core/hypervisor/src/wasm_* files

Phase 4: Purge WASM Templates & Binary Blobs
  ├── Remove templates/universal_sab/
  └── Purge leftover .wasm files from workspace
```
