# Atlas: Working Ontology and Classification Notes

**Status:** Working draft, unreviewed by a second model family. Not part of the governing document hierarchy; whether it becomes one is the owner's decision.
**Last normalized:** 2026-11-18 against `origin/main` at `cc794cb3`.
**Origin:** first drafted by the Gemini seat on 2026-11-17 (attribution inferred from the checkout identity; the draft was never committed), then normalized by the Claude seat. That makes the Claude seat a contributor, not an independent reviewer, so independent review is still needed.

## Relationship to other documents

| Document | Role | If it conflicts with this file |
| :--- | :--- | :--- |
| [AGENTS.md](../AGENTS.md) | Constitution: behavioural and architectural rules | AGENTS.md wins |
| [CRATIFY_SPEC.md](CRATIFY_SPEC.md) | Compliance rules, profiles, ratchet | CRATIFY_SPEC.md wins |
| [architecture/MASTER_ARCHITECTURE.md](architecture/MASTER_ARCHITECTURE.md) | Canonical mechanical design | MASTER_ARCHITECTURE.md wins |
| This file | Vocabulary and classification notes | Fix this file |

## 0. Purpose and stance

Atlas looks for the smallest set of concepts that describes software and other systems without remainder, so that different systems can be compared, navigated and improved with one vocabulary. It is a lens on the work, not a runtime dependency.

Stance, taken from the owner's handoff of 2026-11-18:

- The six primitives below are working primitives. They are the smallest set that has survived attempts to simplify further. They are not claimed to be final or complete.
- The goal is the smallest ontology that survives contact with reality. A concept earns its place by failing to break while larger concepts reduce into it.
- Open questions, which this file does not settle: time, resources, emergence, economics, identity and scale. Each may remain derived or may need promotion to a primitive.
- A good classification decomposes (it says what a thing is made of); a bad one only describes. The test: can this concept be reduced further? If yes, it is not primitive.

## 1. The six working primitives (Layer 0)

| Primitive | Definition | Repo rules that already express it |
| :--- | :--- | :--- |
| State | A discrete, snapshot-able configuration of information at a tick. | Zero ambient authority; flat `Pod`/`Zeroable` memory geometry (AGENTS.md sections 2, 3) |
| Transition | A deterministic map from current state and input to next state: `S(t+1) = f(S(t), I)`. | Deterministic state reducers; three-phase scan separation (AGENTS.md section 1) |
| Relationship | A typed, explicit connection between two loci of state, with direction and capacity. | Declared channels only; SWMR ring buffers (AGENTS.md section 3) |
| Constraint | A boundary that limits the state space or the valid transitions. | Fault tolerance with deadband modes; the ratchet (AGENTS.md section 1) |
| Trust | An independently evaluated attestation of provenance, authorization or correctness, computed from evidence at the moment of use. | Independent review quorum, signed attestations (in `aaroneous-devtools`) |
| Perspective | The lens through which a state space is evaluated. The work unit stays the same when the lens changes. | Role and lens rotation (in `aaroneous-devtools`) |

Invariants proposed for each primitive:

- State: holds no hidden handles; can be serialized without pointers or heap allocation.
- Transition: no side effects during reduction; a retry is a new attempt, and history is not overwritten.
- Relationship: no interaction by proximity or side channel; every interaction crosses a declared edge.
- Constraint: a violated constraint is an operating condition (primary, degraded, safe-hold), not a panic; baselines move forward only.
- Trust: never a stored boolean; a producer cannot accept its own output.
- Perspective: roles are leased bundles of viewpoints, not permanent identities of a worker.

## 2. Candidate reductions (Layers 1 and 2)

These are proposals. Each is a hypothesis about how a larger concept decomposes.

```text
Identity     = State + Trust
Capability   = Transition + Constraint
Resource     = State + Constraint
Event        = Transition + State + Relationship
Artifact     = State + Trust
Entity       = Identity + State
Skill        = Capability + Trust
Credential   = Trust + Constraint
Environment  = State space + Constraints
Role         = Perspective bundle
Guild        = Trust graph + Shared constraints
Knowledge    = Artifact + Relationship
Reasoning    = Perspective acting on State, under Constraints, proposing Transitions
Agent        = Identity + State + Capability + Trust + Perspective
```

Notes:

- Larger concepts such as agent, entity, object, knowledge and reasoning are treated as compositions, not foundations. That is the working position, not a result.
- `Resource` is used above as a derived concept. If physical bounds turn out not to reduce to State plus Constraint, it should be promoted. This is one of the open questions.

## 3. Epistemic vocabulary and current status

| Status | Meaning |
| :--- | :--- |
| Observed | Exists, with evidence in commits, logs or repositories |
| Implemented | Exists as code or schema that compiles in a workspace |
| Tested | A test or mechanical audit exists that could have failed |
| Hypothesized | A coherent explanatory model without empirical proof |
| Aspirational | A desired target with no implementation today |

| Construct | Status | What exists | Not yet shown |
| :--- | :--- | :--- | :--- |
| The six primitives as vocabulary | Hypothesized | Classification tables in section 7 | A test that could fail; use on a system not built with them in mind |
| State, Transition, Relationship, Constraint as code types | Implemented | `EngineSnapshotPod`, `ChannelTopologyPod`, governance interlocks, scan reducers | That every type maps cleanly (see the coupling notes in section 8) |
| Trust as evaluated function | Implemented (in `aaroneous-devtools`) | Independence and review-carry logic in `steward` | Not examined here beyond file names and a few struct fields |
| Perspective | Implemented (in `aaroneous-devtools`) | Role and lens constants in `steward/src/cards.rs` | That lens rotation changes outcomes |
| Resource as derived | Hypothesized | Used in formulas | A case where it cannot reduce |
| Six-primitive sufficiency | Hypothesized | No remainder found in the items classified | An adversarial case; the classifier can always choose a formula, so "none found" is weak evidence |
| Omni graph ingestion | Aspirational | No dataset in the repository | Any ingestion into the `omni` crate |
| Chimera distance optimization | Aspirational | None | An optimizer; a definition of distance |

## 4. Repository census (author-supplied)

The author listed eleven repositories. `aaroneous` (this repository) and `aaroneous-devtools` were examined directly, and the Maelstrom tree under AutoWizard101 partially (see [MAELSTROM_PRIMITIVE_AUDIT.md](MAELSTROM_PRIMITIVE_AUDIT.md)). The other rows are the author's reading and were not checked. The census may be out of date: Maelstrom is now developed under AutoWizard101, and the owner intends the two to progress together.

| Repository | Primary purpose | Concept it is said to own | Examined here |
| :--- | :--- | :--- | :--- |
| AutoWizard101 | Game automation and trainer (now hosts Maelstrom) | Observed environment | Partly |
| Wizard101_DanceBot | Minigame rhythm automation | Deterministic sequence | No |
| AaroneousAutomationSuite | Multi-process orchestration | Coordinated pipeline | No |
| Maelstrom | Perception, input and policy | Embodiment | Partly |
| Merlin | Reasoning and model routing | Hypothesis and strategy | No |
| Guild | Multi-agent collaboration | Institution and delegation | No |
| Library | Long-term knowledge store | Persistent memory | No |
| MyFortress | Credentials and enclaves | Boundary and interlock | No |
| Workbench | Diagnostic visualizer | Observation | No |
| aaroneous | Microkernel and state engine | Physical runtime | Yes |
| aaroneous-devtools | Multi-model review and gates | Institutional trust | Yes |

## 5. Name crosswalk (labels, not established reductions)

| Name | Working label | Open question |
| :--- | :--- | :--- |
| Maelstrom | Perception and actuation | Boundary with AutoWizard101 after the restart |
| Merlin | Reasoning as proposer of transitions | None recorded |
| Guild | Cooperative trust graph | None recorded |
| Steward | Dispatch and lease governor | None recorded |
| Library | Artifact store with provenance | None recorded |
| MyFortress | Zero ambient authority, credential fences | None recorded |
| Omni | Spatial graph of entities and relationships | Whether it is a primitive-level or derived structure |
| Cratify | Compliance ratchet | Whether it is best read as trust in implementation |
| Chimera | Proposed evolution engine | Aspirational |
| Workbench | Read-only instrumentation | None recorded |

Open hypothesis (recorded from the owner, originally a Copilot reconstruction, not adopted): the institutional systems are each trust applied to one domain (Cratify: implementation; Library: knowledge; Merlin: reasoning; Fortress: authority; Guild: collective execution; Steward: process; Chimera: change; Omni: understanding), with a reality layer below and a trust layer above. To count as a finding it needs a case it could fail on, such as an institution that is not mainly about trust, because "can X be trusted?" can be asked of almost anything. Names also conflict with the table above (Cratify, AutoWizard101, Omni) and the owner decides which stands.

## 6. Coverage matrix (author's mapping, unreviewed)

An X means the author judged the component to express that primitive. Source files in this repository were checked to exist; paths under `aaroneous-devtools` are marked.

| Component or artifact | State | Transition | Relationship | Constraint | Trust | Perspective |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| `core-contracts` (`lib.rs`, `metadata.rs`, `base36.rs`) | X | X | X | X | | |
| `compute` (`automata.rs`) | X | X | | X | | |
| `ipc_bus` (`SwrnRingBuffer`) | X | | X | X | | |
| `governance` (SMT interlocks, governors) | | | | X | X | |
| `omni` (spatial graph and indices) | | | X | | | X |
| devtools `steward/src/cards.rs` | | X | | X | | X |
| devtools `steward/src/verification.rs` | | | X | | X | |
| devtools `contracts/guild-operation-v1.md` | X | X | X | X | X | |
| `AGENTS.md` | | X | X | X | X | |
| `docs/CRATIFY_SPEC.md` | | | | X | X | |
| devtools `governance/routines/STEWARD_HEARTBEAT.md` | | X | | X | X | X |

## 7. Classification audits

Each audit lists items by name with the author's classification. File names, enum variants, flag names and struct fields named in the notes were checked against source on 2026-11-18. Other descriptive phrases in the Notes column (what a type "does") are the author's and were not checked. The classification itself is the author's judgment and has not been reviewed by a second party. "Coupling" notes are candidate observations, not defects.

Counting rule: top-level `pub struct`, `pub enum`, `pub trait` and `pub type` definitions in `src/`, plus the two `bitflags!` structs in core-contracts. Nested modules and re-exports are not counted, so these counts are lower bounds.

### 7.1 Audit 001: `core-contracts` (16 of 16 items classified)

| Item | File | Classification | Formula | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `HierarchyTier` | `lib.rs` | Layer 1 | Constraint | Variants: Core, System, SubordinateModule, Extension. Restricts dependency direction. |
| `ExecutionMethodology` | `lib.rs` | Layer 1 | Capability | Variants: LocalQuantizedModel, RemoteApiProxy, HardwareAcceleratedDirect, DeterministicSimulation. |
| `OperationalIntent` | `lib.rs` | Layer 1 | State + Constraint | Bitflags: RealtimeLowLatency, BatchProcessing, HighThroughput, SecureEnclave. |
| `Capability` | `lib.rs` | Layer 1 | Capability | Permitted I/O bitmask (INFERENCE_OUT, AUDIO_OUT, VIDEO_OUT, SENSOR_INPUT, and others). |
| `IpcHeader` | `lib.rs` | Layer 1 | Relationship + Constraint | Fields: `dst`, `src`, `len`, `flags`. |
| `EngineSnapshotPod` | `lib.rs` | Layer 0 | State | Plain-data snapshot of engine performance and pacing. See coupling note. |
| `ComponentManifest` | `lib.rs` | Layer 2 | Entity | Fields include name, version, tier, capabilities, methodology and supported intents. |
| `QueryDescriptor` | `lib.rs` | Layer 1 | Perspective + Constraint | Capability routing request. |
| `FlightEventKind` | `lib.rs` | Layer 0 | Transition | Variants include Unknown, CommandInput, StateTransition, TelemetryTick, Checkpoint. |
| `FlightEventPod` | `lib.rs` | Layer 0 | Transition + State | Carries `pre_state_hash`, `post_state_hash` and `input_hash`. |
| `FlightFileHeaderPod` | `lib.rs` | Layer 1 | Resource | Fixed-slot circular buffer geometry with a wrap counter. |
| `ObservationFramePod` | `lib.rs` | Layer 0 | State + Perspective | Carries `actual_opcode` and `predicted_opcode`. |
| `ComponentMetadataPod` | `metadata.rs` | Layer 2 | Identity + Capability + Constraint | 64 bytes; size asserted in a test. |
| `ChannelTopologyPod` | `metadata.rs` | Layer 0 | Relationship + Constraint | 64 bytes; size asserted in a test. |
| `Base36Error` | `base36.rs` | Layer 0 | Constraint | Variants: InvalidChar, InvalidLength, InputTooLarge, BufferTooSmall, Overflow. |
| `Base36Lineage` | `base36.rs` | Layer 1 | Identity | Fixed 24-character lineage stamp (`LINEAGE_STAMP_WIDTH`). |

Coupling notes:

- `ObservationFramePod` holds observed fields and a predicted opcode with a concurrence value in one record. That is efficient for packing and mixes an observation with an evaluation of it.
- `EngineSnapshotPod` carries `user_xp`, `user_level`, `active_companions_count` and `running_macros_count` next to generic fields such as `measured_fps` and `bus_integrity`. Experience, level, companions and macros are product or game concepts inside a kernel-profile contract. Reading: that is the capability-plus-manifestation pattern (section 8) in the shared contracts, and it is relevant to making Aaroneous and Maelstrom game-agnostic together.

### 7.2 Audit 002: `governance` (25 of 34 items classified; 9 unclassified)

| Item | File | Classification | Formula | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `GovernanceError` | `smt_action_interlock.rs` | Layer 0 | Constraint | Error enum for interlock violations. |
| `InterlockAuditCertificate` | `smt_action_interlock.rs` | Layer 0 | Trust | Certificate from the interlock. |
| `SmtActionInterlock` | `smt_action_interlock.rs` | Layer 0 | Constraint | Pre-execution gate. |
| `NonInterferenceReport` | `interference_checker.rs` | Layer 0 | Trust | Report on disjoint register sets. |
| `InterferenceChecker` | `interference_checker.rs` | Layer 0 | Constraint | Prover for concurrent graphs. |
| `VerificationReport` | `lattice_verifier.rs` | Layer 0 | Trust | Dimensional-consistency report. |
| `LatticeVerifier` | `lattice_verifier.rs` | Layer 0 | Constraint | Dimensional validator. |
| `ConstraintInspectionReport` | `constraint_inspector.rs` | Layer 0 | Trust + State | Combines interlock certification with throttle state. |
| `ConstraintInspector` | `constraint_inspector.rs` | Layer 0 | Constraint | Evaluator over interlocks and limits. |
| `GenerationSnapshot` | `rollback_journal.rs` | Layer 0 | State + Trust | Checkpoint in an append-only generation. |
| `GenerationalJournal` | `rollback_journal.rs` | Layer 1 | Resource | Bounded ring of checkpoints. |
| `DegradationTier` | `resource_governor.rs` | Layer 0 | Constraint | Variants: Nominal, SoftThrottle, HeavyThrottle, EmergencyHalt. |
| `DynamicEquilibriumState` | `resource_governor.rs` | Layer 0 | State | Point-in-time load and token snapshot. |
| `FeedbackRegulator` (alias `ResourceGovernor`) | `resource_governor.rs` | Layer 1 | Capability | Closed-loop regulator. |
| `SystemHealthGovernor` | `system_limits.rs` | Layer 1 | Capability | Token-bucket rate controller. |
| `SpecialistBudget` | `system_limits.rs` | Layer 1 | Resource + Perspective | See coupling note. |
| `ThrottleState` | `system_limits.rs` | Layer 0 | Constraint | Variants: Normal, Throttled, Dormant. |
| `MetricObservation` | `metrics_exporter.rs` | Layer 0 | State | A metric point. |
| `UniversalMetricsSink` (trait), `InMemoryMetricsSink` | `metrics_exporter.rs` | Layer 1 | Relationship | Telemetry channel. |
| `LoadGovernorConfig`, `PredictiveLoadGovernor` | `load_governor.rs` | Layer 3 | Perspective | Load forecaster. |
| `AdaptiveAction`, `AdaptiveGovernor` | `throughput_governor.rs` | Layer 1 | Capability | Closed-loop throughput controller. |

Not yet classified: `AdaptiveForecast`, `AdaptiveGovernorConfig` (`throughput_governor.rs`); `ExecutionBias`, `SpecialistHealth`, `SystemHealthReport` (`system_limits.rs`); `GovernanceAction`, `LoadForecast` (`load_governor.rs`); `MetricSummary`, `UniversalMetricsExporter` (`metrics_exporter.rs`).

Coupling note: `SpecialistBudget` holds token allocation (`tokens`, `max_tokens`, `regen_rate`) and behavioural drivers (`ambition` and related fields) in one struct, which reads as Resource plus Perspective.

### 7.3 Audit 003: `omni` (25 of 48 names classified; 23 unclassified)

| Item | File | Classification | Formula | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `StarNode` | `node_descriptor.rs` | Layer 2 | Entity | Position, a 32-dimension optional latent vector, links, tags. A second, different `StarNode` exists in `ecs_cluster.rs`. |
| `StarNodeType` | `node_descriptor.rs` | Layer 1 | Constraint | Fifteen variants (Feature, Bug, Roadmap, Decision, Architecture, Lore, ...). |
| `StarNodeStatus` | `node_descriptor.rs` | Layer 0 | State | Planned, InProgress, Completed, Archived, Dormant. |
| `LinkType` | `node_descriptor.rs` | Layer 0 | Relationship | DependsOn, Implements, Documents, RelatesTo, Synthesizes. |
| `Priority` | `node_descriptor.rs` | Layer 1 | Constraint | Low, Medium, High, Critical. |
| `SpatialCoord` | `spatial_coord.rs` | Layer 0 | Relationship | 3D coordinate with distance metrics. |
| `SpatialFrustum` | `query_engine.rs` | Layer 1 | Constraint | Region bounds for queries. |
| `OmniQueryFilter` | `query_engine.rs` | Layer 1 | Perspective + Constraint | Filter over types, statuses, domains and frusta. |
| `OmniQueryEngine` | `query_engine.rs` | Layer 1 | Capability | Query evaluation. |
| `OmniEngine` | `lib.rs` | Layer 3 | State + Relationship | Holds nodes and galaxies behind `Arc<RwLock<..>>`. |
| `NonOverlapSolver` | `spatial_layout.rs` | Layer 1 | Capability | Layout solver. |
| `RectAabb` | `spatial_layout.rs` | Layer 1 | Constraint | Axis-aligned box. |
| `WindowArrangementStrategy` | `spatial_layout.rs` | Layer 1 | Perspective | Variants: Cascade, TileHorizontal, TileVertical, TileGrid, AabbRepulsion. |
| `VectorDocument` | `vector_index.rs` | Layer 1 | State + Relationship | Embedding payload with metadata. |
| `VectorIndexEngine` | `vector_index.rs` | Layer 1 | Capability | Similarity search. |
| `VectorSearchResult` | `vector_index.rs` | Layer 1 | Relationship + State | Result with score. |
| `GalacticClusteringEngine` | `node_cluster.rs` | Layer 1 | Capability | Clustering. |
| `GalaxyCluster` | `node_cluster.rs` | Layer 2 | Entity | Cluster aggregate. |
| `EcsGalaxyEngine` | `ecs_cluster.rs` | Layer 1 | State + Relationship | ECS representation of the galaxy graph. |
| `SpatialTransform` | `ecs_cluster.rs` | Layer 0 | State | Transform components. |
| `SemanticEmbedding` | `ecs_cluster.rs` | Layer 0 | Relationship | Latent vector. |
| `MetabolicEnergy` | `ecs_cluster.rs` | Layer 1 | State + Constraint | Activity metric. |
| `OmniProtocolBridge` | `protocol_bridge.rs` | Layer 1 | Relationship | Export bridge. |
| `SabMatrix`, `SabSurface` | `matrix/sab_matrix.rs` | Layer 1 | Relationship + Constraint | Surface block and matrix. |

Not yet classified: `BoundingBox3D` (`vector_index.rs`); `ConstellationLinks` and the second `StarNode` definition (`ecs_cluster.rs`); `GalacticRoamingCameraController`, `SynapsePulseParticle`, `SynapsePulseParticleSystem`, `Vec3` (`cluster_routing.rs`); `InMemorySpatialCanvasSink`, `OmniGalaxySnapshot`, `UniversalSpatialCanvasSink`, `UniversalSpatialLink`, `UniversalSpatialPoint` (`protocol_bridge.rs`); `SabEmbedding`, `SabMetadata`, `SabSimilarityMatrix` (`matrix/sab_tensor.rs`); `SabManifest`, `SabMatrixBuilder` (`matrix/sab_matrix.rs`); and the type aliases in `lib.rs`: `ClusteringContext`, `Constellation`, `ConstellationNode`, `ConstellationQuery`, `NodeStatus`, `NodeType`, `RelationshipType`.

Coupling note: `StarNode` carries persistent fields (id, title, node type, links) and display fields (`activity_pulse`, a glow intensity). That puts a relationship record and a visual state in one struct.

### 7.4 Audit 005: `steward` and the Guild contract (in `aaroneous-devtools`; author's classification, partly checked)

Checked on 2026-11-18: the files `local_authority.rs`, `independence.rs`, `review_carry.rs`, `receipts.rs`, `board.rs`, `cards.rs` exist under `foundation/crates/steward/src`; `Principal` carries `family`, `roles`, `platforms` and a private `public_key`; `Policy` carries `minimum_reviewers` and `minimum_families`; review-carry statuses include `Carried`, `ChangedMainAlsoMoved` and `NewSinceReview`. Everything else in this audit is the author's reading.

Roles and lenses in `cards.rs` are string constants (`DEFAULT_ROLES`, `LENSES`), not types. A "task card" is a JSON object built by `card_for` and `enrich_card`; there is no `TaskCard` Rust type.

| Item | Location | Formula | Coupling note |
| :--- | :--- | :--- | :--- |
| Intent revision, operation attempt, verification association, transition guards, budget envelope | `contracts/guild-operation-v1.md` | Mixed | Not checked here |
| `Principal` | `local_authority.rs` | Identity + Trust, plus roles and platforms | Identity, role authorization and execution environment are in one struct |
| `Policy` | `local_authority.rs` | Constraint + Trust | None recorded |
| Independence report | `independence.rs` | Trust + Perspective | Not checked here |
| Review-carry status | `review_carry.rs` | Trust + Transition | Not checked here |
| Task card (JSON) | `cards.rs` | Intent + Perspective + Constraint | Observed condition and shell recipe are fields of one object |
| `AppendReceipt` | `board.rs` | Artifact | Not checked here |

### 7.5 Not yet audited: Library, Merlin, MyFortress, AutoWizard101, DanceBot

No constructs from these repositories are classified here, because their sources were not opened. The claims that knowledge reduces to Artifact plus Relationship and reasoning to Perspective acting on State under Constraints are carried as candidate reductions in section 2. Identity, economics and time are open questions (section 0), not settled.

## 8. Coupling notes (candidate anti-patterns)

No numeric score or ecosystem average is reported, because any score assigned by hand would only restate the author's judgment. Three coupling patterns recur in the notes above and are kept as hypotheses to test:

1. State plus behaviour: an observed condition bundled with the procedure that remedies it (the task-card JSON; `ObservationFramePod` in part).
2. Identity plus role: who an entity is bundled with what it may do and where it runs (`Principal`).
3. Core capability plus presentation: generic execution logic bundled with domain or theme data (`MacroCard`, see the Maelstrom audit).

A numeric purity metric would need an agreed rubric and scoring done from code, not by hand. Until one exists, treat these as review prompts.

## 9. Reference points for a from-scratch build (not adopted)

For orientation only. None of these is a dependency, a recommendation or a decision, and no property of these tools (for example allocation behaviour) has been verified here. Any adoption goes through AGENTS.md section 4 (dependency admission scoring), and each tool's maintenance status must be checked first.

| Area | Candidates named by the author |
| :--- | :--- |
| Messaging | NATS |
| Workflow engine | Temporal |
| Graph and vector index | Kuzu, Qdrant |
| Analytics | DuckDB |
| Inference | llama.cpp, ONNX Runtime |
| Structural code analysis | Tree-sitter, ast-grep |
| Verification | cargo-nextest, cargo-deny |
| Full-text search | Tantivy |
| Perception | OpenCV, Playwright |

## 10. Next steps and their status

| Step | Status |
| :--- | :--- |
| 1. Write the vocabulary and classification notes | Draft written and normalized; needs independent review |
| 2. Derive the type census from source in Rust (replacing hand-built lists) | Not started |
| 3. Build a graph with real dependency edges (from `cargo metadata`) and edge types matching `omni::LinkType` | Not started |
| 4. Classify the unclassified items in sections 7.2 and 7.3 | Not started |
| 5. Run a falsification case: classify a system not built with these primitives, stating beforehand what would count as failure | Not started |
| 6. Apply the same method to Maelstrom as it is rebuilt to be game-agnostic | Not started |
