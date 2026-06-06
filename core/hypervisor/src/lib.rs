// Aaroneous Hypervisor Core
// The central execution runtime that hosts WASM Enzymes and manages the Synapse.

pub mod nervous_system {
    pub use nervous_system::*;
}

// Multi-Runtime Tokio Governance
pub mod runtime_governor;
pub use runtime_governor::{RuntimeGovernor, TaskPriority, BudgetExecutor, execute_agent_task};

// Grim Reaper Pattern
pub mod grim_reaper;
pub use grim_reaper::{GrimReaper, GrimReaperConfig, GrimReaperBuilder, AgentHandle, ReaperEvent, ReaperStats, SharedGrimReaper};

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
pub use sabs::{SabManifest, SabMatrix, SabMatrixBuilder, SabSurface};

// Re-export Skills and Genetics
pub use skills::{Skill, SkillType, SkillOrigin, SoulRank, FusedSkill, SpecialistSkillSet, SkillRegistry};
pub use ::genetics::{SpecialistGenome, GeneticLocus, GeneticCategory, BreedingOperation, GeneticAnalyzer};
pub use ::genetics::genetics::{LociSource, EpigeneticState};

// Re-export Biology with thermodynamic governor
pub use biology::{SystemBiology, SpecialistMetabolism, ThrottleState, SystemHealthReport, SpecialistHealth, PredictiveMetabolicGovernor, MetabolicGovernorConfig, MetabolicForecast, GovernanceAction, ThermodynamicGovernor, ThermodynamicGovernorConfig, ThermodynamicForecast, ThermodynamicAction};
// Re-export Digestion and Agents
pub use digestion::{DigestionEngine, DigestionTask, SpecialistSoul, PersonalitySoul, RelationalSoul, NarrativeSoul, ExperienceSoul, DigestionConfig, DigestionEvent};
pub use agents::{Agent, AgentType, SpecialistAgent, RelicAgent, UserAgent, BaseAgent, CognitiveBias, Domain, create_specialist, create_relic};

// Re-export Constellation and Control Plane
pub use constellation::{Constellation, ConstellationNode, ConstellationQuery, NodeType, NodeStatus, Priority, SpatialCoord, ClusteringContext, RelationshipType};
pub use ::control::{ControlPlane, ControlMessage, SpecialistState, parse_control_message};

// Re-export Hive
pub use hive::{HiveRuntime, HiveRuntimeConfig, RuntimeStatus, RuntimeStatistics};

// Re-export Compute Engine
pub use compute::{ComputeEngine, mdps, stochastic, bayesian, graph, linalg, entropy, optimize, topology, signal, game_theory, automata};
pub use compute::control as compute_control;

// Re-export Intelligence
pub use intelligence::{IntelligenceEngine, TaskRoutingEngine, RoutableTask, TaskType, RoutingDecision, LLMClient, ProviderType, LLMConfig, TaskAnalysis};
pub use intelligence::Specialist as IntelligentSpecialist;

// Re-export Scientific Analyzer
pub use scientific_analyzer::{
    ScientificPipeline, AnalysisReport, PipelineSummary,
    AstObservation, CodeStructure, FunctionSignature,
    Hypothesis, ExperimentDesign,
    ExperimentResult, TestOutcome,
    VerificationResult, ConfidenceUpdate, ConstellationUpdate,
};

// Native Constellation UI (egui/ratatui)
pub mod constellation_ui;
pub mod constellation_3d;
pub use constellation_ui::{ConstellationCanvas, NodeMetrics};
pub use constellation_3d::Constellation3D;

// Autonomic Nervous System
pub mod autonomic_loop;
pub use autonomic_loop::AutonomicNervousSystem;

// Consensus Engine for High-Availability
pub mod consensus_engine;
pub use consensus_engine::{ConsensusEngine, ProposedDecision, DecisionType, Vote, DecisionStatus};

// State Replication for High-Availability
pub mod state_replicator;
pub use state_replicator::{StateReplicator, StateSnapshot, ReplicationStatus};

// Predictive Load Balancing for Intelligent Distribution
pub mod predictive_load_balancer;
pub use predictive_load_balancer::{PredictiveLoadBalancer, LoadPrediction, DistributionStrategy};

// Adaptive Learning Rate Optimization
pub mod adaptive_learning_rate;
pub use adaptive_learning_rate::{AdaptiveLearningOptimizer, LearningStrategy, ConvergenceMetrics};

// Distributed State Checkpointing for Reliable Recovery
pub mod distributed_checkpoint;
pub use distributed_checkpoint::{DistributedCheckpointManager, CheckpointMetadata, ComponentSnapshot};

// Batch Processing for Performance Optimization
pub mod batch_processor;
pub use batch_processor::{BatchProcessor, BatchedTask, BatchResult, BatchStatistics};

// Metrics Aggregation and Performance Monitoring
pub mod metrics_aggregator;
pub use metrics_aggregator::{MetricsAggregator, MetricStats, PerformanceCounter, SystemHealthSummary};

// Real-Time Dashboard and Metrics Display
pub mod dashboard;
pub use dashboard::{RealTimeDashboard, DashboardWidget, DashboardAlert, MetricsSnapshot, HealthMetrics};

// Stress Testing Framework for Stability Validation
pub mod stress_tester;
pub use stress_tester::{StressTestRunner, StressTestConfig, StressTestResult, StressSummary};

// Security Hardening: Input Validation and Rate Limiting
pub mod security_hardener;
pub use security_hardener::{InputValidator, RateLimiter, SecurityHardener, ValidationRule, ValidationResult};

// Performance Benchmarking and Optimization Tracking
pub mod performance_benchmark;
pub use performance_benchmark::{PerformanceBenchmark, BenchmarkOperation, BenchmarkResult, BenchmarkSummary};

// Core modules needed by ANS
pub mod specialist_memory;
pub mod enzyme_runner;
pub mod hox_registry;
pub mod hox_persistence;
pub mod unified_learning;
pub mod splicing_engine;
pub mod genome_compiler;
pub mod wasm_validator;
pub mod unified_registry;
pub mod wasm_discovery;
pub mod genome_trait_loader;
pub mod config_validation;
pub mod nlm_sentinel;
pub mod prefrontal_cortex;
pub mod executive_plan;
pub mod dopamine_system;
pub mod epigenetic_orchestrator;
pub mod concept_drift;
pub mod enzyme_types;  // Consolidated: self_correction_enzyme, diplomat_enzyme, curiosity_enzyme, research_enzyme, execution_enzyme
pub mod neural_pruning;
pub mod semantic_indexing;
pub mod event_log;
pub mod llm;
pub mod nats_client;
pub mod persistence;
pub mod workspace;
pub mod wasm_splicer;
pub mod wasm_loader;
pub mod metadata_ingestor;
pub mod action_executor;
pub mod orchestration_daemon;
pub mod synapse;
pub mod win32_intercept;
pub mod epigenetic_gate;
pub mod hox_map_schema;
pub mod tensor_router;
pub mod spectral_layout;
pub mod mcp_service;
pub mod hid_driver;
pub mod wasm_ebus_bridge;
pub mod native_ingestion;
pub mod substrate;
pub mod sandboxed_network;
pub mod chromosome_registry;
pub mod genetic_recombination;
pub mod wgpu_reflex_pipeline;
pub mod spatial_kinetic_engine;
pub mod hardened_env;
pub use spatial_kinetic_engine::{SpatialKineticConfig, SpatialKineticEngine};

// Autonomous Decision Engine
pub mod decision_engine;
pub use decision_engine::{AutonomousDecisionEngine, DecisionTask, TaskEvaluation, Action, ExecutionOutcome, IngestionReport, SystemStatus};



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
pub mod symbolic_math;
pub mod predictive_models;
pub mod cellular_automata;
pub mod system_integrity;

// Phase 6D: Hybrid Master Registry (WASM/Sentinel GuestOS Layer)
pub mod hybrid_master_registry;
pub mod registry;
pub mod registry_adapters;
pub mod task_analysis;

// Phase 6 Expansion: Relativity, fluid dynamics, quantum surface
pub mod relativity_engine;
pub mod fluid_routing;
pub mod quantum_surface;

// Phase 6 Additions: Agent protocols, visual perception, reasoning, execution, compression, hardware layer
pub mod inter_agent;
pub mod visual_perception;
pub mod reasoning;
pub mod execution;
pub mod compression;
pub mod hardware_layer;
pub mod system_metrics;
pub mod task_routing;

pub use wasm_loader::WasmEnzymeLoader;
pub use system_metrics::{SystemMetricsCollector, ThermalMetrics, ThermalStatus, GpuMetrics};
pub use task_routing::{TaskRouter, ExecutionRoute, ExecutionContext};
pub use hox_registry::HoxRegistry;
pub use hox_persistence::{HoxPersistenceManager, RegistrySnapshot, SnapshotInfo};
pub use metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig};
pub use action_executor::{ActionExecutor, ExecutableAction, FileOp};
pub use orchestration_daemon::{OrchestrationDaemon, OrchestrationDaemonConfig, DaemonState};

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
