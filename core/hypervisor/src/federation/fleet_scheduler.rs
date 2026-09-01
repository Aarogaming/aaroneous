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

use crate::federation::p2p::types::{P2pNodeId, WorkResult, WorkStealRequest, WorkStealResponse};
use si_ir::NativeComputationalGraph;

/// Metric representation of a remote fleet node's current computational load
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLoadMetric {
    pub node_id: P2pNodeId,
    pub cpu_load_pct: f32,
    pub gpu_load_pct: f32,
    pub active_tasks: usize,
    pub thermodynamic_free_energy: f64,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fleet_scheduler_work_stealing_and_offload() {
        let scheduler = FleetScheduler::new(P2pNodeId("node-local".to_string()));

        // 1. Submit tasks
        let t1 = scheduler.submit_task(NativeComputationalGraph::new(), 1);
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
}
