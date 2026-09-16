# ⚡ Aaroneous: Rust Component Framework & Sovereign Execution Substrate

> **A strict, type-safe Rust Component Framework for building interchangeable, safe, zero-allocation execution blocks, plugins, and SCADA/PLC state-space controllers.**

---

## 🌌 Overview & Core Identity

**Aaroneous is NOT a monolithic application.** It is an atomic, type-safe **Rust Component Framework** designed from first principles for building interchangeable, safe, zero-allocation execution blocks and plugins. Operating on a strict **Sterile Execution Plane (SEP)**, the framework targets deterministic reduction, allocation-free hot paths, injected dependencies, and explicit binary contracts. These are component-level requirements; they are not yet guarantees for every crate or execution path.

### Verification and assurance scope

Run `bash scripts/agent_check.sh`, or `pwsh -File scripts/agent_check.ps1` on Windows (uses Git Bash when available). CI invokes the same gate. It compiles all targets, runs workspace tests, audits source, inspects forbidden text patterns, and runs the emulator harness.

The AST audit reports coverage of functions marked `#[doc = "hot_path"]`; it checks syntax, not transitive allocation behavior or worst-case timing. Latency numbers below are design targets unless accompanied by a reproducible benchmark with machine, build profile, inputs, warmup and percentile results. The governance backend currently performs Rust structural and register-footprint checks; enabling its legacy Z3 feature does not constitute an SMT proof. The `plugin_api::Plugin` trait is an in-process Rust contract, not a stable DLL ABI.

Snapshot transport uses version 3 atomic words and copies a validated payload into private storage. Default endpoints use `engine_state_v3`, keeping them separate from older mappings. A busy writer claim after a process crash fails closed; replace the segment through a new configured path after stopping old peers. See [the transport contract](docs/SNAPSHOT_TRANSPORT.md).

### 🧩 Crate Topology & Component Architecture
The framework decomposes execution into isolated, modular component blocks:
- **`core/hypervisor/`**: Headless microkernel host, hardware timer duty cycle, and execution loop.
- **`crates/orchestrator/`**: Task scheduling, core affinity, and typestate event reduction.
- **`crates/platform_bridge/`**: OS abstraction layer (DXGI, Win32 HID, WASAPI loopback).
- **`crates/ipc_bus/`**: Lock-free SPMC/SWMR shared-memory ring buffers and persistent WAL.
- **`crates/capabilities/`**: `UniversalTool` interface & domain specialist registry.
- **`crates/governance/`**: Formal Z3 SMT verification & thermodynamic safety interlocks.
- **`crates/compute/`**: Solid-state SSM engine, .si container format, and Cranelift JIT compiler.
- **`crates/api/` & `crates/studio_hud/`**: Presentation layer and GUI viewports (`egui`/`eframe` 0.34).

```text
┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                        THE AARONEOUS SOLID-STATE EXECUTION ENGINE SUBSTRATE                            │
├────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                                        │
│  ONE FILE (.si) ──► Mounted via `memmap2` in < 50µs directly into Active Virtual Memory                │
│                                                                                                        │
│  ┌──────────────────────────────────────────────────────────────────────────────────────────────────┐  │
│  │ [BLOCK 1: FROZEN CORE SSM WEIGHTS] (Continuous HiPPO State-Space Recurrence)                     │  │
│  │ • 4× Selective State-Space recurrent layers (S4 / Mamba recurrence: h_t = Ā h_{t-1} + B̄ u_t)    │  │
│  │ • Immutable base model: eliminates catastrophic forgetting of grammar, types, and hardware ops.  │  │
│  ├──────────────────────────────────────────────────────────────────────────────────────────────────┤  │
│  │ [BLOCK 2: DYNAMIC ADAPTATION MATRIX] (Streaming LoRA / Real-Time Error Correction)               │  │
│  │ • Mutable Low-Rank delta matrices: ΔW = A_adapt · B_adapt (Rank r = 16, ~64 KB RAM footprint)    │  │
│  │ • On runtime error/panic: Instant in-place gradient step (< 50µs) steers weights away!           │  │
│  │ • On task success: Instant reinforcement step cements optimal latent route.                     │  │
│  ├──────────────────────────────────────────────────────────────────────────────────────────────────┤  │
│  │ [BLOCK 3: EPISODIC SKILL STACK] (Mined AST DAGs, Habits & Execution Pathways)                   │  │
│  │ • Mined computational DAGs, hotkeys, dimensional signatures, and habits.                         │  │
│  └──────────────────────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                                        │
└────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 🚀 Key Architectural Pillars

### 1. 🧬 Pure Rust Selective State-Space Model (`SiStateSpaceModel`)
- Continuous-time state-space recurrence ($h_t = \bar{\mathbf{A}} h_{t-1} + \bar{\mathbf{B}} u_t$, $y_t = \mathbf{C} h_t + \mathbf{D} u_t$).
- 4 layers, 1024-element state vectors, 256 model dimension, 64 state rank (~890k parameters $\approx$ 3.56 MB RAM footprint).
- Sub-millisecond single-pass state-to-action inference ($< 180\,\mu\text{s}$).

### 2. ⚡ Dynamic Adaptation Matrix & Real-Time Error Steering (`DynamicAdaptationMatrix`)
- Eliminates catastrophic forgetting by pairing an immutable frozen core with a mutable low-rank adapter ($\Delta W = A_{\text{adapt}} \cdot B_{\text{adapt}}$).
- When an execution error or compiler panic occurs, `on_runtime_error` applies an immediate negative gradient step in $< 50\,\mu\text{s}$, steering the model away from repeated failures.

### 3. 💎 Autonomous Skill Expansion & Thermodynamic Minimization (`SkillExpansionEngine`)
- Self-development loop driven by thermodynamic free energy minimization ($F = E - T \cdot S$) and step compression.
- Automatically graduates candidate workflows through a formal maturity ladder:
  $$\text{🌱 Candidate} \longrightarrow \text{🧪 Validated} \longrightarrow \text{💎 Crystallized Module} \longrightarrow \text{⚡ Core Reflex}$$
- High-fitness habits are frozen into portable, memory-mapped `.si` cartridges.

### 4. 🛠️ Universal Capability Toolset (`UniversalTool` & MCP Hub)
- **Dual-Face Execution**: Every capability tool implements the `UniversalTool` trait—exposing standard JSON schemas for external MCP clients (Claude Desktop, Cursor, OpenCode), while concurrently executing zero-copy $\mathbb{R}^{256}$ latent tensor transformations for native `.si` models in VRAM ($< 15\,\mu\text{s}$).
- Standard catalog covers AST repair, pattern rewriting, security audits, semantic queries, and multi-modal sensory inspection.

### 5. 🏛️ Layered Protection Rings & PLC Reducer Architecture
- **Ring 0 (Microkernel Host)**: `core/hypervisor` duty cycle executing cyclical 3-phase scans ($S_{t+1} = f(S_t, I)$).
- **Ring 1 (Interconnect & Compute)**: `ipc_bus` lock-free SWMR ring buffers, `compute` continuous SSM and bond-graph physics compilation, `core-contracts` zero-copy Pod definitions.
- **Ring 2 (Control & Orchestration)**: `orchestrator` typestate event reducers, `orchestration_plane`, `governance` thermal/safety interlocks.
- **Ring 3 (Ingress & Transducers)**: `capabilities`, `llm_gateway` stateless transport, `platform_bridge` Win32 HID and DXGI screen capture.
- **Ring 4 (Presentation)**: `api` and `studio_hud` desktop GUI on `egui`/`eframe` 0.34 with zero raw pointer leakage.

---

## ⚡ Zero-Copy Solid-State Neural Architecture

The Aaroneous runtime executes `.si` v3.0 cartridges via direct virtual memory mapping (`memmap2`):
- **Zero Heap Allocations**: Model weights and parameter loci are addressed directly from page-aligned memory maps.
- **Cache Alignment**: 64-byte aligned SIMD layout for streaming vector operations (`align(64)`).
- **Zero-Copy Contracts**: Public boundary types implement `#[repr(C)]` and derive `bytemuck::Pod` + `bytemuck::Zeroable`.
- **Shared-Memory IPC**: Lock-free SWMR ring buffers communicate via discrete zero-copy frames over `ipc_bus`.

---

## 📦 Quick Start & CLI Usage

### Build and Launch Desktop Studio HUD
```powershell
# Native Desktop Studio & Telemetry HUD (Eframe / WGPU)
cargo run --release -p studio_hud --bin aaroneous

# Headless Microkernel Hypervisor CLI
cargo run --release -p hypervisor --bin a_run -- --help
```

### Static Analysis & AST Invariant Audit
```powershell
# Run the AST Auditor across the entire workspace (Must report 0 violations)
cargo run -p ast_auditor -- audit core/ crates/
```

### Sovereign Hypervisor Commands (`a_run`)
```powershell
# 1. Distill & birth .si solid-state models for Sovereign Specialists
a_run distill-all --samples 10 --epochs 1 --out models/distilled_federation

# 2. Boot a live 4-node P2P cluster & verify gossip consensus
a_run mesh --nodes 4 --live

# 3. Launch an active sovereign socket daemon node
a_run daemon --bind 127.0.0.1:8001 --heartbeat 1500

# 4. Execute autonomous background self-evolution AST mutation
a_run evolve --cycles 3 --threshold 0.70

# 5. Forge a new Tier 3 Kinetic Reflex .si container from scratch
a_run forge --name chimera_ast --tier 3 --samples 20 --epochs 1

# 6. Benchmark zero-copy memory-mapped execution latency
a_run si benchmark data/models/chimera_ast.si --iterations 500
```

---

## 📂 Workspace Architecture

```text
d:\Aaroneous\
├── core/
│   └── hypervisor/             # Microkernel host, execution duty cycle, a_run, profile_compiler
├── crates/
│   ├── api/                    # Presentation boundary, public types, egui/eframe bridge
│   ├── studio_hud/             # Native Desktop Studio & Telemetry HUD (egui/eframe 0.34)
│   ├── ipc_bus/                # Lock-free SWMR ring buffers, LMAX disruptor, UCP protocol
│   ├── compute/                # SiForge, SSM engine, Sparse MoE, bond-graph physics compiler
│   ├── orchestrator/           # Typestate task assimilation, priority scheduler, thread affinity
│   ├── orchestration_plane/    # Orchestration daemon integration, domain classification
│   ├── llm_gateway/            # Stateless transducer transport layer (HTTP/REST/MCP)
│   ├── capabilities/           # UniversalTool registry, security audit, code repair tools
│   ├── platform_bridge/        # Win32 HID injection, DXGI zero-copy screen capture, WASAPI
│   ├── governance/             # Thermal monitoring, Z3 SMT gates, SI lattice verification
│   ├── autonomic_adaptation/   # Continuous adaptive control, LoRA adaptation, GGUF ingestion
│   ├── adaptation_engine/      # Polyglot AST parsing, component forge, shadow sandbox
│   ├── transpiler/             # AST parser & distillation trajectory miner
│   ├── omni/                   # 3D Concept Galaxy Graph & Barnes-Hut gravitational clustering
│   ├── ast_auditor/            # Static analysis AST linter enforcing workspace invariants
│   ├── paths/                  # Configuration-injected path resolver (zero ambient authority)
│   ├── core-contracts/         # Zero-copy memory contracts & Pod/Zeroable derivations
│   ├── si_format/              # Canonical .si container binary layout, SIMD alignment & CRC32
│   ├── si_ir/                  # Computational graphs, MachineOpcode IR & type lattice
│   ├── wire/                   # Network protocol framing and zero-copy packet wire serialization
│   ├── mcp_server/             # Model Context Protocol server exposing Aaroneous to external tools
│   ├── mutation_engine/        # Safe AST transformation, layout normalizer, panic replacer
│   ├── runtime_monitor/        # Telemetry inspection, thread liveness, and watchdog supervisor
│   └── biology/                # Bio-inspired cellular automata & state decay simulations
├── dev/
│   ├── emulator_harness/       # Golden execution harness, trace capture, regression testbed
│   ├── legacy_staging/         # Quarantine sandbox for external/historical code ingestion
│   └── tools/                  # Diagnostics, host safety monitors, maintenance scripts
├── data/                       # Fallback in-tree data root (gitignored)
└── docs/
    ├── architecture.md         # Master Architecture Portal & PLC/SCADA Invariants
    └── architecture/           # Canonical Subsystem Specifications
        ├── MASTER_ARCHITECTURE.md         # Unified 6-pillar master specification
        ├── architecture_overview.md       # Workspace topology & subsystem rings
        ├── component_onboarding_specification.md  # component onboarding and assimilation
        ├── llm_manager_scheduler.md       # Stateless transducer & priority backoff heap
        ├── physics_compiler_dynamics.md   # Bond-graph duality & symplectic integration
        └── human_interface_intent_mirror.md # HIAL, Intent DAG & 3-option intent mirror

EXTERNAL DECOUPLED STORAGE (Zero-Clutter Architecture):
├── C:\CargoTargetCache\        # Global Cargo target build cache (`target-dir`)
└── D:\ArcData\                 # Externalized Neural & State Substrate (`ARC_DATA_ROOT`)
    ├── models/                 # Birthed .si solid-state neural cartridges
    ├── skills/                 # Crystallized .si muscle memory cartridges
    ├── state_banks/            # Episodic state records & trajectory checkpoints
    └── agents/                 # Specialist agent snapshots & persistent memory
```

---

## 📜 Canonical Documentation Portal

For exhaustive technical specifications across all subsystems, consult:
- **[Unified Master Architecture](docs/architecture/MASTER_ARCHITECTURE.md)**
- **[System Architecture Specification](docs/architecture.md)**
- **[Workspace Topology & Subsystem Rings](docs/architecture/architecture_overview.md)**
- **[Component Onboarding and Assimilation](docs/architecture/component_onboarding_specification.md)**
- **[LLM Manager & Priority Scheduler](docs/architecture/llm_manager_scheduler.md)**
- **[Scale-Invariant Dynamics & Physics Compiler](docs/architecture/physics_compiler_dynamics.md)**
- **[Decoupled Human Node & Intent Mirror](docs/architecture/human_interface_intent_mirror.md)**
- **[Architectural Constraints & Dependency Injection](docs/ARCHITECTURAL_CONSTRAINTS.md)**
- **[Compiler Invariant Governance & AST Auditor](docs/CRATIFY_SPEC.md)**
- **[Forensic Ingestion Protocol (RFC-0005)](docs/FORENSICS_RFC0005.md)**

---

## ⚖️ License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
