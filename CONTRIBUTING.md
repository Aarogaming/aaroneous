# Contributing to Aaroneous

Thank you for your interest in contributing to Aaroneous — a sovereign machine-native synthetic intelligence runtime.

## Getting Started

1. **Fork and clone** the repository
2. **Install Rust** (see `rust-toolchain.toml` for required version)
3. **Configure decoupled storage** (see below)
4. **Run the test suite**: `cargo test --workspace`
5. **Run clippy**: `cargo clippy --workspace -- -D warnings`

## Repository Decoupling & Storage Setup

To maintain a zero-clutter repository root and avoid storing compiled binaries or large neural model weights inside the git working tree, configure the following external paths:

### 1. Global Cargo Target Relocation
Relocate all Cargo build artifacts to an external cache directory outside the workspace (e.g., `C:\CargoTargetCache`). Add the following to `%USERPROFILE%\.cargo\config.toml` (Windows) or `~/.cargo/config.toml` (Linux/macOS):

```toml
[build]
target-dir = "C:\\CargoTargetCache"
```

Alternatively, set the environment variable in your shell profile:
```powershell
$env:CARGO_TARGET_DIR = "C:\CargoTargetCache"
```

### 2. External Data Root (`ARC_DATA_ROOT`)
Neural weights, `.si` model cartridges, state banks, and skill matrices are resolved by `crates/paths` from an external data root. The runtime checks paths in the following priority order:
1. **Typed Constructor Injection**: `WorkspacePathsConfig::new().with_external_data_root(path)`
2. **Environment Variable**: `ARC_DATA_ROOT` (e.g., `D:\ArcData`)
3. **Sibling Directory Fallback**: Sibling directory `ArcData` alongside the repository root (e.g., `D:\ArcData` for `D:\Aaroneous`)
4. **In-Tree Fallback**: `data/`

To set the environment variable on Windows:
```powershell
[System.Environment]::SetEnvironmentVariable("ARC_DATA_ROOT", "D:\ArcData", "User")
```

## Development Workflow

### The Systemic Loop

All engineering work, feature implementations, and agent workflows in Aaroneous adhere to a continuous, disciplined closed-loop cycle:

```
┌──────────────────────────────────────────────────────────────────┐
│                    THE AARONEOUS SYSTEMIC LOOP                   │
│                                                                  │
│  1. OBSERVE    ─► Telemetry, system state, profilers, user needs  │
│  2. HYPOTHESIZE─► Formulate expected behavior & performance root │
│  3. DESIGN     ─► Architecture, zero-copy contracts, API shapes  │
│  4. IMPLEMENT  ─► Rust code (safe, zero-copy, Pod, no-unwrap)    │
│  5. TEST       ─► Harness validation, clippy, unit & integration  │
│  6. DEPLOY     ─► Sandboxed cartridge activation / binary stage  │
│  7. MEASURE    ─► Microsecond RDTSC timings, thermal/VRAM impact │
│  8. LEARN      ─► Residual error capture, habit crystallization  │
│  9. REPEAT     ─► Next generational step with consolidated state │
└──────────────────────────────────────────────────────────────────┘
```

### Branch Naming

- `feat/short-description` — new features
- `fix/short-description` — bug fixes
- `refactor/short-description` — code restructuring
- `docs/short-description` — documentation changes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(compute): add SiForge builder API
fix(orchestrator): resolve MDP routing skill matching
docs(readme): update architecture diagram
refactor(chimera): extract AST mutation into separate module
```

### Code Style

- **Rust Edition 2024** across all workspace crates and tools
- **Zero clippy warnings**: `cargo clippy --workspace --all-targets -- -D warnings`
- **No `.unwrap()` or `.expect()` in production or hot-path code** — propagate structured errors via `Result<T, E>`
- **Domain-Aware Unsafe Code**: Standard crates must declare `#![deny(unsafe_code)]`. Performance/kernel crates (`compute`, `hypervisor`) may declare `#![warn(unsafe_code)]` provided every `unsafe` block includes documented `// SAFETY:` invariant comments.
- **Doc comments** on all public items: `///`, `//!`
- **`#[cfg(test)]`** modules for unit tests (using `tempfile::tempdir()` for filesystem isolation)

### Testing & Verification Gate Protocol

All PRs and agent commits must strictly satisfy the Sequential Verification Gate Protocol before submission:
1. `cargo check --workspace --all-targets`
2. `cargo test --workspace`
3. `cargo run -p ast_auditor -- audit core/ crates/` (MUST return 0 violations)
4. `! git grep -n -E "(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)" -- "crates/" "core/" "dev/"`
5. `cargo test -p emulator_harness`
6. `cargo run -p ast_auditor -- verify`

---

## Architectural Design Directives & Predecessor Provenance

Aaroneous synthesizes proven methodologies from high-performance systems, compilers, and real-time control architectures. Contributors and autonomous agents must adhere to the following design patterns:

### 1. Zero-Cost State Safety via the Typestate Pattern (Embedded-HAL Provenance)
Encode subsystem and asset lifecycles into generic type markers (`PhantomData<State>`) rather than relying solely on runtime boolean flags or runtime checks.
- Examples: `.si` Cartridges (`Cartridge<Raw>` $\to$ `Cartridge<Aligned>` $\to$ `Cartridge<SmtVerified>` $\to$ `Cartridge<Executable>`), asset onboarding (`AssimilationTask<Quarantined>` $\to$ `AssimilationTask<Auditing>` $\to$ `AssimilationTask<Committed>`).
- **Benefit**: Illegal lifecycle transitions (e.g., executing an unverified or misaligned buffer) fail at compile time with zero runtime branching penalty.

### 2. Cache-Dense Data Layouts (Mechanical Sympathy & Arrow/Bevy Provenance)
Organize high-frequency telemetry, agent state-spaces, and computational graphs into Structure-of-Arrays (SoA) contiguous memory blocks.
- Pack homogeneous primitives into 64-byte aligned contiguous arrays (e.g., `si_ir::DenseGraphStorage`).
- Prefer packed arrays with a 16-bit or 64-bit valid bitmask over sparse `[Option<T>; N]` arrays to eliminate enum discriminator padding and enable clean CPU L1 prefetching and SIMD autovectorization (AVX-512 / NEON).

### 3. Compile-Time Geometry & Schema Invariant Gates (Static Assertions Provenance)
Eliminate structural ABI drift across decoupled microkernel boundaries at compile time:
- In `core-contracts` and boundary crates, enforce exact struct geometry and alignment using `const { assert!(...) }` compile-time assertions.
- All IPC and shared memory structures must use `#[repr(C)]`, derive `bytemuck::Pod` and `bytemuck::Zeroable`, and declare explicit padding fields (`pub _pad0: u16`).

### 4. Fast, Memory-Safe Code Generation & Sandboxing (Cranelift/Wasmtime Provenance)
Use Cranelift native machine code generation (`compute::CraneliftJitEngine`) for fast-path runtime adaptations, dynamic opcode lowering, and micro-adaptation validation in W^X executable memory regions.
- High-frequency adaptation loops verify candidate logic in under 5 milliseconds in native sandboxes before initiating any static compiler (`rustc`) builds.

### 5. In-Process Hermetic Tooling & Zero Ambient Authority (Gitoxide & Redox Provenance)
Do not spawn external CLI processes (`Command::new("git")`, `Command::new("cargo")`) on hot paths or for non-modifying repository queries.
- Use pure-Rust, in-process, memory-mapped repository inspection (`gix` / Gitoxide) for status, diff, and tree inspection.
- Pass all filesystem roots, endpoints, and credentials via typed configuration structs (`paths::WorkspacePathsConfig`). Never query ambient host environment variables or temp folders.

### 6. Hard Real-Time Determinism & Floating-Point Safety (RTIC & Audio DSP Provenance)
- **Priority Ceiling Concurrency**: Critical safety loops (SMT interlocks, emergency stops) must preempt lower-priority tasks without priority inversion or lock contention.
- **Denormal Float Flushing (DAZ/FTZ)**: Set hardware CPU flags to flush denormals to zero in recursive State Space Model (SSM) computations and Kalman filtering, preventing 100x CPU degradation during floating-point decay.

### 7. Explicit Backpressure & Thermodynamic Rate Limiting (Tower & Firecracker Provenance)
Model capability broker dispatches and autonomous work loops with explicit backpressure and dual-token buckets (burst budget + continuous refill).
- When real-time hardware telemetry indicates high thermal load or VRAM saturation, work is shed or deferred non-blockingly without thread starvation.

---

## Pull Request Process

1. Update documentation if changing public API
2. Add entries to `CHANGELOG.md` under `[Unreleased]`
3. Ensure all CI and verification gates pass (`cargo run -p ast_auditor -- verify`)
4. Request review from a maintainer

## Security

Report security vulnerabilities privately via `SECURITY.md`. Do not open public issues for security bugs.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
