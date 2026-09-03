//! core/hypervisor/src/federation/fleet_scheduler.rs
//! Multi-Node Fleet Federation & Work-Stealing Distributed Scheduler.
//! Balances compute across sovereign nodes via Iroh/QUIC mesh, offloading
//! sub-graphs (`NativeComputationalGraph`) to idle peers when local thresholds are exceeded.

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::federation::p2p::types::{
    CartridgeLoraDeltaSync, ClusterNodeHardwareSpec, P2pNodeId, SyncMessage, SyncMessageKind,
    WorkResult, WorkStealRequest, WorkStealResponse,
};
use si_ir::NativeComputationalGraph;

/// Metric representation of a remote fleet node's current computational load and hardware profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLoadMetric {
    pub node_id: P2pNodeId,
    pub cpu_load_pct: f32,
    pub gpu_load_pct: f32,
    pub active_tasks: usize,
    pub thermodynamic_free_energy: f64,
    pub hardware_spec: ClusterNodeHardwareSpec,
    pub last_heartbeat_ms: u64,
}

/// A computational unit scheduled across the fleet mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetTask {
    pub task_id: u64,
    pub graph: NativeComputationalGraph,
    pub priority: u8,
    pub submitted_at_ms: u64,
}

/// Work-Stealing Fleet Scheduler
pub struct FleetScheduler {
    pub local_node_id: P2pNodeId,
    next_task_id: AtomicU64,
    task_queue: RwLock<VecDeque<FleetTask>>,
    in_flight_remote_tasks: RwLock<HashMap<u64, (P2pNodeId, NativeComputationalGraph)>>,
    peer_metrics: RwLock<HashMap<P2pNodeId, PeerLoadMetric>>,
    completed_results: RwLock<HashMap<u64, WorkResult>>,
    received_lora_deltas: RwLock<HashMap<String, CartridgeLoraDeltaSync>>,
    pub offload_threshold_pct: f32,
}

impl FleetScheduler {
    /// Creates a new FleetScheduler for the local node
    pub fn new(local_node_id: P2pNodeId) -> Self {
        Self {
            local_node_id,
            next_task_id: AtomicU64::new(1),
            task_queue: RwLock::new(VecDeque::new()),
            in_flight_remote_tasks: RwLock::new(HashMap::new()),
            peer_metrics: RwLock::new(HashMap::new()),
            completed_results: RwLock::new(HashMap::new()),
            received_lora_deltas: RwLock::new(HashMap::new()),
            offload_threshold_pct: 75.0,
        }
    }

    /// Submits a computational graph to the local queue
    pub fn submit_task(&self, graph: NativeComputationalGraph, priority: u8) -> u64 {
        let task_id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.task_queue.write().push_back(FleetTask {
            task_id,
            graph,
            priority,
            submitted_at_ms: ts,
        });

        task_id
    }

    /// Updates or registers load metrics reported by a peer node
    pub fn record_peer_metric(&self, metric: PeerLoadMetric) {
        self.peer_metrics.write().insert(metric.node_id.clone(), metric);
    }

    /// Evaluates incoming work-steal request from an idle peer and donates a task if available
    pub fn respond_to_work_steal(&self, req: &WorkStealRequest) -> Option<WorkStealResponse> {
        let mut queue = self.task_queue.write();
        if queue.len() <= 1 {
            return None; // Keep at least one task for local execution
        }

        if let Some(task) = queue.pop_back() {
            self.in_flight_remote_tasks.write().insert(
                task.task_id,
                (req.requester_node_id.clone(), task.graph.clone()),
            );

            Some(WorkStealResponse {
                donor_node_id: self.local_node_id.clone(),
                task_id: task.task_id,
                graph: task.graph,
            })
        } else {
            None
        }
    }

    /// Offloads pending task to the least loaded fleet peer if local capacity is saturated
    pub fn offload_excess_work(&self, current_local_load_pct: f32) -> Option<(P2pNodeId, WorkStealResponse)> {
        if current_local_load_pct < self.offload_threshold_pct {
            return None;
        }

        let mut queue = self.task_queue.write();
        if queue.is_empty() {
            return None;
        }

        // Find peer with lowest CPU + GPU load
        let peers = self.peer_metrics.read();
        let best_peer = peers
            .values()
            .filter(|p| (p.cpu_load_pct + p.gpu_load_pct) / 2.0 < self.offload_threshold_pct)
            .min_by(|a, b| {
                let load_a = a.cpu_load_pct + a.gpu_load_pct;
                let load_b = b.cpu_load_pct + b.gpu_load_pct;
                load_a.partial_cmp(&load_b).unwrap_or(std::cmp::Ordering::Equal)
            })?;

        let task = queue.pop_back()?;
        let target_node = best_peer.node_id.clone();

        self.in_flight_remote_tasks.write().insert(
            task.task_id,
            (target_node.clone(), task.graph.clone()),
        );

        Some((
            target_node,
            WorkStealResponse {
                donor_node_id: self.local_node_id.clone(),
                task_id: task.task_id,
                graph: task.graph,
            },
        ))
    }

    /// Re-integrates completed remote execution result back into the local state
    pub fn integrate_remote_result(&self, result: WorkResult) -> Result<()> {
        let mut in_flight = self.in_flight_remote_tasks.write();
        if let Some((_peer, _graph)) = in_flight.remove(&result.task_id) {
            self.completed_results.write().insert(result.task_id, result);
            Ok(())
        } else {
            Err(anyhow!(
                "Received work result for unknown or completed task {}",
                result.task_id
            ))
        }
    }

    /// Queries the completed result of a task
    pub fn get_task_result(&self, task_id: u64) -> Option<WorkResult> {
        self.completed_results.read().get(&task_id).cloned()
    }

    /// Number of queued local tasks
    pub fn pending_task_count(&self) -> usize {
        self.task_queue.read().len()
    }

    /// Number of in-flight remote tasks offloaded to the fleet
    pub fn in_flight_task_count(&self) -> usize {
        self.in_flight_remote_tasks.read().len()
    }

    /// Serializes current node load metrics into a P2P Heartbeat SyncMessage
    pub fn create_heartbeat_message(
        &self,
        cpu_load_pct: f32,
        gpu_load_pct: f32,
        free_energy: f64,
    ) -> Result<SyncMessage> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let metric = PeerLoadMetric {
            node_id: self.local_node_id.clone(),
            cpu_load_pct,
            gpu_load_pct,
            active_tasks: self.pending_task_count(),
            thermodynamic_free_energy: free_energy,
            hardware_spec: ClusterNodeHardwareSpec::default(),
            last_heartbeat_ms: ts,
        };
        let payload = serde_json::to_vec(&metric)?;
        Ok(SyncMessage {
            kind: SyncMessageKind::Heartbeat,
            payload,
            from: self.local_node_id.0.clone(),
            timestamp: ts,
            intent_version: 1,
        })
    }

    /// Creates an outbound WorkStealRequest SyncMessage
    pub fn create_work_steal_request(&self, max_nodes: usize, min_free_energy: f64) -> Result<SyncMessage> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let req = WorkStealRequest {
            requester_node_id: self.local_node_id.clone(),
            max_nodes,
            min_free_energy,
        };
        let payload = serde_json::to_vec(&req)?;
        Ok(SyncMessage {
            kind: SyncMessageKind::WorkStealRequest,
            payload,
            from: self.local_node_id.0.clone(),
            timestamp: ts,
            intent_version: 1,
        })
    }

    /// Processes an incoming P2P SyncMessage and dispatches appropriate scheduler responses
    pub fn handle_incoming_sync_message(
        &self,
        _from: &P2pNodeId,
        msg: &SyncMessage,
    ) -> Result<Option<SyncMessage>> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        match msg.kind {
            SyncMessageKind::Heartbeat => {
                if let Ok(metric) = serde_json::from_slice::<PeerLoadMetric>(&msg.payload) {
                    self.record_peer_metric(metric);
                }
                Ok(None)
            }
            SyncMessageKind::WorkStealRequest => {
                if let Some(resp) = serde_json::from_slice::<WorkStealRequest>(&msg.payload)
                    .ok()
                    .and_then(|req| self.respond_to_work_steal(&req))
                {
                    let payload = serde_json::to_vec(&resp)?;
                    return Ok(Some(SyncMessage {
                        kind: SyncMessageKind::WorkStealResponse,
                        payload,
                        from: self.local_node_id.0.clone(),
                        timestamp: ts,
                        intent_version: 1,
                    }));
                }
                Ok(None)
            }
            SyncMessageKind::WorkStealResponse => {
                if let Ok(resp) = serde_json::from_slice::<WorkStealResponse>(&msg.payload) {
                    // Enqueue stolen/donated task for local execution
                    self.task_queue.write().push_back(FleetTask {
                        task_id: resp.task_id,
                        graph: resp.graph,
                        priority: 2,
                        submitted_at_ms: ts,
                    });
                }
                Ok(None)
            }
            SyncMessageKind::WorkResult => {
                if let Ok(result) = serde_json::from_slice::<WorkResult>(&msg.payload) {
                    self.integrate_remote_result(result)?;
                }
                Ok(None)
            }
            SyncMessageKind::CartridgeLoraDeltaSync => {
                if let Ok(delta) = serde_json::from_slice::<CartridgeLoraDeltaSync>(&msg.payload) {
                    self.received_lora_deltas
                        .write()
                        .insert(delta.cartridge_id.clone(), delta);
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Broadcasts an adapted .si LoRA delta payload across the P2P mesh
    pub fn create_lora_delta_broadcast(&self, delta: &CartridgeLoraDeltaSync) -> Result<SyncMessage> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let payload = serde_json::to_vec(delta)?;
        Ok(SyncMessage {
            kind: SyncMessageKind::CartridgeLoraDeltaSync,
            payload,
            from: self.local_node_id.0.clone(),
            timestamp: ts,
            intent_version: 1,
        })
    }

    /// Retrieves the most recent LoRA weight delta received for a cartridge
    pub fn get_lora_delta(&self, cartridge_id: &str) -> Option<CartridgeLoraDeltaSync> {
        self.received_lora_deltas.read().get(cartridge_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fleet_scheduler_work_stealing_and_offload() {
        let scheduler = FleetScheduler::new(P2pNodeId("node-local".to_string()));

        // 1. Submit tasks
        let _t1 = scheduler.submit_task(NativeComputationalGraph::new(), 1);
        let t2 = scheduler.submit_task(NativeComputationalGraph::new(), 1);
        assert_eq!(scheduler.pending_task_count(), 2);

        // 2. Respond to peer work steal request
        let steal_req = WorkStealRequest {
            requester_node_id: P2pNodeId("node-remote-1".to_string()),
            max_nodes: 5,
            min_free_energy: 0.05,
        };

        let stolen = scheduler.respond_to_work_steal(&steal_req).unwrap();
        assert_eq!(stolen.task_id, t2);
        assert_eq!(scheduler.pending_task_count(), 1);
        assert_eq!(scheduler.in_flight_task_count(), 1);

        // 3. Integrate completed result
        let result = WorkResult {
            worker_node_id: P2pNodeId("node-remote-1".to_string()),
            task_id: t2,
            execution_trace: vec![0xAA, 0xBB],
            result_status: 0,
            thermodynamic_free_energy: 0.01,
        };

        scheduler.integrate_remote_result(result).unwrap();
        assert_eq!(scheduler.in_flight_task_count(), 0);
        let fetched = scheduler.get_task_result(t2).unwrap();
        assert_eq!(fetched.execution_trace, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_fleet_scheduler_sync_message_protocol_roundtrip() {
        let local_node = FleetScheduler::new(P2pNodeId("node-alpha".to_string()));
        let remote_node = FleetScheduler::new(P2pNodeId("node-beta".to_string()));

        // Submit tasks to local node
        let _t1 = local_node.submit_task(NativeComputationalGraph::new(), 1);
        let t2 = local_node.submit_task(NativeComputationalGraph::new(), 2);
        assert_eq!(local_node.pending_task_count(), 2);

        // Remote node generates a heartbeat
        let heartbeat = remote_node.create_heartbeat_message(25.0, 10.0, 0.02).unwrap();
        let resp = local_node
            .handle_incoming_sync_message(&remote_node.local_node_id, &heartbeat)
            .unwrap();
        assert!(resp.is_none());

        // Remote node sends WorkStealRequest
        let steal_msg = remote_node.create_work_steal_request(4, 0.05).unwrap();
        let steal_resp = local_node
            .handle_incoming_sync_message(&remote_node.local_node_id, &steal_msg)
            .unwrap()
            .expect("Expected work steal response with donated task");

        assert_eq!(steal_resp.kind, SyncMessageKind::WorkStealResponse);
        let stolen: WorkStealResponse = serde_json::from_slice(&steal_resp.payload).unwrap();
        assert_eq!(stolen.task_id, t2);
        assert_eq!(local_node.in_flight_task_count(), 1);

        // Remote node ingests WorkStealResponse into its own execution queue
        let enqueue_resp = remote_node
            .handle_incoming_sync_message(&local_node.local_node_id, &steal_resp)
            .unwrap();
        assert!(enqueue_resp.is_none());
        assert_eq!(remote_node.pending_task_count(), 1);

        // Remote node sends completed WorkResult back
        let work_res = WorkResult {
            worker_node_id: remote_node.local_node_id.clone(),
            task_id: t2,
            execution_trace: vec![0x01, 0x02, 0x03],
            result_status: 0,
            thermodynamic_free_energy: 0.005,
        };
        let res_msg = SyncMessage {
            kind: SyncMessageKind::WorkResult,
            payload: serde_json::to_vec(&work_res).unwrap(),
            from: remote_node.local_node_id.0.clone(),
            timestamp: 1000,
            intent_version: 1,
        };
        let finish_resp = local_node
            .handle_incoming_sync_message(&remote_node.local_node_id, &res_msg)
            .unwrap();
        assert!(finish_resp.is_none());
        assert_eq!(local_node.in_flight_task_count(), 0);

        let completed = local_node.get_task_result(t2).unwrap();
        assert_eq!(completed.execution_trace, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_fleet_scheduler_lora_delta_p2p_sync() {
        let node_a = FleetScheduler::new(P2pNodeId("node-cluster-gpu1".to_string()));
        let node_b = FleetScheduler::new(P2pNodeId("node-cluster-gpu2".to_string()));

        let lora_sync = CartridgeLoraDeltaSync {
            cartridge_id: "vision_reflex_v1".to_string(),
            adaptation_cycle: 12,
            rank: 16,
            lora_b_delta: vec![0.05f32; 16 * 256],
            orthogonality_score: 0.998,
            free_energy_reduction: 0.042,
        };

        // Node A broadcasts LoRA adaptation delta
        let broadcast_msg = node_a.create_lora_delta_broadcast(&lora_sync).unwrap();
        assert_eq!(broadcast_msg.kind, SyncMessageKind::CartridgeLoraDeltaSync);

        // Node B receives and integrates the delta
        let resp = node_b
            .handle_incoming_sync_message(&node_a.local_node_id, &broadcast_msg)
            .unwrap();
        assert!(resp.is_none());

        let retrieved = node_b.get_lora_delta("vision_reflex_v1").unwrap();
        assert_eq!(retrieved.adaptation_cycle, 12);
        assert_eq!(retrieved.rank, 16);
        assert_eq!(retrieved.lora_b_delta.len(), 16 * 256);
        assert_eq!(retrieved.orthogonality_score, 0.998);
    }

    #[test]
    fn test_fleet_scheduler_heterogeneous_hardware_profiles() {
        use crate::federation::p2p::types::PlatformOs;

        let local_win = FleetScheduler::new(P2pNodeId("node-win-dx12".to_string()));
        let remote_linux = FleetScheduler::new(P2pNodeId("node-linux-vulkan".to_string()));

        let linux_spec = ClusterNodeHardwareSpec {
            os: PlatformOs::LinuxVulkan,
            cpu_cores: 64,
            gpu_device_name: "NVIDIA A100-SXM4-80GB".to_string(),
            total_vram_mb: 81920,
            supports_fp16: true,
            supports_simd_warp_scan: true,
        };

        // Record Linux compute server profile
        local_win.record_peer_metric(PeerLoadMetric {
            node_id: remote_linux.local_node_id.clone(),
            cpu_load_pct: 12.0,
            gpu_load_pct: 5.0,
            active_tasks: 0,
            thermodynamic_free_energy: 0.001,
            hardware_spec: linux_spec.clone(),
            last_heartbeat_ms: 1000,
        });

        // Submit heavy task on local Windows machine
        let t_id = local_win.submit_task(NativeComputationalGraph::new(), 1);
        assert_eq!(local_win.pending_task_count(), 1);

        // Offload excess work to idle Linux compute peer
        let (target, steal_resp) = local_win.offload_excess_work(85.0).expect("Should offload to Linux node");
        assert_eq!(target, remote_linux.local_node_id);
        assert_eq!(steal_resp.task_id, t_id);
        assert_eq!(local_win.in_flight_task_count(), 1);

        // Verify recorded hardware telemetry
        let peers = local_win.peer_metrics.read();
        let linux_metric = peers.get(&remote_linux.local_node_id).unwrap();
        assert_eq!(linux_metric.hardware_spec.os, PlatformOs::LinuxVulkan);
        assert_eq!(linux_metric.hardware_spec.total_vram_mb, 81920);
    }
}
