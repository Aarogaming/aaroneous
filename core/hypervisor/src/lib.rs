// Aaroneous Hypervisor Core
// The central execution runtime that hosts WASM Enzymes and manages the Synapse.

pub mod nervous_system {
    pub use nervous_system::*;
}

// Re-export SABs for universal access
pub use sabs::{SabManifest, SabMatrix, SabMatrixBuilder, SabSurface};

// Re-export Skills and Genetics
pub use skills::{Skill, SkillType, SkillOrigin, SoulRank, FusedSkill, SpecialistSkillSet, SkillRegistry};
pub use genetics::{SpecialistGenome, GeneticLocus, GeneticCategory, BreedingOperation, GeneticAnalyzer};

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
pub use intelligence::{IntelligenceEngine, TaskRoutingEngine, RoutableTask, TaskType, Specialist, RoutingDecision, LLMClient, ProviderType, LLMConfig, TaskAnalysis};

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

// Autonomous Decision Engine
pub mod decision_engine;
pub use decision_engine::{AutonomousDecisionEngine, DecisionTask, TaskEvaluation, Action, ExecutionOutcome, IngestionReport, SystemStatus};

// Metadata Ingestor
pub mod metadata_ingestor;
pub use metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig, MetadataEvent, MetadataAnalysis, SystemMetrics};

// Action Executor
pub mod action_executor;
pub use action_executor::{ActionExecutor, ExecutableAction, ActionResult, ExecutionStats, FileOp};

// Orchestration Daemon
pub mod orchestration_daemon;
pub use orchestration_daemon::{OrchestrationDaemon, OrchestrationDaemonConfig, DaemonState, DaemonStatus};

// Native Dashboard (egui/wgpu/ratatui)
pub mod tui_framework;
pub mod dashboard;

pub mod wasm_loader;
pub use wasm_loader::WasmEnzymeLoader;

// Tensor-Based Routing
pub mod tensor_router;
pub use tensor_router::{TensorRouter, RoutingWeights, TaskEmbedding, SpecialistEmbedding, RoutingResult, MultiHeadRouter, RoutingOptimizer};

// Spectral Graph Layout
pub mod spectral_layout;
pub use spectral_layout::{spectral_layout_2d, spectral_layout_3d, build_similarity_edges, compute_modularity};

// Unified Learning Loop
pub mod unified_learning;
pub use unified_learning::{UnifiedLearningLoop, UnifiedLearningConfig, UnifiedSystemState, UnifiedCycleResult, SystemHealthSummary};

// Advanced Intelligence System
pub mod advanced_intelligence;
pub use advanced_intelligence::{AnomalyDetector, Forecaster, AutoScaler, SelfHealingEngine, OptimizationEngine};

/// The Hypervisor manages the lifecycle of all Enzymes (WASM/Python).
pub struct Hypervisor {
    pub synapse: nervous_system::SharedMemorySynapse,
}

impl Hypervisor {
    pub fn new() -> Self {
        Self {
            synapse: nervous_system::SharedMemorySynapse::new("SAB_STORE", 1024 * 1024).unwrap(),
        }
    }
}

pub mod workspace;
pub use workspace::WorkspacePaths;

pub mod enzyme_runner;
pub mod synapse;
pub mod hox_registry;
pub mod splicing_engine;
pub mod nlm_sentinel;
pub mod hox_map_schema;
pub mod research_enzyme;
pub mod executive_plan;
pub mod prefrontal_cortex;
pub mod concept_drift;
pub mod mcp_gateway;
pub mod mcp_bridge;
pub mod event_log;
pub mod self_correction_enzyme;
pub mod wasm_splicer;
pub mod execution_enzyme;
pub mod neural_pruning;
pub mod retina_module;
pub mod compliance_gatekeeper;
pub mod semantic_indexing;
pub mod curiosity_enzyme;
pub mod lora_adapter_vault;
pub mod chromosome_registry;
pub mod epigenetic_orchestrator;
pub mod cognitive_weighting;
pub mod hox_breeding_simulator;
pub mod dopamine_system;
pub mod diplomat_enzyme;
pub mod genetic_recombination;
pub mod simulation_testbed;

// Federation: specialist hive, HTTP API, forge, consensus, multi-hive
pub mod federation;
pub use federation::{
    Specialist, SpecialistId, SpecialistConfig, SpecialistRegistry,
    Sentinel, SentinelConfig,
    Proposal, ProposalId, ProposalStatus,
    Visionary, Omnipresent, Symbiotic, Phygital, Archivist, GenericSpecialist,
    Federation, FederationBuilder, FederationConfig,
    DNABank, DNAEvent,
    Forge, ForgeRecipe, CrystallizationResult,
    HttpStatusServer,
    Intent, IntentPriority,
    Session, SessionManager,
    QuantizationConfig, GPUAccelerationStrategy, BatchConfig,
    HiveCluster, P2PNetwork, FederatedLearningEngine,
    AuditLog, ComplianceMonitor, SecurityConfig, AccessControl,
};

#[cfg(test)]
mod synaptic_test;

#[cfg(test)]
mod rkyv_test;

#[cfg(test)]
mod self_heal_test;

#[cfg(test)]
mod sentinel_test;

#[cfg(test)]
mod integration_tests;
