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

## 0.1 Optional Private Development Tooling

- Local model runners, agent profiles, prompts, workstation provisioning, and experimental automation belong to the private `Aarogaming/aaroneous-devtools` repository.
- Those tools are optional development aids and must never be required to build, test, release, or run Aaroneous.
- Contributors must follow the [tooling boundary policy](docs/architecture/TOOLING_BOUNDARY_POLICY.md) before adding or invoking development automation.

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

## 5.1 Tooling Boundary and Self-Hosting Guardrail

- **Product Boundary**: The workspace contains only reproducible product artifacts: Rust crates,
  tests, auditors, active interface assets, and declarative build/deployment configuration.
- **Rust-First Tooling**: New repository-management, verification, migration, and release logic
  MUST be implemented in Rust. A non-Rust script is permitted only as a thin platform or CI
  adapter with a documented reason and a named Rust command or platform capability.
- **No Toolchain Assimilation**: Product crates MUST NOT depend on local-agent runners, personal
  prompts, workstation provisioning, private operational tools, or research prototypes.
- **External Dev-Tools**: Private development tooling may invoke documented product commands, but
  the product MUST NOT require it as a submodule, workspace member, build dependency, runtime
  dependency, or implicit local prerequisite.
- **Self-Modification Boundary**: Runtime components may not write source, alter build
  configuration, or replace binaries outside an explicit, reviewed control-plane protocol.
- **Migration Inventory**: Existing non-Rust scripts are candidates to port, retain as thin
  adapters, move to private tooling, or remove. No new general-purpose scripting stack is
  permitted while this inventory is incomplete. See
  [docs/architecture/TOOLING_BOUNDARY_POLICY.md](docs/architecture/TOOLING_BOUNDARY_POLICY.md).

---

## 6. Sequential Verification Gate Protocol

Agents must NEVER declare work complete based solely on `cargo check`. Every agent MUST run and verify the following sequence before completing a task:

```bash
# 1. Full Workspace Compilation (all targets, tests, benches)
cargo check --workspace --all-targets

# 2. Workspace Test Suite (Functional determinism)
cargo test --workspace

# 3. Structural & Semantic Invariant Audit (MUST EXIT 0)
cargo run -p ast_auditor -- audit core/ crates/

# 4. Zero-Stub & Soundness Inspection (Must return empty)
! git grep -n -E "(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)" -- "crates/" "core/" "dev/"

# 5. Golden Dogfooding Harness Verification
cargo test -p emulator_harness

# 6. Full Self-Verification Gate Script
cargo run -p ast_auditor -- verify
```

---

## 7. Deep Architecture & Ingestion References

For exhaustive architectural philosophy, historical background, and forensic protocols:
- **System Architecture & PLC Reductions**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Forensic Ingestion Protocol & Quarantine**: [docs/FORENSICS_RFC0005.md](docs/FORENSICS_RFC0005.md)
- **Cratify Invariant & Governance Specification**: [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md)
