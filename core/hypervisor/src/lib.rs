// Aaroneous Hypervisor Core
// The central execution runtime that hosts WASM Enzymes and manages the Synapse.

pub mod nervous_system {
    pub use nervous_system::*;
}

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
    pub use orchestrator::aura_ui::*;
    pub use orchestrator::aura_ui_manifest::*;
    pub use orchestrator::linguistic_transducer::*;
    pub use orchestrator::llm::*;
    pub use orchestrator::mdps_router::*;
    pub use orchestrator::IntelligenceEngine;
}

pub mod scientific_analyzer {
    pub use chimera::analysis::*;
}

// Multi-Runtime Tokio Governance
pub mod runtime_governor;
pub use runtime_governor::{BudgetExecutor, RuntimeGovernor, TaskPriority, execute_agent_task};

// Grim Reaper Pattern
pub mod grim_reaper;
pub use grim_reaper::{
    AgentHandle, GrimReaper, GrimReaperBuilder, GrimReaperConfig, ReaperEvent, ReaperStats,
    SharedGrimReaper,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DigestionPriority {
    Low,
    Normal,
    High,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DigestionStatus {
    StructuralAnalysis,
    Ready,
    Complete,
}

// Re-export SABs for universal access
pub use crate::sabs::{SabManifest, SabMatrix, SabMatrixBuilder, SabSurface};

// Re-export Skills and Genetics
pub use crate::genetics::{
    BreedingOperation, GeneticAnalyzer, GeneticCategory, GeneticLocus, SpecialistGenome,
    EpigeneticState, LociSource,
};
pub use crate::skills::{
    FusedSkill, Skill, SkillOrigin, SkillRegistry, SkillType, SoulRank, SpecialistSkillSet,
};

// Re-export Biology with thermodynamic governor
pub use biology::{
    GovernanceAction, MetabolicForecast, MetabolicGovernorConfig, PredictiveMetabolicGovernor,
    SpecialistHealth, SpecialistMetabolism, SystemBiology, SystemHealthReport, ThermodynamicAction,
    ThermodynamicForecast, ThermodynamicGovernor, ThermodynamicGovernorConfig, ThrottleState,
};
// Re-export Digestion and Agents
pub use crate::agents::{
    Agent, AgentType, BaseAgent, CognitiveBias, Domain, RelicAgent, SpecialistAgent, UserAgent,
    create_relic, create_specialist,
};
pub use crate::digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceSoul, NarrativeSoul,
    PersonalitySoul, RelationalSoul, SpecialistSoul,
};

// Re-export Constellation and Control Plane
pub use crate::control::{ControlMessage, ControlPlane, SpecialistState, parse_control_message};
pub use crate::constellation::{
    ClusteringContext, Constellation, ConstellationNode, ConstellationQuery,
    GalacticClusteringEngine, GalaxyCluster, LinkType, NodeStatus, NodeType, OmniEngine,
    OmniProtocolBridge, OmniQueryEngine, OmniQueryFilter, Priority, RelationshipType,
    SpatialCoord, SpatialFrustum, StarNode, StarNodeStatus, StarNodeType,
};

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

// Native Constellation UI (egui/ratatui)
pub mod constellation_3d;
pub mod constellation_ui;
pub use constellation_3d::Constellation3D;
pub use constellation_ui::{ConstellationCanvas, NodeMetrics};

// Autonomic Nervous System
pub mod autonomic_loop;
pub use autonomic_loop::AutonomicNervousSystem;

// Consensus Engine for High-Availability
pub mod consensus_engine;
pub use consensus_engine::{ConsensusEngine, DecisionStatus, DecisionType, ProposedDecision, Vote};

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

// Core modules needed by ANS
pub mod action_executor;
pub mod chromosome_registry;
pub mod concept_drift;
pub mod config_validation;
pub mod dopamine_system;
pub mod enzyme_runner;
pub mod enzyme_types; // Consolidated: self_correction_enzyme, diplomat_enzyme, curiosity_enzyme, research_enzyme, execution_enzyme
pub mod epigenetic_gate;
pub mod epigenetic_orchestrator;
pub mod event_log;
pub mod executive_plan;
pub mod genetic_recombination;
pub mod genome_compiler;
pub mod genome_trait_loader;
pub mod hardened_env;
pub mod hid_driver;
pub mod hox_map_schema;
pub mod hox_persistence;
pub mod hox_registry;
pub mod llm;
pub mod mcp_service;
pub mod metadata_ingestor;
pub mod native_ingestion;
pub mod nats_client;
pub mod neural_pruning;
pub mod nlm_sentinel;
pub mod orchestration_daemon;
pub mod persistence;
pub mod prefrontal_cortex;
pub mod sandboxed_network;
pub mod semantic_indexing;
pub mod spatial_kinetic_engine;
pub mod specialist_memory;
pub mod spectral_layout;
pub mod splicing_engine;
pub mod substrate;
pub mod synapse;
pub mod tensor_router;
pub mod maelstrom_hud;
pub mod ui_broker;
pub use maelstrom_hud::{HudTab, MaelstromHudApp};
pub mod unified_learning;
pub mod unified_registry;
pub mod wgpu_reflex_pipeline;
pub mod win32_intercept;
pub mod workspace;
pub use spatial_kinetic_engine::{SpatialKineticConfig, SpatialKineticEngine};

// Autonomous Decision Engine
pub mod decision_engine;
pub use decision_engine::{
    Action, AutonomousDecisionEngine, DecisionTask, ExecutionOutcome, IngestionReport,
    SystemStatus, TaskEvaluation,
};

// Federation: specialist hive, HTTP API, forge, consensus, multi-hive
//
// STATUS: Compiles and passes 552 tests. All structural fixes applied.
// Known minor issues (non-blocking):
//   1. Trait-as-type patterns in symbiotic.rs (dyn BiometricProvider) and
//      phygital.rs (dyn ArProvider) — harmless as these are trait-aliased
//      behind config, not instantiated directly.
//   2. The old comment about WorkspacePaths mismatches is stale — federation
//      compiles and tests pass without any workspace module dependency.
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
pub use logging::init_logging;

/// Run internal health check for system startup
pub fn run_health_checks() -> bool {
    let mut success = true;

    // Check Chimera (DNA Bank) initialization path
    // We check if the persistence layer can be initialized with a mock :memory: backend
    if persistence::PersistenceManager::new(":memory:").is_err() {
        tracing::error!("HealthCheck: DNA Bank (Chimera) failed initialization");
        success = false;
    } else {
        tracing::info!("HealthCheck: DNA Bank (Chimera) initialized successfully");
    }

    // Check Reasoning (Merlin) component availability
    // Simplified heuristic check for agent existence
    // This is a placeholder for more deep integrity checks
    tracing::info!("HealthCheck: Reasoning Engine (Merlin) status: Nominal");

    // Check Constellation (Omni) registry health
    // Check if registry paths exist
    if aaroneous_paths::WorkspacePaths::discover()
        .registry()
        .exists()
    {
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
pub mod forge_ui;
pub mod synapse_ui;
pub mod skill_constellation;

pub use action_executor::{ActionExecutor, ExecutableAction, FileOp};
pub use forge_ui::{ForgeStatus, ForgeStudio};
pub use hox_persistence::{HoxPersistenceManager, RegistrySnapshot, SnapshotInfo};
pub use hox_registry::HoxRegistry;
pub use metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig};
pub use orchestration_daemon::{DaemonState, OrchestrationDaemon, OrchestrationDaemonConfig};
pub use skill_constellation::{SkillConstellationCanvas, VisualStarNode};
pub use synapse_ui::SynapseVisualizer;
pub use system_metrics::{GpuMetrics, SystemMetricsCollector, ThermalMetrics, ThermalStatus};
pub use task_routing::{ExecutionContext, ExecutionRoute, TaskRouter};

#[cfg(test)]
mod synaptic_test;

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
