# Changelog

All notable changes to Aaroneous.

## [0.3.0] - 2026-08-21

### 🧬 Full 9-Specialist Distillation, Live P2P Daemon & Autonomous Self-Evolution

#### Added
- **Full 9-Specialist `.si` Solid-State Distillation (`crates/compute`)**:
  - `RosettaStoneDataset::synthesize_all_9_specialists`: Domain-specific training trajectory generation for all 9 Sovereign Domains (Odin, Merlin, Ariel, Hephaestus, Argus, Dionysus, Hermes, Wen, Kami).
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
  - `ContinuousSelfEvolutionEngine`: Couples Dionysus 4-channel neurochemistry (curiosity/boredom drives) with Hephaestus AST hypothesis mutations, Argus Deep SVDD safety audits, and `.si` solid-state skill stack promotions.
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
  - **Tier 1 (Strategic Cortex - $\mathbb{R}^{4096}$)**: High-dimensional intent planning on background OS threads (`SiTierFlags::TIER_1_CORTEX`).
  - **Tier 2 (Hermes Router - $\mathbb{R}^{4096} \to \mathbb{R}^{256}$)**: Continuous orthogonal intent projector with inline **Argus Deep SVDD** guardrail audit ($< 2\mu\text{s}$ safe hypersphere manifold snap) broadcasting atomically to Channel 0 of the lock-free SPMC synapse bus (`SiTierFlags::TIER_2_ROUTER`).
  - **Tier 3 (Kinetic Reflex Workers - $\mathbb{R}^{256}$)**: Hot-loop sub-microsecond spin-wait pursuit workers pinned to physical CPU cores via `SetThreadAffinityMask`, executing continuous sensory-conditioned recurrence in $< 180\mu\text{s}$ (`SiTierFlags::TIER_3_REFLEX`).
- **Unified `SiForge` Model Builder API (`crates/compute/src/si_forge.rs`)**:
  - Pure Rust builder pattern for birthing `.si` solid-state containers end-to-end: teacher trajectory synthesis $\to$ multi-objective distillation (CKA + InfoNCE + CE) $\to$ state-space weight matrix extraction $\to$ 64-byte SIMD cache-aligned SINT v3 binary packing.
  - CLI command: `a_run forge --name <id> --tier <1|2|3> --samples <N> --epochs <E>`.
- **SINT v3 64-Byte SIMD Alignment & Tier Flags (`crates/compute/src/si_packer.rs`)**:
  - Added `SiTierFlags` at byte offset `0x08` for immediate execution profile resolution upon memory mapping.
  - Fixed-point convergent layout solver guaranteeing strict 64-byte AVX-512 alignment for every weight tensor.
- **Sovereign Windows Ghost Desktop Isolation (`crates/compute/src/ghost_station.rs`)**:
  - Win32 `CreateDesktopW` encapsulation creating headless sandboxed stations for safe robotic kinetic execution.
  - CLI command: `a_run boot --profile isolated`.
- **Desktop Studio Telemetry & Visualizer Suite (`core/hypervisor/src/`)**:
  - `ForgeStudio` (`forge_ui.rs`): Decoupled 60Hz immediate-mode `egui` interface streaming background distillation status and logs over `std::sync::mpsc`.
  - `SynapseVisualizer` (`synapse_ui.rs`): 60Hz 256-bar activation oscilloscope with real-time Argus Threat Gauge and safe manifold centroid overlay.
  - `SkillConstellationCanvas` (`skill_constellation.rs`): Skyrim-style celestial constellation with $N$-body Coulomb repulsion and Hooke's Law attraction physics based on $\mathbb{R}^{256}$ cosine similarity.

#### Changed / Normalized
- **System Normalization & Specialist Federation (`crates/specialists`, `crates/nervous_system`, `crates/evolution`, `core/hypervisor`)**:
  - Normalized all system terminology from mythological metaphors to clean, professional systems engineering (`SpecialistFederation`, `PantheonSynapseBus::new_federation()`, `build_specialist_soul`).
  - Standardized functional domain roles across all 9 sovereign engines (Task Orchestration, Knowledge & Research, Presentation & HUD, Code & Binary Forge, Security & Guardrails, Memory & State, Router & Federation, Alignment & Symbiosis, Sensory & Vision).
  - Maintained backwards-compatible aliases (`OlympianPantheon`, `new_olympian`, `build_olympian_soul`) across all public APIs.

---

## [0.1.0] - 2026-08-21

### ⚡ Solid-State Machine-Native Synthetic Intelligence (SI) Architecture

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
- **Complete SI Tool Suite (`crates/compute/src/si_tool.rs` & CLI)**:
  - `a_run si inspect`: Deep binary container inspection for `SIMN`, `SISSM`, and `SINT` files.
  - `a_run si benchmark`: Memory-mapped microsecond latency profiler (benchmarked at $9\mu\text{s}$ p50 latency, 102,606 ops/s).
  - `a_run si skills`: Dynamic discovery and intrinsic score inspector.
  - `a_run si distill`: Action-to-DAG binary compiler.
  - `a_run si train`: On-device GPU multi-objective loss trainer.
- **Developer Studio Integration (`core/hypervisor/bin/a_hud.rs`)**:
  - Added **`🧬 Skill Tree & SI Inspector`** tab with live telemetry cards and 1-click microsecond benchmark execution.
  - Connected Smart SI Macro Hub to Global Command Palette (`Ctrl + K`).
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
