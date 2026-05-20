use anyhow::Result;
use nervous_system::SharedMemorySynapse;
use biology::SystemBiology;
use agents::SpecialistAgent;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// The Hive Component.
/// Manages the runtime execution of agents and tasks.
pub struct HiveRuntime {
    pub synapse: SharedMemorySynapse,
    pub biology: SystemBiology,
    pub agents: RwLock<HashMap<String, SpecialistAgent>>,
}

#[derive(Debug, Clone)]
pub struct HiveRuntimeConfig {
    pub db_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatus {
    pub active_agents: usize,
    pub metabolic_state: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatistics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
}

impl HiveRuntime {
    pub fn new(config: &HiveRuntimeConfig) -> Self {
        Self {
            synapse: SharedMemorySynapse::new("SAB_STORE", 1024 * 1024).unwrap(),
            biology: SystemBiology::new(),
            agents: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register_agent(&self, name: String, agent: SpecialistAgent) {
        let mut agents = self.agents.write().await;
        agents.insert(name, agent);
    }

    pub async fn get_status(&self) -> RuntimeStatus {
        let agents = self.agents.read().await;
        RuntimeStatus {
            active_agents: agents.len(),
            metabolic_state: "OPTIMAL".to_string(),
        }
    }
}
