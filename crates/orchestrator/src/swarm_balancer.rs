//! crates/orchestrator/src/swarm_balancer.rs
//! Swarm Load Balancer — distributes tasks across local and remote workers.
//!
//! Uses in-process tokio channels for local dispatch and a pluggable
//! Transport trait for remote peers (TCP, NATS, etc.).

use crate::mdps_router::RoutableTask;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Worker status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Offline,
    Failed(String),
}

/// A swarm worker (local or remote)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmWorker {
    pub id: String,
    pub name: String,
    pub address: Option<String>, // None for local workers
    pub capacity: f64,           // 0.0-1.0
    pub status: WorkerStatus,
    pub tasks_completed: u64,
    pub avg_latency_ms: f64,
}

/// Task dispatch event sent over channels
#[derive(Debug, Clone)]
pub struct TaskDispatch {
    pub task: RoutableTask,
    pub target_worker: String,
    pub dispatched_at: u64,
}

/// Task completion event
#[derive(Debug, Clone)]
pub struct TaskCompletion {
    pub task_id: String,
    pub worker_id: String,
    pub success: bool,
    pub completion_time_ms: f64,
}

/// Transport trait for remote peer communication
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send_task(&self, worker: &SwarmWorker, task: &RoutableTask) -> Result<()>;
    async fn query_status(&self, worker: &SwarmWorker) -> Result<WorkerStatus>;
}

/// TCP transport for remote peers
pub struct TcpTransport {
    // Placeholder — in production this would hold connection pool
}

#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn send_task(&self, _worker: &SwarmWorker, _task: &RoutableTask) -> Result<()> {
        // TODO: Implement TCP framing with length-delimited messages
        Ok(())
    }

    async fn query_status(&self, _worker: &SwarmWorker) -> Result<WorkerStatus> {
        Ok(WorkerStatus::Idle)
    }
}

/// Channel-based transport for in-process workers
pub struct ChannelTransport {
    sender: mpsc::Sender<TaskDispatch>,
}

#[async_trait::async_trait]
impl Transport for ChannelTransport {
    async fn send_task(&self, worker: &SwarmWorker, task: &RoutableTask) -> Result<()> {
        let dispatch = TaskDispatch {
            task: task.clone(),
            target_worker: worker.id.clone(),
            dispatched_at: timestamp_millis(),
        };
        self.sender
            .send(dispatch)
            .await
            .map_err(|e| anyhow::anyhow!("Channel send failed: {}", e))
    }

    async fn query_status(&self, _worker: &SwarmWorker) -> Result<WorkerStatus> {
        Ok(WorkerStatus::Idle)
    }
}

/// The Swarm Balancer
pub struct SwarmBalancer {
    workers: Arc<RwLock<HashMap<String, SwarmWorker>>>,
    transport: Arc<dyn Transport>,
    completions: mpsc::Sender<TaskCompletion>,
}

impl SwarmBalancer {
    pub fn new(transport: Arc<dyn Transport>, completions: mpsc::Sender<TaskCompletion>) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            transport,
            completions,
        }
    }

    /// Register a worker in the swarm
    pub async fn register_worker(&self, worker: SwarmWorker) {
        let mut workers = self.workers.write().await;
        workers.insert(worker.id.clone(), worker);
    }

    /// Remove a worker from the swarm
    pub async fn remove_worker(&self, worker_id: &str) {
        let mut workers = self.workers.write().await;
        workers.remove(worker_id);
    }

    /// Get all workers
    pub async fn list_workers(&self) -> Vec<SwarmWorker> {
        let workers = self.workers.read().await;
        workers.values().cloned().collect()
    }

    /// Find the best worker for a task
    pub async fn find_best_worker(&self, _task: &RoutableTask) -> Option<SwarmWorker> {
        let workers = self.workers.read().await;

        workers
            .values()
            .filter(|w| w.status == WorkerStatus::Idle && w.capacity > 0.1)
            .min_by(|a, b| {
                let score_a = a.capacity * (1.0 - a.avg_latency_ms / 1000.0);
                let score_b = b.capacity * (1.0 - b.avg_latency_ms / 1000.0);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Dispatch a task to the best available worker
    pub async fn dispatch_task(&mut self, task: RoutableTask) -> Result<DispatchResult> {
        let worker = self
            .find_best_worker(&task)
            .await
            .ok_or_else(|| anyhow::anyhow!("No available workers in swarm"))?;

        // Mark worker as busy
        {
            let mut workers = self.workers.write().await;
            if let Some(w) = workers.get_mut(&worker.id) {
                w.status = WorkerStatus::Busy;
                w.capacity = (w.capacity - task.estimated_cost).max(0.0);
            }
        }

        // Send via transport
        self.transport.send_task(&worker, &task).await?;

        Ok(DispatchResult {
            task_id: task.id,
            worker_id: worker.id,
            worker_name: worker.name,
        })
    }

    /// Record task completion
    pub async fn complete_task(
        &self,
        task_id: &str,
        worker_id: &str,
        success: bool,
        completion_time_ms: f64,
    ) -> Result<()> {
        // Update worker stats
        {
            let mut workers = self.workers.write().await;
            if let Some(worker) = workers.get_mut(worker_id) {
                worker.status = if success {
                    WorkerStatus::Idle
                } else {
                    WorkerStatus::Failed("Task failed".to_string())
                };
                worker.tasks_completed += 1;
                worker.avg_latency_ms =
                    worker.avg_latency_ms * 0.9 + completion_time_ms * 0.1;
                worker.capacity = (worker.capacity + 0.2).min(1.0);
            }
        }

        // Send completion event
        let _ = self
            .completions
            .send(TaskCompletion {
                task_id: task_id.to_string(),
                worker_id: worker_id.to_string(),
                success,
                completion_time_ms,
            })
            .await;

        Ok(())
    }

    /// Get swarm health summary
    pub async fn health(&self) -> SwarmHealth {
        let workers = self.workers.read().await;
        let total = workers.len();
        let idle = workers.values().filter(|w| w.status == WorkerStatus::Idle).count();
        let busy = workers.values().filter(|w| w.status == WorkerStatus::Busy).count();
        let offline = workers.values().filter(|w| w.status == WorkerStatus::Offline).count();
        let avg_capacity = if total > 0 {
            workers.values().map(|w| w.capacity).sum::<f64>() / total as f64
        } else {
            0.0
        };

        SwarmHealth {
            total_workers: total,
            idle_workers: idle,
            busy_workers: busy,
            offline_workers: offline,
            average_capacity: avg_capacity,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    pub task_id: String,
    pub worker_id: String,
    pub worker_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmHealth {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub offline_workers: usize,
    pub average_capacity: f64,
}

fn timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTransport;

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn send_task(&self, _worker: &SwarmWorker, _task: &RoutableTask) -> Result<()> {
            Ok(())
        }
        async fn query_status(&self, _worker: &SwarmWorker) -> Result<WorkerStatus> {
            Ok(WorkerStatus::Idle)
        }
    }

    fn mock_balancer() -> SwarmBalancer {
        let (tx, _rx) = mpsc::channel(16);
        SwarmBalancer::new(Arc::new(MockTransport), tx)
    }

    #[tokio::test]
    async fn test_register_and_list_workers() {
        let balancer = mock_balancer();
        let worker = SwarmWorker {
            id: "w1".to_string(),
            name: "Worker 1".to_string(),
            address: None,
            capacity: 1.0,
            status: WorkerStatus::Idle,
            tasks_completed: 0,
            avg_latency_ms: 0.0,
        };

        balancer.register_worker(worker).await;
        let workers = balancer.list_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].id, "w1");
    }

    #[tokio::test]
    async fn test_find_best_worker() {
        let balancer = mock_balancer();

        balancer
            .register_worker(SwarmWorker {
                id: "w1".to_string(),
                name: "Slow".to_string(),
                address: None,
                capacity: 0.5,
                status: WorkerStatus::Idle,
                tasks_completed: 0,
                avg_latency_ms: 100.0,
            })
            .await;

        balancer
            .register_worker(SwarmWorker {
                id: "w2".to_string(),
                name: "Fast".to_string(),
                address: None,
                capacity: 0.9,
                status: WorkerStatus::Idle,
                tasks_completed: 0,
                avg_latency_ms: 10.0,
            })
            .await;

        let task = RoutableTask {
            id: "t1".to_string(),
            task_type: crate::mdps_router::TaskType::CodeGeneration,
            complexity: 0.5,
            urgency: 0.5,
            required_skills: vec![],
            estimated_cost: 0.1,
        };

        let best = balancer.find_best_worker(&task).await.unwrap();
        assert_eq!(best.id, "w2"); // Fast worker should be selected
    }

    #[tokio::test]
    async fn test_dispatch_and_complete() {
        let balancer = mock_balancer();

        balancer
            .register_worker(SwarmWorker {
                id: "w1".to_string(),
                name: "Worker 1".to_string(),
                address: None,
                capacity: 1.0,
                status: WorkerStatus::Idle,
                tasks_completed: 0,
                avg_latency_ms: 0.0,
            })
            .await;

        let mut balancer = balancer;
        let task = RoutableTask {
            id: "t1".to_string(),
            task_type: crate::mdps_router::TaskType::CodeGeneration,
            complexity: 0.5,
            urgency: 0.5,
            required_skills: vec![],
            estimated_cost: 0.1,
        };

        let result = balancer.dispatch_task(task).await.unwrap();
        assert_eq!(result.worker_id, "w1");

        // Worker should now be busy
        let workers = balancer.list_workers().await;
        assert_eq!(workers[0].status, WorkerStatus::Busy);

        // Complete the task
        balancer.complete_task("t1", "w1", true, 50.0).await.unwrap();

        // Worker should be idle again
        let workers = balancer.list_workers().await;
        assert_eq!(workers[0].status, WorkerStatus::Idle);
        assert_eq!(workers[0].tasks_completed, 1);
    }

    #[tokio::test]
    async fn test_swarm_health() {
        let balancer = mock_balancer();

        balancer
            .register_worker(SwarmWorker {
                id: "w1".to_string(),
                name: "A".to_string(),
                address: None,
                capacity: 0.8,
                status: WorkerStatus::Idle,
                tasks_completed: 0,
                avg_latency_ms: 0.0,
            })
            .await;

        balancer
            .register_worker(SwarmWorker {
                id: "w2".to_string(),
                name: "B".to_string(),
                address: None,
                capacity: 0.4,
                status: WorkerStatus::Busy,
                tasks_completed: 0,
                avg_latency_ms: 0.0,
            })
            .await;

        let health = balancer.health().await;
        assert_eq!(health.total_workers, 2);
        assert_eq!(health.idle_workers, 1);
        assert_eq!(health.busy_workers, 1);
    }
}
