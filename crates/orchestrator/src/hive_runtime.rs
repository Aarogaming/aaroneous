use crate::agents::SpecialistAgent;
use crate::control::ControlPlane;
use crate::mdps_router::{RoutableTask, RoutingDecision, TaskRoutingEngine};
use biology::SystemBiology;
use nervous_system::SharedMemorySynapse;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// The Hive Component.
/// Manages the runtime execution of agents, task dispatch, and lifecycle.
pub struct HiveRuntime {
    pub synapse: SharedMemorySynapse,
    pub biology: SystemBiology,
    pub agents: RwLock<HashMap<String, SpecialistAgent>>,
    pub control_plane: ControlPlane,
    pub router: RwLock<TaskRoutingEngine>,
    pub task_log: RwLock<Vec<TaskRecord>>,
    pub is_running: bool,
}

#[derive(Debug, Clone)]
pub struct HiveRuntimeConfig {
    pub db_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatus {
    pub active_agents: usize,
    pub metabolic_state: String,
    pub tasks_dispatched: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatistics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
}

/// Record of a dispatched task
#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub task_id: String,
    pub specialist_id: String,
    pub specialist_name: String,
    pub dispatched_at: u64, // Unix timestamp in millis
    pub completed_at: Option<u64>,
    pub success: Option<bool>,
}

impl HiveRuntime {
    pub fn new(_config: &HiveRuntimeConfig) -> anyhow::Result<Self> {
        Ok(Self {
            synapse: SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024)?,
            biology: SystemBiology::new(),
            agents: RwLock::new(HashMap::new()),
            control_plane: ControlPlane::new(),
            router: RwLock::new(TaskRoutingEngine::new(Vec::new())),
            task_log: RwLock::new(Vec::new()),
            is_running: false,
        })
    }

    /// Register a specialist agent in the runtime
    pub async fn register_agent(&self, name: String, agent: SpecialistAgent) {
        let mut agents = self.agents.write().await;
        agents.insert(name, agent);
    }

    /// Start the runtime and initialize routing engine with registered agents
    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.is_running = true;

        // Initialize router with registered agents as specialists
        let agents = self.agents.read().await;
        let specialists: Vec<crate::mdps_router::Specialist> = agents
            .values()
            .map(|agent| crate::mdps_router::Specialist {
                id: agent.id.clone(),
                name: agent.name.clone(),
                skills: agent.enzyme_subset.clone(),
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 5.0,
            })
            .collect();

        let mut router = self.router.write().await;
        *router = TaskRoutingEngine::new(specialists);

        Ok(())
    }

    /// Stop the runtime
    pub async fn stop(&mut self) {
        self.is_running = false;
    }

    /// Dispatch a task to the optimal specialist via MDP routing
    pub async fn dispatch_task(&self, task: RoutableTask) -> anyhow::Result<RoutingDecision> {
        let mut router = self.router.write().await;
        let decision = router.find_optimal_specialist(&task);

        // Consume capacity on the selected specialist
        router.consume_capacity(&decision.specialist_id, task.estimated_cost);

        // Record task in log
        let record = TaskRecord {
            task_id: task.id.clone(),
            specialist_id: decision.specialist_id.clone(),
            specialist_name: decision.specialist_name.clone(),
            dispatched_at: timestamp_millis(),
            completed_at: None,
            success: None,
        };

        let mut task_log = self.task_log.write().await;
        task_log.push(record);

        Ok(decision)
    }

    /// Complete a task and update specialist performance
    pub async fn complete_task(
        &self,
        task_id: &str,
        success: bool,
        completion_time: f64,
    ) -> anyhow::Result<()> {
        let mut task_log = self.task_log.write().await;

        if let Some(record) = task_log.iter_mut().find(|r| r.task_id == task_id) {
            record.completed_at = Some(timestamp_millis());
            record.success = Some(success);

            // Update specialist performance in router
            let mut router = self.router.write().await;
            router.update_specialist_performance(
                &record.specialist_id,
                success,
                completion_time,
            );

            return Ok(());
        }

        Err(anyhow::anyhow!("Task {} not found in log", task_id))
    }

    /// Get runtime status
    pub async fn get_status(&self) -> RuntimeStatus {
        let agents = self.agents.read().await;
        let task_log = self.task_log.read().await;

        let tasks_completed = task_log
            .iter()
            .filter(|r| r.completed_at.is_some())
            .count() as u64;
        let tasks_failed = task_log
            .iter()
            .filter(|r| r.success == Some(false))
            .count() as u64;

        RuntimeStatus {
            active_agents: agents.len(),
            metabolic_state: if self.is_running {
                "OPTIMAL".to_string()
            } else {
                "DORMANT".to_string()
            },
            tasks_dispatched: task_log.len() as u64,
            tasks_completed,
            tasks_failed,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> RuntimeStatistics {
        // Synchronous version for non-async contexts
        RuntimeStatistics {
            tasks_completed: 0,
            tasks_failed: 0,
        }
    }
}

/// Get current timestamp in milliseconds
fn timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> HiveRuntimeConfig {
        HiveRuntimeConfig {
            db_path: ":memory:".to_string(),
        }
    }

    #[tokio::test]
    async fn test_hive_runtime_creation() {
        let config = test_config();
        let runtime = HiveRuntime::new(&config);
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_register_agent() {
        let config = test_config();
        let runtime = HiveRuntime::new(&config).unwrap();
        let agent = SpecialistAgent::default();
        runtime
            .register_agent("test_agent".to_string(), agent)
            .await;

        let status = runtime.get_status().await;
        assert_eq!(status.active_agents, 1);
    }

    #[tokio::test]
    async fn test_start_stop() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();
        assert!(!runtime.is_running);

        runtime.start().await.unwrap();
        assert!(runtime.is_running);

        runtime.stop().await;
        assert!(!runtime.is_running);
    }

    #[tokio::test]
    async fn test_dispatch_task() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();

        // Register a specialist
        let mut agent = SpecialistAgent::default();
        agent.id = "spec_test".to_string();
        agent.name = "Test Specialist".to_string();
        agent.enzyme_subset = vec!["rust".to_string()];
        runtime
            .register_agent("test".to_string(), agent)
            .await;

        runtime.start().await.unwrap();

        let task = RoutableTask {
            id: "task_1".to_string(),
            task_type: crate::mdps_router::TaskType::CodeGeneration,
            complexity: 0.5,
            urgency: 0.5,
            required_skills: vec!["rust".to_string()],
            estimated_cost: 0.2,
        };

        let decision = runtime.dispatch_task(task).await.unwrap();
        assert!(!decision.specialist_id.is_empty());
    }

    #[tokio::test]
    async fn test_complete_task_success() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();

        let mut agent = SpecialistAgent::default();
        agent.id = "spec_a".to_string();
        agent.name = "Agent A".to_string();
        agent.enzyme_subset = vec!["rust".to_string()];
        runtime.register_agent("a".to_string(), agent).await;
        runtime.start().await.unwrap();

        let task = RoutableTask {
            id: "task_complete".to_string(),
            task_type: crate::mdps_router::TaskType::CodeGeneration,
            complexity: 0.5,
            urgency: 0.5,
            required_skills: vec!["rust".to_string()],
            estimated_cost: 0.1,
        };

        runtime.dispatch_task(task).await.unwrap();
        let task_id = "task_complete".to_string();

        // Complete the task
        runtime.complete_task(&task_id, true, 3.0).await.unwrap();

        // Verify task record updated
        let log = runtime.task_log.read().await;
        let record = log.iter().find(|r| r.task_id == task_id).unwrap();
        assert_eq!(record.success, Some(true));
        assert!(record.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_complete_task_not_found() {
        let config = test_config();
        let runtime = HiveRuntime::new(&config).unwrap();

        let result = runtime.complete_task("nonexistent_task", true, 1.0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_stats_returns_defaults() {
        let config = test_config();
        let runtime = HiveRuntime::new(&config).unwrap();
        let stats = runtime.stats();
        assert_eq!(stats.tasks_completed, 0);
        assert_eq!(stats.tasks_failed, 0);
    }

    #[tokio::test]
    async fn test_get_status_metabolic_state() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();

        let status = runtime.get_status().await;
        assert_eq!(status.metabolic_state, "DORMANT");

        runtime.start().await.unwrap();
        let status = runtime.get_status().await;
        assert_eq!(status.metabolic_state, "OPTIMAL");
    }

    #[tokio::test]
    async fn test_get_status_counts() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();

        let mut agent = SpecialistAgent::default();
        agent.id = "s1".to_string();
        agent.enzyme_subset = vec!["rust".to_string()];
        runtime.register_agent("a".to_string(), agent).await;
        runtime.start().await.unwrap();

        // Dispatch a task
        let task = RoutableTask {
            id: "t1".to_string(),
            task_type: crate::mdps_router::TaskType::Analysis,
            complexity: 0.3,
            urgency: 0.3,
            required_skills: vec!["rust".to_string()],
            estimated_cost: 0.05,
        };
        runtime.dispatch_task(task).await.unwrap();

        let status = runtime.get_status().await;
        assert_eq!(status.tasks_dispatched, 1);
        assert_eq!(status.tasks_completed, 0);
    }

    #[tokio::test]
    async fn test_multiple_dispatch_accumulates_log() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();

        let mut agent = SpecialistAgent::default();
        agent.id = "s1".to_string();
        agent.enzyme_subset = vec!["rust".to_string()];
        runtime.register_agent("a".to_string(), agent).await;
        runtime.start().await.unwrap();

        for i in 0..5 {
            let task = RoutableTask {
                id: format!("task_{}", i),
                task_type: crate::mdps_router::TaskType::Analysis,
                complexity: 0.5,
                urgency: 0.5,
                required_skills: vec!["rust".to_string()],
                estimated_cost: 0.05,
            };
            runtime.dispatch_task(task).await.unwrap();
        }

        let log = runtime.task_log.read().await;
        assert_eq!(log.len(), 5);

        let status = runtime.get_status().await;
        assert_eq!(status.tasks_dispatched, 5);
    }

    #[tokio::test]
    async fn test_task_record_fields_populated() {
        let config = test_config();
        let mut runtime = HiveRuntime::new(&config).unwrap();

        let mut agent = SpecialistAgent::default();
        agent.id = "s1".to_string();
        agent.name = "TestSpec".to_string();
        agent.enzyme_subset = vec!["rust".to_string()];
        runtime.register_agent("a".to_string(), agent).await;
        runtime.start().await.unwrap();

        let task = RoutableTask {
            id: "task_fields".to_string(),
            task_type: crate::mdps_router::TaskType::Refactor,
            complexity: 0.7,
            urgency: 0.3,
            required_skills: vec!["rust".to_string()],
            estimated_cost: 0.15,
        };
        let decision = runtime.dispatch_task(task).await.unwrap();

        let log = runtime.task_log.read().await;
        let record = log.last().unwrap();
        assert_eq!(record.task_id, "task_fields");
        assert_eq!(record.specialist_id, decision.specialist_id);
        assert_eq!(record.specialist_name, decision.specialist_name);
        assert!(record.dispatched_at > 0);
        assert_eq!(record.completed_at, None);
        assert_eq!(record.success, None);
    }
}
