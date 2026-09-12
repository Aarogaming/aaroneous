# Repository Operating Directives & Agent Constitution (`aaroneous`)

> **STATUS**: BINDING MACHINE CONTRACT & CONSTITUTION  
> **APPLIES TO**: ALL AUTONOMOUS AGENTS (OpenCode, Qwen, Claude, LM Studio, Human Contributors)  
> **WORKSPACE**: `aaroneous` (Pure Logic Controller / SCADA architecture in Rust 2024)  
> **LAST UPDATED**: 2026-09-11

---

## 1. The Core Architectural Invariant (PLC Model)

The `aaroneous` monorepo implements a deterministic, real-time, low-latency **Pure Logic Controller (PLC / SCADA)** architecture. Every core component operates on a strict cyclical scan or discrete step execution paradigm.

### 1.1 Pure State Transition Machines
All domain kernels (`core/hypervisor`, `crates/compute`, `crates/orchestration_plane`, `crates/llm_gateway`, etc.) MUST be implemented as pure, deterministic state machines:
$$S_{t+1} = f(S_t, I)$$
- Given prior state $S_t$ and input payload $I$, the output state $S_{t+1}$ and emitted events MUST be deterministic and reproducible.
- Domain engines MUST NOT perform side effects, background network I/O, or hidden async task launches during state reduction.
- State evaluation MUST be strictly isolated from input acquisition and telemetry output.

### 1.2 Constructor Dependency & Config Injection Only
- **Explicit Injection**: All dependencies, static buffers, communication handles, and configuration parameters MUST be passed explicitly into constructor functions (e.g., `Engine::new(config, buffer)`).
- **No Self-Instantiation**: Sub-components, inner structs, or domain logic must NEVER instantiate their own external dependencies or construct global services.
- **No Ambient Reads**: Sub-components must never read external state, system clocks, file descriptors, or environment settings outside what is explicitly provided via constructor or tick inputs.

---

## 2. The Zero-Ambient-Authority Rule

Ambient authority compromises determinism, introduces hidden coupling, and breaks sandbox guarantees. It is strictly eradicated across the entire workspace.

### 2.1 Explicitly Banned Functions
The following calls are **strictly prohibited** in all domain modules, library crates, and unit tests:
- `std::env::var` / `std::env::var_os`
- `std::env::set_var` / `std::env::remove_var`
- `std::env::temp_dir`
- `std::env::current_dir` (outside CLI bootstrap entrypoints)

### 2.2 Configuration Struct Injection
- All file paths, endpoint URIs, token handles, timeouts, and network credentials MUST arrive strictly via typed configuration structs:
  - e.g., `HttpServiceConfig`, `WorkspacePathsConfig`, `ShmSegmentConfig`.
- Configuration structs are loaded exclusively at the process entrypoint (e.g., `main.rs` of a top-level binary) and passed immutably down the dependency graph.
- Unit and integration tests must construct explicit test configs; tests MUST NEVER rely on the host machine's environment variables or ambient temporary directories.

---

## 3. Memory & Layout Invariants

Every cycle counts. Hot paths, frame ingestors, and shared memory ring buffers operate under real-time constraints.

### 3.1 Zero Dynamic Heap Allocation on Hot Paths
Hot execution paths (specifically in `core/hypervisor`, `crates/ipc_bus`, `crates/compute`, `dev/emulator_harness`, and frame ingestors) must NEVER allocate heap memory:
- **Forbidden Types & Macros**: `String`, `Vec`, `Box`, `format!`, `to_string()`, unbounded collections (`HashMap`, `BTreeMap`).
- **Required Primitives**: Stack-allocated arrays (`[T; N]`), bounded ring buffers (`SwrnRingBuffer`), fixed-size slices, and pre-allocated static arena buffers.
- **No Allocation in Loops**: Any allocation inside a tick/scan loop or packet ingest handler is considered a critical invariant violation.

### 3.2 Memory Geometry & ABI Safety
All boundary types, IPC messages, shared memory structs, and event records MUST:
- Use `#[repr(C)]` to guarantee fixed layout and cross-language ABI compatibility.
- Derive `bytemuck::Pod` and `bytemuck::Zeroable` using macro attributes.
  - **BAN**: Manual `unsafe impl Pod` or `unsafe impl Zeroable` is strictly forbidden.
- Include explicit padding fields (e.g., `pub _pad0: u16`, `pub reserved: u32`) to ensure natural alignment and eliminate implicit compiler padding bytes.

### 3.3 Concurrency Model
- **SWMR (Single-Writer / Multiple-Reader)**: Atomic sequence indexing over pre-allocated static ring buffers.
- **No Mutexes on Hot Paths**: `std::sync::Mutex`, `parking_lot::Mutex`, and `RwLock` are prohibited in telemetry, state extraction, or IPC hot paths.
- **No Deferred Static Initialization**: `OnceLock` and `lazy_static` for shared state are banned. Initialize all buffers statically or at startup before starting the control loop.

---

## 4. Naming Conventions

Maintain strict aesthetic, semantic, and systemic hygiene. Code should read like an industrial systems platform.

### 4.1 Strict Ban on Workspace Prefix Stutter
- Do NOT prepend `aaroneous_` or `aaroneous-` to crates, packages, internal types, modules, or IPC channels.
- *Examples*:
  - **INCORRECT**: `aaroneous_hypervisor`, `aaroneous_compute`, `struct AaroneousFrame`, channel `"aaroneous/events"`.
  - **CORRECT**: `hypervisor`, `compute`, `struct Frame`, channel `"events"`.
- **Exception**: Global external telemetry/metrics namespaces (e.g., Prometheus metric names such as `aaroneous_tick_duration_seconds`) are the sole approved exception.

### 4.2 Generic Systems Engineering Terminology Only
- Obscure monikers, inside jokes, and punny names (e.g., `a_run`, `aaron_loop`, `magic_sync`) are strictly banned.
- Use precise, standard systems engineering terminology:
  - `hypervisor`, `paths`, `wire`, `hud`, `api`, `bridge`, `controller`, `ingestor`, `pipeline`.

---

## 5. Banned Anti-Patterns

| Anti-Pattern | Operational Risk | Enforcement Mechanism |
|---|---|---|
| Ambient `std::env::*` | Nondeterminism, security leakage | Cratify AST check & compiler ban |
| Manual `unsafe impl Pod/Zeroable` | Undefined behavior, invalid bit patterns | Cratify invariant rule |
| `std::sync::Mutex` / `RwLock` on hot paths | Deadlocks, thread parking latency spikes | Hot path linting |
| `OnceLock` / `lazy_static` for runtime state | Deferred init UB, nondeterministic latency | Static check |
| `unsafe transmute` on statics/unaligned data | Instant UB and memory corruption | Cratify audit |
| `todo!()` / `unimplemented!()` in committed code | Silent runtime crashes and denial-of-service | Negative contract tests (`git grep`) |
| `.unwrap()` / `.expect()` on hot paths or errors | Process panics; zero fault recovery | Result propagation enforcement |
| Workspace prefix stutter (`aaroneous_*`) | Namespace pollution and redundancy | Cratify naming lint |

---

## 6. Forensic Ingestion Protocol (RFC-0005)

When processing legacy modules staged in `dev/legacy_staging/`:

1. **Containment**: Do NOT refactor or fix legacy code in place. Keep staged in `dev/legacy_staging/<artifact_id>/`.
2. **Trace & Isolate**: Route execution through `dev/emulator_harness` to extract 40-byte `TraceEvent` streams:
   ```rust
   use emulator_harness::{TraceEvent, extract_state_delta};
   
   let events = /* capture from emulator */;
   let delta = extract_state_delta(&events, target_addr)?;
   ```
3. **Kernel Extraction**: Use `extract_state_delta` to isolate the minimal deterministic state transition needle.
4. **Failure Analysis**: Document anti-patterns in `docs/forensics/<id>_<name>.md`.
5. **Synthesis**: Rebase the clean, certified kernel into `crates/<target>/` using zero-copy fixed buffers.
6. **Codification**: Record post-mortem, add Cratify rule in `crates/cratify/src/rules/`, and write a regression test in `tests/negative_contracts/`.

---

## 7. The Verification & Gate Protocol

Agents must NEVER declare work complete based solely on `cargo check`. A passing typecheck is only the baseline entry ticket; it does not prove structural invariants, layout safety, or zero-allocation guarantees.

### 7.1 Mandatory Gate Protocol (Sequential Execution)

Every agent MUST run and verify the following sequence before completing a task:

```bash
# 1. Full Workspace Compilation (all targets, tests, benches)
cargo check --workspace --all-targets

# 2. Workspace Test Suite (Functional determinism)
cargo test --workspace

# 3. Structural & Semantic Invariant Audit (MUST EXIT 0)
cargo run -p cratify -- audit core/ crates/ dev/

# 4. Zero-Stub & Soundness Inspection (Must return empty)
! git grep -n -E "(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)" -- "crates/" "core/" "dev/"

# 5. Golden Dogfooding Harness Verification
cargo test -p emulator_harness

# 6. Full Self-Verification Gate Script
bash scripts/agent_check.sh
```

### 7.2 Gate Violations
- Any agent that skips `cratify`, leaves `todo!()`/`unimplemented!()` stubs, masks errors with `.unwrap()` in production paths, or introduces prefix stutter is in direct violation of the repository contract and will fail the gate.
