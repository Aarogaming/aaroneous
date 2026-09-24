# System Architecture Specification & Component Framework Invariants

> **TIER 2 ARCHITECTURAL MASTER REFERENCE**  
> **SCOPE**: Rust Component Framework Architecture, Deterministic State Transition Machines, Subsystem Topology, and Boundary Isolation.  
> **BINDING FOR**: All workspace components (`core/hypervisor`, `crates/orchestrator`, `crates/platform_bridge`, `crates/ipc_bus`, `crates/capabilities`, `crates/governance`, etc.).  
> **CORE IDENTITY**: Aaroneous is **NOT** a monolithic app. It is a strict, type-safe **Rust Component Framework** for building interchangeable, safe, zero-allocation execution blocks & plugins.  
> **LAST UPDATED**: 2026-09-14

---

## 1. The Core Architectural Invariant (PLC Model & Component Blocks)

The `aaroneous` workspace implements a deterministic, real-time, low-latency **Rust Component Framework** operating on a Pure Logic Controller (PLC / SCADA) execution model. Every component block operates on a strict cyclical scan or discrete step execution paradigm, designed to be completely modular, interchangeable, and hot-swappable.

### 1.1 Pure State Transition Machines

All domain kernels (`core/hypervisor`, `crates/compute`, `crates/orchestrator`, `crates/orchestration_plane`, `crates/llm_gateway`, etc.) MUST be implemented as pure, deterministic state machines:

$$S_{t+1} = f(S_t, I)$$

- **Deterministic Function**: Given prior state $S_t$ and input payload $I$, the output state $S_{t+1}$ and emitted events MUST be deterministic and reproducible.
- **Zero Side-Effects in Reducers**: Domain engines MUST NOT perform side effects, background network I/O, file system reads, or hidden async task launches during state reduction.
- **Three-Phase Scan Separation**:
  1. **Input Acquisition (Phase 1)**: Poll hardware, network, IPC, or timers into fixed-size, stack-allocated input frames.
  2. **State Reduction (Phase 2)**: Execute pure state transition $S_{t+1} = f(S_t, I)$. No I/O, no blocking. In `kernel`-profile crates, no heap allocation (reducers are marked `#[hot_path]`).
  3. **Telemetry & Actuation Output (Phase 3)**: Emit telemetry records to lock-free ring buffers and dispatch actions over bounded channels.

```
       +---------------------------------------------+
       |           Input Acquisition (I/O)           |
       +---------------------------------------------+
                              |
                              v  Input Frame (I)
       +---------------------------------------------+
       |   Pure State Transition Reducer:            |
       |             S_{t+1} = f(S_t, I)             |
       |   * ZERO side effects                       |
       |   * ZERO heap allocations                   |
       |   * ZERO ambient authority                  |
       +---------------------------------------------+
                              |
                              v  State Delta (S_{t+1}) + Events
       +---------------------------------------------+
       |      Telemetry & Actuation Output (I/O)     |
       +---------------------------------------------+
```

---

## 2. Constructor Dependency & Configuration Injection

To preserve determinism and eliminate ambient authority:
- **Explicit Injection**: All dependencies, static buffers, communication handles, and configuration parameters MUST be passed explicitly into constructor functions (e.g., `Engine::new(config, buffer)`).
- **No Self-Instantiation**: Sub-components, inner structs, or domain logic must NEVER instantiate their own external dependencies or construct global services.
- **No Ambient Reads**: Sub-components must never read external state, system clocks, file descriptors, or environment settings outside what is explicitly provided via constructor or tick inputs.
- **No Self-Started Execution**: Library code never spawns threads or async tasks on its own authority; it receives an injected executor handle or runs under `orchestrator::Supervisor`.

These rules form part of the universal floor that applies to every crate regardless of compliance profile ([CRATIFY_SPEC.md](CRATIFY_SPEC.md) section 1).

---

## 3. Concurrency & Memory Model

The rules below are mandatory for `kernel`-profile crates (`core/hypervisor`, `ipc_bus`, `compute`, `wire`, `si_format`, `si_ir`, `platform_bridge`, `runtime_monitor`, `dev/emulator_harness`). `control`-profile crates may use locks and allocation off the scan loop; see [CRATIFY_SPEC.md](CRATIFY_SPEC.md) section 2.

- **SWMR (Single-Writer / Multiple-Reader)**: Atomic sequence indexing over pre-allocated static ring buffers (`SwrnRingBuffer`).
- **No Mutexes on Hot Paths**: `std::sync::Mutex`, `parking_lot::Mutex`, and `RwLock` are prohibited in telemetry, state extraction, or IPC hot paths to avoid thread parking latency spikes.
- **No Deferred Static Initialization**: `OnceLock` and `lazy_static` for runtime state are banned in every non-`tooling` crate. Initialize all buffers statically or at startup before starting the control loop.

---

## 4. Canonical Subsystem Architecture Specifications

The unified foundational specification is codified in **[Master Architecture Specification](./architecture/MASTER_ARCHITECTURE.md)**, with detailed domain deep-dives in the [`docs/architecture/`](./architecture/) directory:

1. **[Master Architecture Specification](./architecture/MASTER_ARCHITECTURE.md)** (Tier 1 Master Blueprint)
2. **[Workspace Topology & Subsystem Ring Architecture](./architecture/architecture_overview.md)**  
   *Scope*: Monorepo layout, 5-ring layered protection model, single-responsibility crate mapping, and the Zero Prefix Stutter rule.
3. **[Event-Driven Asset Assimilation & Wire Geometry](./architecture/component_onboarding_specification.md)**
   *Scope*: 360-byte `AssimilationRecord` binary contracts (`Pod` / `Zeroable`), typestate machine (`AssimilationTask<State>`), zero-copy reactive reducer (`handle_assimilation_event`), and hypervisor Step 0 non-blocking drain.
4. **[LLM Manager & Priority-Constrained Scheduler](./architecture/llm_manager_scheduler.md)**  
   *Scope*: Stateless transducer interface ($\mathcal{T}: \Sigma^* \times \mathcal{G} \to \Omega$), transport vs. control plane separation (`llm_gateway` vs. `orchestrator`), dynamic priority heap (`Critical`, `Standard`, `Background`), and jittered exponential backoff.
5. **[Scale-Invariant Dynamics & Physics Compilation](./architecture/physics_compiler_dynamics.md)**  
   *Scope*: Unified multi-domain Bond-Graph Duality ($P(t) = e(t) \cdot f(t)$), Symplectic Hamiltonian numerical integration ($d\mathcal{H}/dt \approx 0$), analytical matrix-exponential fast-forwarding, and thermodynamic state freezing at equilibrium ($dG \approx 0$).
6. **[Decoupled Human Node, HIAL & Intent Mirror](./architecture/human_interface_intent_mirror.md)**  
   *Scope*: Human-Interface Abstraction Layer, Socratic vector pinning (Invariants, Dependencies, Trade-offs), Intent DAG compilation, 3-Option Intent Mirror calibration, and presentation layer memory isolation (`crates/api` / `crates/studio_hud`).

---

## 5. Supplementary Framework References

- **[Architectural Constraints & Dependency Injection](./ARCHITECTURAL_CONSTRAINTS.md)**: Concrete anti-patterns and constructor injection guidelines.
- **[Cratify Compliance Specification (v2)](./CRATIFY_SPEC.md)**: Universal floor, compliance profiles, dependency admission, component graduation, the invariant ratchet, and `ast_auditor` enforcement status.
- **[Forensic Ingestion Protocol (RFC-0005)](./FORENSICS_RFC0005.md)**: Quarantine containment and kernel extraction for legacy staged code.
