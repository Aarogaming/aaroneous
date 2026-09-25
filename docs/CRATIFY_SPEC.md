# Cratify Compliance Specification

> **TIER 2 GOVERNANCE & AUDIT REFERENCE**  
> **STATUS**: v2, owner-approved 2026-09-23. Supersedes v1 (uniform absolute rules).  
> **SCOPE**: Compliance profiles, invariant rules, dependency admission, component graduation, and the invariant ratchet.  
> **APPLIES TO**: All crates under `core/`, `crates/`, `dev/`, `sdk/`, and `xtask/`. The gate's `ast_auditor` pass currently covers `core/`, `crates/`, and `dev/emulator_harness/` only (section 7.1).  
> **ACTIVE ENGINE**: `crates/ast_auditor` (static rules), `crates/cratify` (CLI bridge).

---

## 0. What Changed From v1

v1 stated every rule as an absolute ("Always / Never") applied uniformly to every crate. In practice
that produced two failures: rules that could not be met were silently unenforced (only four functions
in the workspace are marked hot, two in `ipc_bus` and two in `dev/emulator_harness`, so the
zero-allocation rule audits almost none of the scan loop), and useful control-plane code could not be
admitted at all.

v2 replaces uniform absolutes with:

1. **A universal floor** that every crate meets without exception (Section 1).
2. **Compliance profiles** that add stricter rules where the execution context demands them (Section 2).
3. **A dependency admission gate** scored by compliance distance (Section 5).
4. **A graduation gate** for code entering the workspace from outside (Section 6).
5. **The ratchet**: every rule may only tighten, and every circumvented defect becomes a rule (Section 7).

---

## 1. Universal Floor (All Profiles)

These rules have no profile exemption. The only exemptions are the listed contexts.

| Rule | Exempt Contexts | Enforcement |
|---|---|---|
| No ambient authority: `std::env::{var, var_os, set_var, remove_var, temp_dir, current_dir}`, `.canonicalize()` (use `paths::normalize_path`) | Bootstrap entrypoints (`src/main.rs`, `src/bin/*`, examples) | `ast_auditor` `no_ambient_authority` - **enforced**, except five library files silenced with `#[allow(ambient_authority)]` (baseline, section 7.1) |
| No ambient clock: `SystemTime::now()`, `Instant::now()` in library code. Time arrives as a tick input or an injected clock (model: `orchestrator::supervision`) | Bootstrap entrypoints, tests, benches | **planned** rule |
| Constructor injection: configs, paths, endpoints, credentials, and buffers arrive via typed config structs (`paths::WorkspacePathsConfig`, `ShmSegmentConfig`); components never construct their own global services | - | review + `no_ambient_authority` |
| No panics on runtime input: `.unwrap()`, `.expect()`, `panic!`, and `assert!` on values derived from I/O, config, or model output. Propagate `Result` or take a degraded path (Section 3) | Tests, bootstrap entrypoints, `build.rs`, `debug_assert!`, provably infallible cases carrying a `// INFALLIBLE:` comment | **planned** rule |
| No stubs: `todo!()`, `unimplemented!()` | - | gate 7 (`git grep`) - **enforced** |
| No manual `unsafe impl Pod` / `Zeroable` (derive only); no `transmute` on statics or unaligned data | - | `soundness`, gate 7 - **enforced** |
| No prefix stutter (`aaroneous_` / `aaroneous-`) in crates, modules, types, IPC channels | External metric namespaces (`is_exempt_metric_identifier`) | `no_workspace_prefix_stutter` - **enforced** |
| Text encoding: UTF-8 without BOM, LF line endings | - | `text_encoding`, `xtask check-encoding` - **enforced** |
| Path literals use `/` separators | - | `path_separator` - **enforced** |
| Test sandboxing: filesystem tests use `tempfile::tempdir()`; tests never read host env vars or ambient temp folders | - | review; **planned** rule |
| No self-started execution: library code never spawns OS threads or async tasks on its own authority. Spawning goes through an injected executor handle or `orchestrator::Supervisor` | Bootstrap entrypoints, tests | **planned** rule |
| Declared profile: every crate declares its profile (Section 2) | - | **planned** rule |

---

## 2. Compliance Profiles

A profile is a **build-time classification of a crate** that selects which rules apply on top of the
universal floor. It is declared in the crate manifest, following the existing
`[package.metadata.capability]` precedent:

```toml
[package.metadata.cratify]
profile = "control"   # kernel | control | presentation | tooling
```

A crate has exactly one profile. A crate needing kernel guarantees for part of its code splits that
part into its own `kernel` crate rather than mixing profiles.

### 2.1 Profile Rules

| Rule | `kernel` | `control` | `presentation` | `tooling` |
|---|---|---|---|---|
| Universal floor (Section 1) | yes | yes | yes | yes |
| Zero heap on `#[hot_path]` code (`String`, `Vec`, `Box`, `format!`, `.to_string()`, `HashMap`, `BTreeMap`) | **required**; scan-loop reducers and ingestors must be marked | n/a | n/a | n/a |
| No `Mutex` / `RwLock` / `OnceLock` / `lazy_static` for runtime state | **required** | allowed off the scan loop; no `OnceLock`/`lazy_static` for runtime state | same as `control` | allowed |
| Boundary types `#[repr(C)]` + derived `bytemuck::Pod`/`Zeroable` + explicit padding | **required** for IPC and shared-memory types | required only for types crossing into a `kernel` crate | n/a | n/a |
| Pure three-phase scan (acquire / reduce / emit) | **required** | reducers pure; I/O confined to adapter layer | n/a | n/a |
| Unsafe code | `#![warn(unsafe_code)]` + `// SAFETY:` on every block | `#![deny(unsafe_code)]` | `#![deny(unsafe_code)]` | `#![deny(unsafe_code)]` |
| Heap allocation, async I/O, network | hot path: no | allowed | allowed | allowed |
| Degraded-path requirement (Section 3) | **required** | required for external-call failures | n/a | n/a |

### 2.2 Profile Assignment

| Profile | Crates |
|---|---|
| `kernel` | `core/hypervisor`, `ipc_bus`, `compute`, `wire`, `si_format`, `si_ir`, `platform_bridge`, `runtime_monitor`, `core-contracts`, `dev/emulator_harness` |
| `control` | `orchestrator`, `orchestration_plane`, `llm_gateway`, `llm_gateway_types`, `governance`, `capabilities`, `adaptation_engine`, `adaptation_plane`, `mcp_server`, `transpiler`, `omni`, `paths`, `sdk/rust` |
| `presentation` | `api`, `studio_hud`, `scratchpad` |
| `tooling` | `ast_auditor`, `cratify`, `compliance_auditor`, `xtask`, `benches` |

`core/hypervisor` has two roles. Its library (`src/`) is `kernel`. Its binaries (`bin/`) are the
workspace **composition root**: they wire every component together and are the one place allowed to
depend on crates of any profile or ring.

### 2.3 Profile Dependency Direction

A crate may depend only on crates whose profile is the same or stricter, in the order
`kernel` > `control` > `presentation` / `tooling`. A `kernel` crate therefore depends only on
`kernel` crates and admitted third-party dependencies. The composition root (section 2.2) is exempt.

**Status: planned.** The rule is not yet enforced. Known violations, measured from `cargo metadata`
(normal and build dependencies) on 2026-09-24 and recorded as the ratchet baseline (14 edges; was
16 before `hotload`/`plugin_api` were removed from the `control` row below on 2026-09-25 — see
that row's note):

| Violation | Edges | Resolution |
|---|---|---|
| `kernel` -> `paths` (`control`) | `ipc_bus`, `compute`, `hypervisor` -> `paths` | Split `paths` into a `kernel`-safe path-value crate and a bootstrap discovery layer called only from entrypoints. |
| `hypervisor` library (`kernel`) -> `control` | `hypervisor` -> `adaptation_engine`, `adaptation_plane`, `governance`, `llm_gateway`, `omni`, `orchestrator`, `transpiler`, `capabilities` | Hypervisor decomposition (M75): move composition logic out of the library into the binaries or a dedicated composition crate, so the library keeps only `kernel` concerns. The composition-root exemption covers `bin/` only, and Cargo declares dependencies per package, so it does not cover these: the library sources use all of them except `capabilities`, which appears declared but unused. (`hotload` and `plugin_api` were removed from this row on 2026-09-25: they were the source of a fixed unauthenticated dynamic-DLL-loading vulnerability on `main` (#34); this branch had restored them as unreachable dead dependencies during a merge, and removed them again on discovering why `main` had dropped them — see the security review this same date for the full trace.) |
| `kernel` / `control` -> `ast_auditor` (`tooling`) | `hypervisor`, `capabilities`, `adaptation_engine` -> `ast_auditor` | Extract the analysis API these crates call (`inspect`, `run_pattern_review`) into a `control`-profile crate that `ast_auditor` also depends on, leaving the CLI and gate rules in `tooling`. |

The count may only decrease. A new edge in any of these directions is a defect even while the rule
is unenforced.

**Changing profile:** moving a crate to a *stricter* profile is always permitted. Moving to a *looser*
profile is a ratchet reversal (Section 7) and requires owner sign-off recorded in the PR.

---

## 3. Fault Tolerance Over Brittle Invariants

A violated precondition at runtime is an operating condition to handle, not a reason to abort.
`kernel` crates, and `control` crates at their external-call boundaries, structure execution as:

- **Primary path**: optimal execution under nominal preconditions (zero-copy, aligned, SIMD).
- **Degraded path**: deterministic, non-panicking fallback when preconditions are not met (scalar path,
  load shedding, bounded queue deferral).
- **Safe-hold**: telemetry and heartbeat continue; risky autonomous actions (code mutation, actuation)
  are suspended.

Transitions between `Nominal`, `Degraded`, and `SafeHold` operating modes use **deadband thresholds**
(distinct trip and recovery levels) so the system does not oscillate between modes under fluctuating
load. Circuit breakers follow the same rule.

Operating modes are **runtime states**; compliance profiles (Section 2) are **build-time classes**.
The two are independent: a `kernel` crate runs in all three modes.

---

## 4. Memory Layout & Safety Geometry (`kernel` profile)

- **Fixed geometry**: IPC and shared-memory types use `#[repr(C)]`; cache-line sensitive types use
  `#[repr(C, align(64))]`.
- **Safe bitcasting**: derive `bytemuck::Pod` and `bytemuck::Zeroable`; manual impls are banned (floor).
- **Explicit padding**: declare padding fields (`pub _pad0: u16`, `pub _pad1: u32`) so layout never
  depends on compiler-inserted holes.
- **Hot-path marking**: functions and files executing inside the scan loop carry `#[hot_path]` /
  `#![hot_path]`, or the doc-attribute form `#[doc = "hot_path"]` (equivalently `/// hot_path`),
  which `ast_auditor` also recognizes. An unmarked scan-loop function is a compliance defect, not an
  exemption.

---

## 5. Dependency Admission

External crates rarely meet these rules out of the box. Every new third-party dependency of a
workspace crate is scored and receives an admission verdict **before** it is added to a manifest.

### 5.1 Compliance Distance

Score each vector 0 (compliant as used) to 4 (requires rewrite). Score the crate **as the caller
experiences it**, meaning what it forces on its caller, not how it is implemented internally.

| Vector | Question |
|---|---|
| `alloc` | Does it force heap allocation onto a `#[hot_path]`? |
| `ambient` | Does it read env vars, the clock, or the filesystem on its own, or spawn threads/tasks the caller did not request? |
| `abi` | Can its data cross a `kernel` boundary without copying into `#[repr(C)]` types? |
| `safety` | Can it panic on caller-supplied input? Is it maintained, with a stable format and a released 1.x (or equivalent)? |

### 5.2 Verdicts

| Verdict | Condition | Meaning |
|---|---|---|
| **Admit** | Every vector relevant to the target profile scores 0 | Add normally. |
| **Admit with conditions** | Non-zero scores are neutralized by documented configuration | Add; the neutralizing configuration is mandatory and recorded. |
| **Extract pattern** | High need, but non-zero scores cannot be neutralized | Do not add the crate. Implement the underlying algorithm or layout clean-room behind a workspace trait. |
| **Reject** | Low need, or high distance with no extractable pattern | Do not add; the crate may still be used outside the workspace. |

Need is judged by the bottleneck test: does the dependency remove a measured latency spike, a runtime
branch on the scan loop, a race condition, or a gap on the active roadmap? If not, it stays in the
reference catalogue regardless of how easy it is to add.

`kernel` crates require **Admit** on every vector. `control`, `presentation`, and `tooling` crates
require `ambient = 0` and `safety <= 1` after conditions; `alloc` and `abi` do not apply to them unless
the dependency's types cross into a `kernel` crate.

### 5.3 Worked Examples

| Crate | Scores | Verdict |
|---|---|---|
| `sled` 0.34 | `ambient` 4 (spawns flusher and threadpool threads on open), `safety` 3 (1.0 still alpha; on-disk format unstable) | **Reject** as a dependency. Persistence goes behind a workspace storage trait backed by the `ipc_bus` write-ahead log. Out-of-workspace consumers may supply a sled backend. |
| `redb` | `ambient` 0, `safety` 1, `alloc` 3 (transaction allocation) | **Admit** for `control`; **Extract pattern** (slotted page layout) for `kernel`. |
| `reqwest` | `ambient` 1 (reads proxy env vars by default) | **Admit with conditions** for `control`: clients built with explicit proxy configuration (`.no_proxy()` or injected proxy). |
| full `tokio` runtime | `ambient` 0 when the runtime is built by a bootstrap entrypoint and injected; `alloc` 4 | **Admit** for `control`; **Reject** for `kernel`. |

### 5.4 Admission Record

Each admitted or conditionally admitted dependency is recorded in the PR that adds it with its scores,
verdict, and any conditions. `deny.toml` continues to enforce license and advisory policy;
admission is an additional gate, not a replacement.

---

## 6. Component Graduation

Code entering the workspace from outside (foundry experiments, external sources, generated drafts)
graduates through a fixed sequence. Nothing lands in the shape its origin happened to give it.

1. **Proven**: the capability has run in its origin environment with recorded results. Speculative
   code does not graduate.
2. **Classified** by responsibility:
   - fills a gap: **new component crate**;
   - overlaps an existing component: **upgrade** of that component (never a second implementation);
   - sits adjacent to one: **new component crate** wired through the dispatcher/registry.
3. **Scored**: the component is evaluated against its target profile (Section 2); its dependencies
   pass admission (Section 5).
4. **Behind a workspace trait**: the rest of the workspace touches the component only through a trait
   the workspace defines. Contract tests are written against that trait, not ported from the origin.
5. **Lands inert, then earns traffic**:
   - new components: compiled, tested, and registered, but disabled behind a feature flag or config
     toggle until enabled;
   - upgrades: run in **shadow mode** alongside the existing path on real inputs with outputs diffed.
     Cut over only after the parity threshold agreed *before* the shadow run begins. The old path stays
     revertible by one dispatcher switch for at least one release.
6. **Single source**: once graduated, the origin copy is deleted and the origin consumes the workspace
   crate. Two maintained copies of one capability is a compliance defect.
7. **Ratcheted**: every violation found while porting becomes an `ast_auditor` rule or a tightened
   baseline (Section 7).

The dependency direction is one-way: external tooling may depend on workspace crates; workspace
crates never depend on external tooling.

---

## 7. The Invariant Ratchet

Compliance only moves forward.

- **Circumvention becomes constraint.** When a defect class is found and fixed, the fix is
  crystallized, in order of preference, into: a type (typestate / `PhantomData` markers making the
  illegal state unrepresentable), a compile-time assertion, an `ast_auditor` rule, or an SMT interlock
  constraint in `governance`.
- **Baselines only decrease.** New `ast_auditor` rules land in warning mode with a per-crate violation
  count baseline. CI fails if any count rises. When a crate's count reaches zero, that rule becomes a
  hard block for that crate.
- **Profiles only tighten** without owner sign-off (Section 2.2).
- **No silent exemptions.** Every exemption is one of the contexts listed in Section 1 or an inline,
  reviewable marker (`// SAFETY:`, `// INFALLIBLE:`). Blanket `#[allow]` for a Cratify rule is banned.

### 7.1 Enforcement Backlog

Rules this specification declares but `ast_auditor` does not yet enforce, in adoption order:

1. `[package.metadata.cratify] profile` declared on every crate.
2. Profile-aware unsafe policy (`deny` vs `warn` + `// SAFETY:`).
3. Hot-path markers on `kernel` scan-loop code (currently 4 functions: 2 in `ipc_bus/src/swmr_shm.rs`,
   2 in `dev/emulator_harness/src/reducer.rs`; none in `core/hypervisor` or `compute`).
4. Panics on runtime input (`unwrap` / `expect` / `panic!` outside exempt contexts).
5. Ambient clock reads.
6. Self-started threads and tasks.
7. Baseline-count ratchet mode in `ast_auditor` and `cargo xtask gate`.
8. Retire the five library-code `#[allow(ambient_authority)]` exemptions (`paths/src/lib.rs`,
   `hypervisor/src/{trait_loader,unified_registry}.rs`, `compute/src/{si_packer,translation_dataset}.rs`):
   move discovery into bootstrap entrypoints or inject it. Until then they are the baseline count.
9. Profile dependency direction check (section 2.3).
10. Extend the gate's audit scope to `xtask/`, `sdk/`, and `benches/`.
11. Documentation gate: relative links resolve with exact case, no `file:///` or drive-letter links.

---

## 8. Standardized Systems Nomenclature

Metaphorical monikers are normalized to standard systems terminology with backward-compatible aliases:

| Legacy Moniker | Standard Systems Nomenclature | Primary API / Path |
|---|---|---|
| Harvesting | Source Tree Extraction | `adaptation_engine::extract_source_tree` (`harvest.rs`) |
| Assimilation | Component Onboarding & Conformance | `orchestrator::ComponentOnboardingTask` (`assimilation.rs`) |
| Naturalizing | Invariant Normalization / Conformance | `ast_auditor::review` (`pattern_reviewer.rs`) |
| Quarantine | Staging Sandbox | `dev/legacy_staging/`, `orchestrator::StagedSandbox` |

The **Continuous Conformance & Architectural Pattern Synthesis Engine (CCPSE)** evaluates workspace
code against declarative pattern specifications under `registry/patterns/` via `ast_auditor review`.
