# Autonomous Action Threat Model & Security Specification

> **Status:** Active Specification & Hardening Roadmap  
> **Target Release:** v0.3.3+  
> **Scope:** Core Hypervisor, Federation HTTP API, IPC / Nervous System, Desktop Automation Engine, and Action Executor.

---

## 1. Security Philosophy
Aaroneous executes autonomous tasks (AST synthesis, file operations, hardware intercept, P2P networking). All autonomous actions are treated as **security boundaries** guarded by deterministic layers rather than unconstrained LLM agency. No agent action is permitted to execute outside explicit capability sandboxes.

---

## 2. Security Boundaries & Containment Guarantees

### A. Filesystem Sandbox Containment
- **Guaranteed Root**: All file operations (`ActionExecutor::execute_file_operation` and MCP tools `tool_read_code`, `tool_search_code`, `tool_list_files`) must strictly canonicalize paths against `WorkspacePaths::discover().root()`.
- **Traversal Prevention**: Relative components (`..`), symlink escapes, and UNC drive traversals outside the workspace root return structured `PermissionDenied` errors.
- **Audit Defect Fix (SEC-01)**: `ActionExecutor::execute_file_operation` must be patched to enforce `canonical_path.starts_with(&sandbox_root)` before performing `fs::write`, `fs::remove_file`, `fs::rename`, or `fs::copy`.

### B. Network & Interface Hardening
- **Local Bind Default**: Hypervisor and MCP servers bind to `127.0.0.1` by default.
- **CORS Lockdown (SEC-02)**: Eliminate wildcard/permissive CORS defaults. Restrict allowed origins strictly to `http://localhost:*` and `http://127.0.0.1:*` to block cross-origin browser drive-by attacks.
- **Mandatory Local Authentication (SEC-03)**: When `AARONEOUS_API_KEY` is not provided via environment variable, generate a secure random cryptographic token on startup and persist it to `%LOCALAPPDATA%\Aaroneous\.session_token`.
- **Constant-Time Verification (SEC-04)**: All bearer token authentication routines must utilize constant-time comparison (`subtle::ConstantTimeEq`) to eliminate timing side-channels.
- **P2P Wire Framing**: Live TCP packets use 4-byte length prefixes and type validation to prevent buffer overflows.

### C. Windows IPC & Memory-Mapped Synapse Hardening
- **Named Pipe DACL Restrictions (SEC-05)**: `AgentBus::create_server` must explicitly construct a Windows Security Descriptor restricting pipe access to the current user SID at matching integrity levels to prevent unprivileged local spoofing.
- **Synapse File Access Controls (SEC-06)**: Apply owner-only ACLs (`0600` on POSIX, single-owner SID on Windows) upon creating `.synapse` shared memory files.

### D. Desktop HID Automation Failsafes
- **Hardware Failsafe Hook (SEC-07)**: Implement a global physical breakout hook (`WH_KEYBOARD_LL` on Windows) that instantly disarms `NativeWin32Marionette` if the user moves the mouse into screen corners or presses an emergency escape combo (`Ctrl+Alt+Escape`).
- **GDI Lifecycle Control**: Prevent GDI handle table exhaustion via strict `Drop` cleanup of device contexts and bitmaps.

### E. Cryptography & Credential Integrity
- **Cryptographic Hash Standardization (SEC-08)**: Replace `std::collections::hash_map::DefaultHasher` in `system_integrity.rs` with standard SHA-256 / BLAKE3 for deterministic and tamper-proof verification.
- **Hashed Credential Storage (SEC-09)**: In `ApiKeyAuth`, store SHA-256 hashes of registered keys rather than plaintext credentials in memory.

### F. Execution & JIT Safety (Sentinel SVDD Guardrail)
- **Latent Manifold Safety**: Before any synthesized code or candidate action vector is dispatched, Sentinel evaluates its distance against the safe SVDD hypersphere ($R = 14.5$).
- **Orthogonal Snapping**: Unsafe candidate vectors exceeding the threshold radius are snapped orthogonally to the nearest safe boundary in $< 2\mu s$.

### G. AST Mutation & Shadow Sandbox Rollback
- **Shadow Sandboxes**: Fabricator and the Dream Engine execute speculative code patches in isolated temp sandboxes.
- **Rollback Guarantee**: If Bayesian posterior confidence $< 0.70$ or unit tests fail, the mutation is immediately rolled back without touching the live tree.

---

## 3. Threat Matrix & Remediation Tracking

| ID | Vulnerability / Threat Vector | Severity | Component | Mitigation | Status |
|---|---|---|---|---|---|
| **SEC-01** | Unchecked path traversal in `ActionExecutor` | 🔴 **Critical** | `core/hypervisor` | Canonicalize & enforce sandbox root | ⏳ Pending |
| **SEC-02** | Permissive CORS & default-open HTTP API | 🔴 **Critical** | `core/hypervisor` | Restrict CORS & auto-generate token | ⏳ Pending |
| **SEC-03** | Timing attack on bearer token comparison | 🟡 **Medium** | `core/hypervisor` | Constant-time string comparison | ⏳ Pending |
| **SEC-04** | Unrestricted Named Pipe DACL | 🟠 **High** | `crates/nervous_system` | Set Owner-Only SID Security Descriptor | ⏳ Pending |
| **SEC-05** | Missing physical HID automation failsafe | 🟠 **High** | `crates/desktop_emulator` | Add corner-mouse / emergency escape hook | ⏳ Pending |
| **SEC-06** | Insecure `DefaultHasher` for integrity gate | 🟡 **Medium** | `core/hypervisor` | Upgrade to SHA-256 / BLAKE3 | ⏳ Pending |
| **SEC-07** | Plaintext API key storage in memory | 🟡 **Medium** | `core/hypervisor` | Store SHA-256 hashed API keys | ⏳ Pending |
| **SEC-08** | DoS panics on multi-byte UTF-8 slices | 🟠 **High** | `hypervisor`, `orchestrator` | Safe character-boundary slicing | ⏳ Pending |
| **SEC-09** | Zero-copy FFI pointer alignment UB | 🔴 **Critical** | `crates/nervous_system` | Validate 8-byte pointer alignment | ⏳ Pending |

