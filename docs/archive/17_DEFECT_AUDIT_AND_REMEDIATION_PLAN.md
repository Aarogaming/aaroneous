# 17: Aaroneous Defect Audit & Top 10 Engineering Remediation Plan

> **Scope:** Full-workspace forensic audit across all 12 sovereign crates, core hypervisor, and desktop automation runtime.  
> **Compiler & Test Baseline:** Pure Rust workspace with 1,509 tests passing.  
> **Status:** Active Tracking & Remediation.

---

## Executive Summary

A comprehensive architectural and forensic audit of the Aaroneous codebase was executed to identify high-risk bugs, memory safety hazards, resource leaks, data corruption vectors, and stubbed subsystem implementations. The top 10 findings have been cataloged below with severity classifications, affected components, and explicit technical remediation requirements.

---

## Top 10 Engineering Findings & Defects

### 1. Unchecked Pointer Alignment in Zero-Copy FFI Bridging
- **Severity:** 🔴 **Critical** (Undefined Behavior / Memory Safety Hazard)
- **Component:** `crates/nervous_system/src/nucleotide_packet.rs:103-108`
- **Root Cause:** `AlignedBitstreamPacket` specifies `#[repr(C, align(8))]`. In `AlignedBitstreamPacket::from_bytes(bytes: &[u8])`, the slice pointer is cast directly via `bytes.as_ptr() as *const Self` without validating 8-byte boundary alignment (`ptr as usize % 8 == 0`). In Rust, creating a reference to an unaligned pointer is undefined behavior (UB) that triggers CPU alignment faults or torn reads on ARM64 and WASM targets.
- **Remediation:** Enforce alignment checks before casting or utilize `std::ptr::read_unaligned`.

---

### 2. GDI Handle Leak in Native Win32 Desktop Emulator
- **Severity:** 🟠 **High** (OS Resource Exhaustion / Desktop Crash)
- **Component:** `crates/desktop_emulator/src/native_win32.rs:74-98`
- **Root Cause:** `NativeWin32Marionette::initialize` allocates Win32 GDI device contexts and bitmaps (`GetDC`, `CreateCompatibleDC`, `CreateCompatibleBitmap`) but lacks an `impl Drop` implementation. Windows enforces a hard per-process limit of 10,000 GDI handles. Re-instantiating emulator hosts in automation loops steadily leaks GDI handles until the OS window manager fails.
- **Remediation:** Implement `impl Drop for NativeWin32Marionette` to call `ReleaseDC`, `DeleteDC`, and `DeleteObject`.

---

### 3. Potential Thread Panic on Multi-Byte UTF-8 String Slicing
- **Severity:** 🟠 **High** (Runtime Panic / DoS Vulnerability)
- **Component:** `core/hypervisor/src/federation/p2p/mod.rs:16`, `crates/orchestrator/src/llm/providers/gguf.rs:56`
- **Root Cause:** `P2pNodeId::short()` and `truncate()` perform byte slicing (`&self.0[..12]` and `&s[..max_len]`). If inputs contain multi-byte UTF-8 sequences (e.g., emojis, international IDs, localized prompts) where the byte boundary falls within a multi-byte code point, Rust triggers an unrecoverable panic (`byte index is not a char boundary`).
- **Remediation:** Use `s.char_indices()` or `s.chars().take(n).collect::<String>()` for safe character-boundary slicing.

---

### 4. Shared Memory Synapse Residual Payload Leakage & Corruption
- **Severity:** 🟡 **Medium-High** (Data Corruption / State Desynchronization)
- **Component:** `core/hypervisor/bin/a_hud.rs:4566-4575` vs `core/hypervisor/bin/a_run.rs:475-477`
- **Root Cause:** In `a_hud.rs`, `inject_live_intent()` copies intent bytes to `mmap[32..32 + payload_len]` without clearing trailing memory. If a previous task intent was longer than a newly injected task intent, residual bytes from the old payload remain in the memory buffer, corrupting intent parsing in the execution engine.
- **Remediation:** Mirror `a_run.rs` by calling `mmap[32 + payload_len..4128].fill(0)` upon every write.

---

### 5. Subprocess Argument Concatenation & Cargo Production Dependency
- **Severity:** 🟡 **Medium-High** (Process Failure in Production Deployments)
- **Component:** `core/hypervisor/src/orchestration_daemon.rs:111-125`
- **Root Cause:** `ProcessLifecycleManager::spawn` aggregates arguments with `descriptor.args.join(" ")` into a single argument rather than calling `.args(&descriptor.args)`, corrupting arguments with embedded spaces or quotes. Additionally, executing `cargo run --bin` requires a full Cargo build toolchain in runtime environments.
- **Remediation:** Pass argument slices directly to `cmd.args(&descriptor.args)` and invoke target pre-compiled binaries directly.

---

### 6. Dead Routing Variant for Deprecated WASM Runtime
- **Severity:** 🟡 **Medium** (Functional Dead End)
- **Component:** `core/hypervisor/src/action_executor.rs:175-190`
- **Root Cause:** `ExecutableAction::SpawnWasm` remains an active variant in the decision engine and action dispatcher, but `spawn_wasm_enzyme()` returns a hardcoded error stub (`"WASM enzyme execution is not available (wasmtime removed)"`). Decisions routed through this path fail silently.
- **Remediation:** Replace `SpawnWasm` with native `.si` container execution or deprecate the variant.

---

### 7. Stubbed Offline Inference in Orchestrator GGUF Provider
- **Severity:** 🟡 **Medium** (Incomplete LLM Capability)
- **Component:** `crates/orchestrator/src/llm/providers/gguf.rs:36-45`
- **Root Cause:** `GgufProvider::chat_completion()` verifies model file existence on disk, but returns a formatted placeholder string without executing tensor operations or KV-cache sampling.
- **Remediation:** Connect `GgufProvider` to `compute::si_ssm` or `candle_persona_engine` for local model weights.

---

### 8. Unbounded Memory Growth in Transpiler Telemetry & Histograms
- **Severity:** 🟡 **Medium** (Memory Leak in Long-Running Daemons)
- **Component:** `crates/transpiler/src/polyglot.rs:38-71`, `crates/transpiler/src/polyglot.rs:84-117`
- **Root Cause:** `TelemetryBuffer::entries` and `MetricsCollector::histograms` continuously append items to unbounded `Vec` collections without capacity limits, eviction mechanisms, or ring buffers.
- **Remediation:** Replace unbounded vectors with fixed-size ring buffers (`VecDeque`) with capacity ceilings.

---

### 9. Divergent Synapse Path Fallbacks Across Crates
- **Severity:** 🟡 **Medium** (Inter-Process State Desync)
- **Component:** `crates/nervous_system/src/swmr_synapse.rs:234-246` vs `crates/paths/src/lib.rs:38-83`
- **Root Cause:** `swmr_synapse.rs` uses a POSIX fallback (`~/.local/share/aaroneous/synapse`) when `AARONEOUS_WORKSPACE` is unset, whereas `aaroneous_paths` uses Windows `%LOCALAPPDATA%\Aaroneous`. This causes the HUD, CLI, and daemons to write and read from disparate files on Windows when run outside a Git repo.
- **Remediation:** Standardize `swmr_synapse::resolve_synapse_path` to delegate directly to `aaroneous_paths::WorkspacePaths`.

---

### 10. Inactive SDK Synapse Bridge Stub
- **Severity:** 🟡 **Medium** (SDK Inoperability)
- **Component:** `sdk/rust/src/lib.rs:5-12`
- **Root Cause:** `SignalBridge` is a zero-sized struct with an empty `.connect()` method that prints to stdout but does not instantiate or bind to `nervous_system::SharedMemorySynapse`.
- **Remediation:** Wire `SignalBridge` to open and exchange packets with the active `SharedMemorySynapse`.

---

## Remediation Roadmap & Tracking Matrix

| ID | Issue Description | Crate / File | Target Milestone | Status |
|:---|:---|:---|:---|:---|
| **#1** | Enforce 8-byte pointer alignment in `AlignedBitstreamPacket` | `crates/nervous_system` | v0.3.3 | ⏳ Pending |
| **#2** | Implement `Drop` for GDI handles in `NativeWin32Marionette` | `crates/desktop_emulator` | v0.3.3 | ⏳ Pending |
| **#3** | UTF-8 char boundary safe truncation in P2P and GGUF | `core/hypervisor`, `orchestrator` | v0.3.3 | ⏳ Pending |
| **#4** | Zero-out trailing bytes in `a_hud.rs` synapse intent writes | `core/hypervisor` | v0.3.3 | ⏳ Pending |
| **#5** | Correct argument passing & binary execution in daemon | `core/hypervisor` | v0.3.3 | ⏳ Pending |
| **#6** | Remove or replace `SpawnWasm` with native `.si` execution | `core/hypervisor` | v0.3.4 | ⏳ Pending |
| **#7** | Connect `GgufProvider` to real candle inference engine | `crates/orchestrator` | v0.3.4 | ⏳ Pending |
| **#8** | Implement bounded capacity for `TelemetryBuffer` | `crates/transpiler` | v0.3.4 | ⏳ Pending |
| **#9** | Unify `swmr_synapse` path resolution with `aaroneous_paths` | `crates/nervous_system` | v0.3.4 | ⏳ Pending |
| **#10**| Implement real shared memory binding in Rust SDK | `sdk/rust` | v0.3.4 | ⏳ Pending |

