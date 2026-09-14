# Compiler Invariant Governance & Static AST Specification (AST Auditor)

> **TIER 2 GOVERNANCE & AUDIT REFERENCE**  
> **SCOPE**: Static AST linting, structural and semantic invariants, and banned anti-pattern specifications.  
> **APPLIES TO**: All crates under `core/`, `crates/`, `dev/`, and `sdk/`.  
> **ACTIVE ENGINE**: `crates/ast_auditor` (dissolved from legacy `cratify`).

---

## 1. Architectural Invariant Rules Matrix

| Anti-Pattern | Operational Risk | Enforcement Mechanism | Failure Policy |
|---|---|---|---|
| Ambient `std::env::*` | Nondeterminism, security leakage | `ast_auditor` AST check & compiler ban | Exit Code 1 (Hard Block) |
| Manual `unsafe impl Pod/Zeroable` | Undefined behavior, invalid bit patterns | `ast_auditor` AST rule | Exit Code 1 (Hard Block) |
| `std::sync::Mutex` / `RwLock` on hot paths | Deadlocks, thread parking latency spikes | Hot path AST linting | Exit Code 1 (Hard Block) |
| `OnceLock` / `lazy_static` for runtime state | Deferred init UB, nondeterministic latency | Static check | Exit Code 1 (Hard Block) |
| `unsafe transmute` on statics/unaligned data | Instant UB and memory corruption | `ast_auditor` audit | Exit Code 1 (Hard Block) |
| `todo!()` / `unimplemented!()` in committed code | Silent runtime crashes and denial-of-service | Negative contract tests (`git grep`) | Exit Code 1 (Hard Block) |
| `.unwrap()` / `.expect()` on hot paths or errors | Process panics; zero fault recovery | Result propagation enforcement | Exit Code 1 (Hard Block) |
| Workspace prefix stutter (`aaroneous_*`) | Namespace pollution and redundancy | `ast_auditor` naming lint | Exit Code 1 (Hard Block) |

---

## 2. Zero Ambient Authority Governance

Ambient authority introduces hidden side-effects and breaks sandbox guarantees:
- **Banned Functions**: `std::env::var`, `std::env::var_os`, `std::env::set_var`, `std::env::remove_var`, `std::env::temp_dir`, `std::env::current_dir` (outside bootstrap CLI entrypoints).
- **Enforcement Engine**: `crates/ast_auditor` (`cargo run -p ast_auditor -- audit core/ crates/`).
- **Capability Replacement**: All paths, tokens, and endpoints must be passed via typed config structs (`paths::WorkspacePathsConfig`, `paths::FederationConfigRegistry`).
- **Test Sandboxing**: Tests must use `tempfile::tempdir()` for filesystem isolation; never touch ambient host paths.

---

## 3. Memory Layout & Safety Geometry

- **Fixed Geometry**: Boundary types and shared memory structs must use `#[repr(C)]`.
- **Safe Bitcasting**: Must derive `bytemuck::Pod` and `bytemuck::Zeroable`. Manual `unsafe impl Pod` or `Zeroable` is strictly forbidden.
- **Explicit Padding**: Types must declare explicit padding fields (`pub _pad0: u16`, `pub _pad1: u32`) to eliminate compiler-dependent alignment holes.

---

## 4. Domain-Aware Unsafe Code Permissions

Cratify scaffolds and verifies domain-specific safety levels:
- **Standard Domain Crates** (`api`, `hud`, `wire`, `orchestrator`, `paths`): Must declare `#![deny(unsafe_code)]`.
- **Performance / Kernel Crates** (`compute`, `hypervisor`): May declare `#![warn(unsafe_code)]` provided every `unsafe` block includes documented `// SAFETY:` invariant comments.

---

## 5. Naming Hygiene & Prefix Stutter Audit

- Banned string monikers, IPC channel names, crate names, and identifiers starting with `aaroneous_` or `aaroneous-`.
- **Exemptions**: External metrics identifiers (e.g. Prometheus counters `aaroneous_tick_duration_seconds`, OpenTelemetry identifiers) matching `is_exempt_metric_identifier`.

---

## 6. Standardized Systems Nomenclature & Pattern Synthesis

To align workspace tooling with standard compiler and systems engineering terminology, metaphorical monikers are normalized to standard computer science nomenclature with 100% backward-compatible aliases:

| Legacy Moniker | Standard Systems Nomenclature | Primary API / Path |
|---|---|---|
| Harvesting | Source Tree Extraction | `adaptation_engine::extract_source_tree` (`harvest.rs`) |
| Assimilation | Component Onboarding & Conformance | `orchestrator::ComponentOnboardingTask` (`assimilation.rs`) |
| Naturalizing | Invariant Normalization / Conformance | `ast_auditor::review` (`pattern_reviewer.rs`) |
| Quarantine | Staging Sandbox | `dev/legacy_staging/`, `orchestrator::StagedSandbox` |

The **Continuous Conformance & Architectural Pattern Synthesis Engine (CCPSE)** evaluates workspace code against declarative pattern specifications under `registry/patterns/` via `ast_auditor review`.
