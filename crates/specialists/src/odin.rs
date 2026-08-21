//! odin.rs
//! Odin (The Allfather / Commander) & Draupnir (Task Scheduler & Token Multiplier Engine).
//! Domain Opcode: 0x0100 (TASK_ORCHESTRATION)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::info;

use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// A subtask in an Odin Directed Acyclic Graph (DAG)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub task_id: String,
    pub description: String,
    pub assigned_specialist: String,
    pub token_cost: f32,
    pub dependencies: Vec<String>,
    pub completed: bool,
}

/// Draupnir Relic Engine: Autonomous task scheduler and token multiplier
#[derive(Debug, Clone)]
pub struct DraupnirRelic {
    pub active_dag_count: usize,
    pub total_tasks_scheduled: usize,
}

impl Default for DraupnirRelic {
    fn default() -> Self {
        Self {
            active_dag_count: 0,
            total_tasks_scheduled: 0,
        }
    }
}

impl RelicEngine for DraupnirRelic {
    fn relic_name(&self) -> &'static str {
        "Draupnir"
    }

    fn supervisor_name(&self) -> &'static str {
        "Odin"
    }

    fn relic_status(&self) -> String {
        format!(
            "Draupnir Active: {} DAGs, {} tasks scheduled",
            self.active_dag_count, self.total_tasks_scheduled
        )
    }
}

/// Odin Sovereign Specialist
pub struct OdinSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub task_queue: VecDeque<TaskNode>,
    pub draupnir: DraupnirRelic,
}

impl Default for OdinSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl OdinSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            task_queue: VecDeque::new(),
            draupnir: DraupnirRelic::default(),
        }
    }

    /// Decomposes an intent into subtasks
    pub fn decompose_intent(&mut self, intent_description: &str) -> Vec<TaskNode> {
        info!(target: "specialist::odin", %intent_description, "Decomposing user intent into task DAG");

        let tasks = vec![
            TaskNode {
                task_id: "task_1_research".to_string(),
                description: format!("Research requirements for: {}", intent_description),
                assigned_specialist: "Merlin".to_string(),
                token_cost: 5.0,
                dependencies: vec![],
                completed: false,
            },
            TaskNode {
                task_id: "task_2_forge".to_string(),
                description: format!("Synthesize and adapt code for: {}", intent_description),
                assigned_specialist: "Hephaestus".to_string(),
                token_cost: 15.0,
                dependencies: vec!["task_1_research".to_string()],
                completed: false,
            },
            TaskNode {
                task_id: "task_3_audit".to_string(),
                description: format!("Security and safety verification for: {}", intent_description),
                assigned_specialist: "Argus".to_string(),
                token_cost: 5.0,
                dependencies: vec!["task_2_forge".to_string()],
                completed: false,
            },
        ];

        self.draupnir.active_dag_count += 1;
        self.draupnir.total_tasks_scheduled += tasks.len();

        for t in &tasks {
            self.task_queue.push_back(t.clone());
        }

        tasks
    }
}

#[async_trait]
impl SovereignSpecialist for OdinSpecialist {
    fn name(&self) -> &'static str {
        "Odin"
    }

    fn domain_opcode(&self) -> u16 {
        0x0100
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let intent_str = String::from_utf8_lossy(&packet.payload);
        let tasks = self.decompose_intent(&intent_str);
        let response_payload = serde_json::to_vec(&tasks)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!("Odin decomposed intent into {} tasks", tasks.len()),
            payload: response_payload,
        })
    }

    fn recharge_metabolism(&mut self, tokens: f32) {
        self.tokens = (self.tokens + tokens).min(self.max_tokens);
    }

    fn health_report(&self) -> SpecialistHealth {
        SpecialistHealth {
            name: self.name().to_string(),
            domain_opcode: self.domain_opcode(),
            tokens: self.tokens,
            max_tokens: self.max_tokens,
            backlog_count: self.task_queue.len(),
            is_dormant: self.tokens < 1.0,
            last_active: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_odin_intent_decomposition() {
        let mut odin = OdinSpecialist::new();
        let tasks = odin.decompose_intent("Build new adapter harness");
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].assigned_specialist, "Merlin");
        assert_eq!(tasks[1].assigned_specialist, "Hephaestus");
        assert_eq!(tasks[2].assigned_specialist, "Argus");
        assert_eq!(odin.draupnir.total_tasks_scheduled, 3);
    }
}
