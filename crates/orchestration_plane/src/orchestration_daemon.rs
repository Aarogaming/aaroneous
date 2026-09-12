use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use crate::action_executor::ActionExecutor;
use crate::metadata_ingestor::MetadataIngestorConfig;

/// Configuration for the orchestration daemon
#[derive(Debug, Clone)]
pub struct OrchestrationDaemonConfig {
    pub ingestor_config: MetadataIngestorConfig,
    pub cycle_interval: Duration,
    pub max_tasks_per_cycle: usize,
}

impl Default for OrchestrationDaemonConfig {
    fn default() -> Self {
        Self {
            ingestor_config: MetadataIngestorConfig::default(),
            cycle_interval: Duration::from_secs(10),
            max_tasks_per_cycle: 5,
        }
    }
}

/// Daemon state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonState {
    Initializing,
    Running,
    Throttled,
    Error(String),
    ShuttingDown,
}

/// Orchestration daemon core state.
pub struct OrchestrationDaemon {
    pub config: OrchestrationDaemonConfig,
    pub executor: ActionExecutor,
    pub state: DaemonState,
    start_time: Instant,
}

impl OrchestrationDaemon {
    pub fn new(config: OrchestrationDaemonConfig, executor: ActionExecutor) -> Self {
        Self {
            config,
            executor,
            state: DaemonState::Initializing,
            start_time: Instant::now(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub async fn process_cycle(&mut self) -> Result<(), String> {
        self.state = DaemonState::Running;
        Ok(())
    }

    pub fn get_state(&self) -> &DaemonState {
        &self.state
    }
}

impl Default for OrchestrationDaemon {
    fn default() -> Self {
        Self::new(OrchestrationDaemonConfig::default(), ActionExecutor::default())
    }
}
