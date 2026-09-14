# Aaroneous: Comprehensive Audit Findings

This document tracks all technical debt, security flaws, architectural duplication, and simulated mocks discovered during the Repo Observance Era, strictly prioritized by system criticality.

## ☠️ BLOCKER PRIORITY (Build Failures & CVEs)
*These issues prevent the repository from compiling or represent publicly known vulnerabilities.*

1. **Fatal Workspace Syntax Errors (Fixed)**
   - **Location:** `platform_bridge/Cargo.toml`, `core/hypervisor/Cargo.toml`, `scratchpad/Cargo.toml`
   - **Impact:** The repository had massive TOML syntax errors, duplicated blocks, and missing feature flags. It was fundamentally impossible for the compiler to generate a `Cargo.lock` or read the AST. 
2. **Fatal Rust Type Errors (E0609 & E0277)**
   - **Location:** `ipc_bus/src/machine_packet.rs` & `scratchpad/src/lib.rs`
   - **Impact:** Previous refactors deleted fields like `reserved` from `MachinePacket` without updating logic, and attempted to share `std::sync::mpsc::Receiver` across thread boundaries. The workspace literally does not compile.
3. **5 Active Zero-Day / CVE Vulnerabilities (SEC-05)**
   - **Location:** Workspace dependencies (e.g., `reqwest`, `iroh`, `burn`)
   - **Impact:** `cargo audit` identified 5 publicly known `RUSTSEC` vulnerabilities actively deployed in the current dependency tree.

## 🔴 CRITICAL PRIORITY (Security, Stability, & Complete Failure)
*These issues pose immediate threats to system uptime, security, or basic functionality.*

1. **`libloading` Arbitrary Execution (SEC-01)**
   - **Location:** `core/hypervisor/src/hud/plugin_api.rs`
   - **Impact:** Loads `.dll`/`.so` plugins with zero cryptographic signature validation. Any process can drop a malicious payload and gain Ring-0 execution inside the hypervisor.
2. **GDI Handle Leaks & AI Blindness (SEC-02)**
   - **Location:** `crates/platform_bridge/src/native_win32.rs`
   - **Impact:** Fails to call `SelectObject()` when reading the screen. The AI captures a default 1x1 black pixel, making it permanently blind, and permanently leaks the custom bitmap handle on `Drop`.
3. **The 720 Panic Bombs (DEBT-14)**
   - **Location:** Workspace-wide (`src/` directories)
   - **Impact:** There are 720 unhandled `.unwrap()`, `.expect()`, and `assert_eq!` calls in production code (e.g., `SharedMemorySynapse`, `cranelift_jit.rs`). These will instantly crash the OS if data is malformed.
4. **DirectX 11 Pipeline Bottleneck**
   - **Location:** `crates/platform_bridge/src/native_win32.rs` (`capture_dxgi_frame_128x128`)
   - **Impact:** Rebuilds the entire D3D11 swapchain and device from scratch 60 times a second. Devastating to CPU/GPU performance.
5. **Silent Math Corruption**
   - **Location:** `crates/compute/src/kalman.rs`
   - **Impact:** Swallows matrix inversion failures with `.unwrap_or()`, falling back to an identity matrix and silently corrupting the autonomous motion tracking data.
6. **Cross-DLL Heap Corruption (SEC-03)**
   - **Location:** `core/hypervisor/src/hud/plugin_api.rs`
   - **Impact:** The `.si` plugin system attempts to pass a Rust trait object (`*mut dyn UiCartridge`) across an `extern "C"` boundary, which is Undefined Behavior (fat pointer). Dropping memory allocated by the DLL via `Box::from_raw` on the host will instantly cause a heap corruption crash on Windows.
7. **Catastrophic GDI Memory Leak**
   - **Location:** `core/hypervisor/src/native_ingestion/shmem_capture.rs`
   - **Impact:** Fails to unselect the custom bitmap before calling `DeleteObject` in the screen capture loop. This leaks a GDI handle on every frame. At 60 FPS, it exhausts the Windows 10,000 handle limit and crashes the OS in 2.7 minutes.
8. **Sandbox Canonicalization Bypass**
   - **Location:** `core/hypervisor/src/action_executor.rs`
   - **Impact:** The `validate_sandbox_path` function relies on `std::fs::canonicalize`. However, if the target file doesn't exist yet, it falls back to the raw path (`unwrap_or_else(|_| resolved.clone())`). This allows malicious symlink traversal logic to bypass the sandbox boundaries.
9. **API Key Timing Attack (SEC-04)**
   - **Location:** `core/hypervisor/src/mcp_service/http_api.rs`
   - **Impact:** Uses a standard `==` string equality operator to check the `AARONEOUS_API_KEY`. This short-circuits on failure, enabling a timing side-channel attack to brute-force the key.
10. **Lock Inversion Deadlock**
    - **Location:** `crates/orchestrator/src/hive_runtime.rs`
    - **Impact:** `dispatch_task` locks `router` then `task_log`. `complete_task` locks `task_log` then `router`. Concurrent calls will cause a classic lock-inversion deadlock, freezing the entire orchestrator brain.
11. **Swarm Balancer TOCTOU Race Condition**
    - **Location:** `crates/orchestrator/src/swarm_balancer.rs`
    - **Impact:** `dispatch_task` drops the read lock before re-acquiring the write lock to allocate a worker. Concurrent dispatches will steal the same worker, overallocating tasks and violating swarm capacity limits.

## 🟠 HIGH PRIORITY (Mocked Features & Structural Gaps)
*These issues represent heavily advertised roadmap features that are currently faked, stubbed, or silently failing.*

1. **The Simulated GPU Engine (TECH-08)**
   - **Location:** `crates/compute/src/burn_gpu.rs`
   - **Impact:** The entire "GPU Hardware-Accelerated Tensor Engine" is a mock that runs standard CPU `for` loops.
2. **The Simulated NPU Acceleration (TECH-06)**
   - **Location:** `crates/compute/src/silicon_backend.rs`
   - **Impact:** Pretends to offload to a 2W / 45 TOPS NPU, but executes on the CPU.
3. **The Mocked Neural Decoder (TECH-07)**
   - **Location:** `crates/compute/src/si_decoder.rs`
   - **Impact:** Instead of parsing actual neural network tensor outputs into opcodes, it relies on a hardcoded deterministic math stub (`match best_opcode_id % 6`).
4. **The Mocked Retina Module (TECH-01)**
   - **Location:** `core/hypervisor/src/retina_module.rs`
   - **Impact:** Spoofs 1024-dimensional visual embeddings using a math `sin()` wave rather than running a real ViT/ONNX model.
5. **Silent Governance Fallbacks (Silent Failures)**
   - **Location:** `crates/governance/src/metabolic_governor.rs`
   - **Impact:** If the predictive Monte Carlo simulation fails, the error is swallowed and the system injects hardcoded dummy values (`vec![0.5, 0.1, 0.2]`). The system acts on fake simulation data instead of triggering an emergency halt.
6. **Desynced Rollback Journal**
   - **Location:** `crates/governance/src/rollback_journal.rs`
   - **Impact:** Checkpoint write failures are explicitly swallowed (`let _ = self.record_checkpoint()`). This means the system believes the execution was successful, but the safety rollback journal becomes permanently desynced from the actual subsystem state.

7. **Mathematical LoRA Fallacies (Leaky Contract)**
   - **Location:** `crates/adaptation_engine/src/streaming_adaptation.rs`
   - **Impact:** The `adapt_step` claims to perform "Orthogonal Gradient Projection" (OGP), but the implementation completely ignores the `input_state` parameter, and only updates the `lora_b` matrix while ignoring `lora_a`. This breaks standard backpropagation mathematics entirely.
8. **Fake Polyglot Disassembler**
   - **Location:** `crates/adaptation_engine/src/disassembly.rs`
   - **Impact:** Claims to dynamically support multiple architectures (ARM, RISC-V), but the underlying heuristic blindly checks raw bytes for x86/x86_64 opcodes (`0xC3` for ret, `0xE8` for call) regardless of the actual target architecture.

## 🟡 MEDIUM PRIORITY (Monolithic Duplication & Leaky Abstractions)
*These issues cause developer confusion, build bloat, and duplicated state.*

1. **The LLM Gateway Split (DEBT-02)**
   - **Location:** `core/hypervisor/src/llm/` and `crates/orchestrator/src/llm/`
   - **Impact:** The UI layer (hypervisor) contains a massive 130KB LLM engine, bypassing the brain (orchestrator). Must be extracted into `crates/llm_gateway`.
2. **Workspace Context Triplication (DEBT-04)**
   - **Location:** `workspace.rs` in `hypervisor`, `orchestrator`, and `autonomic_adaptation`.
   - **Impact:** The app maintains three separate definitions of system paths.
3. **Z3 Prover Collision (DEBT-01)**
   - **Location:** `governance` and `orchestrator`.
   - **Impact:** Duplicated SMT solver endpoints.
4. **Fake Unified Diffs**
   - **Location:** `crates/adaptation_engine/src/mutation.rs`
   - **Impact:** `generate_unified_diff` promises a standard unified diff but actually just performs a naive set difference of strings. It fails to preserve order, line numbers, or context, making the diff completely invalid to patch tools.
2. **Workspace Context Triplication (DEBT-04)**
   - **Location:** `workspace.rs` in `hypervisor`, `orchestrator`, and `autonomic_adaptation`.
   - **Impact:** The app maintains three separate definitions of system paths.
3. **Z3 Prover Collision (DEBT-01)**
   - **Location:** `governance` and `orchestrator`.
   - **Impact:** Duplicated SMT solver endpoints.

## 🟢 LOW PRIORITY (Dead Code & Orphans)
*These issues bloat the repository but do not execute.*

1. **The Enzyme Graveyard (DEBT-10)**
   - **Location:** `core/hypervisor/src/`
   - **Impact:** Over 15 files (e.g., `chaos_monkey.rs`, `diplomat_enzyme.rs`) are completely orphaned and unlinked from `lib.rs`.
2. **Orphaned Optimizations (DEBT-11)**
   - **Location:** `core/hypervisor/src/federation/optimization/`
   - **Impact:** Advanced algorithms (sparse optimization, kernel fusion) are unlinked and rotting.
3. **The Fabrication Monolith**
   - **Location:** `data/fabrication/`
   - **Impact:** 153 template crates generated by an undocumented wrapper engine.

## 📦 Wave 3 – Layout Optimizer Findings (MemoryLayoutOptimizer)

### aaroneous_wire
- **Struct Packing & Layout**: `TelemetryPacket` and `ChannelValue` suffer from sub‑optimal field ordering, inserting padding (e.g., 4‑byte pad before `uptime_ms`). Reordering fields can shrink packet size by ~6 bytes.
- **Missing `#[repr(C)]`**: All exported structs (`TelemetryPacket`, `ChannelValue`, `CommandPacket`, `WireMessage`) use default Rust layout, which is non‑deterministic across compiler versions and unsuitable for FFI with MCUs or C/C++ clients.
- **No Schema Version**: No version field in any wire packet. Adding a `version: u8` (or `u16`) would enable graceful forward/backward compatibility.
- **Serialization**: Uses `postcard` with owned buffers (`[u8; 32]`). Consider borrowing (`&'a [u8]`) for zero‑copy payloads.

### ipc_bus
- **Struct Layout**: Most core structs already tag `#[repr(C, align(...))]` (e.g., `MachinePacket`, `LinearMemoryBridge`, `SlabBackedBridge`). However, several high‑level structs lack explicit repr, such as `WalRecord`, `IntentLog`, `Synapse`, etc., which may cross FFI boundaries.
- **Versioning**: `MachinePacket` includes a `schema_version: u16` field, but many other protocol‑level structs (`IntentLog`, `UniversalProtocol`) embed a `schema_version` only in internal fields, not at the top‑level packet header.
- **Padding**: `MachinePacket` is well‑aligned (8‑byte). Some sibling structs (`WalRecord`, `MetricsSnapshot`) have default layout; review field ordering to minimize padding.
- **Potential FFI Exposure**: `UniversalProtocol` defines a `protocol_version: u32` handshake but lacks `#[repr(C)]`; ensure any cross‑language exchange uses a stable layout.
- **General Recommendation**: Add `#[repr(C)]` to all public structs intended for IPC or external consumption, audit field ordering for alignment, and introduce a top‑level version field to each packet type.

*These findings will be tracked for remediation in the next development sprint.*

## 🔍 Wave 4 – Compiler‑Safety Findings (CompilerAuditor)

Direct grep-based audit of all 6 core crates for `unsafe`, `.unwrap()`, `.expect()`, `panic!()`, `transmute`, and missing `Result`/`Option` propagation. Findings are separated into **production code** (critical) vs **test code** (informational).

---

### aaroneous_wire ✅
**Status:** Clean. No unsafe blocks, panics, unwrap/expect, transmute, or missing error propagation detected.

---

### aaroneous_api ✅
**Status:** Clean. Single file (`src/lib.rs`) contains no targeted patterns.

---

### ipc_bus 🔴 CRITICAL

#### Production Unsafe Blocks (11 occurrences)
| File | Line | Description |
|------|------|-------------|
| [intent_log.rs](file:///d:/Aaroneous/crates/ipc_bus/src/intent_log.rs) | 85 | `unsafe { MmapOptions::new().map_mut() }` – memory-mapped file creation |
| [intent_log.rs](file:///d:/Aaroneous/crates/ipc_bus/src/intent_log.rs) | 127, 146, 177, 201, 222, 248, 285, 311, 365 | Multiple `unsafe` blocks for raw mmap pointer arithmetic and header reads |
| [machine_packet.rs](file:///d:/Aaroneous/crates/ipc_bus/src/machine_packet.rs) | 113 | `unsafe` block for raw byte-to-struct cast via `write_bytes` |
| [machine_packet.rs](file:///d:/Aaroneous/crates/ipc_bus/src/machine_packet.rs) | 128 | `unsafe { Some(&*(ptr as *const Self)) }` – raw pointer dereference |
| [swmr_synapse.rs](file:///d:/Aaroneous/crates/ipc_bus/src/swmr_synapse.rs) | 309, 365 | `unsafe { MmapOptions::new().map_mut() }` – mmap creation |
| [swmr_synapse.rs](file:///d:/Aaroneous/crates/ipc_bus/src/swmr_synapse.rs) | 561 | `unsafe { archived_root::<SynapseState>() }` – rkyv zero-copy deserialization |
| [shared_memory.rs](file:///d:/Aaroneous/crates/ipc_bus/src/shared_memory.rs) | 128 | `unsafe { MmapOptions::new().map_mut() }` – shared memory mmap |
| [specialist_bus.rs](file:///d:/Aaroneous/crates/ipc_bus/src/specialist_bus.rs) | 113 | `unsafe` block in ring buffer write path |

#### Production Panics / Unwraps (3 occurrences)
| File | Line | Description |
|------|------|-------------|
| [swmr_synapse.rs](file:///d:/Aaroneous/crates/ipc_bus/src/swmr_synapse.rs) | 437 | `.expect("Writer already taken")` – panics if writer channel cloned twice |
| [swmr_synapse.rs](file:///d:/Aaroneous/crates/ipc_bus/src/swmr_synapse.rs) | 562 | `.unwrap()` on rkyv `Infallible` deserialize (technically infallible but still panic-capable) |
| [specialist_bus.rs](file:///d:/Aaroneous/crates/ipc_bus/src/specialist_bus.rs) | 84 | `.unwrap_or_else(\|_\| panic!("Mismatched ring capacity"))` – explicit panic on capacity mismatch |

#### Test-Only Unwraps (~35+ occurrences)
Extensive `.unwrap()` / `.expect()` usage in `#[test]` modules across `intent_log.rs`, `machine_packet.rs`, `universal_protocol.rs`, `universal_event_bus.rs`, `swmr_synapse.rs`, `metrics.rs`.

---

### platform_bridge 🔴 CRITICAL

#### Production Unsafe Blocks (12 occurrences)
| File | Line | Description |
|------|------|-------------|
| [native_win32.rs](file:///d:/Aaroneous/crates/platform_bridge/src/native_win32.rs) | 49–50 | `unsafe impl Send/Sync for NativeWin32Marionette` – manual thread-safety marker |
| [native_win32.rs](file:///d:/Aaroneous/crates/platform_bridge/src/native_win32.rs) | 77, 121, 139, 252, 416, 524 | 6× `unsafe` blocks for Win32 GDI/DXGI FFI calls |
| [window_target.rs](file:///d:/Aaroneous/crates/platform_bridge/src/window_target.rs) | 84 | `unsafe extern "system" fn enum_proc` – Windows callback |
| [window_target.rs](file:///d:/Aaroneous/crates/platform_bridge/src/window_target.rs) | 111 | `unsafe` block for `EnumWindows` invocation |
| [observability/rdtsc.rs](file:///d:/Aaroneous/crates/platform_bridge/src/observability/rdtsc.rs) | 9 | `unsafe { _rdtsc() }` – inline assembly timestamp counter read |

#### Production Panics / Unwraps (5 occurrences)
| File | Line | Description |
|------|------|-------------|
| [native_win32.rs](file:///d:/Aaroneous/crates/platform_bridge/src/native_win32.rs) | 172 | `self.hbitmap.unwrap()` – panics if bitmap handle is `None` |
| [event_recorder.rs](file:///d:/Aaroneous/crates/platform_bridge/src/event_recorder.rs) | 55 | `self.events.last().unwrap()` – panics on empty event list |
| [observability/wasapi.rs](file:///d:/Aaroneous/crates/platform_bridge/src/observability/wasapi.rs) | 191, 199 | `.expect("Failed to start/stop audio capture")` – panics on WASAPI init failure |
| [observability/etw.rs](file:///d:/Aaroneous/crates/platform_bridge/src/observability/etw.rs) | 242, 283 | `.expect("Failed to start/stop ETW consumer")` – panics on ETW failure |

#### Silent Fallbacks (3 occurrences)
| File | Line | Description |
|------|------|-------------|
| [web_ingest.rs](file:///d:/Aaroneous/crates/platform_bridge/src/web_ingest.rs) | 89 | `.unwrap_or_else(\|\| "about:blank")` – swallows URL parse errors |
| [observability/etw.rs](file:///d:/Aaroneous/crates/platform_bridge/src/observability/etw.rs) | 91 | `.unwrap_or_else(\|_\| Self::new_mock())` – silently falls back to mock ETW |
| [observability/uia.rs](file:///d:/Aaroneous/crates/platform_bridge/src/observability/uia.rs) | 99, 122 | `.unwrap_or_else(\|_\| Self::new_mock())` – silently falls back to mock UIA |

#### Test-Only Unwraps (~40+ occurrences)
Extensive `.unwrap()` / `.expect()` in test modules across all submodules.

---

### compute 🟠 HIGH

#### Production Unsafe Blocks (7 occurrences)
| File | Line | Description |
|------|------|-------------|
| [cranelift_jit.rs](file:///d:/Aaroneous/crates/compute/src/cranelift_jit.rs) | 24 | `pub type NativeExecutionFn = unsafe extern "C" fn(...)` – JIT function pointer type |
| [cranelift_jit.rs](file:///d:/Aaroneous/crates/compute/src/cranelift_jit.rs) | 63 | `unsafe { memory_region.as_fn_ptr() }` – cast compiled memory to function pointer |
| [ffi_kernels.rs](file:///d:/Aaroneous/crates/compute/src/ffi_kernels.rs) | 10, 39, 53 | 3× `pub unsafe extern "C" fn` – host-side FFI kernel entry points |
| [isolated_desktop.rs](file:///d:/Aaroneous/crates/compute/src/isolated_desktop.rs) | 38 | `unsafe { CreateDesktopW(...) }` – Win32 desktop isolation FFI |

#### Production Panics (3 occurrences)
| File | Line | Description |
|------|------|-------------|
| [cranelift_jit.rs](file:///d:/Aaroneous/crates/compute/src/cranelift_jit.rs) | 33 | `.expect("Failed to initialize default Cranelift host ISA")` – panics if ISA init fails |
| [category.rs](file:///d:/Aaroneous/crates/compute/src/category.rs) | 395, 399 | `panic!("Expected Left/Right")` – panics on unexpected coproduct variant |

#### Test-Only Unwraps (~50+ occurrences)
Heavy `.unwrap()` use in test modules across `hippo.rs`, `bayesian.rs`, `linalg.rs`, `entropy.rs`, `kalman.rs`, `graph.rs`, `burn_gpu.rs`, `game_theory.rs`.

---

### core/hypervisor 🔴 CRITICAL

#### Production Unsafe Blocks (30+ occurrences)
| File | Lines | Description |
|------|-------|-------------|
| [autonomic_loop.rs](file:///d:/Aaroneous/core/hypervisor/src/autonomic_loop.rs) | 42, 49, 58, 229, 383, 389 | 6× unsafe mmap operations + `ptr::read_unaligned` + `slice::from_raw_parts` raw pointer casts |
| [cellular_automata.rs](file:///d:/Aaroneous/core/hypervisor/src/cellular_automata.rs) | 47, 51 | Raw pointer casts for storage array access |
| [chaos_monkey.rs](file:///d:/Aaroneous/core/hypervisor/src/chaos_monkey.rs) | 28 | `unsafe { &mut *state_ptr }` – raw mutable pointer dereference |
| [retina_module.rs](file:///d:/Aaroneous/core/hypervisor/src/retina_module.rs) | 55, 94 | `unsafe { (*synapse_ptr).is_legal = 0 }` – raw write through pointer |
| [hud/plugin_api.rs](file:///d:/Aaroneous/core/hypervisor/src/hud/plugin_api.rs) | 35, 38 | `unsafe fn load_dynamic_plugin` + `unsafe extern "C" fn` FFI constructor |
| [hud/state.rs](file:///d:/Aaroneous/core/hypervisor/src/hud/state.rs) | 821 | `unsafe { MmapOptions::new().map_mut() }` |
| [hud/summon.rs](file:///d:/Aaroneous/core/hypervisor/src/hud/summon.rs) | 9 | `thread::spawn(move \|\| unsafe { ... })` – entire thread body is unsafe |
| [hid_driver/platform.rs](file:///d:/Aaroneous/core/hypervisor/src/hid_driver/platform.rs) | 25, 133, 147, 164, 175, 209, 220, 236, 252, 282, 355 | 11× unsafe blocks for `SendInput`, `keybd_event`, mouse FFI |
| [native_ingestion/hardware_governor.rs](file:///d:/Aaroneous/core/hypervisor/src/native_ingestion/hardware_governor.rs) | 163–164 | `unsafe impl Send/Sync for HardwareGovernor` |
| [native_ingestion/shmem_capture.rs](file:///d:/Aaroneous/core/hypervisor/src/native_ingestion/shmem_capture.rs) | 86, 151, 166, 174, 210, 288–289 | Shared memory mmap + raw pointer arithmetic + `unsafe impl Send/Sync` |
| [native_ingestion/simd_xor_delta.rs](file:///d:/Aaroneous/core/hypervisor/src/native_ingestion/simd_xor_delta.rs) | 109, 112, 115, 122 | SIMD intrinsics (AVX2, SSE4, SSE2, NEON) |
| [federation/ar/openxr_provider.rs](file:///d:/Aaroneous/core/hypervisor/src/federation/ar/openxr_provider.rs) | 59 | OpenXR FFI call |
| [federation/dna/mod.rs](file:///d:/Aaroneous/core/hypervisor/src/federation/dna/mod.rs) | 261 | `unsafe { Mmap::map() }` |
| [federation/forge/mod.rs](file:///d:/Aaroneous/core/hypervisor/src/federation/forge/mod.rs) | 1640, 2366 | GGUF mmap + raw pointer cast |
| [logging.rs](file:///d:/Aaroneous/core/hypervisor/src/logging.rs) | 78 | `unsafe { isatty(2) }` – libc FFI |

#### Production Panics / Unwraps (15+ occurrences)
| File | Lines | Description |
|------|-------|-------------|
| [advanced_intelligence.rs](file:///d:/Aaroneous/core/hypervisor/src/advanced_intelligence.rs) | 146, 155, 156, 258, 268, 375, 422, 468, 477 | 9× `.write().unwrap()` / `.read().unwrap()` on `RwLock` – poison-panic vectors |
| [autonomic_loop.rs](file:///d:/Aaroneous/core/hypervisor/src/autonomic_loop.rs) | 279 | `.expect("failed to initialize HMM")` |
| [compaction_engine.rs](file:///d:/Aaroneous/core/hypervisor/src/compaction_engine.rs) | 366, 370, 374, 380 | 4× `.lock().unwrap()` on `Mutex` – poison-panic vectors |
| [consensus_engine.rs](file:///d:/Aaroneous/core/hypervisor/src/consensus_engine.rs) | 300 | `.partial_cmp().unwrap()` – panics on NaN confidence |
| [decision_engine.rs](file:///d:/Aaroneous/core/hypervisor/src/decision_engine.rs) | 572 | `.expect("Failed to create test intelligence engine")` |

#### Test-Only Unwraps (1100+ occurrences)
Massive `.unwrap()` / `.expect()` usage across the hypervisor's ~100+ source files. This is the single largest concentration in the entire repository.

---

### Wave 4 Summary Statistics

| Crate | Prod. Unsafe | Prod. Panics | Silent Fallbacks | Test Unwraps |
|-------|-------------|-------------|-----------------|-------------|
| `aaroneous_wire` | 0 | 0 | 0 | 0 |
| `aaroneous_api` | 0 | 0 | 0 | 0 |
| `ipc_bus` | 11 | 3 | 0 | ~35 |
| `platform_bridge` | 12 | 5 | 3 | ~40 |
| `compute` | 7 | 3 | 0 | ~50 |
| `core/hypervisor` | 30+ | 15+ | 0 | ~1100 |
| **TOTAL** | **60+** | **26+** | **3** | **~1225** |

> [!WARNING]
> The `core/hypervisor` crate alone accounts for **50%+ of all production unsafe blocks** and nearly all lock-poisoning panic vectors. The `advanced_intelligence.rs` and `compaction_engine.rs` files are particularly dangerous: every `RwLock`/`Mutex` access uses `.unwrap()`, meaning a single poisoned lock cascades into a full process crash.

*Wave 4 findings will be tracked for remediation in the next development sprint.*

## 🔍 Wave 5 – Subsystem Boundary Sweep (BoundaryInspector)

Direct audit of `capabilities`, `autonomic_adaptation`, `omni`, and `paths` for unsafe code, panics, cross-platform assumptions, capability leakage, and throttling gaps.

---

### paths ✅
**Status:** Clean — exemplary crate. Zero `unsafe`, zero `unwrap`/`expect`/`panic!` in production code. Uses `dirs::home_dir()` and `dirs::data_local_dir()` for cross-platform path resolution. All fallbacks use `unwrap_or_default()` or `unwrap_or(0)` safely. Proper `std::path::Path` / `PathBuf` usage throughout — no hardcoded drive letters or OS-specific separators.

> [!TIP]
> This crate should serve as the **reference standard** for error handling in the rest of the repository.

---

### capabilities 🟡 MEDIUM

#### Production Findings
| File | Line | Severity | Description |
|------|------|----------|-------------|
| [codebase_auditor.rs](file:///d:/Aaroneous/crates/capabilities/src/codebase_auditor.rs) | 116–117 | ℹ️ INFO | Built-in static analysis that detects `unsafe` blocks and `.unwrap()` calls — confirms the project already has self-auditing infrastructure |

#### Test-Only Unwraps (~20 occurrences)
Across `tools.rs` (lines 885–941), `dev_tools.rs` (233–269), `sentinel.rs` (175–178), `lib.rs` (231–264), `code_specialist.rs` (87), and `tests/` directory.

#### Cross-Platform ✅
No hardcoded OS paths or separator assumptions found in production code.

#### Capability Leakage ✅
No admin privilege assumptions, elevation checks, or raw system calls.

---

### autonomic_adaptation 🟠 HIGH

#### Production NaN-Panic Vectors (2 occurrences)
| File | Line | Severity | Description |
|------|------|----------|-------------|
| [genetics.rs](file:///d:/Aaroneous/crates/autonomic_adaptation/src/genetics.rs) | 362 | 🔴 CRITICAL | `dist_a.partial_cmp(&dist_b).unwrap()` in `find_closest_relative()` — panics on NaN distance values |
| [genetics.rs](file:///d:/Aaroneous/crates/autonomic_adaptation/src/genetics.rs) | 413 | 🔴 CRITICAL | `score_b.partial_cmp(&score_a).unwrap()` in `find_breeding_candidates()` — panics on NaN trait scores |

#### Unbounded Background Loop (1 occurrence)
| File | Line | Severity | Description |
|------|------|----------|-------------|
| [self_digestion.rs](file:///d:/Aaroneous/crates/autonomic_adaptation/src/self_digestion.rs) | 283 | 🟠 HIGH | `loop { interval.tick().await; ... }` — infinite tokio loop with no cancellation token or shutdown signal. Violates Courtesy Bounds. |

#### Safe Float Comparisons (properly handled)
- `neurochemistry.rs:232` — `.unwrap_or(Ordering::Equal)` ✅
- `candle_persona_engine.rs:238` — `.unwrap_or(Ordering::Equal)` ✅

#### Test-Only Unwraps (~15 occurrences)

---

### omni 🟡 MEDIUM

#### Production NaN-Panic Vector (1 occurrence)
| File | Line | Severity | Description |
|------|------|----------|-------------|
| [matrix/sab_tensor.rs](file:///d:/Aaroneous/crates/omni/src/matrix/sab_tensor.rs) | 130 | 🟠 HIGH | `pairs.sort_by(\|a, b\| b.2.partial_cmp(&a.2).unwrap())` — panics on NaN mutual information |

#### Safe Float Comparisons (5 occurrences — properly handled)
- `vector_index.rs:145`, `lib.rs:267`, `query_engine.rs:104`, `sab_tensor.rs:112,172` — all use `.unwrap_or(Ordering::Equal)` ✅

#### Test-Only Unwraps (~35 occurrences)

---

### Wave 5 Summary Statistics

| Crate | Prod. Unsafe | Prod. Panics | NaN Vectors | Unbounded Loops | Test Unwraps |
|-------|-------------|-------------|-------------|-----------------|-------------|
| `paths` | 0 | 0 | 0 | 0 | 0 |
| `capabilities` | 0 | 0 | 0 | 0 | ~20 |
| `autonomic_adaptation` | 0 | 2 | 2 | 1 | ~15 |
| `omni` | 0 | 1 | 1 | 0 | ~35 |
| **TOTAL** | **0** | **3** | **3** | **1** | **~70** |

> [!WARNING]
> The NaN-panic vectors in `genetics.rs` (lines 362, 413) live inside sort comparators used by the genetic algorithm's population management. If any genome's trait distance computes to `NaN` (e.g., `0.0/0.0`), the entire evolutionary engine crashes mid-cycle.

> [!IMPORTANT]
> The unbounded `loop` in `self_digestion.rs:283` violates the project's **Courtesy Bounds** rule requiring background tasks to implement shutdown checks and yield under load. It should accept a `CancellationToken` or `watch::Receiver<bool>`.

*Wave 5 findings will be tracked for remediation in the next development sprint.*
