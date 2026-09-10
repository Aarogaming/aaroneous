// src/lib.rs

pub mod orchestration_daemon;
pub mod action_executor;
pub mod decision_engine;
pub mod intelligence;
pub mod metadata_ingestor;
pub mod thermodynamics;
pub mod biology;
pub mod assimilation_orchestrator;

pub use orchestration_daemon::{DaemonState, OrchestrationDaemon, OrchestrationDaemonConfig};
pub use action_executor::{ActionExecutor, ActionResult, ExecutionStats};
pub use decision_engine::{AutonomousDecisionEngine, DecisionTask, ExecutionOutcome, TaskEvaluation};
pub use intelligence::{IntelligenceEngine, LLMConfig, ProviderType};
pub use metadata_ingestor::{MetadataAnalysis, MetadataEvent, MetadataIngestor, MetadataIngestorConfig};
pub use thermodynamics::SystemPhase;
pub use biology::SystemHealthReport;
pub use assimilation_orchestrator::AssimilationOrchestrator;
