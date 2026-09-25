# Repository Operating Directives & Agent Constitution (`aaroneous`)

> **STATUS**: BINDING MACHINE CONTRACT & CONSTITUTION  
> **APPLIES TO**: ALL AUTONOMOUS AGENTS (OpenCode, Qwen, Claude, LM Studio, Human Contributors)  
> **WORKSPACE**: `aaroneous` (Type-Safe Rust Component Framework & SCADA/PLC Architecture)  
> **CORE IDENTITY**: Aaroneous is **NOT** a monolithic app. It is a strict, type-safe **Rust Component Framework** for building interchangeable, safe, zero-allocation execution blocks & plugins.  
> **LAST UPDATED**: 2026-09-14

---

## 0. Component Framework Architecture & Topology

Every crate in this repository is an independent, plug-and-play component block with strict trait boundaries, zero-copy contracts, and zero-allocation critical paths:

- **`core/hypervisor/`**: Headless microkernel host & execution loop.
- **`crates/orchestrator/`**: Task scheduling & core affinity.
- **`crates/platform_bridge/`**: OS abstractions (DXGI, Win32, WASAPI).
- **`crates/ipc_bus/`**: Lock-free SPMC/SWMR ring buffers & WAL.
- **`crates/capabilities/`**: `UniversalTool` & domain specialist registry.
- **`crates/governance/`**: Z3 SMT verification & safety interlocks.
- **`crates/compute/`**: Solid-state SSM, .si container engine, JIT compiler.
- **`crates/api/` & `crates/studio_hud/`**: Presentation layer and GUI viewports.

---

## 0.1 Local Agent Delegation & Model Bias

- **Default Agent Bias**: `qwen3.5:9b-q6` (Ollama at `http://localhost:11434`, model tag: `qwen3.5:9b-q6`).
- **Autonomous Task Offloading**: When delegating code generation, method synthesis, refactoring, and test fixtures to local GPU models via `scripts/local_agent_delegate.ps1`, agents must default to `qwen3.5:9b-q6`.
- **Reasoning Runaway Suppression**: When invoking `qwen3.5:9b-q6`, always provide a calibrated system prompt (e.g. `"You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code."`) with `-Temperature 0.0` and generous token headroom (`-NumPredict 4096+`).

---

## 1. Pure State Machine & Dependency Injection Rules

- **Deterministic State Reducers**: Domain engines operate as pure state transitions $S_{t+1} = f(S_t, I)$. No side effects, no background network I/O, and no hidden async task launches during state reduction.
- **Three-Phase Scan Separation**: Strict separation between Input Acquisition (I/O), State Reduction (pure, non-allocating computation), and Telemetry/Actuation Output.
- **Constructor Injection Only**: All static buffers, handles, and configs MUST be passed into constructors (e.g., `Engine::new(config, buffer)`). Sub-components must NEVER instantiate their own dependencies or global services.
- **No Ambient Reads**: Sub-components must never read system clocks, environment variables, or files outside what is explicitly passed via constructor or tick inputs.

---

## 2. Zero Ambient Authority

- **Explicitly Banned Functions**: `std::env::var`, `std::env::var_os`, `std::env::set_var`, `std::env::remove_var`, `std::env::temp_dir`, `std::env::current_dir` (outside bootstrap CLI entrypoints), and `.canonicalize()` (use `paths::normalize_path`).
- **Configuration Injection**: All file paths, endpoint URIs, and credentials arrive via typed configuration structs (`WorkspacePathsConfig`, `ShmSegmentConfig`).
- **Test Sandboxing**: Tests must construct explicit test configs and use `tempfile::tempdir()` for filesystem isolation; tests MUST NEVER touch ambient host environment variables or temp folders.

---

## 3. Zero-Heap Allocation & Memory Geometry

- **No Heap on Hot Paths**: In `core/hypervisor`, `crates/ipc_bus`, `crates/compute`, `dev/emulator_harness`, and frame ingestors, never allocate dynamic heap memory.
- **Banned Types & Macros on Hot Paths**: `String`, `Vec`, `Box`, `format!`, `.to_string()`, and unbounded collections (`HashMap`, `BTreeMap`). Use stack arrays (`[T; N]`), bounded ring buffers (`SwrnRingBuffer`), and fixed slices.
- **Memory Geometry & ABI Safety**: All boundary types and IPC messages must use `#[repr(C)]`, derive `bytemuck::Pod` and `bytemuck::Zeroable`, and include explicit padding fields (e.g., `pub _pad0: u16`) for natural alignment.
- **Concurrency & Statics**: Single-Writer/Multiple-Reader (SWMR) over pre-allocated ring buffers. No `std::sync::Mutex`, `parking_lot::Mutex`, or `RwLock` on hot paths. No `OnceLock` or `lazy_static` for runtime state.

---

## 4. Cratify & Naming Rules

- **Zero Prefix Stutter**: DO NOT prepend `aaroneous_` or `aaroneous-` to crates, packages, internal types, modules, or IPC channels. (Exemption: external Prometheus/OpenTelemetry metrics namespaces).
- **Generic Systems Terminology**: Use standard systems names (`hypervisor`, `paths`, `wire`, `hud`, `api`, `bridge`, `controller`, `ingestor`, `pipeline`). Avoid monikers or puns.
- **Domain-Aware Unsafe Permissions**: Standard domain crates (`api`, `hud`, `wire`, `orchestrator`, `paths`) MUST declare `#![deny(unsafe_code)]`. Performance/kernel crates (`compute`, `hypervisor`) may declare `#![warn(unsafe_code)]` with documented `// SAFETY:` comments.
- **Mandatory Tempdir in Tests**: Unit and integration tests must use `tempfile::tempdir()` for filesystem testing.

---

## 5. Banned Anti-Patterns

- **No Stubs**: `todo!()` and `unimplemented!()` are strictly forbidden in committed code.
- **No Unsafe Implementations**: Manual `unsafe impl Pod` or `unsafe impl Zeroable` is banned (derive only).
- **No Unchecked Transmutes**: `unsafe transmute` on unaligned or static data is banned.
- **No Unhandled Panics**: `.unwrap()` and `.expect()` are banned on hot paths and production error-handling paths. Propagate errors via `Result`.

---

## 6. Sequential Verification Gate Protocol

Agents must NEVER declare work complete based solely on `cargo check`. The single command below (or its shim) runs every gate below, in order, and is mechanically checked to cover the same commands `.github/workflows/ci.yml`'s `check-and-test` job runs (see `xtask/src/gate.rs`'s `tests` module) — running it locally gives the same assurance as a green CI run:

```bash
# Full Self-Verification Gate Script — runs gates 1-11 below
bash scripts/agent_check.sh
# equivalently: cargo run -p xtask -- gate

# 1. Text Encoding Contract (UTF-8 without BOM, LF line endings)
cargo run -p xtask -- check-encoding

# 2. Formatting
cargo fmt --all -- --check

# 3. Strict Clippy (workspace)
cargo clippy --workspace -- -D warnings

# 4. Full Workspace Compilation (all targets, tests, benches)
cargo check --workspace --all-targets

# 4a. Native-host portable core (no allocator or default features)
cargo check -p scan_core --no-default-features

# 4b. Representative ARM bare-metal portable core
cargo check -p scan_core --target thumbv7em-none-eabihf --no-default-features

# 4c. Representative WebAssembly portable core
cargo check -p scan_core --target wasm32-unknown-unknown --no-default-features

# 5. Workspace Test Suite (Functional determinism)
cargo test --workspace

# 6. Structural & Semantic Invariant Audit (MUST EXIT 0)
cargo run -p ast_auditor -- audit core/ crates/ dev/emulator_harness/

# 7. Zero-Stub & Soundness Inspection (Must return empty)
! git grep -n -E "(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)" -- "crates/" "core/" "dev/"

# 8. Golden Dogfooding Harness Verification
cargo test -p emulator_harness

# 9. Release Binary Check
cargo check --release --bin aaroneous --bin hypervisor

# 10. Optional Runtime Features (compile only)
cargo check -p hypervisor --all-targets --features llama-gguf,gpu-metrics,fleet,testing,standalone

# 11. Iroh Compatibility Feature (compile only)
cargo check -p hypervisor --all-targets --features p2p-iroh
```

> **Note:** `scripts/agent_check.sh` is a thin CI shim — all gate logic runs via `cargo xtask gate`. Gates 1-3 and 9-11 mirror `ci.yml`'s directly-declared steps; gates 4-8 are `gate.rs`'s own pre-existing verification, run by CI only indirectly (as part of the "Canonical repository verification" step). If you add a new CI check, add the matching gate in `xtask/src/gate.rs` and its command string to `GATE_COMMANDS` in the same file — a test fails otherwise the next time either drifts from the other.

---

## 7. Deep Architecture & Ingestion References

For exhaustive architectural philosophy, historical background, and forensic protocols:
- **System Architecture & PLC Reductions**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Forensic Ingestion Protocol & Quarantine**: [docs/FORENSICS_RFC0005.md](docs/FORENSICS_RFC0005.md)
- **Cratify Invariant & Governance Specification**: [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md)
