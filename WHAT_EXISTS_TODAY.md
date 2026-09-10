# What Exists Today in Aaroneous (Verified & Audited)

This document provides an accurate, audited summary of the current codebase architecture, physical boundaries, and operational status following the **Cratify Batch 2 Decoupling** and **Terminology Modernization**.

---

## 🏛️ System Architecture & Crate Topology

Aaroneous is organized as a decoupled Rust workspace comprising **22 active member crates** and the core execution engine:

`
crates/
├── studio_hud/              # Pillar 5: Desktop Studio & Telemetry HUD (Eframe/WGPU GUI)
├── llm_gateway/             # Sovereign & Remote LLM Gateway (GGUF, OpenAI, Local, Mock, MCP translation)
├── ipc_bus/                 # Low-latency IPC transport (SPMC lock-free bus, SWMR shared memory, Disruptor)
├── paths/                   # Dynamic, platform-agnostic workspace & model path resolver
├── governance/              # Formal SMT verification (Z3), system limits, resource & throughput governors
├── compute/                 # Machine-native computation (.si containers, SSM recurrence, Cranelift JIT)
├── platform_bridge/         # Native OS abstractions (DXGI capture, Win32 HID, WASAPI loopback, ETW traces)
├── core-contracts/          # Zero-copy memory contracts (Pod derivations, ABI hashes, isolation tiers)
├── cratify/                 # Workspace decoupling & ACC governance certification engine (183/183 passing)
├── capabilities/            # Sovereign toolset, code auditor, and MCP service tools
├── orchestrator/            # Task scheduling, CPU core affinity pinning, and compaction engine
├── autonomic_adaptation/    # Adaptive parameter control, loss metrics, and capability specifications
├── adaptation_engine/       # Polyglot AST analysis, component forge, and FFI synthesis
├── transpiler/              # AST parser & distillation miner
├── omni/                    # 3D spatial graph navigation, node clustering, and vector index
├── si_format/               # Canonical .si container format, SIMD alignment, and CRC32 verification
├── si_ir/                   # Computational graphs, MachineOpcode IR, and dimensional type lattice
├── biology/                 # Process limits and metabolic state definitions
├── plugin_api/              # C-ABI dynamic library plugin interface
├── hotload/                 # Safe dynamic library hot-reloading engine
├── aaroneous_wire/          # Binary wire framing and serialization
└── aaroneous_api/           # Public client API bindings

core/
└── hypervisor/              # Central headless runtime engine (a_run, profile_compiler, shm_dump)
`

---

## 💎 Verified Architectural Boundaries

| Boundary | Architectural Design | Physical Reality in Code | Status |
| :--- | :--- | :--- | :--- |
| **Hypervisor ↔ Presentation / HUD** | Headless hypervisor runtime; UI isolated in standalone crate | Extracted into crates/studio_hud. _run binary has 0 GUI dependencies (egui, eframe). State shared lock-free via EngineStatePublisher snapshots. | **Clean & Decoupled** |
| **Hypervisor ↔ LLM / Inference** | Inference consumed strictly via trait contracts | Extracted into crates/llm_gateway. All inference providers (gguf, openai, local, mock), token cache, and McpGateway live in llm_gateway. | **Clean & Decoupled** |
| **Protocol / MCP ↔ UI** | Server & daemon protocols strictly headless | All constellation_ui and canvas components purged from crates/mcp_server and crates/orchestration_plane. | **Clean & Decoupled** |
| **Hypervisor ↔ Platform / OS** | OS abstractions isolated | Direct Win32, DXGI, WASAPI, and ETW interfaces isolated in crates/platform_bridge. | **Clean & Decoupled** |
| **Hypervisor ↔ Governance** | Formal logic proofs & verification | Z3 SMT non-interference provers and system governors live in crates/governance. | **Clean & Decoupled** |
| **IPC & Transport** | Lock-free, zero-copy messaging | Multi-consumer queues and shared memory isolated in crates/ipc_bus. | **Clean & Decoupled** |

---

## 🏷️ Modernized Terminology

All biological, neurological, and speculative terminology has been replaced across workspace crates, configuration files, and manifests:

- **IPC**: synapse, SynapseState -> ipc_bus, swmr_shm, spmc_shm_bus, shared_channel
- **Capabilities**: chromosome, hox, dna -> capability_schema, profile_schema, capability_registry
- **Execution**: enzyme, EnzymeRunner -> worker_runner, worker_types, 	ask_worker
- **Heuristics**: dopamine, curiosity -> eward_system, exploration_worker
- **Orchestration**: prefrontal_cortex, utonomic_loop -> intent_orchestrator, supervisory_loop
- **Topology**: hive, multi_hive, constellation -> cluster, multi_cluster, spatial_graph
- **Limits**: iology, homeostasis, 	hermodynamic_governor -> system_limits, esource_governor, 	hroughput_governor

---

## 🛠️ Executables & Tooling

1. **studio_hud Desktop HUD (cargo run --release -p studio_hud --bin aaroneous)**:
   - Native WGPU / Eframe 3D Constellation and Telemetry Studio.
   - Separate installer (aroneous-setup) and uninstaller (aroneous-uninstall) binaries.
2. **_run Hypervisor (cargo run --release -p a_run --bin a_run)**:
   - Headless CLI daemon, swarm mesh, and batch compilation runner.
3. **profile_compiler (cargo run --release -p a_run --bin profile_compiler)**:
   - Capability profile compiler and .si container assembler.
4. **shm_dump (cargo run --release -p a_run --bin shm_dump)**:
   - Shared memory diagnostics and state inspector.

---

## 🧪 Verification & Invariant Proofs

- **Workspace Build**: cargo check --workspace compiles cleanly with 0 errors.
- **Compliance Suites**: cargo test -p cratify passes **183 / 183 tests**:
  - udit_harness: 36 passed
  - ing_buffer_harness: 25 passed
  - saturation_harness: 29 passed
  - 	ranslation_harness: 93 passed
- **Gateway Test Suite**: cargo test -p llm_gateway passes **31 unit tests**.
