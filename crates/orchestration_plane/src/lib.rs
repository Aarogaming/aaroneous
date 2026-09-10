// src/lib.rs

pub mod orchestration_daemon;
pub mod assimilation_orchestrator;

pub use orchestration_daemon::{DaemonState, OrchestrationDaemon, OrchestrationDaemonConfig};
pub use assimilation_orchestrator::AssimilationOrchestrator;
