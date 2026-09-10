# Governance Specification: Verification, Error Handling & Host Safety

**Version:** `v1.7.0`  
**Classification:** Core Governance Specification  

---

## 1. The Cratify Pipeline Lifecycle

All software modules incorporated into the Aaroneous workspace must pass through the automated 5-stage Cratify lifecycle before admission into production builds:

```
[ audit ] ──> [ scaffold ] ──> [ translate ] ──> [ verify ] ──> [ harvest ]
```

1. **Audit:** Inspects candidate code, dependencies, and external crates. Checks for `unsafe`, non-reproducible build steps, and non-`no_std` dependencies.
2. **Scaffold:** Generates a compliant Accelerated Component Container (`cratify.toml`) with strict execution tier and `max_blast_radius = "isolated"` settings.
3. **Translate:** Transpiles non-compliant legacy code or script algorithms into safe, idiomatic Rust.
4. **Verify:** Runs automated compiler checks, unit test suites, and SMT non-interference analysis.
5. **Harvest:** Links the crate into the root workspace and activates production build flags.

---

## 2. Accelerated Component Container (ACC) Schema (`cratify.toml`)

Every modular crate is defined by a strictly typed `cratify.toml` manifest:

```toml
[acc]
name = "transducer_acc"
version = "0.1.0"
description = "Machine-native state transducer"
license = "MIT"
max_blast_radius = "isolated" # Enforces sandboxed microkernel tier ("isolated" | "global")

[workspace]
root = "./"
include = ["benches/*"]

[dependencies]
# Dependencies are audited: crates without no_std are rejected unless allow_std = true
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

[tier]
level = "isolated" # "user" (in-process) | "isolated" (Job Object sandbox)
```

---

## 3. AST, DAG & Brand Seal Verification Gates

To prevent circular dependencies, resource starvation, and invalid state transitions:

### 3.1 Structural AST & DAG Fingerprinting
- Decoupled micro-crates and Accelerated Component Containers (ACCs) are not checked via heuristics or human intuition.
- AST mutations and dynamic plugins are parsed via `syn` to validate trait bounds, layout sizes, and memory boundaries prior to compilation.
- **Rules Enforced:** Zero `unsafe` blocks, zero raw pointer dereferencing, and mandatory derivation of `bytemuck::Pod` + `bytemuck::Zeroable` on public message structs.

### 3.2 SMT Non-Interference Verification
- Task execution graphs must form Directed Acyclic Graphs (DAGs).
- Pre-execution topological sorting verifies that concurrent branches do not share write-access to identical memory-mapped ranges.

### 3.3 Cryptographic Brand Seal Certification
For production ACC admission, a tamper-proof Brand Seal is generated:

$$\text{Seal} = \text{SHA-256}\left(\text{Hash}(\text{SourceAST}) \;\|\; \text{Hash}(\text{AuditReport}) \;\|\; 0\text{x}00 \;\|\; \text{Hash}(\text{Binary})\right)$$

- The seal is optionally signed via ECDSA-P256 and matched against the computed ABI layout hash.

---

## 4. Zero-Panic Error Policy

In real-time hypervisor execution, panics unwind stacks, invalidate hardware states, and destabilize host services. Panicking is strictly forbidden.

### 4.1 Strict Invariants
- **Banned Functions:** `.unwrap()`, `.expect()`, `panic!()`, and `unreachable!()` are strictly forbidden in production code.
- **Compiler Enforcement:** The workspace enforces `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.

### 4.2 Error Representation
- All fallible operations must return standard `core::result::Result<T, E>`.
- Domain-specific errors use `thiserror` for deterministic mapping:

```rust
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemError {
    #[error("ring buffer full: sequence={sequence}")]
    BufferOverflow { sequence: u64 },

    #[error("token authorization failed: capability={capability_id}")]
    UnauthorizedCapability { capability_id: u32 },

    #[error("thermal constraint exceeded: temp={current_celsius}C")]
    ThermalViolation { current_celsius: u32 },
}
```

---

## 5. Host Safety, Containment & Threat Model

Aaroneous treats all autonomous actions as explicit security boundaries:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Security Boundary Fences                        │
├────────────────────────────────────────────────────────────────────────┤
│  1. Filesystem Sandbox      │  `starts_with(&sandbox_root)` canonical  │
│  2. Windows IPC DACL        │  Single-owner current user SID on pipes  │
│  3. Physical HID Breakout   │  `WH_KEYBOARD_LL` (Corner / Esc combo)   │
│  4. Constant-Time Auth      │  `subtle::ConstantTimeEq` session tokens │
└────────────────────────────────────────────────────────────────────────┘
```

### 5.1 Filesystem Sandbox Containment
- All file operations must canonicalize paths against `WorkspacePaths::discover().root()`.
- The engine enforces `canonical_path.starts_with(&sandbox_root)` before executing writes, removals, or copies. UNC drive escapes and relative path transversals (`..`) return structured permission denials.

### 5.2 Windows IPC & Named Pipe Hardening
- Named pipes and shared memory regions enforce explicit Windows Security Descriptors restricting access strictly to the owner user SID.

### 5.3 Hardware HID Emergency Breakout
- Desktop automation engines maintain a low-level physical hook (`WH_KEYBOARD_LL`).
- Detection of the mouse pointer in screen corners or the emergency combo (`Ctrl+Alt+Escape`) immediately halts motor emulation and disarms input simulation.

### 5.4 Windows Job Objects & Sandbox Tiers
- Child worker processes run in Windows Job Objects with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
- Restricted tokens enforce hard memory caps and block write privileges to operating system roots.
