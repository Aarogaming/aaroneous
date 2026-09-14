# Sovereign Workspace Topology & Subsystem Ring Architecture

> **CANONICAL SPECIFICATION**  
> **SCOPE**: Monorepo Directory Topology, Layered Protection Rings, Subsystem Invariants, and Crate Boundaries.  
> **BINDING FOR**: All workspace components (`core/`, `crates/`, `dev/`, `sdk/`).  
> **LAST UPDATED**: 2026-09-12

---

## 1. Executive Architectural Model

The `aaroneous` framework is a sovereign, machine-native synthetic intelligence runtime and Pure Logic Controller (PLC / SCADA) architecture implemented in modern pure Rust (2024 edition). The system is designed from first principles for sub-microsecond determinism, zero-heap hot-path execution, zero ambient authority, and strict hierarchical separation between execution, control, and presentation planes.

```
+---------------------------------------------------------------------------------------+
|                                 RING 4: PRESENTATION LAYER                            |
|                 crates/api | crates/studio_hud (eframe / egui 0.34 UI)               |
+---------------------------------------------------------------------------------------+
                                          |
                                          | Shared GPU Textures / IPC Queries
                                          v
+---------------------------------------------------------------------------------------+
|                         RING 3: TRANSDUCERS, CAPABILITIES & INGRESS                   |
|   crates/capabilities | crates/llm_gateway | crates/platform_bridge                   |
|   crates/adaptation_engine | crates/transpiler | crates/mcp_server                    |
+---------------------------------------------------------------------------------------+
                                          |
                                          | Structured Discrete Frames / Decimated Telemetry
                                          v
+---------------------------------------------------------------------------------------+
|                         RING 2: ORCHESTRATION & CONTROL PLANE                         |
|   crates/orchestrator | crates/orchestration_plane | crates/governance                |
|   crates/autonomic_adaptation | crates/mutation_engine | crates/runtime_monitor       |
+---------------------------------------------------------------------------------------+
                                          |
                                          | Lock-Free SWMR Ring Buffers / Zero-Copy Pod
                                          v
+---------------------------------------------------------------------------------------+
|                         RING 1: REAL-TIME COMPUTE & INTERCONNECT                      |
|   crates/ipc_bus | crates/compute | crates/core-contracts | crates/paths              |
|   crates/si_format | crates/si_ir | crates/wire                                       |
+---------------------------------------------------------------------------------------+
                                          |
                                          | Direct System Memory Maps / RDTSC Ticks
                                          v
+---------------------------------------------------------------------------------------+
|                         RING 0: MICROKERNEL HYPERVISOR HOST                           |
|   core/hypervisor (Tick duty cycle, safety gate, process lifecycle, a_run)            |
+---------------------------------------------------------------------------------------+
```

---

## 2. Workspace Directory Topology & Subsystem Boundaries

The workspace rejects monolithic coupling, dynamic scripting wrappers, and monolithic external runtimes. Every component belongs to a discrete, single-responsibility crate or harness adhering to the **Zero Prefix Stutter** mandate (no `aaroneous_` or `aaroneous-` crate names).

### 2.1 Core Microkernel Host (`core/`)

- **[`core/hypervisor`](file:///d:/Aaroneous/core/hypervisor)**: The root microkernel host.
  - Manages the primary execution duty cycle, high-resolution hardware timers (RDTSC), process supervision, and memory-mapped shared regions (`fb.shmem`).
  - Hosts the microkernel entrypoints: `a_run` (headless CLI orchestrator), `profile_compiler`, and `shm_dump`.
  - Integrates the `OrchestrationDaemon`, executing Step 0 non-blocking drains of assimilation frames, sensory ingest, decision reduction, and telemetry actuation.

### 2.2 Subsystem Crates (`crates/`)

| Crate | Ring | Primary Responsibility & Boundary |
|---|---|---|
| **[`crates/ipc_bus`](file:///d:/Aaroneous/crates/ipc_bus)** | Ring 1 | Lock-free Single-Writer / Multiple-Reader (SWMR) shared-memory bus, LMAX Disruptor ring buffers, persistent WAL, and the Universal Communication Protocol (`universal_protocol.rs`). Defines zero-copy boundary contracts (`UniversalClientRequest`, `UniversalServerBroadcast`, `AssimilationRecord`). |
| **[`crates/compute`](file:///d:/Aaroneous/crates/compute)** | Ring 1 | High-throughput synthetic intelligence computation: `SiForge`, Continuous HiPPO Selective State-Space Model (SSM) recurrence engine ($h_t = \bar{\mathbf{A}} h_{t-1} + \bar{\mathbf{B}} u_t$), Sparse Mixture-of-Experts (MoE) register, bond-graph multi-domain physics compiler, and symplectic Hamiltonian numerical integration. |
| **[`crates/core-contracts`](file:///d:/Aaroneous/crates/core-contracts)** | Ring 1 | Foundational zero-copy Plain Old Data (POD) structs deriving `bytemuck::Pod` and `bytemuck::Zeroable` with explicit struct padding and natural alignment. |
| **[`crates/paths`](file:///d:/Aaroneous/crates/paths)** | Ring 1 | Deterministic workspace path resolution. Implements zero ambient authority: zero calls to `std::env::*` or `.canonicalize()`; all paths are resolved via constructor-injected `WorkspacePathsConfig`. |
| **[`crates/si_format`](file:///d:/Aaroneous/crates/si_format)** | Ring 1 | Canonical `.si` v3 solid-state neural container serialization, 64-byte SIMD cacheline alignment, and CRC32 integrity validation. |
| **[`crates/si_ir`](file:///d:/Aaroneous/crates/si_ir)** | Ring 1 | Computational DAGs, `MachineOpcode` intermediate representations, and type lattice specifications. |
| **[`crates/wire`](file:///d:/Aaroneous/crates/wire)** | Ring 1 | Network protocol serialization, zero-copy packet framing, and network wire adapters. |
| **[`crates/orchestrator`](file:///d:/Aaroneous/crates/orchestrator)** | Ring 2 | Stateful orchestration engine. Manages typestate machine transitions (`AssimilationTask<State>`), pure zero-copy event reducers (`handle_assimilation_event`), priority-constrained backoff scheduling, thread affinity allocation (`SetThreadAffinityMask`), and compaction. |
| **[`crates/orchestration_plane`](file:///d:/Aaroneous/crates/orchestration_plane)** | Ring 2 | High-level system coordination, domain classification, and hypervisor daemon bindings. Funnels incoming frames to reactive reducers. Integrates with `adaptation_engine::PatternRewriter` for deterministic code normalization. |
| **[`crates/governance`](file:///d:/Aaroneous/crates/governance)** | Ring 2 | Hardware thermal monitoring, Z3 SMT non-interference solvers, execution budget safety interlocks, and SI lattice constraint enforcement. Houses canonical `SystemBiology` and metabolic throttling governors. |
| **[`crates/autonomic_adaptation`](file:///d:/Aaroneous/crates/autonomic_adaptation)** | Ring 2 | Online parameter steering, dynamic adaptation matrices ($\Delta W = A_{\text{adapt}} \cdot B_{\text{adapt}}$), hyperparameter optimization, and GGUF model ingestion. |
| **[`crates/runtime_monitor`](file:///d:/Aaroneous/crates/runtime_monitor)** | Ring 2 | Pure zero-copy telemetry forwarding to the hypervisor SWMR ring buffer, thread liveness monitoring, and watchdog supervision. |
| **[`crates/capabilities`](file:///d:/Aaroneous/crates/capabilities)** | Ring 3 | Universal capability registry implementing the `UniversalTool` trait (dual JSON schema for MCP and $\mathbb{R}^{256}$ latent tensor execution for `.si` models). Codebase auditing, security screening, and tool dispatch across the 10 sovereign specialists. |
| **[`crates/llm_gateway`](file:///d:/Aaroneous/crates/llm_gateway)** | Ring 3 | Pure stateless transport layer translating external REST/MCP LLM interfaces to typed internal records. Zero agent state, zero memory leaks. |
| **[`crates/platform_bridge`](file:///d:/Aaroneous/crates/platform_bridge)** | Ring 3 | OS and hardware abstraction: Win32 HID injection, DXGI zero-copy desktop screen capture, WASAPI audio loopback, and hardware telemetry decimation (CAN-bus / HIL) into discrete state frames. |
| **[`crates/adaptation_engine`](file:///d:/Aaroneous/crates/adaptation_engine)** | Ring 3 | Polyglot AST analysis, Comby-style structural pattern rewriter, shadow execution sandboxes, and autonomous self-repair engines. |
| **[`crates/transpiler`](file:///d:/Aaroneous/crates/transpiler)** | Ring 3 | AST parsing and distillation trajectory miner, compiling human action sequences into machine-native graphs. |
| **[`crates/mcp_server`](file:///d:/Aaroneous/crates/mcp_server)** | Ring 3 | Anthropic Model Context Protocol (2024-11-05 spec) HTTP/SSE and JSON-RPC 2.0 server wired directly to `capabilities::ToolRegistry` to expose live tools to Claude Desktop, Cursor, and external agents. |
| **[`crates/omni`](file:///d:/Aaroneous/crates/omni)** | Ring 3 | 3D Spatial Concept Galaxy Graph, Barnes-Hut N-body gravitational clustering, and sub-millisecond approximate nearest neighbor vector search. |
| **[`crates/api`](file:///d:/Aaroneous/crates/api)** | Ring 4 | Public boundary for external visualization and client connectivity. Strictly isolated from raw hardware handles, internal pointer types, and ring buffer buffers. |
| **[`crates/studio_hud`](file:///d:/Aaroneous/crates/studio_hud)** | Ring 4 | Native desktop visual studio and telemetry HUD built on `egui` and `eframe` 0.34. Operates in dual-mode: headless `wgpu` texture sharing vs augmented passthrough. |
| **[`crates/ast_auditor`](file:///d:/Aaroneous/crates/ast_auditor)** | Dev / CI | Static AST analysis engine (dissolved from legacy `cratify`). Enforces the zero ambient authority ban, zero allocation on hot paths, and naming hygiene across all workspace crates. |

### 2.3 Development, Verification & Harness Substrates (`dev/`)

- **[`dev/emulator_harness`](file:///d:/Aaroneous/dev/emulator_harness)**: Golden execution harness, deterministic cycle-accurate instruction trace recorder, and forensic triage substrate.
- **[`dev/legacy_staging`](file:///d:/Aaroneous/dev/legacy_staging)**: Quarantine sandbox for incoming legacy modules and external code bases awaiting formal event-driven assimilation.
- **[`dev/tools`](file:///d:/Aaroneous/dev/tools)**: Diagnostics, maintenance scripts, and offline profiling utilities.

---

## 3. Layered Protection Rings & Isolation Contracts

The 5-ring hierarchy defines unidirectional dependency flows. Lower-numbered rings are more privileged, more deterministic, and subjected to stricter allocation and safety constraints.

```
Ring 0 (Hypervisor) ──► Ring 1 (Interconnect) ──► Ring 2 (Control) ──► Ring 3 (Ingress) ──► Ring 4 (UI)
       ▲                                                                                     │
       │                                                                                     │
       +─────────────────────── Zero Raw Pointer Leakage ────────────────────────────────────+
```

### 3.1 Protection Ring Rules

1. **Dependency Inversion**: Higher rings may depend on lower rings; lower rings MUST NEVER depend on higher rings. (e.g., `core/hypervisor` and `crates/ipc_bus` never import `crates/api` or `crates/studio_hud`).
2. **Boundary Sanitization**: All data crossing from Ring 3/4 into Ring 1/2 must pass through typed zero-copy Pod structs (`#[repr(C)]`) or validated JSON schemas.
3. **No Pointer Leakage**: Raw memory addresses, direct CAN frame descriptors, or GPU texture handles must never be exposed beyond their containing ring.
4. **Hot-Path Isolation**: Rings 0 and 1 forbid heap allocations (`String`, `Vec`, `Box`, `format!`), dynamic dispatch (`dyn Trait`), and blocking synchronization (`std::sync::Mutex`).

---

## 4. Architectural Invariants

### 4.1 Pure Logic Controller (PLC) Invariant
Every core engine operates on a strict cyclical scan:
$$S_{t+1} = f(S_t, I)$$
- **Phase 1 (Input Acquisition)**: Ingest input frames from hardware, IPC, or timers into fixed stack buffers.
- **Phase 2 (State Reduction)**: Compute state delta using pure functions with zero heap allocation and zero side effects.
- **Phase 3 (Actuation & Telemetry)**: Dispatch output commands and emit immutable broadcast packets over ring buffers.

### 4.2 Zero Ambient Authority
- Explicitly banned across all production crates: `std::env::var`, `std::env::var_os`, `std::env::set_var`, `std::env::remove_var`, `std::env::temp_dir`, `std::env::current_dir` (outside CLI bootstrap `main.rs`), and `.canonicalize()`.
- All paths, credentials, and hardware endpoints must be passed explicitly via typed configuration structures (`WorkspacePathsConfig`).
- All tests must use `tempfile::tempdir()` for filesystem sandboxing.

### 4.3 Zero Prefix Stutter
- No crate directory, Rust module, internal struct, or IPC channel may prepend `aaroneous_` or `aaroneous-`.
- Generic systems nomenclature is required: `hypervisor`, `paths`, `wire`, `api`, `orchestrator`, `compute`, `platform_bridge`.
- Metric namespaces exported to external collectors (Prometheus / OpenTelemetry) are explicitly exempt (e.g., `aaroneous_tick_duration_seconds`).
