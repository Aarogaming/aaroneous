# Decoupled Human Node, HIAL & Intent Mirror Specification

> **CANONICAL SPECIFICATION**  
> **SCOPE**: Human-Interface Abstraction Layer (HIAL), Socratic Vector Pinning, Intent DAG Compilation, and Presentation Layer Isolation.  
> **APPLIES TO**: `crates/api`, `crates/studio_hud`, `crates/orchestrator`, `crates/platform_bridge`.  
> **LAST UPDATED**: 2026-09-12

---

## 1. The Decoupled Human Node Philosophy

In conventional development harnesses, human input is treated as an ambient interactive prompt loop or omniscient controller. This causes non-deterministic drift, undefined operational scopes, and uncontrolled task expansion.

In Aaroneous, the human developer is formalized as an external, decoupled **Human Node** within the system graph:
- **No Direct Hot-Path Injection**: Human keystrokes and commands do not execute raw side-effects or mutate hypervisor state directly.
- **Human-Interface Abstraction Layer (HIAL)**: Encapsulates human inputs into typed, structured event packets.
- **The `.si` Digital Twin**: A personalized `.si` cartridge encodes developer preferences, architectural tolerances, coding idioms, and verification budgets.

```
       Human Developer (Intent & Goals)
                       │
                       ▼
    +──────────────────────────────────────+
    |   Human-Interface Abstraction Layer  |
    |                (HIAL)                |
    +──────────────────────────────────────+
                       │
                       ▼ Socratic Vector Pinning
    +──────────────────────────────────────+
    |        Intent DAG Compilation        |
    |  • Invariants                        |
    |  • Dependencies                      |
    |  • Trade-offs                        |
    +──────────────────────────────────────+
                       │
                       ▼
    +──────────────────────────────────────+
    |       3-Option Intent Mirror         |
    |   [Conservative] [Redesign] [Quick]  |
    +──────────────────────────────────────+
                       │
                       ▼ User Selection
         Orchestrator Deliberation Floor
```

---

## 2. Socratic Vector Pinning & Intent DAG Compilation

When a task presents high semantic ambiguity or multiple architectural branching paths, the system executes **Socratic Vector Pinning**. Instead of open-ended conversational back-and-forth, the task is pinned across three orthogonal structural axes:

$$\mathbf{V}_{\text{intent}} = \begin{pmatrix} \mathbf{v}_{\text{invariants}} \\ \mathbf{v}_{\text{dependencies}} \\ \mathbf{v}_{\text{trade-offs}} \end{pmatrix}$$

### 2.1 The Three Orthogonal Pinning Axes

1. **Invariants ($\mathbf{v}_{\text{invariants}}$)**: Non-negotiable structural constraints that must hold across all execution states:
   - Must remain strictly zero-allocation on the hot path?
   - What is the maximum cycle latency budget ($T_{\text{max}} \le 25\,\mu\text{s}$)?
   - Are unsafe blocks strictly forbidden (`#![deny(unsafe_code)]`) or permitted with `// SAFETY:` proofs?
2. **Dependencies ($\mathbf{v}_{\text{dependencies}}$)**: Explicit boundary connections:
   - Which crates in the ring topology are modified?
   - What shared memory ring channels or IPC topics are affected?
   - Are hardware bridges (DXGI, HID, CAN-bus) invoked?
3. **Trade-offs ($\mathbf{v}_{\text{trade-offs}}$)**: Explicit optimization priorities:
   - Runtime speed vs. memory geometry footprint.
   - Code refactor depth vs. historical backward compatibility.
   - Immediate deterministic commit vs. speculative branch exploration.

### 2.2 Compilation to the Intent DAG

Once pinned, the vectors are compiled into a formal **Intent DAG** ($\mathcal{G} = (\mathcal{V}, \mathcal{E})$):
- Nodes $\mathcal{V}$ represent discrete, testable transformations (e.g., "Synthesize zero-copy struct", "Implement typestate transition", "Run AST audit").
- Edges $\mathcal{E}$ represent strict dependency orderings.
- The Intent DAG is registered with `crates/orchestrator` as an immutable plan prior to execution.

---

## 3. The 3-Option Intent Mirror

Before non-trivial or irreversible state changes enter the orchestration deliberation floor, the system renders a standardized **3-Option Intent Mirror**:

```
+─────────────────────────────────────────────────────────────────────────────+
|                         AARONEOUS INTENT MIRROR                             |
| Goal: [Target System Transformation Description]                            |
+─────────────────────────────────────────────────────────────────────────────+
|                                                                             |
|  [OPTION A: CONSERVATIVE]                                                   |
|  • Minimal delta; zero breaking changes.                                    |
|  • Retains legacy interfaces with strict fallback adapters.                 |
|  • Lowest risk; preserves 100% ABI and backward compatibility.              |
|                                                                             |
|  [OPTION B: REDESIGN / SYSTEMATIC] (Recommended)                            |
|  • Clean-room architectural refactoring; eliminates technical debt.         |
|  • Updates trait signatures to pure zero-copy Pod patterns.                 |
|  • Maximizes long-term modularity and alignment with the SEP standard.      |
|                                                                             |
|  [OPTION C: QUICK VALIDATE]                                                 |
|  • Fast-path prototype isolated inside `dev/legacy_staging/` sandbox.       |
|  • Emits trace events through `dev/emulator_harness` to verify hypothesis.  |
|  • Zero commitment to production crates until empirical proof is verified.  |
|                                                                             |
+─────────────────────────────────────────────────────────────────────────────+
| [Enter: 1 (A) | 2 (B) | 3 (C)]                                              |
+─────────────────────────────────────────────────────────────────────────────+
```

This ensures low-friction alignment between human intent and machine execution, preventing accidental scope creep or destructive rewrites.

---

## 4. Presentation Layer Isolation & 4-Ring Viewport

The user interface is hosted in [`crates/studio_hud`](../../crates/studio_hud) and exposed via [`crates/api`](../../crates/api), utilizing `egui` and `eframe` (v0.34).

### 4.1 Strict Memory Isolation Invariants

1. **Zero Pointer Leaks**: Raw kernel pointers, internal ring buffer memory addresses, and raw CAN-bus frame handles must NEVER leak into the UI thread.
2. **Discrete Snapshots Only**: The UI consumes immutable, decoupled telemetry snapshots emitted by Ring 1 over the IPC bus.
3. **Off-Thread Rendering**: Heavy computation, 3D galaxy clustering (`omni`), and physics compilation (`compute`) run asynchronously on background worker pools; the UI thread only renders projected 2D/3D vertices.

### 4.2 Dual-Mode Viewport Architecture

The viewport operates in two mutually exclusive rendering modes:

1. **Headless `wgpu` Texture Sharing (Twin Simulation Mode)**:
   - Off-screen `wgpu` renderer runs on an isolated render thread.
   - Renders high-fidelity digital twin simulations, bond-graph topological flows, and 3D concept constellations.
   - Directly maps rendered GPU textures into the `egui::TextureHandle` without round-tripping through CPU host memory.
2. **Augmented Passthrough Mode**:
   - DXGI zero-copy screen capture frames from [`crates/platform_bridge`](../../crates/platform_bridge) are blitted directly to viewport surfaces.
   - Overlays kinetic UI bounding boxes, OCR tokens, and reflex interaction indicators with $< 2\,\text{ms}$ glass-to-glass latency.

### 4.3 Decoupled Ingress via `platform_bridge`

Hardware telemetry ingress (CAN-bus, serial HIL, gamepad, keyboard, audio WASAPI) is completely decoupled from the application logic:
- `platform_bridge` runs high-frequency hardware pollers on dedicated OS threads.
- Ingress streams are **decimated** and packed into discrete, fixed-size state frames.
- Packed frames are published onto `crates/ipc_bus` ring buffers, ensuring downstream controllers never experience hardware I/O blocking.
