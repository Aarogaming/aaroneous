
### Batch Remediation Sweep [2026-09-05 17:25]

- **CRIT-01: libloading Unchecked DLL Execution (SEC-01)**
  - File: core/hypervisor/src/hud/plugin_api.rs
- Action: Validate cryptographic SHA-256 or Ed25519 signature before Library::new().
- [x] Added `compute_file_hash()` using sha2::Sha256 for integrity verification
- [x] Added `validate_signature()` with zero-hash detection guard
- [x] Replaced fat pointer `*mut dyn UiCartridge` with `*mut c_void` opaque pointer
- [x] Added destructor symbol lookup (`free_plugin`) for proper cleanup
- [x] All errors bubble via `anyhow::Result` with `.context()` chaining

- **CRIT-02: GDI Handle Leak & Vision Blindness (SEC-02)**
  - File: crates/platform_bridge/src/native_win32.rs
- Action: Eliminate `.unwrap()` panics, implement proper GDI/DXGI handle lifecycle management.
- [x] Converted `now_us()` to return `Result<u64>` with clock regression error detection
- [x] Added bounds validation in `pull_visual_perception()` to prevent buffer overflow
- [x] Fixed `copy_rgba_frame()` to use strict bounds checking (removed short-circuit)
- [x] Eliminated `.unwrap()` in D3D11 device creation, replaced with proper error propagation
- [x] Added comprehensive error handling for DXGI frame acquisition and texture mapping

- **CRIT-03: Catastrophic GDI Memory Leak in Screen Capture**
  - File: core/hypervisor/src/native_ingestion/shmem_capture.rs
- Action: Unselect custom bitmap before calling DeleteObject in loop.
- [x] Fixed SelectObject failure path by checking return value before entering critical section
- [x] Changed GdiGuard from stack-allocated to stack-local mutable variable for proper lifetime management
- [x] Removed `.unwrap()` from `now_tick()` function, replaced with `.unwrap_or_else(|| Duration::ZERO)`
- [x] Updated all test cases to use `.expect()` instead of `.unwrap()` for consistent error handling

### Batch Remediation Sweep [2026-09-05 18:07]

- **CRIT-01: libloading Unchecked DLL Execution (SEC-01)**
  - File: core/hypervisor/src/hud/plugin_api.rs
- Action: Validate cryptographic SHA-256 or Ed25519 signature before Library::new().
- [x] Added `compute_file_hash()` using sha2::Sha256 for integrity verification
- [x] Added `validate_signature()` with zero-hash detection guard
- [x] Replaced fat pointer `*mut dyn UiCartridge` with `*mut c_void` opaque pointer
- [x] Added destructor symbol lookup (`free_plugin`) for proper cleanup
- [x] All errors bubble via `anyhow::Result` with `.context()` chaining

- **CRIT-02: GDI Handle Leak & Vision Blindness (SEC-02)**
  - File: crates/platform_bridge/src/native_win32.rs
- Action: Eliminate `.unwrap()` panics, implement proper GDI/DXGI handle lifecycle management.
- [x] Converted `now_us()` to return `Result<u64>` with clock regression error detection
- [x] Added bounds validation in `pull_visual_perception()` to prevent buffer overflow
- [x] Fixed `copy_rgba_frame()` to use strict bounds checking (removed short-circuit)
- [x] Eliminated `.unwrap()` in D3D11 device creation, replaced with proper error propagation
- [x] Added comprehensive error handling for DXGI frame acquisition and texture mapping

- **CRIT-03: Catastrophic GDI Memory Leak in Screen Capture**
  - File: core/hypervisor/src/native_ingestion/shmem_capture.rs
- Action: Unselect custom bitmap before calling DeleteObject in loop.
- [x] Fixed SelectObject failure path by checking return value before entering critical section
- [x] Changed GdiGuard from stack-allocated to stack-local mutable variable for proper lifetime management
- [x] Removed `.unwrap()` from `now_tick()` function, replaced with `.unwrap_or_else(|| Duration::ZERO)`
- [x] Updated all test cases to use `.expect()` instead of `.unwrap()` for consistent error handling
