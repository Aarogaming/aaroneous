// src/lib.rs

pub mod orchestration_daemon;
pub mod action_executor;
pub mod decision_engine;
pub mod intelligence;
pub mod metadata_ingestor;
pub mod duty_cycle;
pub mod subsystem_health;
pub mod normalization_pipeline;
pub mod github_poller;
pub mod batch_runner;

pub use orchestration_daemon::{DaemonState, OrchestrationDaemon, OrchestrationDaemonConfig};
pub use action_executor::{ActionExecutor, ActionResult, ExecutionStats};
pub use decision_engine::{AutonomousDecisionEngine, DecisionTask, ExecutionOutcome, TaskEvaluation};
pub use intelligence::{IntelligenceEngine, LLMConfig, ProviderType};
pub use metadata_ingestor::{MetadataAnalysis, MetadataEvent, MetadataIngestor, MetadataIngestorConfig};
pub use duty_cycle::ExecutionPhase;
pub use subsystem_health::SubsystemHealthReport;
pub use normalization_pipeline::NormalizationPipeline;
pub use github_poller::{GitHubPoller, JobStatus, PollerStats};
