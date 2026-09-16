use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::action_executor::ActionExecutor;
use crate::metadata_ingestor::MetadataIngestorConfig;
use ipc_bus::universal_protocol::{
    AssimilationPhase, AssimilationRecord, UniversalServerBroadcast,
};

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
            ingestor_config: MetadataIngestorConfig,
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
    pub assimilation_count: u64,
    start_time: Instant,
}

impl OrchestrationDaemon {
    pub fn new(config: OrchestrationDaemonConfig, executor: ActionExecutor) -> Self {
        Self {
            config,
            executor,
            state: DaemonState::Initializing,
            assimilation_count: 0,
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

    /// Reactive event handler for zero-copy binary assimilation frames over ipc_bus.
    /// Ingests a frame, advances the state machine deterministically, and produces a server broadcast.
    pub fn process_assimilation_frame(
        &mut self,
        bytes: &[u8],
    ) -> Result<UniversalServerBroadcast, String> {
        if bytes.len() < core::mem::size_of::<AssimilationRecord>() {
            return Err("Binary frame smaller than AssimilationRecord".to_string());
        }

        let record = bytemuck::try_from_bytes::<AssimilationRecord>(
            &bytes[..core::mem::size_of::<AssimilationRecord>()],
        )
        .map_err(|e| format!("Zero-copy cast failed: {e}"))?;

        let mut transitioned = *record;
        match transitioned.phase {
            0 => transitioned.phase = AssimilationPhase::Quarantined as u32,
            1 => transitioned.phase = AssimilationPhase::Auditing as u32,
            2 => transitioned.phase = AssimilationPhase::Synthesizing as u32,
            3 => transitioned.phase = AssimilationPhase::Certifying as u32,
            4 => transitioned.phase = AssimilationPhase::Committed as u32,
            _ => {}
        }

        self.assimilation_count += 1;
        Ok(transitioned.to_broadcast(self.assimilation_count, 0))
    }

    pub fn get_state(&self) -> &DaemonState {
        &self.state
    }
}

impl Default for OrchestrationDaemon {
    fn default() -> Self {
        Self::new(OrchestrationDaemonConfig::default(), ActionExecutor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_bus::universal_protocol::UcpBroadcastType;

    #[test]
    fn test_orchestration_plane_daemon_assimilation_frame() {
        let mut daemon = OrchestrationDaemon::default();
        let record = AssimilationRecord::new([9u8; 16], 1000);
        let bytes = bytemuck::bytes_of(&record);

        let broadcast = daemon
            .process_assimilation_frame(bytes)
            .expect("frame process succeeds");
        assert_eq!(
            broadcast.broadcast_type,
            UcpBroadcastType::AssimilationState as u32
        );
        assert_eq!(broadcast.sequence, 1);
        assert_eq!(daemon.assimilation_count, 1);
    }
}
