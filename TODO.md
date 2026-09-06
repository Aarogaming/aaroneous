# Aaroneous Orchestrator — Findings, Remediation & Prioritization Matrix

> Canonical master roadmap, issue tracker, and architectural prioritization for the Aaroneous Sovereign Runtime.

---

## Terminology Reference

| Concept | Technical Term | Crate / Location |
|---------|---------------|------------------|
| **Aaroneous** | Sovereign Machine-Native Runtime & Hypervisor | `a_run` (`core/hypervisor`) |
| **Specialists** | 9 Domain Task Engines & Cognitive Dispatch | `specialists` (`crates/specialists`) |
| **Tier Allocator** | Hypervisor CPU Affinity & Multi-Tier Thread Allocator | `orchestrator::tier_allocator` |
| **Desktop Emulator** | Win32 / Cross-Platform HID Input & Visual Overlay | `desktop_emulator` (`crates/desktop_emulator`) |
| **Maelstrom Native HUD** | Sovereign Pure-Rust DirectX 12/Vulkan Desktop Cockpit | `a_hud` (`core/hypervisor/bin/a_hud.rs`) |
| **Adaptation Engine** | Polyglot AST Transpiler, Binary Decompiler & Program Mutation | `adaptation_engine` (`crates/adaptation_engine`) |
| **IPC Bus** | Lock-Free SPMC Synapse Bus, LMAX Disruptor & Persistent WAL | `ipc_bus` (`crates/ipc_bus`) |
| **Compute Substrate** | Machine-Native Neural Execution (`.si` containers + SSM + JIT) | `compute` (`crates/compute`) |
| **Omni Galaxy** | 3D Spatial Knowledge Graph & Semantic Clustering Index | `omni` (`crates/omni`) |
| **Adaptive Control** | Closed-Loop State Regulation (`AutonomicStateRegulator` / `AdaptiveControlState`) | `autonomic_adaptation` (`crates/autonomic_adaptation`) |
| **Resource Governance** | Hardware Thermals & Compute Budget (`FeedbackRegulator` / `SystemHealthGovernor`) | `governance` (`crates/governance`) |

> [!NOTE]
> **Benchmarking Policy**: Performance benchmarking is deferred in favor of functional robustness, end-to-end integration stability, and multi-node federation correctness. No premature synthetic benchmark claims or micro-benchmarks are prioritized at this stage.

---

## 🛤️ Architectural Pillars & Phased Roadmap

### The 5 Architectural Pillars

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                           AARONEOUS UNIFIED STUDIO & HUD                         │
│       (Unified wgpu Context: Studio UI, DAG Visualizer, Telemetry, Recorders)    │
└────────┬──────────────────────┬──────────────────────────┬───────────────────────┘
         │                      │                          │
         ▼                      ▼                          ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────────────────┐
│ DESKTOP ENGINE   │  │ ADAPTIVE RUNTIME │  │ SYNTHETIC INTELLIGENCE & COMPILER    │
│ (Desktop Interact│  │ (Live Patch / Hot│  │  - Native Computational Graph (DAG)  │
│  & DXGI Capture) │  │  Reload Engine)  │  │  - Cranelift JIT / SSM Recurrence    │
└────────┬─────────┘  └────────┬─────────┘  │  - Edge Linguistic Lens (GGUF Ingest)│
         │                      │           └──────────────────┬───────────────────┘
         └──────────────────────┼──────────────────────────────┘
                                ▼
         ┌──────────────────────────────────────────────┐
         │             .si CARTRIDGE RUNTIME            │
         │  (Frozen Core + Streaming LoRA + Skill Stack) │
         └──────────────────────────────────────────────┘
```

| Pillar | Scope | Key Components |
|---|---|---|
| **P1: Desktop Interaction Engine** | High-speed vision, window topology, HID dispatch | DXGI capture (`windows-capture`), `SpatialDeltaGate` GPU shader, UIA accessibility indexing, parameterized action DAGs |
| **P2: Synthetic Intelligence & Compiler** | Non-linguistic reasoning, thermodynamic verification, native JIT | `NativeComputationalGraph`, `cranelift-codegen` JIT, `cubecl` GPU SSM, `EdgeLinguisticLens` boundary translation |
| **P3: Model Host & Distillation Foundry** | `.si` cartridge lifecycle, GGUF ingestion, multi-model hosting | `SiForge` builder, tensor extraction, frozen core + streaming LoRA, SSM (<180μs) + GGUF background |
| **P4: Adaptive Runtime Engine** | Live code patching, dynamic plugin swapping, self-modification | `libloading` ABI hot-reload, OGP LoRA adaptation, generational rollback journal |
| **P5: Developer Studio & Telemetry HUD** | Management UI, visual debugger, real-time instrumentation | Unified `wgpu` context, 3D DAG visualizer, latency oscilloscope, NVML hardware telemetry |

---

### Phased Implementation Roadmap

| Phase | Version | Focus | Pillars | Core Deliverables |
|---|---|---|---|---|
| **Phase 1: Stabilization & Activations** | `v0.4.0` | Defect resolution & flag activations | All | Fix packet alignment & GDI leaks; activate `llama-gguf`, `nvml-wrapper`, `iroh` P2P; terminology cleanup |
| **Phase 2: Unified Pipeline & DXGI** | `v0.5.0` | Vision & rendering convergence | P1, P5 | Replace GDI with `windows-capture` (DXGI); migrate `eframe` to `wgpu`; share GPU context |
| **Phase 3: Compiler & True JIT** | `v0.6.0` | Native code generation | P2 | `cranelift-codegen` for `MachineOpcode` JIT; `cubecl` for GPU SSM recurrence |
| **Phase 4: Foundry & Distillation** | `v0.7.0` | `.si` tooling & model hosting | P3 | `SiForge` CLI/UI, GGUF tensor extraction, episodic skill DAG inspection |
| **Phase 5: Adaptive Runtime & Live Patching** | `v0.8.0` | Safe dynamic modification | P4 | Hot-reload ABI plugins, generational rollbacks, sandboxed adaptation |
| **Phase 6: Multi-Node Fleet Mesh** | `v1.0.0` | Distributed execution | All | Full Iroh QUIC mesh, work stealing, P2P `.si` cartridge sync |
| **Phase 7: Deep OS Observability & GPU Acceleration** | `v1.1.0` | Sensor fusion & GPU associative scans | P1, P2, P5 | UIA tree walker, WASAPI loopback audio, ETW kernel consumer, `cubecl` GPU SSM, intent-to-fascia daemon |
| **Phases 8–17: Native Performance & 3D Galaxy Studio** | `v1.2.0` | Micro-architectural hardening & 3D cosmos | All | `mimalloc`, Fat LTO, `smol_str`, native WGPU 3D Constellation Studio, SSE telemetry |
| **Phases 18–24: Sovereign Cartridge & Formal Governance** | `v1.3.0` | `.si` container standard & Z3 SMT action interlocks | P2, P3, P4 | Canonical `.si` v3.0, Z3 SMT non-interference gates, Fitts's law Bézier kinematics, Macro-SSM recurrence |
| **Phases 25–29: Hardware Saturation & Sparse MoE** | `v1.4.0` | Memory-mapped MoE module registers & self-play | P1, P2, P3 | 16-slot sparse expert register, CAN 2.0B/FD, contiguous VRAM slab, Crucible virtual sandbox |
| **Phases 30–33: Machine-Native Intent & Heterogeneous NPU** | `v1.5.0` | Frictionless user ecosystem & NPU offloading | All | Auto-tuner, drag-and-drop `.si-pack`, decoupled Linguistic Lens, `.lib` state bank, 45 TOPS NPU acceleration |
| **Phase 34: Industrial OT Edge Mesh & Event Bus** | `v1.6.0` | Deterministic fieldbus & monotonic IPC | P1, P5 | COBS `aaroneous_wire`, `OtEdgeGateway`, Workbench OT tab, `UniversalEventBus` |
| **Phases 35–37: Console-OS Shell & Merlin Companion** | `v1.6.0` | Biometrics, gamified routines & interactive assistant | **Complete** | Decoupled Console-OS, 3D Constellation Skills, UserBaseline, Merlin companion |
| **Phase 38: Triad Shell Convergence & Micro-Latency** | `v1.7.0` | Capability broker, zero-lock snapshots, resource governor | **In Progress** | `CapabilityBroker`, `EngineStatePublisher`, dirty-flag pacing, panic isolation, `rkyv`, disruptor |

> [!TIP]
> **Completed Phases & Defects Archive**: Detailed task-level records for completed historical phases (Phases 1 through 37) and defect remediation audits are preserved in [dev/docs/blueprints/COMPLETED_PHASES_ARCHIVE.md](file:///d:/Aaroneous/dev/docs/blueprints/COMPLETED_PHASES_ARCHIVE.md).

---

## 🎯 Master Prioritization Matrix (Re-Prioritized by Safety & ROI)

#### 🛡️ Tier 0: Critical Safety, Containment & High-Velocity ROI (Immediate Focus)
*Direct safety containment, zero-risk learning loops, and immediate user-facing value.*

| Priority | Phase / Area | Strategic Rationale (Safety & ROI) | Status |
|---|---|---|---|
| **P0-A (Safety)** | **Phase 29: The Crucible: Sealed Virtual Self-Play & Verification Studio** | **Safety: Maximum.** Airgapped W^X sandbox ensures models cannot escape or issue rogue commands.<br>**ROI: Massive.** 10,000 self-play cycles/min using local LMStudio/free APIs. Instant bootstrap. | **Complete** |
| **P0-B (ROI)** | **Phase 31: Machine-Native Intent Mapping & Linguistic Lens** | **Safety: High.** Direct Opcode DAG invariants eliminate prompt injection and syntax hallucinations.<br>**ROI: Immediate.** Enables conversational speech/chat & zero-latency phoneme voice out (`AcousticVoiceSynthesizer`). | **Complete** |
| **P0-C (Hardware ROI)**| **Phase 33: Heterogeneous Silicon: Native NPU Acceleration** | **Safety: Medium.** Hardware memory isolation via DirectML/NPU drivers.<br>**ROI: Immediate.** Wakes up unused 45 TOPS NPU at ~2W power. Zero FPS drop in games/rendering. | **Complete** |
| **P0-D (Data ROI)** | **Phase 32: The `.lib` Binary State Bank & Zero-Copy Vault** | **Safety: High.** Cryptographic Merkle audit trail, crash WAL snapshotter, & sleep memory consolidation.<br>**ROI: High.** SIMD-compressed storage eliminates text serialization parsing bottlenecks. | **Complete** |

#### 🏎️ Tier 1: High-Performance Hardware & Physical Actuation (Expansion Focus)
*Physical hardware deployment, maximum throughput, and real-world execution.*

| Priority | Phase / Area | Strategic Rationale (Safety & ROI) | Status |
|---|---|---|---|
| **P1-A (Physical ROI)**| **Phase 26: Automotive, Embedded Robotics & Head-Unit Bridge** | **Safety: Critical.** SMT mathematical fail-safe fence & autonomous protocol entropy analyzer.<br>**ROI: High.** Immediate physical proof on mobile robotics and vehicle CAN translation. | **Complete** |
| **P1-B (Perf ROI)** | **Phase 27: Full-Tilt Hardware Saturation & Zero-Copy VRAM** | **Safety: High.** Eliminates SMT solver timeouts and race conditions under 120 FPS maximum load.<br>**ROI: High.** Unlocks raw wire-speed performance by keeping 4K video entirely in GPU VRAM. | **Complete** |
| **P1-C (User ROI)** | **Phase 30: Zero-Friction End-User Simplicity & Single-Click** | **Safety: High.** Auto-hardware detection prevents users from misconfiguring memory or GPU profiles.<br>**ROI: Universal.** 1-click installer and drag-and-drop `.si-pack` enables non-coder adoption. | **Complete** |

#### 🌌 Tier 2: Spatial Immersion & Deep Future Frontiers (Exploration Focus)
*Visual spatial immersion, long-term research, and quantum horizons.*

| Priority | Phase / Area | Strategic Rationale (Safety & ROI) | Status |
|---|---|---|---|
| **P2-A (Spatial ROI)**| **Phase 28: 3D Spatial Constellation Engine & Galactic Roaming** | **Safety: Low.** Read-only visual spatial HUD canvas.<br>**ROI: High Immersion.** Converts abstract node telemetry into an interactive 3D galaxy. | **Complete** |

---

## Active & Frontier Roadmap

> [!TIP]
> **Completed Phases & Defects Archive**: Detailed task-level records for completed historical phases (Phases 1 through 37) and defect remediation audits are preserved in [dev/docs/blueprints/COMPLETED_PHASES_ARCHIVE.md](file:///d:/Aaroneous/dev/docs/blueprints/COMPLETED_PHASES_ARCHIVE.md).

### Phase 38: Triad Shell Convergence, Resource Governance & Micro-Latency Architecture (In Progress)
*Low-overhead state-sharing, adaptive dirty-flag frame pacing, thermal/VRAM backpressure, and isolated shell resilience.*

- [x] **SHELL-01: Intermediary Capability Broker (`CapabilityBroker`)**
  - Implemented `core/hypervisor/src/capability_broker.rs`: dynamic self-describing capability registry and latency-tracked execution dispatch across 7 domains connecting Action Palette, Intercom, and Specialists.
- [x] **SHELL-02: Zero-Lock Engine State Publisher & Projections (`EngineStatePublisher`)**
  - Implemented `core/hypervisor/src/hud/state_snapshot.rs`: lock-free point-in-time state snapshots with shell-tailored projections (`StudioProjection`, `ConsoleProjection`, `HudProjection`) eliminating rendering locks.
- [x] **PERF-01: Event-Driven Frame Pacing & Dirty-Flag Invalidation**
  - Implemented reactive frame pacing in `core/hypervisor/src/hud/app.rs`: detects dirty `EngineSnapshot::bus_generation`, recent user interaction, and active modals, dropping idle repainting to a quiet 250ms cadence (4 FPS) to free up host cycles for inference.
- [x] **PERF-02: Resource Governor & Thermal/VRAM Backpressure**
  - Implemented `GovernorPacing` (`FullPerformance`, `ThermalThrottled`, `CriticalVramSave`) in `core/hypervisor/src/hud/state_snapshot.rs`: dynamically throttles shell target frame durations (8ms, 16ms, 33ms) based on hardware headroom.
- [ ] **PERF-03: Zero-Copy String Interning & Memory-Mapped Telemetry**
  - Replace ad-hoc `String` heap allocations during 120 FPS render ticks with borrowed slices or `SmolStr` directly from the 64MB memory-mapped ring buffer.
- [ ] **CMD-01: Compile-Time Strongly Typed Command Registry**
  - Transition Action Palette macros to static enum and trait definitions to prevent string typo failures and bypass JSON serialization overhead.
- [ ] **CMD-02: Input Debouncing & Ring Buffer Queue Backpressure**
  - Protect `CapabilityBroker` from input flood (gamepad analog stick oscillations or rapid key repeats) via token-bucket debouncing.
- [x] **STAB-01: Isolated Panic Boundaries for Visual Shells**
  - Implemented `std::panic::catch_unwind` isolation around `render_full_studio`, `ConsoleOsLauncher::render`, `render_transparent_hud`, and `render_compact_recorder_overlay` in `core/hypervisor/src/hud/app.rs`: recovers into a visual safe-mode banner without terminating hypervisor background tasks or auto-pilot loops.
- [ ] **STAB-02: Unified Structured Tracing Filtered per Shell**
  - Configure tiered `tracing::LevelFilter` per shell: HUD (`WARN/ERROR`), Console (`INFO`), Studio (`DEBUG/TRACE`).
- [ ] **STAB-03: Hot-Reloadable Styling & Spatial Canvas Layouts**
  - Extend `.ron` spatial canvas persistence to theme tokens and window layouts for zero-recompile visual iteration.
- [ ] **UX-01: Seamless Gamepad / Keyboard Focus Snapping Across Studio & Console**
  - Unify D-Pad / Arrow navigation across Console cards and Studio sidebar tabs.
- [ ] **UX-02: Audio-Loopback Intercom Activation (`WasapiVoiceTrigger`)**
  - Connect WASAPI loopback capture directly to the HUD intercom for hands-free co-pilot intent triggers.
- [ ] **SHELL-03: Detached Transparent Window Pipeline (Click-Through Win32 HUD)**
  - Spawn standalone click-through viewport with `WS_EX_TRANSPARENT | WS_EX_LAYERED` attributes so the HUD floats unobtrusively over IDEs without stealing input focus.
- [ ] **SHELL-04: 2D Spatial Focus Grid for 10-Foot Console Navigation**
  - Replace 1D index snapping with a 2D geometric nearest-neighbor spatial navigator for analog stick and D-pad movement across asymmetrical cartridge panels.
- [ ] **SHELL-05: Headless Autonomous Flight Target (`--headless`)**
  - Decouple engine lifecycle from display context: execute full ANS loop, background AST mutation, and test repairs in headless environments without initializing GUI graphics.
- [ ] **DIST-01: Delta-Compressed Telemetry Streams (`EngineDelta`)**
  - Transmit bitmask-tagged differential state deltas instead of full monolithic struct snapshots across high-frequency 200Hz IPC channels.
- [ ] **DIST-02: Zero-Copy Multi-Process Shared Memory (`shm`) Ring Buffer**
  - Wire memory-mapped circular ring buffer for out-of-process visual shell isolates, providing $< 1\mu\text{s}$ state synchronization without socket serialization.
- [ ] **DIST-03: Black-Box Flight Recorder (Deterministic Event Replay)**
  - Journal causal sequence of capability broker commands, inputs, and state hashes into a 16MB circular `.flight` binary log for deterministic debugging of model stalls or build failures.
- [ ] **LLM-01: Prefix Cache-Aware Prompt Templating**
  - Pin immutable system prompts, specialist tool schemas, and repository guidelines to byte offsets $0..K$ to maximize KV-cache reuse in LM Studio / llama.cpp.
- [ ] **LLM-02: Context-Aware Workspace Slicing & AST Pruning**
  - Prune private function bodies and non-essential implementations before injecting codebase context into local models, preserving attention span on public APIs and active diffs.
- [ ] **SWARM-01: Multi-Hive Swarm Sub-Agent Offloading (`SwarmOffloader`)**
  - Connect `core/hypervisor/src/federation/multi_hive/swarm_offloader.rs` to autonomic sub-agents: offloads heavy AST parsing, SVDD audits, and test generation to peer nodes when workstation load exceeds threshold.
- [ ] **ADAPT-01: Streaming Self-Correction & Autonomous Pacing Regulation**
  - Wire `streaming_adaptation.rs` into the hypervisor main loop: dynamically adjusts telemetry polling cadences and JIT recompilation rates based on real-time system thermodynamics and CPU/GPU pressure.
- [ ] **SAFE-01: SMT Formal Verification Interlock Gatekeeper (`Z3Prover`)**
  - Connect `crates/governance/src/smt_action_interlock.rs` as a mandatory pre-commit gate before applying AST rewrites, proving non-interference and invariant safety mathematically to eliminate rollbacks.
- [ ] **SAFE-02: Sandboxed In-Process Micro-VM & WASM Enzyme Isolation**
  - Enforce `core/hypervisor/src/micro_vm.rs` gas-metered execution on all ad-hoc diagnostic scripts and custom automation macros, isolating host OS files and child process creation.
- [ ] **OS-01: MMCSS Thread Priority Boosting & Core Affinity Pinning**
  - Register the hypervisor reflex loop and capability broker with Windows MMCSS ("Games" / "Pro Audio" profile via `mmcss.rs`) to prevent scheduler preemption during heavy local LLM inference.
- [ ] **OS-02: Direct DXGI / Vulkan Swapchain Present Hooking**
  - Inject telemetry overlay directly into graphics presentation via `swapchain_present.rs`, eliminating DWM desktop composition lag for true zero-latency in-game overlays.
- [ ] **ACCEL-01: WGSL Reflex Compute Shader Pipeline Offloading**
  - Activate `wgpu_reflex_pipeline.rs` compute shaders for 128x128 screen delta gating and sensory diff aggregation on GPU, bypassing CPU host round-trips.
- [ ] **ACCEL-02: Cranelift JIT Compilation for Hot Enzyme Rules**
  - Compile high-frequency routing paths, telemetry transforms, and filter pipelines directly into native machine instructions via `cranelift_jit.rs` to eliminate AST evaluation overhead.
- [ ] **DRIFT-01: Concept Drift & Autonomic Cognitive Equilibrium Monitor**
  - Ingest latent trajectory drift scores from `concept_drift.rs`: automatically detect model degenerate loops or repetitive failures, triggering automated context compaction and prompt recalibration.
- [ ] **DRIFT-02: Chaos Monkey Fault Injection for Safety & Rollback Hardening**
  - Intermittently inject dropped packets, synthetic VRAM pressure spikes, and simulated I/O errors via `chaos_monkey.rs` during automated tests to formally certify fault-recovery resilience.
- [ ] **MEM-01: Zero-Copy Memory-Mapped Deserialization (`rkyv` Archives)**
  - Integrate `rkyv` zero-copy archiving (validated in `core/hypervisor/src/rkyv_test.rs`) for telemetry trees across thread and process boundaries, allowing shells to read snapshot bytes directly in-place without heap allocations.
- [x] **MEM-02: LMAX Disruptor Lock-Free Ring Buffer Upstream Command Queue**
  - Connected `crates/ipc_bus/src/disruptor.rs` into `CapabilityBroker` for upstream shell-to-engine command ingress and execution logging, avoiding crossbeam channel cache-line bouncing and locking command latency in the sub-microsecond range.
- [ ] **MEM-03: Single-Writer Multi-Reader (SWMR) Synapse Bus**
  - Leverage `crates/ipc_bus/src/swmr_synapse.rs` and `spmc_synapse_bus.rs` for atomic broadcast slots where the hypervisor writes state monotonically and all shells read concurrently without lock contention.
- [x] **PROF-01: Cycle-Accurate Hardware Timing (RDTSC Engine Ticks)**
  - Integrated `crates/platform_bridge/src/observability/rdtsc.rs` into `CapabilityBroker` (`timing.rdtsc_profiler`) for cycle-accurate, sub-microsecond timing without OS system-call overhead.
- [ ] **PROF-02: Native Windows ETW Kernel Trace Ingestion**
  - Connect `crates/platform_bridge/src/observability/etw.rs` to real-time telemetry, tracking GPU context switching, driver stalls, and CPU scheduler preemptions from local LLMs.
- [ ] **PROF-03: Direct Raw Input Hooking**
  - Wire `crates/platform_bridge/src/observability/raw_input.rs` to bypass Windows desktop `WM_INPUT` message pump latency for immediate gamepad and shortcut response in Console and HUD.
- [ ] **SSM-01: HiPPO Polynomial Long-Horizon State-Space Memory**
  - Connect `crates/compute/src/hippo.rs` and `macro_ssm.rs` to compress chronological execution logs into continuous polynomial memory projections, preserving multi-hour session context without token blowout.
- [ ] **SSM-02: Latent-Space Semantic Guardrailing**
  - Integrate `crates/compute/src/latent_guardrail.rs` and `latent_router.rs` to evaluate shell commands against embedding vectors prior to LLM invocation, catching redundant or invalid requests locally.
- [ ] **DEV-01: Embedded Debug Adapter Protocol (DAP) Server**
  - Wire `crates/orchestrator/src/dap_server.rs` into Studio and IDEs to step through agent decision trees, pause on failed assertions, and inspect live memory state mid-flight.
- [ ] **DEV-02: Automated AST Pattern Rewriting & Patch Verification**
  - Connect `crates/adaptation_engine/src/pattern_rewriter.rs` and `crates/capabilities/src/codebase_auditor.rs` to execute deterministic AST search-and-replace rules for trivial syntax and deprecation fixes without LLM round trips.
- [ ] **SENS-01: WASAPI Audio Feature Extraction & Voice Intercom**
  - Connect `crates/platform_bridge/src/observability/wasapi.rs` and `audio_features.rs` directly to the HUD Intercom for zero-overhead local speech transcription and ambient audio visualization.
- [ ] **SENS-02: Hardware RGB Telemetry Status Sync**
  - Wire `crates/platform_bridge/src/observability/hardware_rgb.rs` to hypervisor health channels, mapping CI safety gates, active LLM inference, and test failures to chassis/peripheral LEDs.
- [ ] **SENS-03: MIDI & OSC Physical Control Deck Integration**
  - Bind `crates/platform_bridge/src/adapters/midi_osc.rs` to Studio and Console parameters (timeline scrub, model temperature, telemetry mute) with zero focus interruption.
- [ ] **SENS-04: NDI Zero-Latency Video Broadcasting**
  - Activate `crates/platform_bridge/src/adapters/ndi_broadcast.rs` to stream rendered Studio panels or the 3D Constellation canvas over LAN to secondary auxiliary tablets.
- [ ] **EXEC-01: Neurochemical & Dopamine Reinforcement Engine for Agent Loops**
  - Wire `core/hypervisor/src/dopamine_system.rs` and `crates/autonomic_adaptation/src/neurochemistry.rs` into AFC autonomy thresholds, penalizing context blowout and rewarding clean builds.
- [ ] **EXEC-02: Executive Metacognition & Prefrontal Planning Gate**
  - Enforce `core/hypervisor/src/prefrontal_cortex.rs` and `executive_plan.rs` supervisory decomposition of tasks into formally verifiable sub-goals prior to modifying code.
- [ ] **EXEC-03: Temporal Event Dilation & Simulation Engine (Relativity Clock)**
  - Integrate `core/hypervisor/src/relativity_engine.rs` to decouple task execution from wall-clock time, allowing stress tests and regressions to run at virtual warp speeds.
- [ ] **MATH-01: Game-Theoretic Multi-Agent Resource Arbitration (Nash Equilibrium)**
  - Connect `crates/compute/src/game_theory.rs` to arbitrate competing specialist access to limited VRAM and CPU cycles without thread starvation or priority inversion.
- [ ] **MATH-02: Thermodynamic Entropy Budgeting & Metabolic Governance**
  - Integrate `crates/compute/src/thermodynamics.rs` and `crates/governance/src/metabolic_governor.rs` to enforce hourly thermodynamic entropy caps and prevent workstation thermal throttling.
- [ ] **MATH-03: Symbolic Algebraic Simplification for Static Bounds Checking**
  - Wire `core/hypervisor/src/symbolic_math.rs` ahead of rustc compilation to catch out-of-bounds slice indexing and array panics symbolically.
- [ ] **GEN-01: HOX Gene-Regulatory Network for Specialist Agent Breeding**
  - Connect `core/hypervisor/src/hox_registry.rs` and `hox_breeding_simulator.rs` to dynamically evolve task-specialized sub-agent personas and parameter archetypes.
- [ ] **GEN-02: Autonomic Codebase Self-Digestion (Dead-Code Harvesting)**
  - Wire `crates/autonomic_adaptation/src/self_digestion.rs` to continuously crawl the workspace AST, removing orphaned interfaces to maintain compact model context.
- [ ] **GEN-03: 3D Constellation & Galaxy Node Graph Visualization**
  - Integrate `core/hypervisor/src/constellation_3d.rs`, `galaxy_map_3d.rs`, and `crates/omni/src/ecs_galaxy.rs` into the Studio shell for real-time spatial graph telemetry.
- [x] **PERC-01: Windows UI Automation (UIA) Tree Interception**
  - Connected `crates/platform_bridge/src/observability/uia.rs` into `CapabilityBroker` (`screen.inspect_uia`) to extract native button states, text controls, and accessibility hierarchies with zero GPU vision overhead.
- [ ] **PERC-02: Zero-Latency Win32 Desktop Duplication Direct into Shared Memory**
  - Wire `core/hypervisor/src/win32_intercept/capture.rs` and `shmem_capture.rs` to stream frames directly from the DirectX Desktop Duplication API into shared memory for instantaneous perception.
- [ ] **HW-01: CAN Bus Telemetry Gateway for Vehicle & Engine Telemetry**
  - Wire `crates/platform_bridge/src/robotics/canbus.rs` into the `CapabilityBroker` and Studio dashboard to ingest vehicle OBD-II metrics and ECU states directly.
- [ ] **HW-02: Direct GPIO & Serial Port Microcontroller Polling**
  - Connect serial port and GPIO interfaces from `data/fabrication/` into the `CapabilityBroker` to allow agents to directly trigger and monitor physical hardware pins and relays.
- [ ] **XR-01: OpenXR Native Spatial Rig Provider**
  - Connect `core/hypervisor/src/federation/ar/openxr_provider.rs` to project the 3D Constellation galaxy and DAG visualizer into mixed-reality headsets for immersive spatial debugging.
- [ ] **BIO-01: Bluetooth Low-Energy (BLE) Biometric Telemetry Streaming**
  - Wire `core/hypervisor/src/federation/biometric/ble_provider.rs` directly into the engine's `dopamine_system` and user baseline engine to dynamically regulate UI intensity and notification cadences from wearable biometrics.
- [ ] **RESIL-01: Dynamic Self-Rebuild & In-Flight Binary Swapping**
  - Connect `crates/adaptation_engine/src/self_rebuild.rs` and `self_repair.rs` to trigger automated compilations, verify binary integrity, and hot-swap executables without losing hypervisor background session state.
- [ ] **RESIL-02: Automated Scientific Hypothesis Engine**
  - Enforce `crates/adaptation_engine/src/analysis/hypothesis.rs` and `experiment.rs` in autonomous loops to formalize explicit hypotheses and record experimental outcomes directly into `episodic_memory`.
- [ ] **ARCH-01: Demand-Driven Query Memoization (The `rust-analyzer` Salsa Model)**
  - Wrap AST and semantic index parsing in a fine-grained, demand-driven query cache (`salsa`/lazy computation): when files are modified by the splicing engine or agent, invalidate only affected dependency nodes, preserving instant warm cache for prompt injection and model context retrieval.
- [x] **ARCH-02: Atomic Pointer Swapping & Lock-Free UI Projections (The `Zed` ArcSwap Model)**
  - Decoupled background language servers, autonomic loops, and telemetry ingestion from UI render passes via `EngineStatePublisher` point-in-time snapshot swapping; allows visual overlays (`iced`, `slint`, `ratatui`, `egui`) to acquire immutable `Arc<EngineSnapshot>` and domain projections in nanoseconds with zero lock contention.
- [ ] **ARCH-03: Data-Oriented Archetype Component Storage (The `Bevy` ECS Model)**
  - Restructure swarm agents, hardware metrics, and `mcp_gateway` events into Struct-of-Arrays (SoA) archetypes in `crates/omni/src/ecs_galaxy.rs`; enables the `spatial_kinetic_engine` and batch processor to scan thousands of agent states sequentially with maximal L1/L2 CPU cache hit rates.
- [ ] **ARCH-04: SIMD Finite State Machine Scanning (The `ripgrep` Aho-Corasick Model)**
  - Implement SIMD-accelerated multi-pattern string search (via `aho-corasick`) in `crates/capabilities/src/fs_crawl.rs` and `simd_xor_delta.rs` for extracting tool calls, fence boundaries, and token markers from raw model streams without heap allocations.
- [ ] **ARCH-05: Arena Bump Allocation for Ephemeral Flight Contexts (The `bumpalo` Model)**
  - Allocate temporary token chunks, `si_distiller` buffers, and `tensor_router` flight cycle payloads into a scratch bump-allocation arena (`bumpalo`), resetting the entire arena in an $O(1)$ pointer reset at the end of each flight cycle to completely eliminate heap fragmentation.
- [x] **ARCH-06: Compile-Time Typestate Invariants for Plan Safety (The Typestate Model)**
  - Refactored `executive_plan.rs` into compile-time typestates (`ExecutivePlanState<Draft>` $\to$ `ExecutivePlanState<Verified>` $\to$ `ExecutivePlanState<Executing>`); statically guarantees that plans can only execute after formal mathematical validation via `verify_with_proof` or `verify_interlock` through `SmtActionInterlock`.

---

*Last updated: 2026-09-06 | Complete 5-pillar, 38-phase architectural framework*


