# Repository Operating Directives & Agent Constitution (`aaroneous`)

> **STATUS**: BINDING MACHINE CONTRACT & CONSTITUTION  
> **APPLIES TO**: ALL AUTONOMOUS AGENTS (OpenCode, Qwen, Claude, LM Studio, Human Contributors)  
> **WORKSPACE**: `aaroneous` (Type-Safe Rust Component Framework & SCADA/PLC Architecture)  
> **CORE IDENTITY**: Aaroneous is **NOT** a monolithic app. It is a strict, type-safe **Rust Component Framework** for building interchangeable, safe, zero-allocation execution blocks & plugins.  
> **LAST UPDATED**: 2026-09-23

---

## Quick Start for Agents

**Before editing any file, run:**

```bash
cargo xtask gate
```

This is the single verification command that validates your workspace matches CI. It runs encoding, formatting, clippy, compilation, tests, AST audit, stub check, emulator harness, and feature compilation in order.

**Before claiming work is done, run it again.** If it passes, you're done.

**Canonical type names live in `crates/governance`.** All other names (`SystemBiology`, `SpecialistMetabolism`, `HomeostasisGovernor`, etc.) are deprecated legacy aliases. Use the canonical names.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full rule reference.

**Joining as an agent alongside others (Claude, Codex, Antigravity/Gemini,
local Qwen):** cross-agent coordination — the shared claim queue, a
low-cost change-ping mechanism, and the circuit breaker used to halt
another agent's pipeline — is documented in the private companion repo
`aaroneous-devtools`, at `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md`
and `governance/ROUND_TABLE_INTRO_PROMPT.md` (the latter is a ready-to-paste
onboarding prompt). Read those before working here alongside another agent
session, if you have access to that repo. Rules in this document still
apply regardless of which agent is editing.

---

## 0. Component Framework Architecture & Topology

Every crate in this repository is an independent, plug-and-play component block with strict trait boundaries, zero-copy contracts, and zero-allocation critical paths:

- **`core/hypervisor/`**: Headless microkernel host & execution loop.
- **`crates/orchestrator/`**: Task scheduling & core affinity.
- **`crates/platform_bridge/`**: OS abstractions (DXGI, Win32, WASAPI).
- **`crates/ipc_bus/`**: Lock-free SPMC/SWMR ring buffers & WAL.
- **`crates/capabilities/`**: `UniversalTool` & domain specialist registry.
- **`crates/governance/`**: Interference checking, safety interlocks, and resource governors.
- **`crates/compute/`**: Solid-state SSM, .si container engine, JIT compiler.
- **`crates/api/` & `crates/studio_hud/`**: Presentation layer and GUI viewports.

---

## 0.1 Local Agent Delegation & Model Bias

- **Default Agent Bias**: `qwen3.5:9b-q6` (Ollama at `http://localhost:11434`, model tag: `qwen3.5:9b-q6`).
- **Autonomous Task Offloading**: When delegating code generation, method synthesis, refactoring, and test fixtures to local GPU models via `scripts/local_agent_delegate.ps1`, agents must default to `qwen3.5:9b-q6`.
- **Reasoning Runaway Suppression**: When invoking `qwen3.5:9b-q6`, always provide a calibrated system prompt (e.g. `"You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code."`) with `-Temperature 0.0` and generous token headroom (`-NumPredict 4096+`).

---

## 1. Compliance Model: Floor, Profiles, Ratchet

Full specification: [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md) (v2, owner-approved 2026-09-23). This section is the binding summary; the spec wins on any conflict.

- **Universal Floor**: Every crate meets the rules in sections 2 and 5 without exception. The only exempt contexts are bootstrap entrypoints (`src/main.rs`, `src/bin/*`, examples), tests, benches, and `build.rs`, as listed per rule in the spec.
- **Compliance Profiles**: Every crate declares exactly one profile in its manifest, which adds stricter rules on top of the floor:

  ```toml
  [package.metadata.cratify]
  profile = "control"   # kernel | control | presentation | tooling
  ```

  | Profile | Crates | Adds |
  |---|---|---|
  | `kernel` | `core/hypervisor`, `ipc_bus`, `compute`, `wire`, `si_format`, `si_ir`, `platform_bridge`, `runtime_monitor`, `dev/emulator_harness` | Section 3 in full; `#![warn(unsafe_code)]` + `// SAFETY:` |
  | `control` | `orchestrator`, `orchestration_plane`, `llm_gateway`, `llm_gateway_types`, `governance`, `capabilities`, `adaptation_engine`, `adaptation_plane`, `mcp_server`, `transpiler`, `omni`, `hotload`, `plugin_api`, `core-contracts`, `paths`, `sdk/rust` | Pure reducers with I/O confined to adapters; degraded paths at external-call boundaries; `#![deny(unsafe_code)]` |
  | `presentation` | `api`, `studio_hud`, `scratchpad` | `#![deny(unsafe_code)]` |
  | `tooling` | `ast_auditor`, `cratify`, `compliance_auditor`, `xtask`, `benches` | `#![deny(unsafe_code)]` |

  Moving a crate to a stricter profile is always allowed. Moving to a looser profile requires owner sign-off recorded in the PR.
- **Deterministic State Reducers**: Domain engines operate as pure state transitions $S_{t+1} = f(S_t, I)$. No side effects, no background network I/O, and no hidden task launches during state reduction.
- **Three-Phase Scan Separation** (`kernel`): Strict separation between Input Acquisition (I/O), State Reduction (pure, non-allocating computation), and Telemetry/Actuation Output.
- **Fault Tolerance Over Brittle Invariants**: A violated runtime precondition is an operating condition, not a reason to abort. Use primary / degraded / safe-hold paths, with deadband (separate trip and recovery) thresholds between `Nominal`, `Degraded`, and `SafeHold` modes.
- **The Ratchet**: Compliance only moves forward. Every fixed defect class is crystallized into a type, compile-time assertion, `ast_auditor` rule, or SMT constraint. New auditor rules land as warnings with a per-crate baseline that may only decrease; at zero the rule becomes a hard block for that crate. Blanket `#[allow]` of a Cratify rule is banned.

---

## 2. Zero Ambient Authority (Floor)

- **Constructor Injection Only**: All buffers, handles, configs, endpoints, and credentials arrive via constructors and typed config structs (`WorkspacePathsConfig`, `ShmSegmentConfig`). Components never instantiate their own global services.
- **Banned Functions**: `std::env::var`, `std::env::var_os`, `std::env::set_var`, `std::env::remove_var`, `std::env::temp_dir`, `std::env::current_dir`, and `.canonicalize()` (use `paths::normalize_path`), outside bootstrap entrypoints.
- **No Ambient Clock**: No `SystemTime::now()` / `Instant::now()` in library code. Time arrives as a tick input or an injected clock (model: `orchestrator::supervision`).
- **No Self-Started Execution**: Library code never spawns OS threads or async tasks on its own authority. Spawning goes through an injected executor handle or `orchestrator::Supervisor`.
- **Test Sandboxing**: Tests construct explicit test configs and use `tempfile::tempdir()`; tests never touch host environment variables or ambient temp folders.

---

## 3. Zero-Heap Hot Paths & Memory Geometry (`kernel` profile)

- **Hot-Path Marking**: Scan-loop reducers and frame ingestors carry `#[hot_path]` (function) or `#![hot_path]` (file). An unmarked scan-loop function is a defect, not an exemption.
- **Banned on Hot Paths**: `String`, `Vec`, `Box`, `format!`, `.to_string()`, and unbounded collections (`HashMap`, `BTreeMap`). Use stack arrays (`[T; N]`), bounded ring buffers (`SwrnRingBuffer`), and fixed slices.
- **Memory Geometry & ABI Safety**: IPC and shared-memory types use `#[repr(C)]` (`#[repr(C, align(64))]` where cache-line sensitive), derive `bytemuck::Pod` and `bytemuck::Zeroable`, and declare explicit padding fields (e.g., `pub _pad0: u16`). `control` crates follow this only for types crossing into a `kernel` crate.
- **Concurrency & Statics**: Single-Writer/Multiple-Reader (SWMR) over pre-allocated ring buffers. No `std::sync::Mutex`, `parking_lot::Mutex`, or `RwLock` on hot paths. No `OnceLock` or `lazy_static` for runtime state in any non-`tooling` crate.

---

## 4. Naming, Dependencies & Graduation

- **Zero Prefix Stutter**: DO NOT prepend `aaroneous_` or `aaroneous-` to crates, packages, internal types, modules, or IPC channels. (Exemption: external Prometheus/OpenTelemetry metrics namespaces.)
- **Generic Systems Terminology**: Use standard systems names (`hypervisor`, `paths`, `wire`, `hud`, `api`, `bridge`, `controller`, `ingestor`, `pipeline`). Avoid monikers or puns.
- **Dependency Admission**: Before adding any third-party dependency, score it on compliance distance (`alloc`, `ambient`, `abi`, `safety`; 0-4 each, as the caller experiences it) and record the verdict in the PR: **Admit**, **Admit with conditions**, **Extract pattern** (clean-room the algorithm behind a workspace trait, do not add the crate), or **Reject**. `kernel` crates require Admit on every vector; other profiles require `ambient = 0` and `safety <= 1` after conditions. Example: `sled` is rejected (spawns its own threads; unstable format).
- **Component Graduation**: Code entering from outside the workspace (companion tooling, external sources, generated drafts) must be proven in its origin, classified (new component vs. upgrade of an existing one, never a second implementation), placed behind a workspace-defined trait with contract tests, landed inert (feature flag) or run in shadow mode, and then its origin copy deleted. Aaroneous never depends on external tooling.

---

## 5. Banned Anti-Patterns (Floor)

- **No Stubs**: `todo!()` and `unimplemented!()` are forbidden in committed code.
- **No Unsafe Implementations**: Manual `unsafe impl Pod` or `unsafe impl Zeroable` is banned (derive only).
- **No Unchecked Transmutes**: `unsafe transmute` on unaligned or static data is banned.
- **No Panics on Runtime Input**: `.unwrap()`, `.expect()`, `panic!`, and `assert!` on values derived from I/O, config, or model output are banned in every profile. Propagate `Result` or take a degraded path. Exempt: tests, bootstrap entrypoints, `build.rs`, `debug_assert!`, and provably infallible cases marked `// INFALLIBLE: <reason>`.
- **Enforcement Status**: Some floor rules are not yet mechanically enforced by `ast_auditor` (see CRATIFY_SPEC section 7.1). Unenforced does not mean optional: reviewers apply them by hand until the rule lands.

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
- **Cratify Compliance Specification (v2: profiles, admission, graduation, ratchet)**: [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md)
