use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveStep {
    pub id: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub status: StepStatus,
    pub assigned_specialist: String, // e.g., "synthesizer", "fabricator"
    pub input_data: Option<String>,
    pub output_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutivePlan {
    pub plan_id: String,
    pub goal: String,
    pub steps: HashMap<String, CognitiveStep>,
    pub current_step_id: Option<String>,
}

impl ExecutivePlan {
    pub fn new(goal: &str) -> Self {
        Self {
            plan_id: uuid::Uuid::new_v4().to_string(),
            goal: goal.to_string(),
            steps: HashMap::new(),
            current_step_id: None,
        }
    }

    pub fn add_step(&mut self, step: CognitiveStep) {
        self.steps.insert(step.id.clone(), step);
    }

    /// Returns the next steps that are ready to be executed (dependencies met)
    pub fn get_ready_steps(&self) -> Vec<String> {
        self.steps
            .values()
            .filter(|s| s.status == StepStatus::Pending)
            .filter(|s| {
                s.dependencies.iter().all(|dep_id| {
                    self.steps
                        .get(dep_id)
                        .map(|dep| dep.status == StepStatus::Completed)
                        .unwrap_or(false)
                })
            })
            .map(|s| s.id.clone())
            .collect()
    }
}
