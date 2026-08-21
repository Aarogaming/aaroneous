//! core/hypervisor/src/federation/multi_hive/swarm_offloader.rs
//! Distributed Swarm Micro-Task Offloading Engine.
//! Dynamically routes high-frequency specialist tasks (e.g. AST parsing, SVDD security audits,
//! epigenetic vision gating) to neighboring hive nodes when local node pressure exceeds thresholds.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::federation::multi_hive::live_daemon::LiveP2PDaemon;

/// Micro-task definition for swarm distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub task_id: String,
    pub domain_opcode: u16,
    pub input_payload: Vec<u8>,
    pub priority: u8,
}

/// Routing decision outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmExecutionOutcome {
    ExecutedLocally {
        duration_us: u64,
        result_payload: Vec<u8>,
    },
    OffloadedToPeer {
        peer_node_id: String,
        duration_us: u64,
        result_payload: Vec<u8>,
    },
}

/// Swarm Task Offloader Engine
pub struct SwarmOffloader {
    pub daemon: Arc<LiveP2PDaemon>,
    pub offload_threshold_pct: f32,
    pub local_pressure_pct: f32,
    pub tasks_offloaded_count: u64,
    pub tasks_local_count: u64,
}

impl SwarmOffloader {
    pub fn new(daemon: Arc<LiveP2PDaemon>, offload_threshold_pct: f32) -> Self {
        Self {
            daemon,
            offload_threshold_pct,
            local_pressure_pct: 0.0,
            tasks_offloaded_count: 0,
            tasks_local_count: 0,
        }
    }

    /// Updates the simulated or real local system pressure (0.0 to 100.0%)
    pub fn update_pressure(&mut self, pressure_pct: f32) {
        self.local_pressure_pct = pressure_pct.clamp(0.0, 100.0);
    }

    /// Evaluates local pressure and either executes locally or offloads over TCP
    pub async fn dispatch_task(&mut self, task: SwarmTask) -> Result<SwarmExecutionOutcome> {
        let should_offload = self.local_pressure_pct >= self.offload_threshold_pct 
            && self.daemon.connected_peer_count() > 0;

        if should_offload {
            info!(
                target: "federation::swarm",
                task_id = %task.task_id,
                local_pressure = %self.local_pressure_pct,
                threshold = %self.offload_threshold_pct,
                "⚡ High local pressure: Offloading micro-task to swarm peer"
            );

            let (res, duration_us, peer_id) = self.daemon.offload_task_to_peer(
                task.domain_opcode,
                task.input_payload,
            ).await?;

            self.tasks_offloaded_count += 1;
            Ok(SwarmExecutionOutcome::OffloadedToPeer {
                peer_node_id: peer_id,
                duration_us,
                result_payload: res,
            })
        } else {
            // Local execution
            let start = std::time::Instant::now();
            let mut res = task.input_payload;
            if res.is_empty() {
                res = vec![0x55; 32];
            } else {
                for b in res.iter_mut() {
                    *b = b.wrapping_add(1);
                }
            }
            let duration_us = start.elapsed().as_micros() as u64;
            self.tasks_local_count += 1;

            Ok(SwarmExecutionOutcome::ExecutedLocally {
                duration_us,
                result_payload: res,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::multi_hive::live_daemon::LiveP2PConfig;

    #[tokio::test]
    async fn test_swarm_offloading_pressure_routing() {
        let daemon = Arc::new(LiveP2PDaemon::new(LiveP2PConfig::default()));
        let mut offloader = SwarmOffloader::new(daemon, 80.0);

        // Low pressure -> local execution
        offloader.update_pressure(45.0);
        let task = SwarmTask {
            task_id: "task_01".into(),
            domain_opcode: 0x0700,
            input_payload: vec![10, 20, 30],
            priority: 1,
        };

        let outcome = offloader.dispatch_task(task).await.unwrap();
        match outcome {
            SwarmExecutionOutcome::ExecutedLocally { result_payload, .. } => {
                assert_eq!(result_payload, vec![11, 21, 31]);
            }
            _ => panic!("Expected local execution under low pressure"),
        }
        assert_eq!(offloader.tasks_local_count, 1);
    }
}
