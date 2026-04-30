# Phase 6: Ecosystem Integration - Aaroneous as Federation Engine

**Status:** In Development  
**Duration:** 4-6 weeks  
**Objective:** Transform Aaroneous from advanced intelligence system into the **high-performance binary engine** powering the entire AAS Federation (AaroneousAutomationSuite, Guild, Merlin, Library, Maelstrom)

---

## Executive Summary

Aaroneous will evolve from a standalone intelligent system into the **stem cell engine** that provides:

1. **MCP Bridge Layer** - Unified communication between AAS (Python) and Aaroneous (Rust)
2. **Distributed Event Log** - RocksDB-backed federation-wide transaction log with replication
3. **Distributed Tracing** - OpenTelemetry-based causal ordering across all domains
4. **Raft Consensus** - Atomic mutations affecting multiple domains with automatic rollback
5. **Universal Critic Loop** - Cross-domain validation framework for all operations
6. **Distillation Pipeline** - Autonomous knowledge compression from logs → GGUF models
7. **Failure Recovery** - Automatic cascade detection and checkpoint-restore

---

## Architecture Overview

### Current State (Phase 5)
```
Aaroneous (Rust)
├── Advanced Intelligence
│   ├── Anomaly Detection
│   ├── Forecasting
│   ├── Auto-Scaling
│   ├── Self-Healing
│   └── Optimization
└── Enterprise Features
    ├── Auth & RBAC
    ├── Monitoring
    └── Scaling
```

### Phase 6 Target State
```
Aaroneous (Rust - Federation Engine)
├── Phase 5 Intelligence (Preserved)
├── Federation Layer
│   ├── MCP Bridge ←→ AAS Python
│   ├── Event Log (RocksDB)
│   ├── Distributed Tracing (OpenTelemetry)
│   ├── Raft Consensus
│   ├── Critic Loop Framework
│   ├── Distillation Pipeline
│   └── Recovery Engine
└── NATS Federation Bus
    ├── Heartbeat → Guild, Merlin, Library
    ├── Mutations ← All domains
    └── Telemetry ← Maelstrom
```

---

## Phase 6A: Infrastructure (Weeks 1-2)

### 1. MCP Bridge Layer (High Priority)

**Goal:** Establish bidirectional communication between AAS (Python asyncio) and Aaroneous (Rust async)

**Components:**

```rust
// src/mcp_bridge/mod.rs - MCP server exposing Aaroneous services
pub mod server;      // MCP server implementation
pub mod client;      // Client to call AAS MCP methods
pub mod protocol;    // Wire format and versioning
pub mod types;       // Shared type definitions

// src/mcp_bridge/server.rs
pub struct McpServer {
    nats: NatsConnection,
    capabilities: CapabilityRegistry,
}

impl McpServer {
    async fn start(&self) -> Result<()>;
    async fn register_capability(&mut self, cap: MpcCapability);
    async fn call_aas_method(&self, method: &str, args: serde_json::Value) -> Result<serde_json::Value>;
}

// src/mcp_bridge/types.rs
pub struct MpcCapability {
    domain: String,           // "anomaly_detection", "consensus", "tracing"
    method_name: String,
    version: String,
    schema: JsonSchema,
}

#[derive(Serialize, Deserialize)]
pub struct MpcMessage {
    trace_id: String,         // For distributed tracing
    timestamp: i64,
    method: String,
    params: Map<String, Value>,
    reply_to: String,
}
```

**Exposed Capabilities:**
- `aaroneous.federation.healthcheck` → Returns federation status
- `aaroneous.event_log.append` → Append to distributed log
- `aaroneous.tracing.emit_span` → Record distributed trace
- `aaroneous.consensus.propose_mutation` → Submit mutation to Raft
- `aaroneous.critic.validate` → Run validation on output
- `aaroneous.recovery.checkpoint` → Create federation checkpoint

**Testing:**
- Unit tests: MPC message serialization/deserialization
- Integration tests: AAS MPC client calling Aaroneous methods
- Stress tests: 10K messages/sec throughput

**Estimated effort:** 400 lines Rust + 200 lines tests = 600 lines total

---

### 2. Event Log Infrastructure (High Priority)

**Goal:** Build RocksDB-backed federation-wide transaction log

**Components:**

```rust
// src/event_log/mod.rs
pub mod store;      // RocksDB wrapper
pub mod types;      // Event schema
pub mod replicator; // Async replication to siblings

// src/event_log/types.rs
#[derive(Serialize, Deserialize, Clone)]
pub struct FederationEvent {
    event_id: String,              // UUID4
    timestamp: i64,
    trace_id: String,              // Links to distributed trace
    source_repo: String,           // "AaroneousAutomationSuite", "Guild", etc.
    source_domain: String,         // "leadership", "intelligence", "knowledge"
    event_type: EventType,
    operation: Operation,          // "plugin_load", "mutation", "health_check"
    payload: Map<String, Value>,
    consensus_round: Option<u64>,  // Raft round if mutation
    replicas_acked: Vec<String>,   // Which siblings have replicated
}

pub enum EventType {
    Boot,
    PluginLoad,
    PluginExec,
    Mutation,
    HealthCheck,
    Validation,
    Repair,
    Distillation,
    Failure,
    Recovery,
}

pub enum Operation {
    Create(String),
    Update(String),
    Delete(String),
    Query(String),
    Replicate(String),
    Rollback(String),
}

// src/event_log/store.rs
pub struct EventLogStore {
    db: rocksdb::DB,
    index: BTreeMap<String, Vec<u64>>, // trace_id → event offsets
}

impl EventLogStore {
    pub async fn append(&mut self, event: FederationEvent) -> Result<LogOffset>;
    pub async fn read_range(&self, start: LogOffset, end: LogOffset) -> Result<Vec<FederationEvent>>;
    pub async fn query_by_trace(&self, trace_id: &str) -> Result<Vec<FederationEvent>>;
    pub async fn create_snapshot(&self) -> Result<SnapshotId>;
    pub async fn restore_from_snapshot(&mut self, snapshot: SnapshotId) -> Result<()>;
}

// src/event_log/replicator.rs
pub struct EventLogReplicator {
    nats: NatsConnection,
    peers: Vec<RepoId>,
}

impl EventLogReplicator {
    pub async fn replicate_to_peers(&self, event: FederationEvent) -> Result<ReplicationAck>;
    pub async fn handle_incoming_event(&mut self, event: FederationEvent) -> Result<()>;
    pub async fn sync_full_log(&self, peer: RepoId) -> Result<()>;
}
```

**Persistence Strategy:**
- Primary: RocksDB in `/data/event_log.db`
- Replicas: Async replication to siblings via NATS
- Snapshots: Compressed event batches for fast recovery

**Testing:**
- Unit tests: Event serialization, log ordering
- Integration tests: Replication across 3 repos
- Chaos tests: Network partition → recovery

**Estimated effort:** 600 lines Rust + 300 lines tests = 900 lines total

---

### 3. Distributed Tracing Framework (High Priority)

**Goal:** Enable causal ordering of operations across federation

**Components:**

```rust
// src/tracing/mod.rs
pub mod coordinator;   // TraceId generation and management
pub mod exporter;      // OpenTelemetry export
pub mod spans;         // Span record types

// src/tracing/coordinator.rs
pub struct TracingCoordinator {
    config: TracingConfig,
    exporter: Box<dyn SpanExporter>,
}

#[derive(Clone)]
pub struct TraceId {
    uuid: String,                    // UUID4
    repo_id: String,                 // e.g. "AaroneousAutomationSuite"
    sequence: u64,                   // Monotonic counter per repo
}

impl TraceId {
    pub fn generate(repo_id: &str) -> Self;
    pub fn parse(s: &str) -> Result<Self>;
    pub fn to_string(&self) -> String;
}

pub struct Span {
    trace_id: TraceId,
    span_id: String,
    parent_span_id: Option<String>,
    operation_name: String,
    start_time: i64,
    end_time: Option<i64>,
    status: SpanStatus,
    attributes: Map<String, Value>,
    events: Vec<SpanEvent>,
}

pub enum SpanStatus {
    Running,
    Ok,
    Error(String),
    Cancelled,
}

// src/tracing/exporter.rs
pub trait SpanExporter: Send + Sync {
    async fn export(&self, spans: Vec<Span>) -> Result<()>;
}

pub struct OpenTelemetryExporter {
    endpoint: String,
    client: reqwest::Client,
}

impl SpanExporter for OpenTelemetryExporter {
    async fn export(&self, spans: Vec<Span>) -> Result<()> {
        // Convert to OTLP format and send to collector
    }
}

// src/tracing/spans.rs - Pre-built spans for common operations
pub fn span_federation_boot(trace_id: TraceId) -> Span;
pub fn span_mcp_call(trace_id: TraceId, method: &str) -> Span;
pub fn span_consensus_round(trace_id: TraceId, round: u64) -> Span;
pub fn span_critic_validation(trace_id: TraceId, domain: &str) -> Span;
pub fn span_distillation(trace_id: TraceId, sop_count: usize) -> Span;
```

**Trace Injection into NATS:**
- Every NATS message carries `trace_id` header
- Every MCP call carries `trace_id` parameter
- Every database operation logged with `trace_id`

**Visualization:**
- Traces stored in event log
- Exported to OpenTelemetry collector
- Visualized in Jaeger UI
- Replayed in Maelstrom 3D space

**Testing:**
- Unit tests: TraceId generation, uniqueness
- Integration tests: Trace propagation through 3-hop request
- Visualization tests: Jaeger UI shows complete traces

**Estimated effort:** 500 lines Rust + 300 lines tests = 800 lines total

---

## Phase 6B: Consensus & Safety (Weeks 3-4)

### 4. Raft Consensus Engine (High Priority)

**Goal:** Atomic mutations affecting multiple domains with automatic rollback

**Components:**

```rust
// src/consensus/mod.rs
pub mod raft;        // Raft state machine implementation
pub mod log;         // Replication log
pub mod state;       // Follower/candidate/leader state
pub mod leader;      // Leader election
pub mod log_replication; // Log entry replication

// src/consensus/raft.rs
pub struct RaftNode {
    peer_id: String,                  // e.g. "AaroneousAutomationSuite"
    peers: Vec<String>,               // All federation members
    state: RaftState,                 // Follower/Candidate/Leader
    current_term: u64,
    voted_for: Option<String>,
    log: Vec<LogEntry>,
    commit_index: usize,
    last_applied: usize,
}

pub struct LogEntry {
    term: u64,
    index: u64,
    command: MutationCommand,
    trace_id: String,
    applied_at: Option<i64>,
}

pub enum MutationCommand {
    UpdateConfig { path: String, value: Value },
    PromotePlugin { plugin_id: String, from_domain: String, to_domain: String },
    CrystalizeRelic { sop_source: String, relic_target: String },
    RollbackTo { checkpoint_id: String },
    UpdateSpectrumConfig { spectrum: String, config: Value },
}

impl RaftNode {
    pub async fn propose_mutation(&mut self, cmd: MutationCommand) -> Result<CommitId>;
    pub async fn handle_append_entries(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse>;
    pub async fn handle_request_vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse>;
    pub async fn become_leader(&mut self) -> Result<()>;
    pub async fn replicate_log_to_peers(&self) -> Result<Vec<ReplicationAck>>;
    pub async fn apply_committed_entries(&mut self) -> Result<()>;
}

// src/consensus/state_machine.rs - Applies mutations
pub struct FederationStateMachine {
    config: Map<String, Value>,
    spectrum: Map<String, SpectrumConfig>,
    relic_index: Map<String, RelicMetadata>,
}

impl FederationStateMachine {
    pub async fn apply(&mut self, cmd: MutationCommand) -> Result<()>;
    pub async fn snapshot(&self) -> Result<StateSnapshot>;
    pub async fn restore_from_snapshot(&mut self, snapshot: StateSnapshot) -> Result<()>;
}
```

**Mutation Flow:**
```
1. Client (AAS plugin) calls: `aaroneous.consensus.propose_mutation`
2. MCP Bridge receives mutation, assigns trace_id
3. Leader appends to Raft log (term, index, trace_id)
4. Leader replicates to followers via NATS
5. Followers append to local logs
6. Leader waits for quorum (>50%) ACKs
7. Leader applies to state machine
8. Leader broadcasts commit index
9. Followers apply to state machine
10. Event logged in event_log
```

**Rollback on Failure:**
```
If validation fails:
  1. Critic plugin detects error
  2. Request rollback via `aaroneous.consensus.rollback`
  3. Leader appends RollbackTo command
  4. Replicates to followers
  5. All apply previous snapshot
  6. Emit failure event with repair suggestions
```

**Testing:**
- Unit tests: Raft state transitions, log replication
- Integration tests: 3-node consensus, mutation propagation
- Chaos tests: Network partition, Byzantine failures, leader crashes

**Estimated effort:** 1000 lines Rust + 500 lines tests = 1500 lines total

---

### 5. Control Plane Leasing (Medium Priority)

**Goal:** Prevent concurrent mutations of same domain

**Components:**

```rust
// src/consensus/leasing.rs
pub struct ControlPlaneLease {
    holder: String,                    // "Guild", "Merlin", etc.
    acquired_at: i64,
    expires_at: i64,
    domains: Vec<String>,              // Which domains are locked
    trace_id: String,
}

pub struct LeasingManager {
    held_leases: Map<String, ControlPlaneLease>,
    nats: NatsConnection,
}

impl LeasingManager {
    pub async fn acquire_lease(
        &mut self,
        holder: &str,
        domains: Vec<String>,
        ttl_ms: u64,
    ) -> Result<LeaseToken>;
    
    pub async fn release_lease(&mut self, token: LeaseToken) -> Result<()>;
    
    pub async fn extend_lease(&mut self, token: LeaseToken, additional_ms: u64) -> Result<()>;
    
    pub async fn check_domain_free(&self, domain: &str) -> Result<bool>;
    
    // Auto-expiry on lease timeout
    pub async fn reap_expired_leases(&mut self) -> Result<()>;
}

// Mutation requires lease:
pub async fn propose_mutation_with_lease(
    &mut self,
    domains: Vec<String>,
    cmd: MutationCommand,
) -> Result<CommitId> {
    let lease = self.lease_manager.acquire_lease("AAS", domains.clone(), 5000).await?;
    let result = self.raft_node.propose_mutation(cmd).await;
    self.lease_manager.release_lease(lease).await?;
    result
}
```

**Estimated effort:** 300 lines Rust + 200 lines tests = 500 lines total

---

## Phase 6C: Validation & Learning (Weeks 5-6, overlapping)

### 6. Universal Critic Loop Framework (Medium Priority)

**Goal:** Cross-domain validation framework all domains inherit from

**Components:**

```rust
// src/critic_framework/mod.rs
pub mod validator;     // Generic validation logic
pub mod repairer;      // Repair strategy orchestration
pub mod rules;         // Validation rule definitions

// src/critic_framework/validator.rs
pub struct UniversalCriticLoop {
    validation_rules: Map<String, ValidationRule>,  // domain → rules
    repair_strategies: Map<String, RepairStrategy>,
}

pub struct ValidationRule {
    domain: String,
    name: String,
    check_fn: Box<dyn Fn(&ExecutionOutput) -> ValidationResult + Send + Sync>,
    severity: ValidationSeverity,
    auto_repair: bool,
}

pub enum ValidationResult {
    Pass { confidence: f32 },
    Warn { reason: String, confidence: f32 },
    Fail { reason: String, suggested_repair: Option<RepairAction> },
}

pub enum ValidationSeverity {
    Advisory,     // Warn but allow
    Warning,      // Warn and escalate
    Mandatory,    // Must pass
    Critical,     // Failure causes rollback
}

impl UniversalCriticLoop {
    pub async fn validate_execution(
        &self,
        domain: &str,
        output: ExecutionOutput,
        trace_id: TraceId,
    ) -> Result<ValidationResult>;
    
    pub async fn attempt_repair(
        &self,
        failure: ValidationResult,
        output: ExecutionOutput,
        attempt: u32,
    ) -> Result<RepairOutcome>;
}

// src/critic_framework/rules.rs
impl UniversalCriticLoop {
    pub fn load_default_rules(&mut self) {
        // Correctness: Output matches expected schema
        // Completeness: All required fields present
        // Safety: No security violations
        // Efficiency: Execution time within bounds
        // Coherence: Output logically consistent
    }
}
```

**Domain-Specific Rules:**
```rust
// For Leadership domain: "Actions must be reversible"
// For Intelligence domain: "Conclusions must cite evidence"
// For Knowledge domain: "Updates must be idempotent"
// For Distillation domain: "Knowledge loss < 5%"
```

**Repair Strategies (inherit from Phase 5 self-healing):**
1. **Faster Tier** - Re-run with smaller model
2. **Heavier Tier** - Re-run with larger model + more context
3. **More Context** - Re-run with additional memory
4. **Escalation** - Hand to human specialist

**Testing:**
- Unit tests: Rule evaluation, repair orchestration
- Integration tests: Critic loop for each domain
- Regression tests: Ensure Phase 5 self-healing still works

**Estimated effort:** 600 lines Rust + 300 lines tests = 900 lines total

---

### 7. Distillation Pipeline (Medium Priority)

**Goal:** Autonomous knowledge compression from logs → GGUF models

**Components:**

```rust
// src/distillation_pipeline/mod.rs
pub mod extractor;     // Knowledge extraction from logs
pub mod compressor;    // GGUF shard generation
pub mod splicing;      // Model update mechanism

// src/distillation_pipeline/types.rs
pub struct DistillationJob {
    job_id: String,
    source_domain: String,            // "leadership", "intelligence"
    sop_pattern: String,               // e.g. "federation_boot_safe"
    occurrence_count: usize,
    confidence: f32,
    extracted_knowledge: Vec<KnowledgeAtom>,
    target_relic: String,             // e.g. "Odin_agent.gguf"
    compression_ratio: f32,
}

pub struct KnowledgeAtom {
    sop_step: String,
    preconditions: Vec<String>,
    actions: Vec<String>,
    postconditions: Vec<String>,
    success_rate: f32,
    failure_modes: Vec<String>,
}

// src/distillation_pipeline/extractor.rs
pub struct KnowledgeExtractor;

impl KnowledgeExtractor {
    pub async fn extract_from_logs(
        &self,
        domain: &str,
        log_entries: Vec<FederationEvent>,
    ) -> Result<Vec<KnowledgeAtom>>;
    
    pub async fn identify_patterns(
        &self,
        atoms: Vec<KnowledgeAtom>,
    ) -> Result<Vec<SopPattern>>;
    
    pub async fn calculate_metrics(
        &self,
        pattern: SopPattern,
    ) -> Result<DistillationMetrics>;
}

pub struct DistillationMetrics {
    occurrence_count: usize,
    success_rate: f32,
    avg_latency: Duration,
    confidence: f32,
}

// src/distillation_pipeline/compressor.rs
pub struct GgufCompressor;

impl GgufCompressor {
    pub async fn generate_shard(
        &self,
        knowledge: Vec<KnowledgeAtom>,
        target_relic: RelicId,
    ) -> Result<GgufShard>;
    
    pub async fn estimate_compression(
        &self,
        atom_count: usize,
    ) -> Result<CompressionEstimate>;
}

pub struct GgufShard {
    relic_id: String,
    knowledge_vectors: Vec<EmbeddedKnowledge>,
    compression_metadata: Map<String, Value>,
    knowledge_loss_percent: f32,
}

// src/distillation_pipeline/splicing.rs
pub struct RelicSplicer;

impl RelicSplicer {
    pub async fn validate_shard(
        &self,
        shard: GgufShard,
        relic: RelicId,
    ) -> Result<ValidationResult>;
    
    pub async fn splice_into_relic(
        &self,
        shard: GgufShard,
        relic: RelicId,
    ) -> Result<UpdatedRelicId>;
    
    pub async fn test_updated_relic(
        &self,
        relic: RelicId,
        test_cases: Vec<TestCase>,
    ) -> Result<TestResults>;
}
```

**Distillation Trigger:**
1. Evolution ledger shows >5 occurrences of pattern "X"
2. Extractor pulls relevant events from event log
3. Compressor generates GGUF shard
4. Splicing validates and updates relic
5. Tests confirm no performance regression
6. Emit distillation event

**Periodic Distillation Schedule:**
```
- Every 6 hours: Lightweight distillation (high-occurrence patterns)
- Every 24 hours: Full distillation (all accumulated knowledge)
- Every 7 days: Cross-domain synthesis (knowledge connections)
```

**Testing:**
- Unit tests: Pattern extraction, GGUF generation
- Integration tests: Full distillation → relic update
- Regression tests: Updated relic maintains accuracy

**Estimated effort:** 800 lines Rust + 400 lines tests = 1200 lines total

---

## Phase 6D: Recovery & Optimization (Weeks 6-8, overlapping)

### 8. Catastrophic Failure Recovery Engine (Medium Priority)

**Goal:** Automatic detection and recovery from cascading failures

**Components:**

```rust
// src/recovery_engine/mod.rs
pub mod detector;      // Cascade detection
pub mod checkpointer;  // Checkpoint/restore
pub mod orchestrator;  // Recovery orchestration

// src/recovery_engine/detector.rs
pub struct CascadeDetector {
    error_window: Duration,
    threshold: u32,  // errors per window
}

pub enum CascadePattern {
    PluginCrash {
        plugin_id: String,
        occurrence_count: u32,
        affected_domains: Vec<String>,
    },
    StateCorruption {
        domain: String,
        inconsistency_type: String,
    },
    NetworkPartition {
        unreachable_repos: Vec<String>,
    },
    ResourceExhaustion {
        resource: String,
        utilization: f32,
    },
}

impl CascadeDetector {
    pub async fn detect_cascades(
        &self,
        recent_errors: Vec<FederationEvent>,
    ) -> Result<Vec<CascadePattern>>;
    
    pub async fn severity_score(&self, pattern: CascadePattern) -> u32;
    
    pub async fn estimate_blast_radius(&self, pattern: CascadePattern) -> BlastRadius;
}

pub struct BlastRadius {
    affected_repos: Vec<String>,
    affected_domains: Vec<String>,
    data_at_risk: bool,
    human_intervention_required: bool,
}

// src/recovery_engine/checkpointer.rs
pub struct FederationCheckpointer;

impl FederationCheckpointer {
    pub async fn create_checkpoint(&self) -> Result<CheckpointId>;
    
    pub async fn list_recent_checkpoints(&self) -> Result<Vec<CheckpointMetadata>>;
    
    pub async fn restore_to_checkpoint(
        &self,
        checkpoint_id: CheckpointId,
    ) -> Result<RestoreResult>;
}

pub struct CheckpointMetadata {
    checkpoint_id: String,
    timestamp: i64,
    raft_index: u64,
    affected_repos: Vec<String>,
    size_mb: u64,
}

// src/recovery_engine/orchestrator.rs
pub struct RecoveryOrchestrator {
    detector: CascadeDetector,
    checkpointer: FederationCheckpointer,
    human_notifier: Box<dyn HumanNotifier + Send + Sync>,
}

pub enum RecoveryApproval {
    Auto,                              // Execute immediately
    HumanRequired,                     // Wait for approval
    HumanVeto,                         // Human rejected recovery
}

impl RecoveryOrchestrator {
    pub async fn handle_cascade(
        &self,
        pattern: CascadePattern,
    ) -> Result<RecoveryOutcome>;
    
    pub async fn propose_recovery_plan(
        &self,
        pattern: CascadePattern,
    ) -> Result<RecoveryPlan>;
    
    pub async fn execute_recovery(
        &self,
        plan: RecoveryPlan,
    ) -> Result<RecoveryOutcome>;
}

pub struct RecoveryPlan {
    cascade_pattern: CascadePattern,
    severity: u32,
    proposed_steps: Vec<RecoveryStep>,
    estimated_duration_ms: u64,
    required_approval: RecoveryApproval,
    human_summary: String,
}

pub enum RecoveryStep {
    StopAffectedRepos(Vec<String>),
    RestoreCheckpoint(CheckpointId),
    ClearCorruptedCache(Vec<String>),
    RestartProcesses(Vec<String>),
    ValidateState,
    EmitNotification(String),
}

pub trait HumanNotifier: Send + Sync {
    async fn notify_cascade(&self, pattern: CascadePattern) -> Result<()>;
    async fn request_approval(&self, plan: RecoveryPlan) -> Result<RecoveryApproval>;
    async fn notify_recovery_complete(&self, outcome: RecoveryOutcome) -> Result<()>;
}
```

**Recovery Levels:**

| Level | Trigger | Action | Approval |
|-------|---------|--------|----------|
| 1 | Single plugin crash | Restart plugin | Auto |
| 2 | 3+ plugin crashes/hour | Clear cache + restart | Auto |
| 3 | State corruption detected | Restore from checkpoint | Human |
| 4 | Network partition >5min | Isolate partition + continue | Human |
| 5 | Multi-domain cascade | Restore all repos | Human + Veto |

**Testing:**
- Unit tests: Cascade detection patterns
- Integration tests: 3-node recovery scenarios
- Chaos tests: Disk full, memory exhaustion, network failures
- Approval tests: Human notification + veto

**Estimated effort:** 900 lines Rust + 400 lines tests = 1300 lines total

---

## Integration Architecture Summary

```
┌──────────────────────────────────────────────────────────────┐
│                 Aaroneous Phase 6 Stack                      │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Application Layer (AAS, Guild, Merlin, Library)      │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │ MCP Bridge Layer                                      │  │
│  │ ├─ AAS MCP Server                                    │  │
│  │ ├─ Capability Registry                               │  │
│  │ └─ Error Handling                                    │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │ Federation Engine (Aaroneous Core)                    │  │
│  │ ├─ Event Log (RocksDB)                               │  │
│  │ ├─ Distributed Tracing (OpenTelemetry)               │  │
│  │ ├─ Raft Consensus (mutations)                        │  │
│  │ ├─ Control Plane Leasing                             │  │
│  │ ├─ Critic Loop Framework                             │  │
│  │ ├─ Distillation Pipeline                             │  │
│  │ └─ Failure Recovery Engine                           │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │ NATS Federation Bus                                   │  │
│  │ ├─ Guild, Merlin, Library heartbeats                 │  │
│  │ ├─ Mutation proposals & ACKs                         │  │
│  │ ├─ Trace events                                       │  │
│  │ └─ Maelstrom telemetry                               │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │ Persistence Layer                                     │  │
│  │ ├─ Event Log (RocksDB)                               │  │
│  │ ├─ Checkpoints (compressed snapshots)                │  │
│  │ ├─ Relics (GGUF models)                              │  │
│  │ └─ Configuration (encrypted vault)                   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Testing Strategy

### Unit Tests (50% of effort)
- All component logic isolated
- Mock external dependencies (NATS, RocksDB)
- Target coverage: >90%

### Integration Tests (30% of effort)
- 3-node federation (AAS + Guild + Merlin)
- Real RocksDB, real NATS
- Full mutation lifecycle
- Distributed tracing end-to-end

### Chaos Tests (20% of effort)
- Network partitions
- Byzantine failures (corrupt messages)
- Resource exhaustion
- Clock skew

### Performance Tests
- Mutation throughput: 100+ mutations/sec
- Trace latency: <10ms tracing overhead
- Log replication: <100ms to quorum

---

## Dependencies & Versions

```toml
# Cargo.toml additions
rocksdb = "0.21"
async-raft = "0.9"
nats = "0.26"
opentelemetry = "0.22"
opentelemetry-otlp = "0.15"
tonic = "0.11"
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Rollout Strategy

### Week 1-2: Infrastructure
- Merge MCP bridge, event log, tracing
- All hidden behind feature flags
- No impact on existing code
- Tests must pass

### Week 3-4: Consensus
- Merge Raft consensus
- Initially used only for test mutations
- Fallback to direct execution if Raft unavailable
- Gradual rollout to real mutations

### Week 5-6: Validation & Learning
- Critic loop becomes optional validator
- Distillation pipeline runs in background
- Failures logged but not blocking

### Week 7-8: Recovery
- Recovery engine monitors silently
- Auto-recovery only for low-severity cascades
- All human interactions logged
- Veto capability always available

---

## Success Criteria

**Functional:**
- ✅ MCP bridge: 10K messages/sec throughput
- ✅ Event log: Full federation state recoverable
- ✅ Tracing: 100% of mutations traced with <10ms overhead
- ✅ Consensus: Quorum-based mutations atomic
- ✅ Critic loop: <5% false positive rate
- ✅ Distillation: Knowledge loss <5%
- ✅ Recovery: <1min cascade detection latency

**Reliability:**
- ✅ All tests pass (including chaos)
- ✅ No data loss on network partition
- ✅ Human veto always respected
- ✅ Automatic recovery auditable

**Performance:**
- ✅ No regression from Phase 5 (356 tests still pass)
- ✅ <100ms mutation commit latency
- ✅ <500MB RocksDB footprint per repo

---

## Risk Mitigations

| Risk | Mitigation |
|------|-----------|
| Raft consensus breaks | Feature flagged; fallback to direct execution |
| Event log corruption | Periodic checksums; restore from NATS replicas |
| Distributed tracing overhead | Sampling enabled; can be disabled in production |
| Distillation creates bad models | Validation tests before relic update; rollback capability |
| Cascading failures destroy state | Multiple checkpoint intervals; human veto required |

---

## Next Steps (After Phase 6)

- **Phase 7:** Advanced Analytics (ML-powered insights, SLA forecasting)
- **Phase 8:** Multi-Hive Federation (cross-site replication)
- **Phase 9:** Autonomous Optimization (self-tuning parameters)
- **Phase 10:** Human-in-the-Loop Learning (specialist expertise capture)

---

## Glossary

- **MCP** - Model Context Protocol (AAS ↔ Aaroneous communication)
- **Raft** - Consensus algorithm for distributed state machines
- **RocksDB** - Embedded key-value store for event log
- **OpenTelemetry** - Observability standard (tracing, metrics)
- **Relic** - Specialized GGUF model trained on distilled knowledge
- **SOP** - Standard Operating Procedure (extracted pattern)
- **Critic Loop** - Validation + repair orchestration (Phase 5)
- **Cascade** - Propagating failure across multiple domains
- **Checkpoint** - Point-in-time federation state snapshot
- **Leasing** - Mutual exclusion for control plane mutations

---

**Phase 6 Timeline:** 4-6 weeks  
**Estimated Lines of Code:** 6,000+ Rust + 3,000+ tests  
**Current v3.0 Status:** 15,000+ lines, 356 tests, production-ready  
**Target Post-Phase 6:** 21,000+ lines, 450+ tests, federation-ready
