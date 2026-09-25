#![recursion_limit = "512"]

// Aaroneous Hypervisor Core
// The central execution runtime that hosts WASM Enzymes and manages the SignalBridge.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;
pub extern crate adaptation_plane as evolution;
pub use adaptation_plane;

pub mod error;
pub mod state;
pub mod util;

pub extern crate governance as biology;
pub use governance as system_health;
pub use governance;

pub mod sabs {
    pub use omni::matrix::*;
}

pub mod constellation {
    pub use omni::*;
}

pub mod genetics {
    pub use evolution::genetics::*;
}

pub mod digestion {
    pub use evolution::self_digestion::*;
}

pub mod skills {
    pub use evolution::skills::*;
}

pub mod agents {
    pub use orchestrator::agents::*;
    pub use orchestrator::workspace::*;
}

pub mod control {
    pub use orchestrator::control::*;
}

pub mod hive {
    pub use orchestrator::hive_runtime::*;
}

pub mod intelligence {
    pub use orchestrator::IntelligenceEngine;
    pub use orchestrator::aura_ui::*;
    pub use orchestrator::aura_ui_manifest::*;
    pub use orchestrator::linguistic_transducer::*;
    pub use orchestrator::llm::*;
    pub use orchestrator::mdps_router::*;
}

pub mod scientific_analyzer {
    pub use adaptation_engine::analysis::*;
}

// Multi-Runtime Tokio Governance
pub mod runtime_governor;
pub use runtime_governor::{BudgetExecutor, RuntimeGovernor, TaskPriority, execute_agent_task};

// Compaction Engine Pattern
pub mod compaction_engine;
pub use compaction_engine::{
    AgentHandle, CompactionEngine, CompactionEngineBuilder, CompactionEngineConfig,
    CompactionEvent, ReaperStats, SharedCompactionEngine, SlabCompactionEngine,
};

// GGUF Seeding & Cartridge Compiler
pub mod cartridge_compiler;
pub use cartridge_compiler::{CartridgeCompiler, GgufSeedingConfig, GgufSeedingReport};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DigestionPriority {
    Low,
    Normal,
    High,
}

pub mod assimilation;
pub mod onboarding;

// Re-export SABs for universal access
pub use crate::sabs::{SabManifest, SabMatrix, SabMatrixBuilder, SabSurface};

// Re-export Skills and Genetics / Adaptation
pub use crate::genetics::{
    AdaptationState, AgentProfile, BreedingOperation, LociSource, ProfileAnalyzer, ProfileCategory,
    ProfileLocus,
};
#[allow(deprecated)]
pub use crate::genetics::{
    EpigeneticState, GeneticAnalyzer, GeneticCategory, GeneticLocus, SpecialistGenome,
};
pub use crate::skills::{
    FusedSkill, PersonaRank, Skill, SkillOrigin, SkillRegistry, SkillType, SpecialistSkillSet,
};

// Re-export Biology with thermodynamic governor
pub use biology::{
    GovernanceAction, MetabolicForecast, MetabolicGovernorConfig, PredictiveMetabolicGovernor,
    SpecialistHealth, SpecialistMetabolism, SystemBiology, SystemHealthReport, ThermodynamicAction,
    ThermodynamicForecast, ThermodynamicGovernor, ThermodynamicGovernorConfig, ThrottleState,
};
// Re-export Digestion and Agents
pub use crate::agents::{
    Agent, AgentType, BaseAgent, BaselineReferenceAgent, CognitiveBias, Domain, RelicAgent,
    SpecialistAgent, UserAgent, create_reference_agent, create_relic, create_specialist,
};
pub use crate::digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceProfile,
    NarrativeProfile, PersonalityProfile, RelationalProfile, SpecialistPersona,
};

// Re-export Constellation and Control Plane
pub use crate::constellation::{
    ClusteringContext, Constellation, ConstellationNode, ConstellationQuery,
    GalacticClusteringEngine, GalaxyCluster, LinkType, NodeStatus, NodeType, OmniEngine,
    OmniProtocolBridge, OmniQueryEngine, OmniQueryFilter, Priority, RelationshipType, SpatialCoord,
    SpatialFrustum, StarNode, StarNodeStatus, StarNodeType,
};
pub use crate::control::{ControlMessage, ControlPlane, SpecialistState, parse_control_message};

// Re-export Hive
pub use crate::hive::{HiveRuntime, HiveRuntimeConfig, RuntimeStatistics, RuntimeStatus};

// Re-export Compute Engine
pub use compute::control as compute_control;
pub use compute::{
    ComputeEngine, automata, bayesian, entropy, game_theory, graph, linalg, mdps, optimize, signal,
    stochastic, topology,
};

// Re-export Intelligence
pub use crate::intelligence::Specialist as IntelligentSpecialist;
pub use crate::intelligence::{
    IntelligenceEngine, LLMClient, LLMConfig, ProviderType, RoutableTask, RoutingDecision,
    TaskAnalysis, TaskRoutingEngine, TaskType,
};

// Re-export Scientific Analyzer
pub use crate::scientific_analyzer::{
    AnalysisReport, AstObservation, CodeStructure, ConfidenceUpdate, ConstellationUpdate,
    ExperimentDesign, ExperimentResult, FunctionSignature, Hypothesis, PipelineSummary,
    ScientificPipeline, TestOutcome, VerificationResult,
};

// Runtime Supervisory Loop
pub mod supervisory_loop;
pub use supervisory_loop as autonomic_loop;
#[allow(deprecated)]
pub use supervisory_loop::AutonomicNervousSystem;
pub use supervisory_loop::{AutonomousControlLoop, SupervisoryDaemon};

// Sandboxed Micro-Worker Bytecode Virtual Machine
pub mod micro_vm;
pub use micro_vm::{
    DEFAULT_GAS_LIMIT, DEFAULT_MEMORY_LIMIT, MicroBytecodeVm, REGISTER_COUNT, VmError,
    VmExecutionResult, VmInstruction, VmProgram,
};

// Consensus Engine for High-Availability
pub mod consensus_engine;
pub use consensus_engine::{
    ConsensusEngine, DecisionStatus, DecisionType, DistributedWalEntry, ProposedDecision,
    RaftAppendEntriesRequest, RaftAppendEntriesResponse, RaftClusterState, RaftRole,
    RaftVoteRequest, RaftVoteResponse, Vote,
};

// State Replication for High-Availability
pub mod state_replicator;
pub use state_replicator::{ReplicationStatus, StateReplicator, StateSnapshot};

// Predictive Load Balancing for Intelligent Distribution
pub mod predictive_load_balancer;
pub use predictive_load_balancer::{DistributionStrategy, LoadPrediction, PredictiveLoadBalancer};

// Adaptive Learning Rate Optimization
pub mod adaptive_learning_rate;
pub use adaptive_learning_rate::{AdaptiveLearningOptimizer, ConvergenceMetrics, LearningStrategy};

// Distributed State Checkpointing for Reliable Recovery
pub mod distributed_checkpoint;
pub use distributed_checkpoint::{
    CheckpointMetadata, ComponentSnapshot, DistributedCheckpointManager,
};

// Batch Processing for Performance Optimization
pub mod batch_processor;
pub use batch_processor::{BatchProcessor, BatchResult, BatchStatistics, BatchedTask};

// Metrics Aggregation and Performance Monitoring
pub mod metrics_aggregator;
pub use metrics_aggregator::{
    MetricStats, MetricsAggregator, PerformanceCounter, SystemHealthSummary,
};

// Real-Time Dashboard and Metrics Display
pub mod dashboard;
pub use dashboard::{
    DashboardAlert, DashboardWidget, HealthMetrics, MetricsSnapshot, RealTimeDashboard,
};

// Stress Testing Framework for Stability Validation
pub mod stress_tester;
pub use stress_tester::{StressSummary, StressTestConfig, StressTestResult, StressTestRunner};

// Security Hardening: Input Validation and Rate Limiting
pub mod security_hardener;
pub use security_hardener::{
    InputValidator, RateLimiter, SecurityHardener, ValidationResult, ValidationRule,
};

// Performance Benchmarking and Optimization Tracking
pub mod performance_benchmark;
pub use performance_benchmark::{
    BenchmarkOperation, BenchmarkResult, BenchmarkSummary, PerformanceBenchmark,
};

// Core modules needed by runtime supervisor
pub mod action_executor;
pub mod capability_registry;
pub use capability_registry as node_registry;
pub mod concept_drift;
pub mod config_validation;
pub mod reward_system;
pub use reward_system as dopamine_system;
pub mod worker_runner;
pub use worker_runner as enzyme_runner;
pub use worker_runner::MicroTaskRunner;
pub mod delta_orchestrator;
pub mod worker_types;
pub use worker_types as enzyme_types;
pub mod event_log;
pub mod executive_plan;
pub mod profile_merger;
pub use profile_merger as genetic_recombination;
pub mod profile_compiler;
pub use profile_compiler as genome_compiler;
pub mod hardened_env;
pub mod hid_driver;
pub mod profile_schema;
pub mod trait_loader;
pub use profile_schema as hox_map_schema;
pub mod profile_persistence;
pub use profile_persistence as hox_persistence;
pub mod profile_registry;
pub use capability_registry as chromosome_registry;
pub use profile_registry as hox_registry;
pub mod spatial_delta_gate;
pub use llm_gateway as llm;
pub use llm_gateway::McpGateway;
pub use profile_registry::CapabilitySchemaRegistry;
pub mod intent_orchestrator;
pub mod lora_adapter_vault;
pub mod mcp_service;
pub mod metadata_ingestor;
pub mod native_ingestion;
pub mod nats_client;
pub mod neural_pruning;
pub mod nlm_sentinel;
pub mod orchestration_daemon;
pub mod persistence;
pub use intent_orchestrator as prefrontal_cortex;
pub mod sandboxed_network;
pub mod semantic_indexing;
#[cfg(windows)]
pub mod spatial_kinetic_engine;
pub mod specialist_memory;
pub mod spectral_layout;
pub mod splicing_engine;
pub use splicing_engine::{PluginHotSwapEngine, WasmHotSwapEngine, WasmSplicingEngine};
pub mod signal_bridge;
pub mod substrate;
pub mod interconnect {
    pub use crate::signal_bridge::*;
}
pub use interconnect::{
    InterconnectBus, InterconnectMcpFrame, InterconnectPayload, InterconnectState,
    SpecialistBusDialogue,
};
pub use reward_system::{FeedbackEvent, FeedbackSignalProcessor, RewardSignalProcessor};
pub mod screen_capture;
pub use screen_capture as retina_module;
pub use screen_capture::{
    SharedBusWebIngest, TokenIngestionEngine, WebIngestionEngine, WebSamplerModule,
};
pub mod state_snapshot;
pub use state_snapshot::{
    ConsoleProjection, EngineSnapshot, EngineStatePublisher, GovernorPacing, HudProjection,
    NodeMetrics, SpatialCanvasState, StudioProjection,
};
pub mod capability_broker;
pub mod tensor_router;
pub mod ui_broker;
pub use capability_broker::{
    CapabilityBroker, CapabilityCategory, CapabilityDescriptor, CapabilityExecutionOutcome,
};

#[cfg(feature = "fault_injector")]
pub mod fault_injector;
#[cfg(feature = "fault_injector")]
pub use fault_injector::FaultInjector;
#[cfg(feature = "knowledge_gap")]
pub mod knowledge_gap_detector;
#[cfg(feature = "knowledge_gap")]
pub use knowledge_gap_detector::KnowledgeGapDetector;
pub mod unified_learning;
pub mod unified_registry;
pub mod wgpu_reflex_pipeline;
pub mod win32_intercept;
pub mod workspace;
#[cfg(windows)]
pub use spatial_kinetic_engine::{SpatialKineticConfig, SpatialKineticEngine};

// Autonomous Decision Engine
pub mod decision_engine;
pub use decision_engine::{
    Action, AutonomousDecisionEngine, DecisionTask, ExecutionOutcome, IngestionReport,
    SystemStatus, TaskEvaluation,
};

// Federation: specialist hive, HTTP API, forge, consensus, multi-hive
pub mod federation;

// Phase 6 Expansion: Computational logic systems
pub mod cellular_automata;
pub mod config;
pub mod predictive_models;
pub mod symbolic_math;
pub mod system_integrity;

// Phase 6D: Hybrid Master Registry (WASM/Sentinel GuestOS Layer)
pub mod hybrid_master_registry;
pub mod registry;
pub mod registry_adapters;
pub mod task_analysis;

// Resilience patterns: circuit breakers, retry policies, recovery
pub mod resilience;
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState, RetryError,
    RetryPolicy, with_circuit_breaker, with_retry,
};

// Structured logging facade: single init point, idempotent
pub mod logging;
pub use logging::{ShellType, init_logging, init_shell_logging};

/// Run internal health check for system startup
pub fn run_health_checks() -> bool {
    let mut success = true;

    // Check Adaptation Engine (ArtifactRegistry) initialization path
    if persistence::PersistenceManager::new(":memory:").is_err() {
        tracing::error!("HealthCheck: ArtifactRegistry (Adaptation Engine) failed initialization");
        success = false;
    } else {
        tracing::info!(
            "HealthCheck: ArtifactRegistry (Adaptation Engine) initialized successfully"
        );
    }

    // Check Reasoning (Synthesizer) component availability
    tracing::info!("HealthCheck: Reasoning Engine (Synthesizer) status: Nominal");

    // Check Constellation (Omni) registry health
    let ws = paths::WorkspacePaths::from_config(paths::WorkspacePathsConfig::default());
    if ws.registry().exists() {
        tracing::info!("HealthCheck: Constellation Registry (Omni) status: Nominal");
    } else {
        tracing::warn!(
            "HealthCheck: Constellation Registry (Omni) path missing, registry will be re-initialized"
        );
    }

    success
}

// Per-key rate limiting (token bucket)
pub mod rate_limit;
pub use rate_limit::{
    TokenBucketConfig, TokenBucketDecision, TokenBucketLimiter, key_from_request,
};

// Lightweight input validation helpers for HTTP bodies and tasks
pub mod input_validation;
pub use input_validation::{
    ValidationError, validate_bytes, validate_enum, validate_identifier, validate_optional_string,
    validate_range, validate_string,
};

// Phase 6 Expansion: Relativity, fluid dynamics, quantum surface
pub mod fluid_routing;
pub mod quantum_surface;
pub mod relativity_engine;

// Phase 6 Additions: Agent protocols, visual perception, reasoning, execution, compression, hardware layer
pub mod compression;
pub mod execution;
pub mod hardware_layer;
pub mod inter_agent;
pub mod reasoning;
pub mod system_metrics;
pub mod task_routing;
pub mod visual_perception;

#[cfg(any(feature = "testing", feature = "simulation", test))]
pub mod task_worker;

#[cfg(any(feature = "testing", feature = "simulation", test))]
pub use task_worker::ExecutionEnzyme;

pub use action_executor::{ActionExecutor, ExecutableAction, FileOp};
pub use lora_adapter_vault::{LiveLoraAdapter, LoraAdapterVault};
pub use metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig};
pub use orchestration_daemon::{DaemonState, OrchestrationDaemon, OrchestrationDaemonConfig};
pub use profile_persistence::{HoxPersistenceManager, RegistrySnapshot, SnapshotInfo};
pub use profile_registry::ProfileRegistry;
pub use task_routing::{ExecutionContext, ExecutionRoute, TaskRouter};

/// Core hypervisor bootstrap loop connecting RuntimeGovernor, OrchestrationDaemon,
/// and the central lock-free Disruptor event bus.
pub async fn run_hypervisor(
    config: OrchestrationDaemonConfig,
    shutdown_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    tracing::info!("Initializing Aaroneous Hypervisor Microkernel Component Block...");
    let _governor = runtime_governor::RuntimeGovernor::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize RuntimeGovernor: {}", e))?;

    let mut daemon = OrchestrationDaemon::new(config)
        .map_err(|e| anyhow::anyhow!("Failed to initialize OrchestrationDaemon: {}", e))?;
    tracing::info!("OrchestrationDaemon mounted. Entering sovereign execution loop.");

    while !shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
        let stats = daemon
            .step()
            .await
            .map_err(|e| anyhow::anyhow!("OrchestrationDaemon step error: {}", e))?;
        if stats.total_executions > 0 {
            tracing::debug!(
                "Hypervisor scan completed: {} actions executed cleanly",
                stats.success_count
            );
        }
        // Yield to allow concurrent background tasks and I/O polling
        tokio::task::yield_now().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    tracing::info!("Hypervisor shutdown signal received. Executing graceful teardown.");
    daemon.shutdown();
    Ok(())
}

#[cfg(test)]
mod bus_test;

#[cfg(test)]
mod rkyv_test;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod spatial_kinetic_test;

#[cfg(test)]
mod registry_sync_tests;

#[cfg(test)]
mod phase_6d_integration_tests;

#[cfg(test)]
mod end_to_end_tests;

#[cfg(test)]
mod phase_5_integration_tests;

#[cfg(test)]
mod phase1_integration_tests;

#[cfg(test)]
mod phase2_integration_tests;

#[cfg(test)]
mod core_integration_tests;

#[cfg(test)]
mod graceful_degradation_tests;
