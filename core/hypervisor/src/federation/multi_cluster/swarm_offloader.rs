//! core/hypervisor/src/federation/multi_hive/swarm_offloader.rs
//! Distributed Swarm Micro-Task Offloading Engine.
//! Dynamically routes high-frequency specialist tasks (e.g. AST parsing, SVDD security audits,
//! epigenetic vision gating) to neighboring hive nodes when local node pressure exceeds thresholds.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::federation::multi_hive::live_daemon::LiveP2PDaemon;

/// Standard micro-task domain opcodes for swarm distribution
pub const OPCODE_AST_PARSE: u16 = 0x0700;
pub const OPCODE_TEST_GENERATION: u16 = 0x0701;
pub const OPCODE_SVDD_SECURITY_AUDIT: u16 = 0x0702;
pub const OPCODE_EPIGENETIC_GATING: u16 = 0x0703;
pub const OPCODE_GENERIC_COMPUTE: u16 = 0x07FF;

/// Load-adaptive backpressure metrics across queue depth, thermals, and VRAM
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BackpressureMetrics {
    /// Current queue depth backpressure (0.0 to 100.0%)
    pub queue_depth_pct: f32,
    /// Thermal throttling / temperature load (0.0 to 100.0%)
    pub thermal_pct: f32,
    /// GPU / Neural accelerator VRAM allocation pressure (0.0 to 100.0%)
    pub vram_pct: f32,
    /// CPU utilization pressure (0.0 to 100.0%)
    pub cpu_pct: f32,
}

impl Default for BackpressureMetrics {
    fn default() -> Self {
        Self {
            queue_depth_pct: 0.0,
            thermal_pct: 0.0,
            vram_pct: 0.0,
            cpu_pct: 0.0,
        }
    }
}

impl BackpressureMetrics {
    pub fn new(queue_depth_pct: f32, thermal_pct: f32, vram_pct: f32, cpu_pct: f32) -> Self {
        Self {
            queue_depth_pct: queue_depth_pct.clamp(0.0, 100.0),
            thermal_pct: thermal_pct.clamp(0.0, 100.0),
            vram_pct: vram_pct.clamp(0.0, 100.0),
            cpu_pct: cpu_pct.clamp(0.0, 100.0),
        }
    }

    /// Calculates composite peak backpressure across all hardware and queue dimensions
    pub fn composite_pressure(&self) -> f32 {
        self.queue_depth_pct
            .max(self.thermal_pct)
            .max(self.vram_pct)
            .max(self.cpu_pct)
    }
}

/// Micro-task definition for swarm distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub task_id: String,
    pub domain_opcode: u16,
    pub input_payload: Vec<u8>,
    pub priority: u8,
}

impl SwarmTask {
    pub fn new(
        task_id: impl Into<String>,
        domain_opcode: u16,
        input_payload: Vec<u8>,
        priority: u8,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            domain_opcode,
            input_payload,
            priority,
        }
    }

    pub fn new_ast_parse(task_id: impl Into<String>, ast_payload: Vec<u8>, priority: u8) -> Self {
        Self::new(task_id, OPCODE_AST_PARSE, ast_payload, priority)
    }

    pub fn new_test_generation(
        task_id: impl Into<String>,
        test_spec: Vec<u8>,
        priority: u8,
    ) -> Self {
        Self::new(task_id, OPCODE_TEST_GENERATION, test_spec, priority)
    }

    pub fn new_security_audit(task_id: impl Into<String>, payload: Vec<u8>, priority: u8) -> Self {
        Self::new(task_id, OPCODE_SVDD_SECURITY_AUDIT, payload, priority)
    }

    /// Decoupled constructor from orchestrator TaskMetadata
    pub fn from_orchestrator(
        task: &orchestrator::TaskMetadata,
        domain_opcode: u16,
        input_payload: Vec<u8>,
    ) -> Self {
        let priority_val = match task.priority_tier {
            orchestrator::PriorityTier::Critical => 2,
            orchestrator::PriorityTier::Standard => 1,
            orchestrator::PriorityTier::Background => 0,
        };
        Self {
            task_id: task.task_id.to_string(),
            domain_opcode,
            input_payload,
            priority: priority_val,
        }
    }
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

impl SwarmExecutionOutcome {
    pub fn result_payload(&self) -> &[u8] {
        match self {
            Self::ExecutedLocally { result_payload, .. } => result_payload,
            Self::OffloadedToPeer { result_payload, .. } => result_payload,
        }
    }

    pub fn duration_us(&self) -> u64 {
        match self {
            Self::ExecutedLocally { duration_us, .. } => *duration_us,
            Self::OffloadedToPeer { duration_us, .. } => *duration_us,
        }
    }

    pub fn is_offloaded(&self) -> bool {
        matches!(self, Self::OffloadedToPeer { .. })
    }
}

/// Swarm Task Offloader Engine
pub struct SwarmOffloader {
    pub daemon: Arc<LiveP2PDaemon>,
    pub offload_threshold_pct: f32,
    pub local_pressure_pct: f32,
    pub backpressure: BackpressureMetrics,
    pub tasks_offloaded_count: u64,
    pub tasks_local_count: u64,
    pub fallback_local_count: u64,
}

impl SwarmOffloader {
    pub fn new(daemon: Arc<LiveP2PDaemon>, offload_threshold_pct: f32) -> Self {
        Self {
            daemon,
            offload_threshold_pct,
            local_pressure_pct: 0.0,
            backpressure: BackpressureMetrics::default(),
            tasks_offloaded_count: 0,
            tasks_local_count: 0,
            fallback_local_count: 0,
        }
    }

    /// Updates the simulated or real local system pressure (0.0 to 100.0%)
    pub fn update_pressure(&mut self, pressure_pct: f32) {
        let clamped = pressure_pct.clamp(0.0, 100.0);
        self.local_pressure_pct = clamped;
        self.backpressure.queue_depth_pct = clamped;
    }

    /// Updates structured backpressure metrics across queue depth, thermals, and VRAM
    pub fn update_backpressure(&mut self, metrics: BackpressureMetrics) {
        self.backpressure = metrics;
        self.local_pressure_pct = metrics.composite_pressure().clamp(0.0, 100.0);
    }

    /// Evaluates whether offload conditions are satisfied
    pub fn should_offload(&self) -> bool {
        self.local_pressure_pct >= self.offload_threshold_pct
            && self.daemon.connected_peer_count() > 0
    }

    /// Executes micro-task locally on this node
    pub async fn execute_locally(&mut self, task: &SwarmTask) -> Result<(Vec<u8>, u64)> {
        let start = std::time::Instant::now();
        let res = LiveP2PDaemon::execute_task_payload(task.domain_opcode, &task.input_payload);
        let duration_us = start.elapsed().as_micros() as u64;
        self.tasks_local_count += 1;
        Ok((res, duration_us))
    }

    /// Evaluates local pressure and either executes locally or offloads over TCP with work-stealing fallback
    pub async fn dispatch_task(&mut self, task: SwarmTask) -> Result<SwarmExecutionOutcome> {
        if self.should_offload() {
            info!(
                target: "federation::swarm",
                task_id = %task.task_id,
                local_pressure = %self.local_pressure_pct,
                threshold = %self.offload_threshold_pct,
                "⚡ High local pressure: Offloading micro-task to swarm peer"
            );

            match self
                .daemon
                .offload_task_to_peer(task.domain_opcode, task.input_payload.clone())
                .await
            {
                Ok((res, duration_us, peer_id)) => {
                    self.tasks_offloaded_count += 1;
                    Ok(SwarmExecutionOutcome::OffloadedToPeer {
                        peer_node_id: peer_id,
                        duration_us,
                        result_payload: res,
                    })
                }
                Err(err) => {
                    warn!(
                        target: "federation::swarm",
                        task_id = %task.task_id,
                        error = %err,
                        "⚠️ Swarm offload failed or peer went offline. Work-stealing fallback to local execution."
                    );
                    self.fallback_local_count += 1;
                    let (res, duration_us) = self.execute_locally(&task).await?;
                    Ok(SwarmExecutionOutcome::ExecutedLocally {
                        duration_us,
                        result_payload: res,
                    })
                }
            }
        } else {
            let (res, duration_us) = self.execute_locally(&task).await?;
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
            domain_opcode: OPCODE_AST_PARSE,
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

    #[test]
    fn test_backpressure_composite_threshold_triggers() {
        let daemon = Arc::new(LiveP2PDaemon::new(LiveP2PConfig::default()));
        let mut offloader = SwarmOffloader::new(daemon.clone(), 80.0);

        // All low
        let low_bp = BackpressureMetrics::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(low_bp.composite_pressure(), 40.0);
        offloader.update_backpressure(low_bp);
        assert!(!offloader.should_offload());

        // High VRAM triggers threshold
        let vram_bp = BackpressureMetrics::new(10.0, 20.0, 88.0, 30.0);
        assert_eq!(vram_bp.composite_pressure(), 88.0);
        offloader.update_backpressure(vram_bp);
        // Even if pressure is high, without connected peers it shouldn't offload
        assert_eq!(daemon.connected_peer_count(), 0);
        assert!(!offloader.should_offload());

        // Thermal trigger
        let thermal_bp = BackpressureMetrics::new(10.0, 95.0, 20.0, 15.0);
        assert_eq!(thermal_bp.composite_pressure(), 95.0);
        offloader.update_backpressure(thermal_bp);
        assert_eq!(offloader.local_pressure_pct, 95.0);
    }

    #[test]
    fn test_orchestrator_task_conversion() {
        let task_id = uuid::Uuid::new_v4();
        let meta =
            orchestrator::TaskMetadata::new(task_id, orchestrator::PriorityTier::Critical, 100);
        let swarm_task = SwarmTask::from_orchestrator(&meta, OPCODE_TEST_GENERATION, vec![1, 2, 3]);

        assert_eq!(swarm_task.task_id, task_id.to_string());
        assert_eq!(swarm_task.domain_opcode, OPCODE_TEST_GENERATION);
        assert_eq!(swarm_task.priority, 2);
        assert_eq!(swarm_task.input_payload, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_peer_offline_work_stealing_fallback() {
        let daemon = Arc::new(LiveP2PDaemon::new(LiveP2PConfig::default()));
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        // Inject a peer that will drop the channel to simulate connection failure
        daemon.inject_mock_peer("offline-peer", "127.0.0.1:9999", 5.0, tx);
        drop(rx); // Drop receiver so transmit or response fails

        let mut offloader = SwarmOffloader::new(daemon.clone(), 80.0);
        offloader.update_pressure(90.0); // High pressure triggers offload attempt

        assert!(offloader.should_offload());

        let task = SwarmTask::new_ast_parse("task_fallback", vec![5, 10, 15], 1);
        let outcome = offloader.dispatch_task(task).await.unwrap();

        // Must seamlessly execute locally via work-stealing fallback without panicking
        match outcome {
            SwarmExecutionOutcome::ExecutedLocally { result_payload, .. } => {
                assert_eq!(result_payload, vec![6, 11, 16]);
            }
            _ => panic!("Expected ExecutedLocally after work-stealing fallback"),
        }

        assert_eq!(offloader.fallback_local_count, 1);
        assert_eq!(offloader.tasks_local_count, 1);
        assert_eq!(offloader.tasks_offloaded_count, 0);
    }

    #[test]
    fn test_swarm_task_serialization_roundtrip() {
        let task = SwarmTask::new_security_audit("audit_task", vec![0x01, 0x02], 2);
        let serialized = serde_json::to_string(&task).unwrap();
        let deserialized: SwarmTask = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.task_id, "audit_task");
        assert_eq!(deserialized.domain_opcode, OPCODE_SVDD_SECURITY_AUDIT);
        assert_eq!(deserialized.input_payload, vec![0x01, 0x02]);
        assert_eq!(deserialized.priority, 2);
    }
}
