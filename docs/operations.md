# Operational Playbook: Assimilation Pipelines, Local Model Infrastructure & Automotive Diagnostics

**Version:** `v1.7.0`  
**Classification:** Operational Context & Auxiliary Engineering Manual  

---

## 1. Reverse-Engineering Assimilation & Legacy Unification Pipeline

The Reverse-Engineering Assimilation Pipeline ingests external, unmanaged, or legacy codebases (such as legacy iterations of the Aaroneous Automation Suite) and systematically converts them into compliant Accelerated Component Containers (ACCs) without violating hypervisor memory-safety invariants.

```
┌────────────────────────────────┐
│ Legacy / Unmanaged Codebase    │
└───────────────┬────────────────┘
                │
                ▼ [1. Quarantine & Ingestion]
┌────────────────────────────────┐
│ Read-Only Staging Volume       │ ◄── Enforces static path fences & zero execution
└───────────────┬────────────────┘
                │
                ▼ [2. AST/DAG Structural Audit]
┌────────────────────────────────┐
│ Cratify AST / DAG Gatekeeper   │ ◄── `syn` AST analysis: Uncovers hidden coupling & heap footprints
└───────────────┬────────────────┘
                │
                ▼ [3. Transmutation & Synthesis]
┌────────────────────────────────┐
│ Isolated Scratchpad Generator  │ ◄── Converts `Vec`/`String`/`Box` ➔ Fixed `[u8; N]` + `Pod`
└───────────────┬────────────────┘
                │
                ▼ [4. Kernel Integration & Harvest]
┌────────────────────────────────┐
│ Production ACC Microkernel     │ ◄── `max_blast_radius = "isolated"` & priority backoff scheduled
└────────────────────────────────┘
```

### 1.1 Ingestion & Structural Extraction Stages
1. **Quarantine & Ingestion:** Legacy artifacts enter an isolated, read-only staging directory (`WorkspacePaths::discover().quarantine()`), completely separated from active kernel compilation to prevent compilation pollution.
2. **AST/DAG Structural Audit:** Utilizing **Cratify**, automated AST and DAG analysis maps dependencies, identifies hidden architectural coupling, and isolates dynamic memory footprints (`String`, `Vec`, `Box`).
3. **Transmutation:** Rather than manual rewriting, the assimilation pipeline refactors legacy logic by stripping heuristic layers, replacing unconstrained heap allocations with fixed-size stack buffers (`[u8; N]`, `[f32; N]`), and enforcing strict `bytemuck::Pod` + `bytemuck::Zeroable` contracts.
4. **Kernel Integration & Harvest:** Once code passes static AST verification and memory-layout checks, it is compiled into isolated Accelerated Component Containers (ACCs) governed by strict blast-radius parameters (`max_blast_radius = "isolated"`). Background harvesting is regulated by the Priority-Constrained Backoff Scheduler to ensure zero hypervisor starvation.

---

## 2. Expansion & Dogfooding Methods (Internal & External Sourcing)

To validate the framework under real-world workload pressure and safely ingest outside-influenced components, the system employs three synchronized expansion methods:

### 2.1 Internal Dogfooding via Developer HUD & Studio
- **Workstation-Native Stress Testing:** The unified `wgpu` studio context, NVML hardware telemetry, and latency oscilloscope are used directly by the operator to monitor and stress-test the local system.
- **Micro-Architectural Feedback:** By executing the actual hypervisor, telemetry consumers, and motor loops on the development host, scheduler jitter, frame drops, and cacheline contention are detected and mitigated natively.

### 2.2 The Transducer Sandbox (Outside Sourcing)
- **Zero-Trust Data Boundaries:** External third-party libraries and outside code are treated strictly as external data sources.
- **Stateless Transducer Mapping:** Foreign components are processed through the quarantine ingestion framework, analyzed for host safety, and mapped directly to single-instruction stateless transducer contracts (Text-in / Structured-Data-out).

### 2.3 Modular Combinatorial Expansion ($n!$)
- **Plug-and-Play State Bus:** Components decouple behind fixed C-ABI contracts (`MachinePacketHeader`) and atomic CAS ring buffers over memory-mapped shared regions.
- **Architectural Immunity:** Any new or externally-sourced module connects into the state bus seamlessly. If an incoming module violates deterministic zero-allocation rules or ABI alignment, Cratify rejects it at compile-time—protecting the platform against architectural drift.

---

## 3. Local Model Infrastructure & OpenCode Integration

To preserve privacy, eliminate external cloud dependencies, and guarantee deterministic latency, the hypervisor interfaces with local model infrastructure within the OpenCode workspace.

### 3.1 Local Model Orchestration (Qwen Family)
- **Engine Baseline:** Optimized for local GGUF/AWQ model backends (specifically Qwen-series models running via Ollama, LM Studio, or local llama.cpp endpoints).
- **Context Window Management:** Strict token budgeting caps prompt contexts to prevent memory spikes and keep inference latencies predictable.
- **Display & Identity Management:** Model names, routing tags, and display aliases are standardized across configuration files to ensure deterministic dispatch.

### 3.2 Local API Bridging & Tunneling
- **Loopback Sockets:** Primary communication binds to `127.0.0.1` via high-throughput HTTP/REST and WebSocket transports.
- **Secure Remote Tunneling:** Remote telemetry and diagnostic interfaces support authenticated ngrok and SSH reverse tunnels, requiring constant-time bearer token validation (`subtle::ConstantTimeEq`).

---

## 4. Hardware Diagnostics & Automotive Systems

The Aaroneous systems engineering mindset extends into physical, mechanical, and automotive embedded computing substrates.

### 4.1 Android Head Unit Diagnostics & Telemetry
Aaroneous integrates diagnostics for Android-based automotive infotainment platforms and head units (e.g., TopWay, Phoenix Automotive, and Rockchip/Allwinner-based motherboards):

```
┌─────────────────────────────────────────────────────────────┐
│               Automotive Hardware Diagnostic Bus             │
├───────────────────────────────┬─────────────────────────────┤
│ Protocol / Interface          │ Operational Target          │
├───────────────────────────────┼─────────────────────────────┤
│ ADB Shell Over TCP/USB        │ Low-level Android debugging │
│ CAN 2.0B / CAN-FD Ingestion   │ Real-time vehicle telemetry │
│ Factory Engineering Access    │ Root settings configuration │
└───────────────────────────────┴─────────────────────────────┘
```

- **Factory Access Codes & Engineering Menus:**
  - Common master access codes for factory configuration and MCU updates:
    - TopWay Platform: `123456`, `7890XX`
    - Phoenix Automotive / Universal Platforms: `7098HH`, `8888`, `000000`
- **ADB Shell Diagnostics:** Direct programmatic bridge to inspect system logs (`logcat`), monitor hardware thermal profiles, trace audio HAL routing, and query CAN-bus bridge daemons over USB and TCP.

### 4.2 Mechanical & Electrical Engineering Principles
- **Physical Boundary Isolation:** Software commands driving actuators, relay modules, or CAN injection enforce strict timeout and voltage/current bounds mirroring physical hardware limits.
- **Thermodynamic Reality:** Software backpressure algorithms and GPU/CPU throttling curves are directly calibrated against hardware heat dissipation characteristics and automotive operating temperature ranges (-40°C to +85°C).
