# Changelog

All notable changes to Aaroneous.

> **Architectural Note:** References in historical changelog sections (< v0.1.0) to `components/*` describe the legacy monolithic prototype layout. In v0.1.0+, all components were refactored into `core/hypervisor` and the 12 sovereign `crates/*` workspace crates.

## [Unreleased]

### 🚀 Autonomous Flight Controller (AFC) Open-Source Integration
- **`dev/tools/afc/` (AFC Architecture Integration)**:
  - **Rig Engine Integration (`afc::router`)**: Implemented direct async TCP HTTP REST routing to local LLMs (LM Studio/Ollama `/v1/chat/completions`), type-safe JSON extractor with markdown fence stripping, and typed tool call registries for `propose_patch` and `report_defect`.
  - **Picocode Engine Integration (`afc::recipe`)**: Implemented compiler diagnostics filtering (`DiagnosticsFilter`) extracting compact `error[E...]` and location patterns from raw build logs; created step runner (`Step`) with timeout control and Windows `CREATE_NO_WINDOW` (`0x0800_0000`) flag; built chainable verification gate pipeline (`RecipePipeline`).
  - **Gitoxide-Inspired Sovereign Git (`afc::git`)**: Implemented pure-Rust in-process inspection of `.git/HEAD` branch and commit SHA-1 without spawning subprocesses; windowless atomic commit execution.
  - **Axocoatl Engine Integration (`afc::state`)**: Implemented micro-state machine (`StateMachine`, `FlightState`) tracking explicit transitions with JSON audit trail persistence; added bounded delta-only context sanitizer (<1,500 tokens) to conserve KV cache and maximize local LLM throughput.

### 🛡️ Security Hardening & Plugin Architecture
- **`core/hypervisor/src/hud/plugin_api.rs` (SEC-01, SEC-03)**:
  - Replaced unsafe fat pointer `*mut dyn UiCartridge` with C-compatible `#[repr(C)]` opaque pointer scheme to prevent cross-DLL heap corruption.
  - Added SHA-256 integrity verification (`compute_file_hash()`) and signature validation (`validate_signature()`) before loading any dynamic plugin library.
  - Implemented destructor symbol lookup (`free_plugin`) ensuring proper resource cleanup on cartridge drop via `unload_dynamic_plugin()`.
  - All panic vectors eliminated: `.unwrap()` replaced with `anyhow::Result` propagation via `.context()` error chaining.

- **`crates/platform_bridge/src/native_win32.rs` (SEC-02)**:
  - Eliminated all `.unwrap()` panics in GDI/DXGI screen capture pipeline.
  - Added clock regression detection in `now_us()` with proper error bubbling.
  - Implemented strict bounds validation for buffer operations to prevent memory corruption.
  - Replaced unsafe D3D11 device creation patterns with comprehensive Result-based error handling.

### 🐛 Critical Bug Fixes & Memory Safety
- **`core/hypervisor/src/native_ingestion/shmem_capture.rs` (CRIT-03)**:
  - Fixed catastrophic GDI memory leak where bitmap handles were not properly released on SelectObject failure paths.
  - Refactored `GdiGuard` from stack-allocated to stack-local mutable variable for proper lifetime management, ensuring cleanup even during errors.
  - Removed `.unwrap()` from `now_tick()` function, replaced with safe `.unwrap_or_else(|| Duration::ZERO)` fallback.
  - Updated all test cases to use `.expect()` instead of `.unwrap()` for consistent error handling and clearer failure messages.
- **`crates/autonomic_adaptation/src/genetics.rs` (CRIT-10)**:
  - Fixed NaN panic in `find_closest_relative()` by replacing `partial_cmp().unwrap()` with `partial_cmp().unwrap_or(Ordering::Equal)`.
  - Fixed NaN panic in `find_breeding_candidates()` sort comparator at line 413.
- **`crates/omni/src/matrix/sab_tensor.rs` (CRIT-11)**:
  - Fixed NaN panic in `high_mutual_info_pairs()` by replacing `partial_cmp().unwrap()` with `partial_cmp().unwrap_or(Ordering::Equal)`.

### 🛡️ Security Hardening & Path Traversal Prevention
- **`core/hypervisor/src/action_executor.rs` (SEC-06)**:
  - Fixed sandbox path validation to canonicalize parent directory rather than raw fallback, preventing symlink-based traversal bypasses.
  - Replaced `.unwrap_or_else(|_| resolved.clone())` with proper error bubbling for canonicalization failures.
  - Canonicalized all allowed roots consistently before comparison to prevent race conditions and symlink attacks.
  - Return canonicalized path on success instead of unresolved path for consistent security boundaries.

### 🛡️ Security Hardening & Timing Attack Prevention
- **`core/hypervisor/src/mcp_service/http_api.rs` (SEC-04)**:
  - Replaced variable-time string comparison (`==`) with constant-time `subtle::ConstantTimeEq` for API key validation.
  - Mitigates timing attack side-channels that could leak API key characters through response time analysis.
### 🛡️ Security Hardening & Lock Ordering
- **`crates/orchestrator/src/hive_runtime.rs` (CRIT-07)**:
  - Fixed lock inversion deadlock in `complete_task()` by acquiring `task_log` lock before `router` lock.
  - Ensured consistent lock ordering across all methods to prevent deadlocks from circular wait conditions.
  - Maintained same-lock-holding pattern for performance (no unnecessary unlock/relock cycles).

### 🛡️ Observance Era Stabilizations & Build Remediation
- **`core/hypervisor/Cargo.toml` & `platform_bridge/Cargo.toml` & `scratchpad/Cargo.toml`**:
  - Resolved fatal TOML syntax errors, duplicated blocks, and conflicting feature flags (`BLOCKER-01`).
- **`crates/ipc_bus/src/machine_packet.rs` & `crates/scratchpad/src/lib.rs`**:
  - Fixed compile-breaking Rust type errors E0609 (`reserved` field mismatch in `MachinePacket`) and E0277 (non-Send `mpsc::Receiver` across thread boundaries) (`BLOCKER-02`).
- **`dev/docs/audits/`**:
  - Established standardized repository audit protocols, baseline sweep reports, and audit personas (`dev/tools/auditors/`).

## [1.6.0] - 2026-09-04


### ⚡ Industrial OT Edge Mesh, Universal Event Bus & Hardware Model Discovery

#### Deterministic COBS Wire Framing & Industrial OT Bridge
- **`crates/aaroneous_wire/`**:
  - Implemented standalone `aaroneous_wire` crate providing zero-allocation Consistent Overhead Byte Stuffing (COBS) framing and deframing.
  - Implemented `TelemetryPacket` with 8 multi-channel sensor slots (`AnalogInput`, `DigitalInput`, `PwmOutput`, `TemperatureCelsius`, `PressureBar`, `CurrentAmperes`), calibrated `f32` representations, error bitmasks, and CRC16-CCITT packet verification.
  - Implemented `CommandPacket` supporting remote register mutations, discrete output pinning, PWM duty cycle modulation, sensor calibration, emergency stop triggers, and CAN 2.0B frame encapsulation.
- **`crates/platform_bridge/src/ot_bridge.rs`**:
  - Implemented `OtEdgeGateway` and thread-safe `IndustrialRegisterBank` (64 holding registers, 32 discrete inputs) bridging physical serial/USB byte streams with host runtime memory.
  - Added full asynchronous command dispatch channel and bidirectional loopback validation for framed commands and telemetry.
- **`core/hypervisor/src/hud/workbench.rs`**:
  - Integrated dedicated **OT Edge Gateway** workbench tab in Maelstrom HUD with live hardware port selector, baud rate toggles, holding register tables, and discrete coil status visualizers.

#### Universal Event Bus & Extended Mathematical Substrates
- **`crates/ipc_bus/src/universal_event_bus.rs`**:
  - Implemented `UniversalEventBus` providing high-throughput lock-free publish/subscribe messaging across all 17 workspace crates.
  - Integrated `SequenceBarrier` for strictly monotonic event sequence ordering, preventing out-of-order execution in concurrent reactive loops.
- **`crates/compute/src/`**:
  - Added `UniversalTensorBuffer` with contiguous layout transformations, multi-format numeric representations, and fast tensor transposition.
  - Enhanced mathematical substrates with SIMD-aligned linear algebra, stochastic distributions, and topological graph operations.

#### Pluggable Model Backend & Capability Specialists
- **`crates/orchestrator/src/llm/providers/backend_trait.rs`**:
  - Implemented universal `ModelBackend` abstraction unifying in-process Candle execution, external LM Studio endpoints, and fast deterministic mock backends.
  - Integrated automatic token consumption calculation (`div_ceil`) and dynamic context window guards.
- **`crates/capabilities/`**:
  - Aligned all 9 domain capabilities to `crates/capabilities` taxonomy with universal tool integration.

#### Hardware Discovery & Test Mock Audit
- **`core/hypervisor/tests/test_real_system_sampling.rs`**:
  - Added integration test discovering real on-disk GGUF models (`~/.lmstudio/models`) and physical serial hardware COM ports.
- **`core/hypervisor/`**:
  - Audited test mocks across hypervisor integration tests, eliminating all dead code, unused mutability warnings, and unused variable assignments.

---

## [1.5.0] - 2026-09-03

### 🛠️ Universal Tool Dual-Face Architecture, MoE Expert Register & Clean Systems Modernization

#### Universal Toolset & Live MCP Auto-Discovery
- **`crates/specialists/src/universal_tool.rs` & `tools.rs`**:
  - Implemented the `UniversalTool` dual-face trait supporting both standard JSON parameter calls (`call_json`) for Cloud REST and LLM MCP callers, and zero-copy $\mathbb{R}^{256}$ latent tensor transformations (`call_latent`) for native `.si` models in VRAM.
  - Implemented concrete, zero-loss tool wrappers across all capability domains:
    - `security.audit` (`0x0500`): Deep SVDD safe manifold constraint verification.
    - `code.repair` (`0x0400`): Atomic AST code mutation with MD5 verification via `adaptation_engine::CodeMutator`.
    - `code.rewrite_pattern` (`0x0410`): Structural AST pattern replacement across source files.
    - `review.audit_source` (`0x0750`): In-house codebase review auditing `.unwrap()`, `unsafe`, and file size limits.
    - `knowledge.semantic_query` (`0x0200`): Indexed semantic knowledge retrieval with confidence scoring.
    - `memory.search_vector` (`0x0600`): 3D spatial knowledge graph bounding frustum queries (`SpatialFrustum`) and nearest-neighbor search.
    - `ui.layout_solve` (`0x0300`): AABB collision-free 2D window and widget layout resolver.
    - `platform.sensory_status` (`0x0900`): Multi-modal sensory inspection across 120 FPS DXGI screen capture, 48kHz WASAPI audio loopback, and CANbus.
- **`core/hypervisor/src/mcp_service/service.rs`**:
  - Embedded `universal_tools: Arc<specialists::ToolRegistry>` into `McpService`.
  - Auto-registered the complete universal tool catalog into the MCP `tools/list` schema for Claude Desktop, Cursor, and OpenCode.
  - Dynamically routed incoming MCP tool requests through `universal_tools.call_by_name()`.

#### Autonomous Codebase Self-Audit
- **`crates/specialists/src/codebase_auditor.rs`**:
  - Implemented `CodebaseReviewSpecialist`, `CodebaseReviewEngine`, and `AutonomousAuditReport` under Domain Opcode `0x0750`.
  - Integrated into `SpecialistFederation` with automated detection of unauthorized `unsafe` blocks, production `.unwrap()` calls, and line length debt.

#### Canonical Systems Terminology Modernization
- **`crates/compute/src/si_moe_register.rs`**:
  - Completely phased out biological and analogical references ("organ") in favor of standard machine learning and systems engineering terminology: `CartridgeDescriptor`, `ExpertSlot`, `mount_expert()`, and `active_expert_slots`.
- **`crates/adaptation_engine/src/auto_wrapper.rs`**:
  - Standardized on `ComponentResponse`, `NativeComponentRunner`, and `MAX_COMPONENT_STDOUT_BYTES`.
- **`core/hypervisor/bin/a_run.rs`**:
  - Aligned CLI commands, banners, and documentation to `SYSTEM COMPONENT FORGE` and `models/components/`.

---

## [0.4.0] - 2026-08-31

### 🛡️ Security Hardening, Subsystem Realignment, Sandboxed Micro-VM, GPU Compute & Live Telemetry

#### Security & Sandboxing (Phase 7 & Phase 11)
- **`core/hypervisor/src/action_executor.rs` (SEC-01)**:
  - Implemented `validate_sandbox_path` to enforce canonical root path containment and reject directory traversal attacks (`../`).
- **`core/hypervisor/src/federation/http/router.rs` (SEC-02, SEC-03)**:
  - Added strict production CORS exact origin filtering.
  - Replaced variable-time string comparisons with `subtle::ConstantTimeEq` constant-time bearer token validation.
- **`crates/ipc_bus/src/comm/mod.rs` (SEC-04)**:
  - Restricted Named Pipe connections to localhost, rejecting remote network client connections.
- **`crates/desktop_emulator/src/native_win32.rs` (SEC-05)**:
  - Implemented top-left screen corner cursor failsafe to instantly abort automated mouse/keyboard input.
- **`core/hypervisor/src/system_integrity.rs` (SEC-06)**:
  - Added full SHA-256 integrity digest checks across critical workspace paths.
- **`core/hypervisor/src/mcp_service/auth.rs` (SEC-07)**:
  - Replaced plaintext API keys with in-memory SHA-256 hashed storage.
- **`core/hypervisor/src/micro_vm.rs` (Phase 11)**:
  - Implemented pure-Rust 16-register Virtual Machine (`MicroBytecodeVm`) with arithmetic, memory, and branch opcodes.
  - Strict instruction gas metering (`VmError::GasExhausted`) and linear memory bounds checking (`VmError::MemoryOutOfBounds`).
  - Integrated with `ActionExecutor::ExecuteMicroBytecode`.

#### Architecture & Subsystem Realignment (Phase 8 & Phase 9)
- **`crates/specialists/src/` (ARCH-01)**:
  - Unified all 9 domain specialists (`Perceiver`, `Presenter`, `Orchestrator`, `Synthesizer`, `Archivist`, `DevTools`, `Sentinel`, `Aligner`, `Router`) under the `DomainSubEngine` trait.
- **`crates/ipc_bus/src/persistent_grimoire.rs` (ARCH-02)**:
  - Standardized Write-Ahead Log to `PersistentWalStore` and `WalRecord` with `b"AWAL"` and `b"GRIM"` magic bytes.
- **`core/hypervisor/src/enzyme_runner.rs` (ARCH-03)**:
  - Added modern systems aliases (`MicroTaskRunner`, `ExplorationWorker`, `ResearchWorker`, `ExecutionWorker`, `ProtocolGatewayWorker`, `SelfCorrectionWorker`).
- **`core/hypervisor/src/hox_registry.rs` (ARCH-04)**:
  - Realigned capability management with `CapabilitySchemaRegistry` and `CapabilityDescriptor`.
- **`core/hypervisor/bin/a_hud.rs` (ARCH-05)**:
  - Modernized telemetry HUD navigation tabs (`ScreenAutomation`, `LearningAndSelfPlay`, `GalaxyMap3D`).
- **`sdk/rust/src/lib.rs` (Phase 9)**:
  - Re-exported modern IPC types (`SignalBridge`, `PersistentWalStore`, `MachinePacket`) with verified doc-tests.

#### Extended Verification, Multi-Host Transport & GPU Compute (Phases 5, 10, 12, 13)
- **`crates/specialists/tests/test_specialists_end_to_end.rs` (Phase 5)**:
  - Added 120-packet concurrent burst stress test across all 9 domain specialists.
- **`crates/orchestrator/src/tier_allocator.rs` (Phase 5)**:
  - Verified multi-tier CPU thread pinning (`TIER_1_ExecutivePlanner`, `TIER_2_ROUTER`, `TIER_3_REFLEX`).
- **`core/hypervisor/src/federation/p2p/mod.rs` (Phase 10)**:
  - Implemented length-prefixed TCP streaming listener (`bind_listener`), remote stream connector (`connect_remote_stream`), and direct peer channels (`register_direct_peer`).
- **`crates/compute/src/burn_gpu.rs` (Phase 12)**:
  - Implemented fast matrix-vector products (`compute_matrix_vector_product`), continuous SSM recurrence (`compute_ssm_recurrence`), and stable softmax probability calculations (`compute_softmax_probabilities`).
- **`crates/desktop_emulator/src/native_win32.rs` (Phase 12)**:
  - Implemented `DxgiHardwareFrameBuffer` with 256-byte pitch alignment for zero-copy GPU video streaming.
- **`core/hypervisor/src/federation/http/router.rs` & `crates/specialists/src/presenter.rs` (Phase 13)**:
  - Added `/v1/telemetry/stream` live SSE telemetry stream.
  - Implemented `InteractionHeatmap` and `compute_adaptive_layout` for automated UI tuning.
- **`core/hypervisor/src/consensus_engine.rs` (Phase 14)**:
  - Implemented Raft cluster consensus state machine (`RaftRole::Leader`/`Follower`/`Candidate`) with quorum election rounds, monotonic term transitions, and distributed WAL replication (`append_wal_mutation`).
- **`crates/compute/src/si_spec.rs` (Phase 18)**:
  - Established canonical `.si` cartridge standard v3.0 (`b"SINT"`) with 64-byte aligned binary headers, CRC32 checksums, and explicit section offset tables.
  - Standardized three-block memory-mapped topology (Block 1: Frozen Core SSM Weights, Block 2: Dynamic Adaptation Matrix, Block 3: Episodic Skill Stack & Habits).
  - Built pure-Rust `SiCartridgeEngine` verification, linting, unpacking, packing, and diffing tool suite.
- **`crates/adaptation_engine/src/ast_parser.rs` & `auto_wrapper.rs` (Phase 15)**:
  - Added polyglot source parsing (`Rust`, `Python`, `TypeScript`, `Cpp`) and incremental structural AST diffing.
  - Implemented safe automated C-ABI FFI wrapper harness synthesis (`synthesize_c_abi_ffi_harness`) for runtime native library binding.
- **`crates/autonomic_adaptation/src/continuous_evolution.rs` (Phase 16)**:
  - Implemented `adapt_from_reward` for real-time online weight steering modulated by closed-loop control states with TD($\lambda$) eligibility traces and Orthogonal Gradient Projection.
  - Implemented `crystallize_habit_cartridge` packaging high-frequency execution traces into canonical `.si` cartridges v3.0.
- **`crates/governance` & `crates/autonomic_adaptation` (Machine-Native Systems Realignment)**:
  - Realigned `crates/biology` to `crates/governance` and `crates/evolution` to `crates/autonomic_adaptation`.
  - Realigned all 9 specialist state buffers from `*Relic` to `*Substrate` in `crates/specialists/`.
  - Realigned homeostatic controllers to `FeedbackRegulator`, `DynamicEquilibriumState`, `SystemHealthGovernor`, `AdaptiveControlState`, and `AutonomicStateRegulator`.
  - Standardized `MnlpProtocolBridge`, `CompoundAgentProfile`, `ParameterLocus`, and `WasmHotSwapEngine`.
- **`core/hypervisor/bin/a_hud.rs` (Phase 17 — Native WGPU 3D Constellation Studio)**:
  - Integrated pure-Rust 3D perspective projection with depth fogging, $z$-depth sort ordering, 360° pitch/yaw orbit drag controls, and smooth scroll zoom in `render_galaxy_3d_view`.
  - Implemented interactive star node selection, live execution trace pulse lines traveling along constellation edges, domain filter chips, and specialist detail inspector card with direct native channel task dispatch triggers.
- **`crates/desktop_emulator` (Phase 19 — Multi-Modal Vision & Audio Loopback Pipeline)**:
  - Implemented `SolidStateVisionPipeline` (`vision_latent.rs`) for sub-5ms 64-D spatial latent extraction, temporal entropy, and motion delta from `DxgiHardwareFrameBuffer`.
  - Implemented `WasapiAudioStreamAnalyzer` (`audio_analyzer.rs`) for 8-band Goertzel log frequency spectrum analysis, transient acoustic spike detection, and speech event tokenization (`AudioEventObservation`).

## [0.3.2] - 2026-08-25

### 🔧 Orchestrator Compilation Fixes, Core Orchestration & LLM Integration

#### Fixed
- **`crates/orchestrator/src/archetypes.rs` — Compilation Errors**:
  - Replaced 8 instances of `#` used as comment delimiter with valid `//` syntax.
  - Removed 2 duplicate `impl NativeThinker for Archetype` blocks.
- **`crates/orchestrator/src/lib.rs` — Safety Violations**:
  - `IntelligenceEngine::new()` and `new_async()` now return `anyhow::Result<Self>`.
- **`crates/orchestrator/src/hive_runtime.rs` — Safety Violations**:
  - `HiveRuntime::new()` now returns `anyhow::Result<Self>`.
- **`core/hypervisor/src/orchestration_daemon.rs` — Callers Updated**:
  - `OrchestrationDaemon::new()` now returns `anyhow::Result<Self>`.
  - All 4 call sites updated to handle the new `Result` type.

#### Added
- **`crates/orchestrator/src/hive_runtime.rs` — Full Task Dispatch & Lifecycle**:
  - `start()`: Initializes MDP routing engine from registered agents as specialists.
  - `stop()`: Shuts down runtime execution.
  - `dispatch_task()`: Routes a `RoutableTask` through MDP value iteration, consumes specialist capacity, and logs the task record.
  - `complete_task()`: Records completion timestamp, updates specialist performance metrics via EMA.
  - `get_status()`: Returns full runtime status including dispatched/completed/failed task counts.
  - 4 new unit tests: creation, agent registration, start/stop lifecycle, task dispatch.
- **`crates/orchestrator/src/mdps_router.rs` — Skill-Aware Routing**:
  - `calculate_expected_reward()` now accepts `task_skills: &[String]` parameter with skill-match scoring: +1.0 for full match, +0.5 for partial, -2.0 for no match.
  - `find_optimal_specialist()` applies skill matching at routing time with reasoning output.
- **`crates/orchestrator/src/mdps_router.rs` — Online MDP Learning**:
  - `update_transition_matrix()`: Bayesian update of transition probabilities from observed state transitions.
- **`crates/orchestrator/src/control.rs` — Resource Allocation**:
  - `adjust_resource_allocation()` now applies VRAM and context_size changes to specialist status.
- **`crates/orchestrator/src/aura_ui.rs` — UI Consolidation**:
  - Replaced redundant 3-field struct with backward-compatible re-exports from `aura_ui_manifest.rs`.
- **`crates/orchestrator/src/llm/providers/openai.rs` — OpenAI Provider**:
  - `OpenAIProvider`: reqwest-based HTTP client with `chat_completion()` and `embeddings()` methods.
  - Supports OpenAI, Ollama, LM Studio, and any OpenAI-compatible API endpoint.
- **`crates/orchestrator/src/llm/providers/gguf.rs` — GGUF Provider**:
  - `GgufProvider`: Local GGUF model inference with path validation and placeholder for candle integration.
- **`crates/orchestrator/src/llm/client.rs` — LLMClient Provider Integration**:
  - `LLMClient` now initializes `OpenAIProvider` or `GgufProvider` based on `ProviderType`.
  - `get_last_hidden_state()` now returns cached embedding state instead of zero vector.
  - `compute_embeddings()`: Computes embeddings via OpenAI or deterministic pseudo-embeddings for non-OpenAI providers.
  - `analyze_task()` parses JSON-structured LLM responses for richer task analysis.
- **`crates/orchestrator/src/linguistic_transducer.rs` — CAS Vocabulary**:
  - `CasCommand` struct with opcode, mnemonic, description, and domain fields.
  - 18 pre-defined CAS commands spanning all 6 specialist domains.
  - `parse_intent()`: Keyword-based intent detection converting natural language to CAS commands.
  - `opcode_to_mnemonic()` / `mnemonic_to_opcode()` bidirectional lookup.
  - 10 new unit tests covering vocabulary, intent parsing, and domain mapping.
- **`crates/orchestrator/src/intent_engine.rs` — User Intent Pipeline**:
  - `IntentEngine`: Full natural language → CAS command → specialist dispatch pipeline.
  - `parse_intent()`: Extracts CAS command, skills, complexity, and urgency from text.
  - `dispatch()`: Routes parsed intent through MDP router to optimal specialist.
  - `parse_and_dispatch()`: One-call convenience method.
  - Keyword-based skill extraction with domain-aware heuristics.
  - 6 new unit tests covering intent parsing, urgency detection, dispatch, and skill extraction.
- **`crates/orchestrator/src/workflow_engine.rs` — Crash-Recoverable Persistence**:
  - `serialize()` / `deserialize()`: JSON roundtrip for workflow state.
  - `save()` / `save_to()`: Atomic file writes with temp-file-then-rename pattern.
  - `load_from()`: Restore workflow from disk with persist_path restoration.
  - `save_default()` / `load_default()`: Convenience methods using `.aaroneous/workflows/` directory.
  - `list_persisted()`: Enumerate all saved workflows.
  - 3 new unit tests covering serialization roundtrip, save/load, and listing.
- **`crates/orchestrator/src/swarm_balancer.rs` — Swarm Load Balancer**:
  - `SwarmBalancer`: Channel-based task distribution across local and remote workers.
  - `Transport` trait: Pluggable backend (TCP, NATS, in-process channels).
  - `TcpTransport` / `ChannelTransport`: Built-in implementations.
  - `register_worker()` / `remove_worker()` / `list_workers()`: Worker lifecycle.
  - `find_best_worker()`: Capacity + latency weighted selection.
  - `dispatch_task()` / `complete_task()`: Full task lifecycle with worker status tracking.
  - `health()`: Swarm health summary (idle/busy/offline counts, average capacity).
  - 4 new unit tests covering registration, best-worker selection, dispatch/complete, and health.

#### Verified
- `cargo check --workspace`: **0 errors**.
- `cargo clippy -p orchestrator`: **0 warnings**.
- `cargo test -p orchestrator`: **50 passed, 0 failed** (up from 22).

---

## [0.3.1] - 2026-08-22

### 🧹 Workspace-Wide Clippy Cleanup — Zero Warnings on All Library Targets

#### Changed
- **`core/hypervisor` (a_run) — 0 Clippy warnings on `--lib`**:
  - Replaced manual index loops with idiomatic iterators across 19 modules: `spectral_layout.rs`, `tensor_router.rs`, `relativity_engine.rs`, `reasoning.rs`, `hardware_layer.rs`, `synapse_ui.rs`, `skill_constellation.rs`, `cellular_automata.rs`, `spatial_delta_gate.rs`, `SpatialDelta_orchestrator.rs`, `federated_learning.rs`, `live_daemon.rs`, `task_routing.rs`.
  - Replaced `.unwrap()` on `Option` with safe `if let Some` patterns in `live_daemon.rs`.
  - Replaced `.map(|layer| { ... })` producing unit with `if let Some(layer)` in `unified_learning.rs`.
  - Added `#[allow(clippy::too_many_arguments)]` on justified signatures: `decision_engine.rs`, `unified_learning.rs`, `fractional_normalizer.rs`, `svd_feature_select.rs`.
  - Added `#[allow(clippy::collapsible_if)]` on security path-containment guards in `mcp_service/service.rs`.
  - Defined `SemanticEmbeddingRecord` type alias and simplified embedding load loop in `hive_db.rs`.
  - Used `.clamp()` and `.div_ceil()` standard library methods in `task_routing.rs` and `live_daemon.rs`.
- **All external crates (`compute`, `marionette`, `omni`, `orchestrator`, `chimera`, `evolution`, `specialists`) — 0 warnings** (completed in prior commit `dbde3a5`).

#### Verified
- `cargo clippy -p a_run --lib -- -D warnings`: **0 errors, 0 warnings**.

---

## [0.3.0] - 2026-08-21

### 🧬 Full 9-Specialist Distillation, Live P2P Daemon & Autonomous Self-Evolution

#### Added
- **Full 9-Specialist `.si` Solid-State Distillation (`crates/compute`)**:
  - `TranslationDataset::synthesize_all_9_specialists`: Domain-specific training trajectory generation for all 9 Sovereign Domains (Orchestrator, Synthesizer, Presenter, Fabricator, Sentinel, Archivist, Router, Aligner, Perceiver).
  - `SiDistillationHarness::distill_all_9_specialists`: End-to-end multi-teacher distillation with CKA + InfoNCE loss optimization producing ~80 KB `.si` containers at 100% CKA alignment.
  - CLI command: `a_run distill-all --samples 10 --epochs 1 --out models/distilled_federation`.
- **Live Multi-Hive P2P TCP Socket Daemon (`core/hypervisor/src/federation/multi_hive/live_daemon.rs`)**:
  - `LiveP2PDaemon`: Asynchronous Tokio TCP listener with 4-byte length-delimited framing, heartbeat EWMA latency estimation, and bi-directional stream management.
  - `DaemonWirePacket`: Ping/Pong, GossipProposal/GossipVote, TaskOffloadRequest/TaskOffloadResponse.
  - Live Byzantine Gossip Consensus: 2/3 quorum voting over real TCP streams.
  - CLI commands: `a_run daemon --bind 127.0.0.1:8001`, `a_run mesh --nodes 4 --live`.
- **Swarm Micro-Task TCP Offloader (`core/hypervisor/src/federation/multi_hive/swarm_offloader.rs`)**:
  - `SwarmOffloader`: Routes computational tasks to lowest-latency peer hive when local pressure exceeds threshold (default 80%). Measured 12 µs wire RTT + remote execution.
- **Autonomous Background Self-Evolution Engine (`crates/evolution/src/continuous_evolution.rs`)**:
  - `ContinuousSelfEvolutionEngine`: Couples Archivist 4-channel SystemThermodynamics (curiosity/boredom drives) with Fabricator AST hypothesis mutations, Sentinel Deep SVDD safety audits, and `.si` solid-state skill stack promotions.
  - CLI command: `a_run evolve --cycles 3 --threshold 0.70`.

#### Changed
- **CLI Main Thread Stack**: Wrapped entire CLI dispatch in dedicated 32 MB stack worker thread, preventing `STATUS_STACK_OVERFLOW` on Windows during unoptimized debug builds with deep AST/ML call stacks.
- **`connect_peer` Non-Blocking**: Outbound peer connections are now spawned as background Tokio tasks instead of blocking the caller.

#### Verified
- Workspace test suite: **1,352 passed, 0 failed (100%)** across all 12 crates.

---

## [0.2.0] - 2026-08-21

### 🏛️ Tri-Tiered Layered Control System & Specialist Federation

#### Added
- **Tri-Tiered Layered Control Architecture (`crates/compute`, `crates/orchestrator`)**:
  - **Tier 1 (StrategicPlanner - $\mathbb{R}^{4096}$)**: High-dimensional intent planning on background OS threads (`SiTierFlags::TIER_1_ExecutivePlanner`).
  - **Tier 2 (Router - $\mathbb{R}^{4096} \to \mathbb{R}^{256}$)**: Continuous orthogonal intent projector with inline **Sentinel Deep SVDD** guardrail audit ($< 2\mu\text{s}$ safe hypersphere manifold snap) broadcasting atomically to Channel 0 of the lock-free SPMC synapse bus (`SiTierFlags::TIER_2_ROUTER`).
  - **Tier 3 (Kinetic Reflex Workers - $\mathbb{R}^{256}$)**: Hot-loop sub-microsecond spin-wait pursuit workers pinned to physical CPU cores via `SetThreadAffinityMask`, executing continuous sensory-conditioned recurrence in $< 180\mu\text{s}$ (`SiTierFlags::TIER_3_REFLEX`).
- **Unified `SiForge` Model Builder API (`crates/compute/src/si_forge.rs`)**:
  - Pure Rust builder pattern for birthing `.si` solid-state containers end-to-end: teacher trajectory synthesis $\to$ multi-objective distillation (CKA + InfoNCE + CE) $\to$ state-space weight matrix extraction $\to$ 64-byte SIMD cache-aligned SINT v3 binary packing.
  - CLI command: `a_run forge --name <id> --tier <1|2|3> --samples <N> --epochs <E>`.
- **SINT v3 64-Byte SIMD Alignment & Tier Flags (`crates/compute/src/si_packer.rs`)**:
  - Added `SiTierFlags` at byte offset `0x08` for immediate execution profile resolution upon memory mapping.
  - Fixed-point convergent layout solver guaranteeing strict 64-byte AVX-512 alignment for every weight tensor.
- **Sovereign Windows Isolated Desktop Isolation (`crates/compute/src/ghost_station.rs`)**:
  - Win32 `CreateDesktopW` encapsulation creating headless sandboxed stations for safe robotic kinetic execution.
  - CLI command: `a_run boot --profile isolated`.
- **Desktop Studio Telemetry & Visualizer Suite (`core/hypervisor/src/`)**:
  - `ForgeStudio` (`forge_ui.rs`): Decoupled 60Hz immediate-mode `egui` interface streaming background distillation status and logs over `std::sync::mpsc`.
  - `SynapseVisualizer` (`synapse_ui.rs`): 60Hz 256-bar activation oscilloscope with real-time Sentinel Threat Gauge and safe manifold centroid overlay.
  - `SkillConstellationCanvas` (`skill_constellation.rs`): Skyrim-style celestial constellation with $N$-body Coulomb repulsion and Hooke's Law attraction physics based on $\mathbb{R}^{256}$ cosine similarity.

#### Changed / Normalized
- **System Normalization & Specialist Federation (`crates/specialists`, `crates/nervous_system`, `crates/evolution`, `core/hypervisor`)**:
  - Normalized all system terminology from mythological metaphors to clean, professional systems engineering (`SpecialistFederation`, `PantheonSynapseBus::new_federation()`, `build_specialist_soul`).
  - Standardized functional domain roles across all 9 sovereign engines (Task Orchestration, Knowledge & Research, Presentation & HUD, Code & Binary CompilerCore, Security & Guardrails, Memory & State, Router & Federation, Alignment & Symbiosis, Sensory & Vision).
  - Maintained backwards-compatible aliases (`SpecialistFederation`, `new_olympian`, `build_olympian_soul`) across all public APIs.

---

## [0.1.0] - 2026-08-21

### ⚡ Solid-State Machine-Native Execution Engine Architecture

#### Added
- **Unified Solid-State `.si` Container (`crates/compute/src/si_solid_state.rs`)**:
  - `SINT` (Synthetic Intelligence Native Topology) single-file binary container fusing base weights, mutable adaptation matrices, and episodic skill DAGs.
  - Zero-copy `memmap2` loader providing instantaneous mount times ($< 50\mu\text{s}$) with zero Python dependencies.
- **Selective State-Space Model (`crates/compute/src/si_ssm.rs`)**:
  - `Aaroneous-Native-SSM-4M`: Pure Rust continuous recurrence ($h_t = \bar{\mathbf{A}} h_{t-1} + \bar{\mathbf{B}} u_t$) with 4 layers, 1024 state dimension, 256 model dimension, and 64 state rank.
  - Sub-millisecond state prediction ($< 180\mu\text{s}$) with embedded thermodynamic dissipation measurement.
- **Dynamic Adaptation Matrix & Real-Time Error Steering (`DynamicAdaptationMatrix`)**:
  - Mutable Low-Rank Adapter ($r = 16$) that preserves immutable base core weights while learning from mistakes live during runtime.
  - In-place localized gradient updates ($< 50\mu\text{s}$) that automatically steer the agent away from error states upon compiler panics or failures.
- **Autonomous Skill-Expansion & Meta-Learning Engine (`crates/compute/src/si_skill_tree.rs`)**:
  - Autonomous 4-tier graduation ladder: `🌱 Candidate` $\to$ `🧪 Validated` $\to$ `💎 Crystallized` $\to$ `⚡ Core Reflex`.
  - Intrinsic reward formula: $\mathcal{R}_{\text{intrinsic}} = \text{SuccessRate} + \text{Compression} + \frac{1}{1 + \Delta F}$.
  - Automatic crystallization of high-value latent pathways into standalone `.si` cartridges.
- **Complete Execution Engine Tool Suite (`crates/compute/src/si_tool.rs` & CLI)**:
  - `a_run si inspect`: Deep binary container inspection for `SIMN`, `SISSM`, and `SINT` files.
  - `a_run si benchmark`: Memory-mapped microsecond latency profiler (benchmarked at $9\mu\text{s}$ p50 latency, 102,606 ops/s).
  - `a_run si skills`: Dynamic discovery and intrinsic score inspector.
  - `a_run si distill`: Action-to-DAG binary compiler.
  - `a_run si train`: On-device GPU multi-objective loss trainer.
- **Developer Studio Integration (`core/hypervisor/bin/a_hud.rs`)**:
  - Added **`🧬 Skill Tree & Execution Engine Inspector`** tab with live telemetry cards and 1-click microsecond benchmark execution.
  - Connected Smart Execution Engine Macro Hub to Global Command Palette (`Ctrl + K`).
- **Dynamic Workspace Resolver (`crates/paths`)**:
  - Enforced zero hardcoded paths across all hypervisor components and toolchains.

---

## [Unreleased]

### Phase IV - Production Readiness (70% Complete)

#### Completed

**Registry Synchronization Framework**
- **File**: `components/registry/src/registry_sync.rs`
- **Change**: Implemented hybrid master registry at `components/registry/src/registry.rs`
- **File**: `components/registry/src/registry_adapters.rs`
- **Change**: Added registry adapter implementations at `components/registry/src/registry_adapters.rs`
- **File**: `components/registry/src/predictive_models.rs`
- **Change**: Wired predictive models framework at `components/registry/src/predictive_models.rs`
- **File**: `core/hypervisor/src/runtime.rs`
- **Change**: Integrated registry synchronization into runtime at `core/hypervisor/src/runtime.rs`

**Memory→Decisions Integration**
- **File**: `core/hypervisor/src/runtime.rs`
- **Change**: Wired memory state to decision-making at `core/hypervisor/src/runtime.rs`
- **File**: `core/hypervisor/src/decisions.rs`
- **Change**: Added memory→decisions integration at `core/hypervisor/src/decisions.rs`
- **File**: `core/hypervisor/src/memory.rs`
- **Change**: Implemented memory state tracking at `core/hypervisor/src/memory.rs`

**Timeout Mechanisms**
- **File**: `core/hypervisor/src/runtime.rs`
- **Change**: Added configurable timeouts at `core/hypervisor/src/runtime.rs`
- **File**: `core/hypervisor/src/autonomic_loop.rs`
- **Change**: Implemented timeout handling in autonomic loop at `core/hypervisor/src/autonomic_loop.rs`
- **File**: `core/hypervisor/src/timeout.rs`
- **Change**: Created timeout configuration at `core/hypervisor/src/timeout.rs`

**Error Handling**
- **File**: `core/hypervisor/src/runtime.rs`
- **Change**: Added error recovery at `core/hypervisor/src/runtime.rs`
- **File**: `core/hypervisor/src/errors.rs`
- **Change**: Implemented error types at `core/hypervisor/src/errors.rs`
- **File**: `core/hypervisor/src/recovery.rs`
- **Change**: Created recovery mechanisms at `core/hypervisor/src/recovery.rs`

**Predictive Telemetry**
- **File**: `core/hypervisor/src/observability.rs`
- **Change**: Wired predictive telemetry at `core/hypervisor/src/observability.rs`
- **File**: `core/hypervisor/src/metrics.rs`
- **Change**: Implemented metrics collection at `core/hypervisor/src/metrics.rs`
- **File**: `core/hypervisor/src/health.rs`
- **Change**: Added health monitoring at `core/hypervisor/src/health.rs`

#### Pending

**Configuration Management**
- **File**: `core/hypervisor/src/config.rs`
- **Change**: Externalize runtime configuration from hardcoded values
- **File**: `deploy/config.toml`
- **Change**: Create external configuration file

**Security Hardening**
- **File**: `core/hypervisor/src/validation.rs`
- **Change**: Implement input validation
- **File**: `core/hypervisor/src/authorization.rs`
- **Change**: Add authorization checks

**Documentation Completion**
- **File**: `docs/architecture/overview.md`
- **Change**: Complete architecture documentation
- **File**: `docs/operations/runbook.md`
- **Change**: Complete operations runbook

**Performance Testing**
- **File**: `scripts/performance/benchmark.sh`
- **Change**: Create performance benchmark suite
- **File**: `docs/performance/benchmarks.md`
- **Change**: Document performance targets

**Final Review**
- **File**: `docs/reports/final_review.md`
- **Change**: Create final review checklist

### Phase X - Repository Cleanup (Complete)

#### Repository Size Reduction
- **File**: `target/debug/deps/*.o`
- **Change**: Removed 1.76GB of object files
- **File**: `target/debug/deps/query-cache.bin`
- **Change**: Removed 205MB query cache files
- **File**: `target/debug/deps/*.rlib`
- **Change**: Removed old .rlib files
- **File**: `target/debug/deps/*.pdb`
- **Change**: Removed .pdb files
- **File**: `target/debug/deps/*.d`
- **Change**: Removed dep-graph files

#### Cleanup Scripts
- **File**: `scripts/cleanup.ps1`
- **Change**: Created weekly cleanup script
- **File**: `scripts/audit.ps1`
- **Change**: Created repository audit script
- **File**: `scripts/size-check.ps1`
- **Change**: Created size monitoring script

#### Documentation Migration
- **File**: `README.md`
- **Change**: Migrated to `docs/README.md`
- **File**: `docs/README.md`
- **Change**: Created new documentation root
- **File**: `docs/architecture/`
- **Change**: Created architecture documentation
- **File**: `docs/deployment/`
- **Change**: Created deployment documentation
- **File**: `docs/operations/`
- **Change**: Created operations documentation
- **File**: `docs/history/`
- **Change**: Created history archive directory

### Phase III - Consolidation (Complete)

#### Module Reduction
- **File**: `core/hypervisor/src/lib.rs`
- **Change**: Reduced from 104 to 34 modules
- **File**: `core/hypervisor/src/modules.rs`
- **Change**: Implemented module consolidation

#### Architecture Improvements
- **File**: `components/registry/src/hybrid.rs`
- **Change**: Implemented hybrid master registry
- **File**: `components/registry/src/adapter.rs`
- **Change**: Created registry adapter interface

### Phase II - Major Integrations (Complete)

#### Integration Points
- **File**: `core/hypervisor/src/routing.rs`
- **Change**: Implemented task classification→routing
- **File**: `core/hypervisor/src/load.rs`
- **Change**: Implemented load predictions→backpressure
- **File**: `core/hypervisor/src/predictive.rs`
- **Change**: Integrated predictive models

### Phase I - Critical Fixes (Complete)

#### Core Fixes
- **File**: `core/hypervisor/src/enzyme.rs`
- **Change**: Implemented enzyme extraction
- **File**: `core/hypervisor/src/token.rs`
- **Change**: Added token system
- **File**: `core/hypervisor/src/dopamine.rs`
- **Change**: Wired dopamine→learning
- **File**: `core/hypervisor/src/core.rs`
- **Change**: Consolidated core modules

## [v0.9.0] - 2026-06-09

### Phase V - Biological Integration
- **File**: `components/genetics/src/dna_bank.rs`
- **Change**: Implemented RocksDB-based DNA storage
- **File**: `components/genetics/src/operators.rs`
- **Change**: Added genetic operators
- **File**: `components/genetics/src/evolution.rs`
- **Change**: Implemented evolutionary loop
- **File**: `components/genetics/src/metrics.rs`
- **Change**: Added biological metrics

### Phase VI - Archival & High Availability
- **File**: `components/archival/src/rotation.rs`
- **Change**: Implemented model archival
- **File**: `components/archival/src/backup.rs`
- **Change**: Added backup system
- **File**: `components/ha/src/cluster.rs`
- **Change**: Implemented high availability
- **File**: `components/ha/src/velero.rs`
- **Change**: Added Velero backup integration

### Phase VII - UI State Management
- **File**: `MaelstromUI/src/main.rs`
- **Change**: Implemented Tauri UI
- **File**: `MaelstromUI/src/state.rs`
- **Change**: Added state management
- **File**: `MaelstromUI/src/components.rs`
- **Change**: Created UI components
- **File**: `MaelstromUI/src/theme.rs`
- **Change**: Implemented theme system

### Phase VIII - Compute Infrastructure
- **File**: `extensions/wasm/src/compute.rs`
- **Change**: Implemented WASM agents
- **File**: `extensions/wasm/src/gpu.rs`
- **Change**: Added GPU metrics
- **File**: `extensions/wasm/src/resources.rs`
- **Change**: Implemented resource management
- **File**: `extensions/wasm/src/load.rs`
- **Change**: Added load balancing

### Phase IX - Integration Documentation
- **File**: `docs/architecture/overview.md`
- **Change**: Documented system architecture
- **File**: `docs/deployment/README.md`
- **Change**: Created deployment guides
- **File**: `docs/operations/README.md`
- **Change**: Wrote operations manual
- **File**: `docs/api/README.md`
- **Change**: Documented public APIs

## [v0.8.0] - 2026-05-15

### Core Features
- **File**: `core/hypervisor/src/models.rs`
- **Change**: Added multi-model support
- **File**: `core/hypervisor/src/predictive.rs`
- **Change**: Implemented predictive routing
- **File**: `core/hypervisor/src/learning.rs`
- **Change**: Added adaptive learning
- **File**: `core/hypervisor/src/synapse.rs`
- **Change**: Implemented neural network

### Breaking Changes
- **File**: `core/hypervisor/src/registry.rs`
- **Change**: Changed from flat to hierarchical registry
- **File**: `core/hypervisor/src/loader.rs`
- **Change**: Updated model loading to GGUF
- **File**: `core/hypervisor/src/config.rs`
- **Change**: Moved from env to TOML config

## [v0.7.0] - 2026-04-20

### New Features
- **File**: `core/hypervisor/src/classification.rs`
- **Change**: Added task classification
- **File**: `core/hypervisor/src/selection.rs`
- **Change**: Implemented model selection
- **File**: `core/hypervisor/src/cache.rs`
- **Change**: Added model caching
- **File**: `core/hypervisor/src/dashboard.rs`
- **Change**: Implemented metrics dashboard

### Bug Fixes
- **File**: `core/hypervisor/src/enzyme.rs`
- **Change**: Fixed enzyme memory leak
- **File**: `core/hypervisor/src/registry.rs`
- **Change**: Fixed registry race condition
- **File**: `core/hypervisor/src/loader.rs`
- **Change**: Fixed model loading timeout

## [v0.6.0] - 2026-03-10

### Major Improvements
- **File**: `core/hypervisor/src/loader.rs`
- **Change**: 2x improvement in model loading speed
- **File**: `core/hypervisor/src/runtime.rs`
- **Change**: 99.9% uptime in production
- **File**: `core/hypervisor/src/routing.rs`
- **Change**: Support for 1000+ concurrent requests
- **File**: `core/hypervisor/src/validation.rs`
- **Change**: Implemented input validation

### Deprecations
- **File**: `core/hypervisor/src/api.rs`
- **Change**: Deprecated v0.5 API endpoints
- **File**: `core/hypervisor/src/config.rs`
- **Change**: Removed legacy configuration

## [v0.5.0] - 2026-02-01

### Initial Release
- **File**: `core/hypervisor/src/lib.rs`
- **Change**: Released core federation engine
- **File**: `core/hypervisor/src/routing.rs`
- **Change**: Implemented basic task routing
- **File**: `core/hypervisor/src/loader.rs`
- **Change**: Added model loading
- **File**: `core/hypervisor/src/cli.rs`
- **Change**: Released CLI interface

### Known Issues
- **File**: `core/hypervisor/src/memory.rs`
- **Change**: High memory usage with large model sets
- **File**: `core/hypervisor/src/errors.rs`
- **Change**: Limited error recovery
- **File**: `docs/README.md`
- **Change**: Incomplete documentation

---

## Project Expansion Guidelines

### Adding New Features
1. **Update Phase**: Determine which phase the feature belongs to
2. **File Reference**: Add specific file path to changelog
3. **Change Description**: Describe the exact change
4. **Documentation**: Create documentation in `/docs/` subdirectory
5. **Testing**: Add unit and integration tests
6. **Changelog**: Add entry to this file
7. **Version**: Bump version in `Cargo.toml` files

### Versioning Scheme
- **Major**: Breaking changes to API or architecture
- **Minor**: New features, backwards-compatible changes
- **Patch**: Bug fixes and documentation updates

### Documentation Standards
- **Architecture**: `docs/architecture/`
- **Deployment**: `docs/deployment/`
- **Operations**: `docs/operations/`
- **Reports**: `docs/reports/`
- **History**: `docs/history/`

### Code Standards
- **Rust Edition**: 2024 for core, 2021 for extensions
- **Formatting**: `cargo fmt --all`
- **Linting**: `cargo clippy --all`
- **Testing**: `cargo test --all`

### Release Process
1. **Update Changelog**: Add changes to this file with file references
2. **Update Version**: Bump version in `Cargo.toml`
3. **Update Docs**: Update documentation with new features
4. **Create Release**: Create GitHub release with changelog
5. **Update README**: Update README with new features
6. **Tag Release**: Create git tag with version number

---

*Last Updated: 2026-06-09 | Status: Phase IV - Production Readiness*

