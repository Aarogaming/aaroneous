# Sovereign Machine-Native Master Architecture (`aaroneous`)

> **STATUS**: CANONICAL UNIFIED ARCHITECTURAL SPECIFICATION (TIER 1 MASTER SPEC)  
> **STANDARD**: Sterile Execution Plane (SEP) / Zero-Allocation Substrate / SCADA-PLC Invariants  
> **WORKSPACE**: `aaroneous` (Pure Rust 2024 Edition)  
> **CORE IDENTITY**: Aaroneous is **NOT** a monolithic app. It is a strict, type-safe **Rust Component Framework** for building interchangeable, safe, zero-allocation execution blocks & plugins.  
> **LAST UPDATED**: 2026-09-14

---

## Executive Architectural Summary

The **Aaroneous** project is an atomic, type-safe **Rust Component Framework** for building interchangeable, safe, zero-allocation execution blocks and plugins operating on a Pure Logic Controller (PLC / SCADA) execution substrate. Designed from first principles to eliminate garbage-collection jitter, Python interpreter latency, and unconstrained agent loops, Aaroneous establishes a deterministic, memory-safe execution substrate governed by 6 unified architectural pillars:

1. **Workspace Topology & Protection Rings**: Independent plug-and-play component blocks, pure atomic modularity, `core/hypervisor` microkernel host, and a strict ban on `aaroneous_` namespace stutters.
2. **Event-Driven Asset Assimilation**: 360-byte zero-copy `AssimilationRecord`, `AssimilationTask<State>` compile-time typestate machine, and Step 0 non-blocking microkernel duty cycle drain.
3. **LLM Manager & Priority-Constrained Scheduler**: Decoupled stateless transport (`crates/llm_gateway`) vs stateful control (`crates/orchestrator`), mathematical transducer loop ($\mathcal{T}: \Sigma^* \times \mathcal{G} \to \Omega$), dynamic `PriorityHeap`, and jittered priority-weighted exponential backoff.
4. **Scale-Invariant Dynamics & Physics Compilation**: Multi-domain Bond-Graph Effort/Flow duality, symplectic Hamiltonian integration ($d\mathcal{H}/dt \approx 0$), analytical fast-forwarding, and thermodynamic state freezing at equilibrium ($dG \approx 0$).
5. **Presentation Layer, 4-Ring Viewport & Decoupled Ingress**: `crates/api` / `crates/studio_hud` isolated on `egui`/`eframe` 0.34, headless `wgpu` texture sharing, and `crates/platform_bridge` decimating high-frequency CAN/HIL telemetry to `ipc_bus`.
6. **Decoupled Human Node & Intent Mirror**: Human-Interface Abstraction Layer (HIAL), Socratic vector pinning (Invariants, Dependencies, Trade-offs), Intent DAG compilation, and low-friction 3-Option Intent Mirror calibration.

---

## Component Toolbelt and Operation Through Use

Aaroneous is a component toolbelt whose contributors use it to perform real work
as they build it. The development loop is: select an existing capability,
operate it on an authorized task, retain the actual outcome, improve the
responsible component where use exposes a limitation, and exercise it again.
Agent and human workflows are consumers of the framework and sources of
concrete evidence for its next improvements.

This loop progresses through overlapping cycles, like a spiral staircase.
Each cycle returns to the shared center: the task's intent, component contracts,
operating standards, and observed evidence. It then advances from what the
previous cycle established. Grounding is a recurring part of motion: reuse
retains proven behavior while the next step extends capability.

The owner's stepped double-helix analogy describes two intertwined activities:
operating the tools and building them. Concrete tasks, observations, reviewed
changes, and verification connect those activities at each step. Their overlap
lets experience carry forward between layers and cycles. This is an intuition
for how contributors work, not a prescribed data structure or execution topology.

The owner's "4096 maze" analogy expresses the breadth of possible routes
through the component system; it specifies no literal route count. Each
component should provide enough depth to be useful independently, with explicit
contracts that allow callers to select an entry point, intermediate layers, and
an exit point. The same capability may be reached through a CLI, API, UI, agent,
or another component. Different compositions can serve different operating
methods while respecting protection rings, dependency direction, three-phase
scan separation, and the applicable allocation and authority rules.

The platform should carry an operator's task, context, tools, permissions, and
review obligations across interfaces and local or cloud model selections.
Model output remains input to the governed execution path. Changing a model or
route does not itself grant additional authority or establish successful
execution. This is architectural intent; each implemented route requires its
own retained evidence before it is described as operational.

Aaroneous and its companion devtools are developed through use together.
Devtools can exercise, evaluate, and help prove components; reusable behavior
graduates into the appropriate existing workspace component under
[CRATIFY_SPEC section 6](../CRATIFY_SPEC.md#6-component-graduation). Aaroneous
remains independent of devtools, and graduation preserves a single source of
implementation. External libraries or inference servers may supply an admitted
adapter behind a workspace contract; they do not replace the framework's
composition and governance responsibilities.

Contributor practice is documented in [CONTRIBUTING.md](../../CONTRIBUTING.md#use-the-tools-while-building-them)
and the agent operating directives in [AGENTS.md](../../AGENTS.md#operating-principle-use-and-improve-the-component-toolbelt).

### Localized maintenance work

The local model is a maintenance operator for bounded workspace work. Its
recurring workload begins with artifact inventory and index freshness and
extends to audit-result classification, internal compliance triage, and
housekeeping briefs. Program tools acquire facts, enforce action bounds, and
produce mechanical check results. The model interprets those declared results;
its findings and recommendations retain a separate review status.

Frontier operators consume these briefs for broader synthesis, difficult
changes, and independent review. Both participate through the same toolbelt
and authority contracts. Cache cleanup becomes an executable capability only
when explicit roots, retention rules, and authorized tool actions are defined.
Inventory counts, hashes, and model explanations alone establish neither
behavioral correctness nor compliance.

### Revalidation and evolution of the standard

The entire framework must eventually be re-evaluated through its own current
compliance gates. As components and standards evolve, repeat that evaluation
against the declared revision and scope. Findings from actual use and local
maintenance should improve the testing system: preserve a reproducer, add a
meaningful regression or adversarial case, and crystallize the validated rule
into the existing gate or compliance ratchet. Fixing a component and improving
the check that detects its defect are connected steps in the same cycle.

The verification tools themselves need repeated adversarial evaluation from
new perspectives. Challenge missed violations, false passes, scope exclusions,
boundary cases, and assumptions in the auditors and gate orchestration. Use
isolated fixtures and explicit authority for vulnerability testing; retain the
tested revision, command, result, and limits. Local maintenance can classify
evidence and suggest cases; independent operators verify the findings and
strengthen the checks. A passing gate establishes the coverage exercised at
that revision, and ongoing challenges continue to test its blind spots.

Re-version Aaroneous and Cratify to a newer standard only after the standard
and migration criteria are explicit, the applicable framework has been
revalidated, remaining exceptions are recorded, and independent review and
release authorization are obtained. Version advancement records demonstrated
conformance; it does not substitute for that work or erase retained baselines.

## Pillar 1: Component Framework Topology & Layered Protection Rings

The repository operates as a single Cargo workspace consisting of discrete, single-responsibility component crates, completely devoid of directory or package naming prefixes (`aaroneous_*` ban).

```text
+---------------------------------------------------------------------------------------+
|                                 RING 4: PRESENTATION LAYER                            |
|                 crates/api | crates/studio_hud (eframe / egui 0.34 UI)               |
+---------------------------------------------------------------------------------------+
                                          │
                                          │ Shared GPU Textures / IPC Telemetry Snapshots
                                          ▼
+---------------------------------------------------------------------------------------+
|                         RING 3: TRANSDUCERS, CAPABILITIES & INGRESS                   |
|   crates/capabilities | crates/llm_gateway | crates/platform_bridge                   |
|   crates/adaptation_engine | crates/transpiler | crates/mcp_server | crates/omni      |
+---------------------------------------------------------------------------------------+
                                          │
                                          │ Discrete Structured Frames / Decimated Telemetry
                                          ▼
+---------------------------------------------------------------------------------------+
|                         RING 2: ORCHESTRATION & CONTROL PLANE                         |
|   crates/orchestrator | crates/orchestration_plane | crates/governance                |
|   crates/autonomic_adaptation | crates/runtime_monitor                                |
+---------------------------------------------------------------------------------------+
                                          │
                                          │ Lock-Free SWMR Ring Buffers / Zero-Copy Pod
                                          ▼
+---------------------------------------------------------------------------------------+
|                         RING 1: REAL-TIME COMPUTE & INTERCONNECT                      |
|   crates/ipc_bus | crates/compute | crates/core-contracts | crates/paths              |
|   crates/si_format | crates/si_ir | crates/wire | crates/scratchpad                   |
+---------------------------------------------------------------------------------------+
                                          │
                                          │ Direct System Memory Maps / RDTSC Ticks
                                          ▼
+---------------------------------------------------------------------------------------+
|                         RING 0: MICROKERNEL HYPERVISOR HOST                           |
|   core/hypervisor (Execution duty cycle, safety gate, process supervision, a_run)     |
+---------------------------------------------------------------------------------------+
```

### 1.1 Invariant Rules of the Protection Rings

- **Unidirectional Dependency Flow**: Lower-numbered rings are more privileged and deterministic. Library crates in lower rings never import crates from higher rings (verified 2026-09-23 at `6321a63`). The `core/hypervisor` binaries are the composition root and are exempt: they wire every ring together.
- **Zero Ambient Authority**: Banned functions (`std::env::var`, `std::env::temp_dir`, `std::env::current_dir`, `.canonicalize()`) are rejected at compile time by `crates/ast_auditor`. All configurations are injected via typed constructors.
- **Compliance Profiles**: Ring placement governs dependency direction; the compliance profile declared in each crate's `[package.metadata.cratify]` governs which invariants apply ([CRATIFY_SPEC.md](../CRATIFY_SPEC.md)).
- **Zero Heap on Hot Paths**: `#[hot_path]` code in `kernel`-profile crates strictly forbids dynamic allocation (`String`, `Vec`, `Box`, `format!`). Data transits via stack buffers (`[u8; N]`) and zero-copy plain-old-data contracts (`bytemuck::Pod` + `Zeroable`).

---

## Pillar 2: Event-Driven Asset Assimilation & Wire Geometry

Imperative file-traversal harvesting (`cratify::harvest`) has been replaced by a reactive, zero-copy event stream operating over `crates/ipc_bus`.

```text
       UniversalClientRequest (544 bytes)
             [UcpRequestType::AssimilationEvent]
                            │
                            ▼
              ipc_bus SWMR Ring Buffer
                            │
                            ▼ (try_recv / Step 0 non-blocking drain)
           core/hypervisor::OrchestrationDaemon
                            │
                            ▼ handle_assimilation_event(&bytes)
          crates/orchestrator::assimilation
   ┌────────────────────────────────────────────────────────┐
   │            Typestate Machine Transition:               │
   │  Idle ──► Quarantined ──► Auditing ──► Synthesizing   │
   │                                             │          │
   │  Committed ◄────────── Certifying ◄─────────┘          │
   │       ▲                      │                         │
   │       │                      ▼ (retries >= max)        │
   │       └──────────────── Rejected                       │
   └────────────────────────────────────────────────────────┘
                            │
                            ▼ to_broadcast()
          UniversalServerBroadcast (304 bytes)
             [UcpBroadcastType::AssimilationState]
```

### 2.1 Wire Geometry: `AssimilationRecord` (360 Bytes)

Defined in [`crates/ipc_bus/src/universal_protocol.rs`](../../crates/ipc_bus/src/universal_protocol.rs):

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct AssimilationRecord {
    pub source_id: [u8; 16],          // Offset: 0,   Size: 16 bytes
    pub phase: u32,                   // Offset: 16,  Size: 4 bytes
    pub retries: u32,                 // Offset: 20,  Size: 4 bytes
    pub started_at_us: u64,           // Offset: 24,  Size: 8 bytes
    pub mount_path: FixedString256,   // Offset: 32,  Size: 260 bytes
    pub ir_hash: FixedString64,       // Offset: 292, Size: 68 bytes
}
```

- **Natural Alignment**: Aligned to 8 bytes ($360 \pmod 8 = 0$); zero trailing padding required.
- **Envelope Framing**: Embedded within `UniversalClientRequest` (544 bytes) and reported via `UniversalServerBroadcast` (304 bytes).

### 2.2 Typestate State Reducer

In [`crates/orchestrator/src/assimilation.rs`](../../crates/orchestrator/src/assimilation.rs):
- Compile-time lifecycle: `AssimilationTask<State>` over states `Idle`, `Quarantined`, `Auditing`, `Synthesizing`, `Certifying`, `Committed`, `Rejected`.
- Zero-allocation hot reducer: `pub fn handle_assimilation_event(bytes: &[u8]) -> Result<AssimilationRecord, AssimilationError>` (`#![deny(unsafe_code)]`).
- **Hypervisor Step 0 Drain**: `core/hypervisor/src/orchestration_daemon.rs` non-blockingly drains pending frames via `try_recv()` at the start of every microkernel cycle, guaranteeing that assimilation processing never stalls the real-time duty cycle.

---

## Pillar 3: LLM Manager & Priority-Constrained Scheduler

External language models are decoupled from internal system state and treated strictly as **stateless mathematical transducers**:

$$\mathcal{T}: \Sigma^* \times \mathcal{G} \longrightarrow \Omega$$

Where $\Sigma^*$ is the token context window, $\mathcal{G}$ is a rigid grammar/JSON schema, and $\Omega$ is the validated structured output.

```text
+─────────────────────────────────────────────────────────────────────────────+
|                    crates/orchestrator (STATEFUL CONTROL PLANE)             |
|  • Goal Trees & Task DAGs                                                   |
|  • Dynamic Priority Heap (`Critical`, `Standard`, `Background`)             |
|  • Jittered Exponential Backoff Scheduler                                   |
|  • Context Window Assembly & Token Budget Allocation                        |
+─────────────────────────────────────────────────────────────────────────────+
                                       │
                                       │ Stateless Requests (Prompt + Schema)
                                       ▼
+─────────────────────────────────────────────────────────────────────────────+
|                    crates/llm_gateway (STATELESS TRANSPORT PLANE)           |
|  • HTTP/REST & Streaming MCP Wire Adapters                                  |
|  • Zero Agent State & Zero Session Retention                                |
|  • Request Transformation & Header Authentication                           |
+─────────────────────────────────────────────────────────────────────────────+
```

### 3.1 Dynamic Priority Heap & Weighted Exponential Backoff

Outbound tasks are managed in `crates/orchestrator` across three priority tiers:
1. **`Critical` ($W_p = 4.0$)**: AST compile failures, safety interlocks, kernel panic remediation.
2. **`Standard` ($W_p = 1.0$)**: Feature code generation, unit tests, Socratic queries.
3. **`Background` ($W_p = 0.25$)**: Offline `.si` model distillation, habit mining, embedding clustering.

Retries are scheduled according to the dynamic priority-weighted formula:

$$\text{Delay}(p, k) = \min\left(D_{\text{max}},\ \left(D_{\text{base}} \cdot 2^k\right) \cdot \frac{1}{W_p}\right) + J$$

- $D_{\text{base}} = 100\,\text{ms}$, $k \le 5$, $D_{\text{max}} = 30\,\text{s}$.
- $J$: Deterministic pseudorandom jitter ($0 \le J \le 0.2 \cdot D_{\text{base}}$).
- **Event Escalation**: Any downstream `CompilationFailure` or `InvariantViolation` immediately escalates the task to `Critical` ($W_p = 4.0$), preempting background workloads.

---

## Pillar 4: Scale-Invariant Dynamics & Physics Compilation

In [`crates/compute`](../../crates/compute), physical systems, robotic joints, power buses, and thermal sinks are compiled using unified **Bond-Graph Duality**:

$$P(t) = e(t) \cdot f(t)$$

Every interaction maps to generalized **Effort ($e$)** and **Flow ($f$)** variables:

| Domain | Effort $e(t)$ | Flow $f(t)$ | Storage Elements |
|---|---|---|---|
| **Mechanical (Translational)** | Force $F\ [\text{N}]$ | Velocity $v\ [\text{m/s}]$ | Inertia ($I$), Spring Compliance ($C$) |
| **Mechanical (Rotational)** | Torque $\tau\ [\text{N}\cdot\text{m}]$ | Angular Velocity $\omega\ [\text{rad/s}]$ | Moment of Inertia ($I$), Torsion Spring ($C$) |
| **Electrical** | Voltage $V\ [\text{V}]$ | Current $i\ [\text{A}]$ | Inductance ($I$), Capacitance ($C$) |
| **Hydraulic** | Pressure $P\ [\text{Pa}]$ | Volume Flow $Q\ [\text{m}^3/\text{s}]$ | Fluid Inertance ($I$), Accumulator ($C$) |
| **Thermal** | Temperature $T\ [\text{K}]$ | Heat Flow Rate $\dot{Q}\ [\text{W}]$ | Thermal Capacitance ($C$), Conduction ($R$) |

### 4.1 Symplectic Hamiltonian Integration

To prevent secular energy drift ($dE/dt \ne 0$) over long integration epochs, dynamical systems are integrated via symplectic Störmer-Verlet algorithms in canonical phase-space $(\mathbf{q}, \mathbf{p})$:

$$\dot{\mathbf{q}} = \frac{\partial \mathcal{H}}{\partial \mathbf{p}}, \quad \dot{\mathbf{p}} = -\frac{\partial \mathcal{H}}{\partial \mathbf{q}} \quad \Longrightarrow \quad \frac{d\mathcal{H}}{dt} \approx 0$$

### 4.2 Fast-Forwarding & Thermodynamic Freezing

- **Analytical Fast-Forwarding**: For quiescent or unobserved linear subgraphs ($\dot{\mathbf{x}} = \mathbf{A}\mathbf{x} + \mathbf{B}\mathbf{u}$), state is advanced across macroscopic epochs $\Delta T$ via closed-form matrix exponentials $\exp(\mathbf{A} \Delta T)$, bypassing millions of micro-step iterations.
- **Thermodynamic State Freezing**: Subsystems reaching thermodynamic equilibrium ($dG \approx 0$, $dS_{\text{total}}/dt \le \epsilon_{\text{freeze}}$) are frozen into static, read-only slices, consuming exactly $0\,\mu\text{s}$ of CPU time until an external input flux breaks equilibrium.

---

## Pillar 5: Presentation Layer, 4-Ring Viewport & Decoupled Ingress

The presentation layer is implemented in [`crates/studio_hud`](../../crates/studio_hud) and exposed via [`crates/api`](../../crates/api), utilizing `egui` and `eframe` (v0.34).

### 5.1 Presentation Isolation Invariants

- **Zero Pointer Leakage**: Raw memory pointers, ring buffer memory addresses, and raw CAN frame structs are prohibited from entering the UI thread.
- **Discrete Snapshots**: The GUI consumes immutable telemetry snapshots published over `crates/ipc_bus`.
- **Dual-Mode Viewport**:
  1. *Twin Simulation Mode*: Headless off-screen `wgpu` rendering sharing textures directly with `egui::TextureHandle`.
  2. *Augmented Passthrough Mode*: DXGI zero-copy desktop screen capture from `crates/platform_bridge` blitted with $< 2\,\text{ms}$ glass-to-glass latency.
- **Decoupled Telemetry Ingress**: `crates/platform_bridge` handles high-frequency CAN-bus/HIL and hardware polling on dedicated OS threads, decimating telemetry into discrete state frames over `crates/ipc_bus`.

---

## Pillar 6: Decoupled Human Node & Intent Mirror

The human operator is formalized as an external, decoupled **Human Node** interacting through the Human-Interface Abstraction Layer (HIAL) and a personalized `.si` digital twin cartridge.

### 6.1 Socratic Vector Pinning & Intent DAG

High-uncertainty requirements are pinned along three orthogonal structural vectors:

$$\mathbf{V}_{\text{intent}} = \begin{pmatrix} \mathbf{v}_{\text{invariants}} \\ \mathbf{v}_{\text{dependencies}} \\ \mathbf{v}_{\text{trade-offs}} \end{pmatrix}$$

1. **Invariants**: Rules the target crate's compliance profile requires (universal floor, plus e.g. zero allocations and latency bounds for `kernel` crates, `#![deny(unsafe_code)]` for non-`kernel` crates).
2. **Dependencies**: Target crates, shared memory ring channels, and hardware bridges involved.
3. **Trade-offs**: Latency vs throughput, refactor depth vs backward compatibility.

Pinned vectors compile into an immutable, directed acyclic execution plan (**Intent DAG**).

### 6.2 The 3-Option Intent Mirror

Before non-trivial actions hit the deliberation floor, the system renders a standardized calibration screen:
- **Option A: Conservative**: Minimal delta, zero breaking changes, retains legacy adapters.
- **Option B: Redesign / Systematic**: Clean architectural refactoring, pure zero-copy Pod patterns, eliminates technical debt.
- **Option C: Quick Validate**: Fast-path prototype evaluated in `dev/legacy_staging/` sandbox with `dev/emulator_harness` trace verification before committing.

---

## Subsystem Specifications Directory

For granular equations, memory layouts, and API contracts, refer to the specialized specifications in [`docs/architecture/`](./):
- [Workspace Topology & Subsystem Rings](./architecture_overview.md)
- [Event-Driven Asset Assimilation](./component_onboarding_specification.md)
- [LLM Manager & Priority Scheduler](./llm_manager_scheduler.md)
- [Scale-Invariant Dynamics & Physics Compiler](./physics_compiler_dynamics.md)
- [Decoupled Human Node & Intent Mirror](./human_interface_intent_mirror.md)
