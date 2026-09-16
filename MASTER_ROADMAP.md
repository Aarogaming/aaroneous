# Aaroneous – Master Roadmap (vX.Y.Z)

## Table of Contents
1. [Executive Summary](#executive-summary)  
2. [Current State & Decoupling Progress](#current-state--decoupling-progress)  
3. [The 7-Horizon Frontier Framework](#the-7-horizon-frontier-framework)  
4. [Systemic Execution Loop](#systemic-execution-loop)  
5. [Architecture Audit Findings & Inspiration Matrix](#architecture-audit-findings--inspiration-matrix)  
6. [Prioritized Action Plan](#prioritized-action-plan)  
7. [Phase-by-Phase Roadmap Integration](#phase-by-phase-roadmap-integration)  
8. [Goals & Success Metrics](#goals--success-metrics)  
9. [Build & Deployment Guide](#build--deployment-guide)  
10. [Appendices](#appendices)

---

### Executive Summary
Aaroneous is a self-compiling, zero-copy, machine-native synthetic intelligence operating system substrate that unifies hypervisor execution, neural-symbolic state-space reasoning, and hardware offload. This roadmap consolidates the architecture audit, borrowing matrix, prioritized work groups, the 7-Horizon frontier matrix, and phase-by-phase integration into a single, actionable specification.

---

### Current State & Decoupling Progress
Following Cratify Batches 1 & 2, the workspace has eliminated monolithic couplings:
- **Headless Hypervisor**: core/hypervisor is decoupled from all UI dependencies. GUI binaries (aroneous, aroneous-setup) reside in crates/studio_hud.
- **Decoupled Inference Gateway**: Provider routing (GGUF, OpenAI, Local, Mock) and token caching reside in crates/llm_gateway.
- **Integrated Machine Engines**: crates/mutation_engine and crates/runtime_monitor are fully integrated with zero-panic contracts and ytemuck::Pod layouts.
- **Assimilation Pipeline in Flight**: crates/mcp_server and crates/orchestration_plane are undergoing modular headless compilation.
- **Formal Governance**: Cratify certification harness passes 190/190 invariant checks across memory layout, ring buffers, saturation, translation, compute, and platform backpressure.

---

### The 7-Horizon Frontier Framework

| Horizon | Architectural Domain | Status | Technical Mechanism & Target Deliverables |
|---|---|---|---|
| **H1** | **Autonomous Skill Synthesis & Trace Crystallization** | **Complete** | Automatic extraction of high-frequency execution traces into compiled Cranelift native plugins embedded in .si Block 3 habit stacks. `adaptation_engine` (AST pattern rewriter), `runtime_monitor`, `mcp_server`, and `orchestration_plane` fully integrated. |
| **H2** | **Deep OS Observability & Multi-Modal Sensor Fusion** | **Expanding** | Quad-stream sensory pipeline: DXGI screen capture + UIA element tree + WASAPI loopback audio + non-polling ETW kernel events. Expansion targets: si_ir Machine Tokenizer, in-memory RLS adaptive filter in crates/compute/src/state_bank.rs, synthetic trace mining harness via llm_gateway, continuous telemetry ingestion via ing_buffer.rs. |
| **H3** | **Formal SMT & Thermodynamic Verification** | **Active Standard** | Continuous lattice validation of 7-exponent SI base units with SMT-backed algebraic non-interference proofs for concurrent task graphs (crates/governance). |
| **H4** | **Associative Vector Memory Fabric** | **Complete** | In-memory hnsw_rs indexing over $\mathbb{R}^{256}$ latent trajectories providing $< 1\mu\text{s}$ nearest-neighbor habit and reflex recall. |
| **H5** | **Heterogeneous Fleet Swarm & Work-Stealing** | **Active Standard** | Multi-host Iroh QUIC mesh with Ed25519 node identities, dynamic load telemetry, and decentralized work-stealing for heavy computation graphs. |
| **H6** | **Sovereign SI-OS & Compositor** | **In Progress** | Fluid RON spatial window canvas evolving toward a standalone Wayland/Direct3D12 compositor and bare-metal microkernel substrate. |
| **H7** | **In-Game Graphics Hooking & Zero-Latency Overlays** | **Complete** | In-process graphics injection via hudhook for DirectX 9/11/12 and Vulkan rendering pipelines with sub-frame action overlays. |

---

### Systemic Execution Loop

All ongoing feature cycles and autonomous runtime evolution adhere to the systemic progression:
\textbf{Observe} \longrightarrow \textbf{Hypothesize} \longrightarrow \textbf{Design} \longrightarrow \textbf{Implement} \longrightarrow \textbf{Test} \longrightarrow \textbf{Deploy} \longrightarrow \textbf{Measure} \longrightarrow \textbf{Learn} \longrightarrow \textbf{Repeat}

- **Observe**: Gather deep sensory telemetry (DXGI, UIA, ETW, audio loopback) and profile hotspot bottlenecks.
- **Hypothesize**: Formulate concrete behavioral hypotheses regarding efficiency gains, AST mutations, or routing.
- **Design**: Model zero-copy structs (ytemuck::Pod), verify SMT non-interference, and establish API invariants.
- **Implement**: Write memory-safe, panic-free Rust adhering to strict blast radius isolation.
- **Test**: Execute unit test suites, integration harnesses, and Cratify ABI certification.
- **Deploy**: Mount into running hypervisor via .si containers or hot-loaded C-ABI plugins.
- **Measure**: Sample nanosecond hardware counters (_rdtsc), VRAM budgets, and thermal stability.
- **Learn**: Record trajectory outcomes into HNSW associative memory and update streaming LoRA deltas.
- **Repeat**: Feed learned weights into observation for continuous recursive refinement.

---

### Architecture Audit Findings & Inspiration Matrix
| Need | Source Project | Pattern to Borrow | Integration Point |
|------|----------------|-------------------|-------------------|
| OS-level sandbox & capability tokens | **OpenFang** | Capability-based security, Windows Job Objects | Extend capability_broker to issue signed tokens and enforce sandbox at process spawn |
| Compile-time typed action graph | **Rig** (ig.rs) | Generic Tool/AgentAction<I,O> trait, compile-time graph validation | Replace loosely-typed Skill/Instruction with AgentAction<I,O> and enum ActionPayload deriving kyv |
| Supervision & auto-restart | **Ractor / Actix** | Supervisor trees, boxed async tasks, restart policies | Add supervision.rs with Supervisor that watches 	okio::task::JoinHandles and registers budgets with RuntimeGovernor |
| Unified low-latency event bus | **LMX Disruptor** (already present) | Central ring buffer for all messages | Promote disruptor.rs to the sole inter-component bus; deprecate universal_event_bus |
| Fast binary (de)serialization | **rkyv + mimalloc** | Zero-copy archives | Ensure every IPC payload (Skill, Instruction, TelemetryEvent, etc.) derives Archive, Serialize, Deserialize |

---

### Prioritized Action Plan (Threaded & Parallel)

#### 🔥 Critical (Sequential & In-Flight)
- [x] **Decouple HUD from Hypervisor** – Extracted to crates/studio_hud (`a_run` is 100% headless).
- [x] **Decouple LLM Gateway from Hypervisor** – Extracted to crates/llm_gateway.
- [x] **Consolidate mutation_engine into adaptation_engine** – Bytemuck Pod MutationHeader, zero-panic Comby-style AST patch rewriter.
- [x] **Integrate runtime_monitor** – Zero-copy telemetry ingestion and SWMR POD trigger events.
- [x] **Complete Assimilation Pipeline** – Finalized `crates/mcp_server` and `crates/orchestration_plane` headless compilation, live tool dispatch, and zero-ambient invariant conformance.
- [ ] **Define run_hypervisor()** in core/hypervisor/src/lib.rs connecting RuntimeGovernor, supervised tasks, and the central Disruptor.
- [ ] **Add supervision.rs** implementing a Supervisor struct with restart policy and budget registration.
- [ ] **Convert CapabilityBroker** to issue signed CapabilityToken objects and integrate Windows Job sandbox creation.

#### 🟢 Horizon 2 Expansion Milestones
- [ ] **si_ir Machine Tokenizer Definition**: Formulate token grammar mapping MachineOpcode, Win32 events, DXGI dirty sector bits, and typed IPC frames.
- [ ] **In-Memory RLS Adaptive Filter**: Implement sub-microsecond Recursive Least Squares covariance updates directly in crates/compute/src/state_bank.rs.
- [ ] **Synthetic Trace Mining via llm_gateway**: Asynchronous harvesting and distillation of anomalous execution traces into canonical .si container blocks.
- [ ] **Continuous Telemetry Ingestion via ing_buffer.rs**: Stream live DXGI/RawInput/ETW frames directly into the disruptor without intermediate heap serialization.

#### 🟡 Governance & Release Hardening
- [ ] **Edge-device orchestration** via 
ats_client.rs to schedule NPU tasks on remote nodes.
- [ ] **Formal verification** of compiled AgentAction graphs using Z3 (crates/governance).
- [ ] **Dynamic plugin hot-swap** improvements in crates/hotload once supervision is stabilized.

---

### Phase-by-Phase Roadmap Integration
| Phase | Version | Focus | Status | Key Deliverables |
|---|---|---|---|---|
| **Phase 1** | 0.4.0 | Defect resolution & flag activations | **Complete** | Fix packet alignment & GDI leaks; activate llama-gguf, 
vml-wrapper, iroh P2P |
| **Phase 2** | 0.5.0 | Vision & rendering convergence | **Complete** | DXGI desktop capture, modular HUD modes, spatial window manager |
| **Phase 3** | 0.6.0 | Compiler & True JIT | **Complete** | si_ir extraction, LatticeVerifier, cranelift-codegen JIT |
| **Phase 4** | 0.7.0 | .si tooling & associative memory | **Complete** | SiForge pipeline, HNSW memory fabric |
| **Phase 5** | 0.8.0 | Safe dynamic modification | **Complete** | libloading dynamic loader, streaming LoRA, GenerationalJournal |
| **Phase 6** | 1.0.0 | Distributed execution & SMT gate | **Complete** | Iroh QUIC fleet, FleetScheduler, Z3Prover |
| **Phase 7** | 1.1.0 | Sensor fusion & GPU SSM | **Complete** | UIA tree, WASAPI loopback, ETW ingestion, cubecl GPU SSM, intent-to-fascia daemon |
| **Phase 8** | 1.2.0 | Mechanical sympathy & micro-architectural tuning | **Complete** | mimalloc, Fat LTO, smol_str, _rdtsc, SoA storage |
| **Phases 9–17** | 1.2.0 | Native performance & 3D HUD | **Complete** | Native WGPU 3D studio, SSE telemetry, Raft consensus, polyglot tree-sitter |
| **Phases 18–24** | 1.3.0 | Sovereign cartridge & formal governance | **Complete** | Canonical .si v3.0, Z3 SMT gates, Fitts's law Bézier kinematics |
| **Phases 25–29** | 1.4.0 | Hardware saturation & sparse MoE | **Complete** | 16-slot sparse expert register, CAN 2.0B/FD, VRAM slab, Crucible sandbox |
| **Phases 30–33** | 1.5.0 | Machine-native intent & NPU | **Complete** | Auto-tuner, drag-and-drop .si-pack, .lib state bank, 45 TOPS NPU |
| **Phases 34–37** | 1.6.0 | Console-OS shell & Merlin companion | **Complete** | Decoupled console-OS, 3D skills, biometrics, Merlin |
| **Phase 38** | 1.7.0 | Triad shell convergence, resource governance | **In Progress** | CapabilityBroker, EngineStatePublisher, dirty-flag pacing, thermal backpressure, kyv, disruptor |

---

### Goals & Success Metrics
| Goal | Metric | Target | Owner |
|------|--------|--------|-------|
| Zero-copy telemetry | % of telemetry paths using MachinePacket + Disruptor | > 95 % | telemetry team |
| Supervision stability | Mean-time-to-recover after a panic | < 5 s | runtime team |
| Fast-path adaptive filter | In-memory RLS covariance update latency | < 1 µs | compute team |
| NPU off-load latency | End-to-end inference latency on Intel NPU | < 10 ms | hw team |
| Test coverage & governance | Passing ACC checks in crates/cratify | ≥ 190 | governance lead |
| Release cadence | Time between major releases | ≤ 3 months | release lead |

---

### Build & Deployment Guide
1. **Prerequisites** – Install Rust stable, cargo, mimalloc, and OpenVINO (if using Intel NPU). See CONTRIBUTING.md.
2. **Bootstrapping** – Build the unified hypervisor binary:
   `ash
   cargo build --release --bin aaroneous
   `
3. **Run the hypervisor** – New entry point starts supervisors, disruptor, and all subsystems:
   `ash
   ./target/release/aaroneous run_hypervisor
   `
4. **Testing** – After any change run:
   `ash
   cargo test --workspace --all-features
   `
5. **Deployment** – Use the provided Dockerfile (docker/Dockerfile) or the systemd unit (scripts/aaroneous.service).

---

### Appendices
- **RFC-0004**: Machine-Native State Language Model & Bounded Dynamicism (docs/rfcs/RFC-0004-MACHINE_NATIVE_SLM_AND_BOUNDED_DYNAMICISM.md).
- **Strategic Vision & Doctrine**: STRATEGIC_VISION.md.
- **System Inventory**: WHAT_EXISTS_TODAY.md.
- Full **Architecture Audit** (ARCHITECTURE_AUDIT.md).
- Complete **TODO list** (TODO.md).
- Detailed **build instructions** (CONTRIBUTING.md).
- Change history (CHANGELOG.md).
