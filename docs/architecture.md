# System Architecture Specification & SCADA/PLC Invariants

> **TIER 2 ARCHITECTURAL REFERENCE**  
> **SCOPE**: Pure Logic Controller (PLC / SCADA) Architecture, Deterministic State Transition Machines, and Boundary Isolation.  
> **BINDING FOR**: All workspace kernels (`core/hypervisor`, `crates/compute`, `crates/orchestration_plane`, `crates/llm_gateway`, etc.).

---

## 1. The Core Architectural Invariant (PLC Model)

The `aaroneous` monorepo implements a deterministic, real-time, low-latency **Pure Logic Controller (PLC / SCADA)** architecture. Every core component operates on a strict cyclical scan or discrete step execution paradigm.

### 1.1 Pure State Transition Machines

All domain kernels (`core/hypervisor`, `crates/compute`, `crates/orchestration_plane`, `crates/llm_gateway`, etc.) MUST be implemented as pure, deterministic state machines:

$$S_{t+1} = f(S_t, I)$$

- **Deterministic Function**: Given prior state $S_t$ and input payload $I$, the output state $S_{t+1}$ and emitted events MUST be deterministic and reproducible.
- **Zero Side-Effects in Reducers**: Domain engines MUST NOT perform side effects, background network I/O, file system reads, or hidden async task launches during state reduction.
- **Three-Phase Scan Separation**:
  1. **Input Acquisition (Phase 1)**: Poll hardware, network, IPC, or timers into fixed-size, stack-allocated input frames.
  2. **State Reduction (Phase 2)**: Execute pure state transition $S_{t+1} = f(S_t, I)$. No I/O, no blocking, no heap allocation.
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

---

## 3. Concurrency & Memory Model

- **SWMR (Single-Writer / Multiple-Reader)**: Atomic sequence indexing over pre-allocated static ring buffers (`SwrnRingBuffer`).
- **No Mutexes on Hot Paths**: `std::sync::Mutex`, `parking_lot::Mutex`, and `RwLock` are prohibited in telemetry, state extraction, or IPC hot paths to avoid thread parking latency spikes.
- **No Deferred Static Initialization**: `OnceLock` and `lazy_static` for runtime state are banned. Initialize all buffers statically or at startup before starting the control loop.
