# Agent Configuration

## System Prompt

<system>
Role: Expert multi-platform Rust engineer. Editor: OpenCode Desktop.
Shell Execution Freedom: You have access to a multi-profile environment including Git Bash, PowerShell, and native Windows CMD. You are explicitly authorized to use any terminal setup required for the operation.
</system>

<rust_rules>
- Safety: Strict ownership/borrowing/lifetimes. No `unsafe`.
- Syntax: Clean `cargo clippy`/`cargo fmt`. Filenames above code blocks (e.g. `// src/main.rs`). Only output changed functions.
- Errors: No `.unwrap()`, `.expect()`, or panics. Use standard `Result`/`Option`. Use `thiserror`/`anyhow`.
- Concurrency: Use `tokio`, atomics, or safe channels.
</rust_rules>

<dynamic_shell_orchestration>
- Contextual Routing: Choose the shell that guarantees execution success. Use Git Bash for standard POSIX commands (ls, grep, cat). Use PowerShell or CMD for native Windows paths, system operations, or binary execution.
- Syntax Alignment: Match your command syntax perfectly to the chosen shell profile. Do not mix Windows backward slashes into Bash scripts, and do not use Unix pipes (`| head`) in CMD.
- Tool Preference: For reading files or navigating directories, always prefer native OpenCode API tools (`Read`, `ViewDirectory`) to completely bypass shell path parsing limitations.
- Anti-Looping: If a command fails in one shell with a syntax error, do not repeat it. Immediately pivot to an alternative shell profile or simplify the command format.
- Limit: Max 2 failed tool attempts per task before stopping to report the failure state.
- Atomic Git: One logical change per commit. End output with: `git commit -m "<type>(<scope>): <desc>"` (feat, fix, refactor, test).
</dynamic_shell_orchestration>

<format>
1. Concise architecture explanation (max 2 sentences).
2. Filename + code block.
3. Cargo verification or test commands specified for the appropriate shell.
4. Conventional Git commit command block.
</format>

---

## Active Roadmap & Frontier Framework

Aaroneous follows a 5-pillar, phased execution model progressing from foundational stabilization into deep OS observability and autonomous execution. See `TODO.md` for full details.

### Architectural Pillars

| Pillar | Scope | Key Components |
|---|---|---|
| **P1: Desktop Interaction Engine** | High-speed vision, window topology, HID dispatch, deep OS telemetry | DXGI capture (`windows-capture`), UIA tree walker, WASAPI audio loopback, ETW kernel consumer, `SpatialDeltaGate` |
| **P2: Synthetic Intelligence & Compiler** | Non-linguistic reasoning, thermodynamic verification, native JIT | `NativeComputationalGraph`, `cranelift-codegen` JIT, `cubecl` GPU SSM associative scan, SMT non-interference prover |
| **P3: Model Host & Distillation Foundry** | `.si` cartridge lifecycle, GGUF ingestion, multi-model hosting | `SiForge` builder, tensor extraction, frozen core + streaming LoRA, HNSW $\mathbb{R}^{256}$ associative memory |
| **P4: Adaptive Runtime Engine** | Live code patching, dynamic plugin swapping, self-modification | `libloading` C-ABI hot-reload, OGP LoRA adaptation, generational rollback journal |
| **P5: Developer Studio & Telemetry HUD** | Management UI, visual debugger, real-time instrumentation | Unified `wgpu` context, 3D DAG visualizer, latency oscilloscope, NVML hardware telemetry, spatial scene persistence |

---

### Phased Progression

| Phase | Version | Focus | Status | Key Deliverables |
|---|---|---|---|---|
| **Phase 1: Stabilization & Activations** | `v0.4.0` | Defect resolution & flag activations | **Complete** | Fix packet alignment & GDI leaks; activate `llama-gguf`, `nvml-wrapper`, `iroh` P2P; terminology cleanup |
| **Phase 2: Unified Pipeline & DXGI** | `v0.5.0` | Vision & rendering convergence | **Complete** | DXGI desktop capture (`platform_bridge`), modular HUD modes (`HUDModeManager`), spatial window manager |
| **Phase 3: Compiler & True JIT** | `v0.6.0` | Native code generation | **Complete** | `si_ir` extraction, `LatticeVerifier` dimensional unit checks, `cranelift-codegen` JIT compilation with W^X memory |
| **Phase 4: Foundry & Distillation** | `v0.7.0` | `.si` tooling & associative memory | **Complete** | `SiForge` pipeline (`distill`, `align`, `pack`, `verify`), `EpisodicMemoryFabric` (HNSW $\mathbb{R}^{256}$), GGUF seeding |
| **Phase 5: Adaptive Runtime & Live Patching** | `v0.8.0` | Safe dynamic modification | **Complete** | `libloading` dynamic C-ABI loader, streaming LoRA with OGP, `GenerationalJournal` thermodynamic rollback |
| **Phase 6: Multi-Node Fleet Mesh** | `v1.0.0` | Distributed execution & SMT gate | **Complete** | Full Iroh QUIC fleet, `FleetScheduler` work-stealing, `Z3Prover` non-interference formal gate |
| **Phase 7: Deep OS Observability & GPU Acceleration** | `v1.1.0` | Sensor fusion & GPU associative scans | **Complete** | UI Automation tree indexing, WASAPI audio loopback, ETW kernel ingestion, `cubecl` GPU SSM, intent-to-fascia daemon |
| **Phase 8: Systems Optimization & Release Hardening** | `v1.2.0` | Mechanical sympathy & micro-architectural tuning | **Complete** | `mimalloc` global allocator, Fat LTO profile, `smol_str` AST inlining, `_rdtsc` micro-timing, SoA storage |

---

### The 7-Horizon Frontier Matrix

| Horizon | Architectural Domain | Technical Mechanism & Target Deliverables |
|---|---|---|
| **H1** | **Autonomous Skill Synthesis & Trace Crystallization** | Automatic extraction of high-frequency execution traces into compiled Cranelift native plugins embedded in `.si` Block 3 habit stacks. |
| **H2** | **Deep OS Observability & Multi-Modal Sensor Fusion** | Quad-stream sensory pipeline: DXGI screen capture + UIA element tree + WASAPI loopback audio + non-polling ETW kernel events. |
| **H3** | **Formal SMT & Thermodynamic Verification** | Continuous lattice validation of 7-exponent SI base units with SMT-backed algebraic non-interference proofs for concurrent task graphs. |
| **H4** | **Associative Vector Memory Fabric** | In-memory `hnsw_rs` indexing over $\mathbb{R}^{256}$ latent trajectories providing $< 1\mu\text{s}$ nearest-neighbor habit and reflex recall. |
| **H5** | **Heterogeneous Fleet Swarm & Work-Stealing** | Multi-host Iroh QUIC mesh with Ed25519 node identities, dynamic load telemetry, and decentralized work-stealing for heavy computation graphs. |
| **H6** | **Sovereign SI-OS & Compositor** | Fluid RON spatial window canvas evolving toward a standalone Wayland/Direct3D12 compositor and bare-metal microkernel substrate. |
| **H7** | **In-Game Graphics Hooking & Zero-Latency Overlays** | In-process graphics injection via `hudhook` for DirectX 9/11/12 and Vulkan rendering pipelines with sub-frame action overlays. |

---

### Phase 7 Architecture Specification (`v1.1.0`)

1. **UI Automation (UIA) Engine (`crates/platform_bridge/src/observability/uia.rs`):**
   - Direct integration with Windows `IUIAutomation` to walk the active accessibility element tree in parallel with DXGI screen acquisition.
   - Extracts bounding rectangles, control types, accessibility names, and input focus states into structured metadata.
2. **WASAPI Audio Loopback Capture (`crates/platform_bridge/src/observability/wasapi.rs`):**
   - Dedicated background capture thread using `IAudioClient` in loopback mode (`AUDCLNT_STREAMFLAGS_LOOPBACK`).
   - Streams raw PCM audio frames directly into `WasapiAudioStreamAnalyzer` for real-time acoustic event tokenization.
3. **ETW Kernel Ingestion (`crates/platform_bridge/src/observability/etw.rs`):**
   - Real-time Event Tracing for Windows (ETW) consumer listening for process creation/termination, file I/O operations, and registry mutations without polling.
4. **Real GPU SSM Compute (`crates/compute/src/burn_gpu.rs`):**
   - High-throughput parallel associative scan compute shaders written in `cubecl` replacing CPU sequential recurrence loops for ultra-low latency inference ($< 180\mu\text{s}$).
5. **Intent-to-Fascia Watcher Daemon (`core/hypervisor/src/hud/fascia/`):**
   - Process & window title watcher monitoring foreground window transitions and dynamically loading matching `.ron` spatial canvas scenes via `notify` file watching.

---

### Terminology Migration Reference

| Legacy Term | Target Engineering Term | Domain / Scope |
|---|---|---|
| `epigenetic_gate` / `epigenetic sensory` | `TemporalSparsityFilter` / `SpatialDeltaGate` | Vision capture / compute shaders |
| `HermesRouter` | `LatentOrthogonalRouter` / `ProjectionRouter` | Layer 4 routing |
| `Cortex` | `ExecutivePlanner` / `StrategicPlanner` | Layer 4 compute |
| `Synapse` / `SynapseBridge` / `Oscilloscope` | `SignalBridge` / `BusChannel` / `SignalAnalyzer` | IPC / Telemetry |
| `NucleotidePacket` | `AlignedBitstreamPacket` / `FrameChunk` | IPC / Serialization |
| `DNA Bank` / `RocksDB DNA` | `ArtifactRegistry` / `StateBank` | Persistence |
| `Genome binary` | `ModelCartridge` / `CartridgeBinary` | `.si` runtime containers |
| `Neurochemistry` | `SystemThermodynamics` / `TelemetryState` | Telemetry UI / Metrics |
