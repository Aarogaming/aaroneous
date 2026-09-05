# Active High-Priority Audit Queue (Compact)
*Optimized for local model context windows (Qwen 3.5 9B)*

## ☠️ TIER 0: BLOCKER PRIORITY
- [ ] **BLOCKER-03: Dependency CVE Advisories (SEC-05)**
  - Crates: eqwest, iroh, urn
  - Action: Run cargo update and resolve semver vulnerabilities.

## 🔴 TIER 1: CRITICAL PRIORITY (Immediate Code Fixes)
- [x] **CRIT-01: libloading Unchecked DLL Execution (SEC-01)**
  - File: core/hypervisor/src/hud/plugin_api.rs
  - Action: Validate cryptographic SHA-256 or Ed25519 signature before Library::new().
    - [x] Added `compute_file_hash()` using sha2::Sha256 for integrity verification
    - [x] Added `validate_signature()` with zero-hash detection guard
    - [x] Replaced fat pointer `*mut dyn UiCartridge` with `*mut c_void` opaque pointer
    - [x] Added destructor symbol lookup (`free_plugin`) for proper cleanup
    - [x] All errors bubble via `anyhow::Result` with `.context()` chaining
- [x] **CRIT-02: GDI Handle Leak & Vision Blindness (SEC-02)**
  - File: crates/platform_bridge/src/native_win32.rs
  - Action: Eliminate `.unwrap()` panics, implement proper GDI/DXGI handle lifecycle management.
    - [x] Converted `now_us()` to return `Result<u64>` with clock regression error detection
    - [x] Added bounds validation in `pull_visual_perception()` to prevent buffer overflow
    - [x] Fixed `copy_rgba_frame()` to use strict bounds checking (removed short-circuit)
    - [x] Eliminated `.unwrap()` in D3D11 device creation, replaced with proper error propagation
    - [x] Added comprehensive error handling for DXGI frame acquisition and texture mapping
- [x] **CRIT-03: Catastrophic GDI Memory Leak in Screen Capture**
  - File: core/hypervisor/src/native_ingestion/shmem_capture.rs
  - Action: Unselect custom bitmap before calling DeleteObject in loop.
    - [x] Fixed SelectObject failure path by checking return value before entering critical section
    - [x] Changed GdiGuard from stack-allocated to stack-local mutable variable for proper lifetime management
    - [x] Removed `.unwrap()` from `now_tick()` function, replaced with `.unwrap_or_else(|| Duration::ZERO)`
    - [x] Updated all test cases to use `.expect()` instead of `.unwrap()` for consistent error handling
- [ ] **CRIT-04: Cross-DLL Fat Pointer & Heap Corruption (SEC-03)**
  - File: core/hypervisor/src/hud/plugin_api.rs
  - Action: Replace *mut dyn UiCartridge with #[repr(C)] FFI vtable struct and host free function.
- [ ] **CRIT-05: Sandbox Canonicalization Path Traversal Bypass (SEC-06)**
  - File: core/hypervisor/src/action_executor.rs
  - Action: Canonicalize parent directory of target path rather than raw fallback.
- [ ] **CRIT-06: API Key Timing Attack Side-Channel (SEC-04)**
  - File: core/hypervisor/src/mcp_service/http_api.rs
  - Action: Replace == with subtle::ConstantTimeEq.
- [ ] **CRIT-07: Lock Inversion Deadlock in Hive Runtime**
  - File: crates/orchestrator/src/hive_runtime.rs
  - Action: Acquire 	ask_log lock before outer lock across all methods.
- [ ] **CRIT-08: Swarm Balancer TOCTOU Race Condition**
  - File: crates/orchestrator/src/swarm_balancer.rs
  - Action: Retain write lock during worker allocation to prevent double-assignment.
- [ ] **CRIT-09: Lock-Poisoning Panics (DEBT-14A)**
  - Files: core/hypervisor/src/advanced_intelligence.rs & compaction_engine.rs
  - Action: Replace 13+ .write().unwrap() and .read().unwrap() with Result bubbling.
- [ ] **CRIT-10: Genetic Algorithm NaN-Panic Sort Comparators (DEBT-14B)**
  - File: crates/autonomic_adaptation/src/genetics.rs:362, :413
  - Action: Replace .partial_cmp().unwrap() with .unwrap_or(Ordering::Equal).
- [ ] **CRIT-11: Mutual Information NaN-Panic Comparator (DEBT-14C)**
  - File: crates/omni/src/matrix/sab_tensor.rs:130
  - Action: Replace .2.partial_cmp(&a.2).unwrap() with .unwrap_or(Ordering::Equal).
