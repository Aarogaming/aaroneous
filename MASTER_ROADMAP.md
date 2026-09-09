# Aaroneous – Master Roadmap (vX.Y.Z)

## Table of Contents
1. [Executive Summary](#executive-summary)  
2. [Current State (Baseline)](#current-state-baseline)  
3. [Architecture Audit Findings](#architecture-audit-findings)  
4. [Borrowing & Inspiration Matrix](#borrowing--inspiration-matrix)  
5. [Prioritized Action Plan (Threaded & Parallel)](#prioritized-action-plan)  
6. [Phase‑by‑Phase Roadmap Integration](#phase-by-phase-roadmap-integration)  
7. [Goals & Success Metrics](#goals--success-metrics)  
8. [Build & Deployment Guide](#build--deployment-guide)  
9. [Appendices](#appendices)

---

### Executive Summary
Aaroneous is a self‑compiling, zero‑copy, AGI‑centric OS substrate that unifies hyper‑visor execution, neural‑symbolic reasoning, and hardware‑offload. This roadmap consolidates the architecture audit, borrowing matrix, prioritized work groups, and phase‑by‑phase integration into a single, actionable document for developers and local‑hosted LLM agents.

---

### Current State (Baseline)
- **Hypervisor core** (`core/hypervisor/src/lib.rs`) is a library of re‑exports with no single entry‑point or orchestrated event loop. Runtime is started by scattered binaries.
- **IPC / Data Path** (`crates/ipc_bus`) provides zero‑copy binary envelopes (`MachinePacket` with `rkyv`) and a disruptor ring buffer, but some hot paths still emit JSON strings, creating hidden copies.
- **Supervision & Fault‑tolerance** are missing; panics can crash the whole process.
- **Hardware offload** placeholders exist without concrete implementation.

---

### Architecture Audit Findings
| Need | Source Project | Pattern to Borrow | Integration Point |
|------|----------------|-------------------|-------------------|
| OS‑level sandbox & capability tokens | **OpenFang** | Capability‑based security, Windows Job Objects | Extend `capability_broker` to issue signed tokens and enforce sandbox at process spawn |
| Compile‑time typed action graph | **Rig** (`rig.rs`) | Generic `Tool`/`AgentAction<I,O>` trait, compile‑time graph validation | Replace loosely‑typed `Skill`/`Instruction` with `AgentAction<I,O>` and `enum ActionPayload` deriving `rkyv` |
| Supervision & auto‑restart | **Ractor / Actix** | Supervisor trees, boxed async tasks, restart policies | Add `supervision.rs` with `Supervisor` that watches `tokio::task::JoinHandle`s and registers budgets with `RuntimeGovernor` |
| Unified low‑latency event bus | **LMX Disruptor** (already present) | Central ring buffer for all messages | Promote `disruptor.rs` to the sole inter‑component bus; deprecate `universal_event_bus` |
| Fast binary (de)serialization | **rkyv + mimalloc** | Zero‑copy archives | Ensure every IPC payload (`Skill`, `Instruction`, `TelemetryEvent`, etc.) derives `Archive`, `Serialize`, `Deserialize` |

---

### Borrowing & Inspiration Matrix
*(Same as above table – retained for quick reference.)*

---

### Prioritized Action Plan (Threaded & Parallel)
#### 🔥 Critical (sequential)
- [ ] **Define `run_hypervisor()`** in `core/hypervisor/src/lib.rs` that starts `RuntimeGovernor`, spawns supervised tasks for `autonomic_loop`, `decision_engine`, and connects each to the central `Disruptor`.
- [ ] **Add `supervision.rs`** implementing a `Supervisor` struct with restart policy and budget registration.
- [ ] **Convert `CapabilityBroker`** to issue signed `CapabilityToken` objects and integrate Windows Job sandbox creation.
- [ ] **Replace top‑level `Skill`/`Instruction`** with a generic `AgentAction<I,O>` trait and concrete `enum ActionPayload` (all variants `#[repr(C)]` + `rkyv`).
- [ ] **ACC Governance** – Implement ACC manifest validation, zero‑copy contract enforcement, and sandbox isolation as per updated standards.


#### 🟢 Parallelizable (concurrent)
- [ ] **IPC Cleanup** – Migrate remaining JSON‑based logging to binary `MachinePacket` and feed into `Disruptor`.
- [ ] **Hardware Offload** – Implement `intel_npu` crate (OpenVINO) and optional `cuda_backend` feature; expose zero‑copy `InferenceRequest`/`Response` structs.
- [ ] **Telemetry Refactor** – Replace `tracing::info!` in hot paths with `TelemetryEvent` pushes to the disruptor.
- [ ] **Documentation Consolidation** – Consolidate duplicated sections from `README.md`, `TODO.md`, etc. into this master roadmap.
- [ ] **Test Harness** – Add unit tests for `MachinePacket` round‑trip through shared memory and disruptor, plus supervisor restart scenarios.

#### 🟡 Low‑Priority / Future Extensions
- [ ] **Edge‑device orchestration** via `nats_client.rs` to schedule NPU tasks on remote nodes.
- [ ] **Formal verification** of the compiled `AgentAction` graph using Z3.
- [ ] **Dynamic plugin hot‑swap** improvements in `splicing_engine.rs` once supervision is stable.

---

### Phase‑by‑Phase Roadmap Integration
| Phase | Version | Focus | Status | Key Deliverables |
|---|---|---|---|---|
| **Phase 1** | `v0.4.0` | Defect resolution & flag activations | **Complete** | Fix packet alignment & GDI leaks; activate `llama‑gguf`, `nvml‑wrapper`, `iroh` P2P |
| **Phase 2** | `v0.5.0` | Vision & rendering convergence | **Complete** | DXGI desktop capture, modular HUD modes, spatial window manager |
| **Phase 3** | `v0.6.0` | Compiler & True JIT | **Complete** | `si_ir` extraction, `LatticeVerifier`, `cranelift‑codegen` JIT |
| **Phase 4** | `v0.7.0` | `.si` tooling & associative memory | **Complete** | `SiForge` pipeline, HNSW memory fabric |
| **Phase 5** | `v0.8.0` | Safe dynamic modification | **Complete** | `libloading` dynamic loader, streaming LoRA, `GenerationalJournal` |
| **Phase 6** | `v1.0.0` | Distributed execution & SMT gate | **Complete** | Iroh QUIC fleet, `FleetScheduler`, `Z3Prover` |
| **Phase 7** | `v1.1.0` | Sensor fusion & GPU SSM | **Complete** | UIA tree, WASAPI loopback, ETW ingestion, `cubecl` GPU SSM, intent‑to‑fascia daemon |
| **Phase 8** | `v1.2.0` | Mechanical sympathy & micro‑architectural tuning | **Complete** | `mimalloc`, Fat LTO, `smol_str`, `_rdtsc`, SoA storage |
| **Phases 9‑17** | `v1.2.0` | Native performance & 3D HUD | **Complete** | Native WGPU 3D studio, SSE telemetry, Raft consensus, polyglot tree‑sitter |
| **Phases 18‑24** | `v1.3.0` | Sovereign cartridge & formal governance | **Complete** | Canonical `.si` v3.0, Z3 SMT gates, Fitts's law Bézier kinematics |
| **Phases 25‑29** | `v1.4.0` | Hardware saturation & sparse MoE | **Complete** | 16‑slot sparse expert register, CAN 2.0B/FD, VRAM slab, Crucible sandbox |
| **Phases 30‑33** | `v1.5.0` | Machine‑native intent & NPU | **Complete** | Auto‑tuner, drag‑and‑drop `.si‑pack`, `.lib` state bank, 45 TOPS NPU |
| **Phases 34‑37** | `v1.6.0` | Console‑OS shell & Merlin companion | **Complete** | Decoupled console‑OS, 3D skills, biometrics, Merlin |
| **Phase 38** | `v1.7.0` | Triad shell convergence, resource governance | **In Progress** | `CapabilityBroker`, `EngineStatePublisher`, dirty‑flag pacing, thermal back‑pressure, `rkyv`, disruptor |

---

### Goals & Success Metrics
| Goal | Metric | Target | Owner |
|------|--------|--------|-------|
| Zero‑copy telemetry | % of telemetry paths using `MachinePacket` + `Disruptor` | > 95 % | telemetry team |
| Supervision stability | Mean‑time‑to‑recover after a panic | < 5 s | runtime team |
| NPU off‑load latency | End‑to‑end inference latency on Intel NPU | < 10 ms | hw team |
| Test coverage | % of crates with > 80 % line coverage | 80 % | qa team |
| Release cadence | Time between major releases | ≤ 3 months | release lead |

---

### Build & Deployment Guide
1. **Prerequisites** – Install Rust stable, `cargo`, `mimalloc`, and `OpenVINO` (if using Intel NPU). See `CONTRIBUTING.md`.
2. **Bootstrapping** – Build the unified hypervisor binary:
   ```bash
   cargo build --release --bin aaroneous
   ```
3. **Run the hypervisor** – New entry point starts supervisors, disruptor, and all subsystems:
   ```bash
   ./target/release/aaroneous run_hypervisor
   ```
4. **Testing** – After any change run:
   ```bash
   cargo test --workspace --all-features
   ```
5. **Deployment** – Use the provided Dockerfile (`docker/Dockerfile`) or the systemd unit (`scripts/aaroneous.service`).

---

### Appendices
- Full **Architecture Audit** (`ARCHITECTURE_AUDIT.md`).
- Complete **TODO list** (`TODO.md`).
- Detailed **build instructions** (`CONTRIBUTING.md`).
- Change history (`CHANGELOG.md`).

---

*This document is intended for both human developers and the local‑hosted LLM agent. It provides clear, ordered actionable items with explicit file references, enabling automated execution without ambiguity.*
