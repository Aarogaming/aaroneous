// src/lib.rs

pub mod action_executor;
pub mod ast_transformer;
pub mod batch_runner;
pub mod decision_engine;
pub mod domain_classifier;
pub mod duty_cycle;
pub mod github_poller;
pub mod grafter;
pub mod intelligence;
pub mod metadata_ingestor;
pub mod normalization_pipeline;
pub mod orchestration_daemon;
pub mod subsystem_health;

pub use action_executor::{ActionExecutor, ActionResult, ExecutionStats};
pub use ast_transformer::{AmbientAstRewriter, RewriteSummary};
pub use decision_engine::{
    AutonomousDecisionEngine, DecisionTask, ExecutionOutcome, TaskEvaluation,
};
pub use domain_classifier::{
    AmbientRiskSite, ClassifierConfig, Domain, DomainClassifier, IngestionReport, ItemKind,
    PublicItem,
};
pub use duty_cycle::ExecutionPhase;
pub use github_poller::{GitHubPoller, JobStatus, PollerStats};
pub use grafter::{GraftReport, graft_module, graft_module_to_crate};
pub use intelligence::{IntelligenceEngine, LLMConfig, ProviderType};
pub use metadata_ingestor::{
    MetadataAnalysis, MetadataEvent, MetadataIngestor, MetadataIngestorConfig,
};
pub use normalization_pipeline::NormalizationPipeline;
pub use orchestration_daemon::{DaemonState, OrchestrationDaemon, OrchestrationDaemonConfig};
pub use subsystem_health::SubsystemHealthReport;
