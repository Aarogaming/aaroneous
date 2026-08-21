//! crates/orchestrator/src/workflow_engine.rs
//! Resilient Task Workflow DAG & Specialist Consensus State Machine
//! inspired by Temporal.io, Kubernetes Workflow Controllers, and LangGraph.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Status of an individual workflow step
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    RolledBack,
}

/// An individual executable step within an orchestrator workflow DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: String,
    pub assigned_specialist: String, // e.g. "Hephaestus", "Argus", "Odin"
    pub dependencies: Vec<String>,   // Step IDs that must complete first
    pub action_name: String,
    pub payload: String,
    pub status: StepStatus,
    pub retry_count: usize,
    pub max_retries: usize,
}

/// Master Workflow DAG managing multi-step specialist collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    pub workflow_id: String,
    pub steps: HashMap<String, WorkflowStep>,
    pub is_completed: bool,
    pub is_failed: bool,
}

impl WorkflowGraph {
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            steps: HashMap::new(),
            is_completed: false,
            is_failed: false,
        }
    }

    pub fn add_step(
        &mut self,
        step_id: impl Into<String>,
        specialist: impl Into<String>,
        action: impl Into<String>,
        payload: impl Into<String>,
        dependencies: Vec<String>,
        max_retries: usize,
    ) {
        let id = step_id.into();
        self.steps.insert(
            id.clone(),
            WorkflowStep {
                step_id: id,
                assigned_specialist: specialist.into(),
                dependencies,
                action_name: action.into(),
                payload: payload.into(),
                status: StepStatus::Pending,
                retry_count: 0,
                max_retries,
            },
        );
    }

    /// Identifies all steps ready for execution (all dependencies completed)
    pub fn get_ready_steps(&self) -> Vec<WorkflowStep> {
        let mut ready = Vec::new();

        let completed_steps: HashSet<&String> = self
            .steps
            .iter()
            .filter(|(_, s)| s.status == StepStatus::Completed)
            .map(|(id, _)| id)
            .collect();

        for step in self.steps.values() {
            if step.status == StepStatus::Pending {
                let deps_satisfied = step.dependencies.iter().all(|dep| completed_steps.contains(dep));
                if deps_satisfied {
                    ready.push(step.clone());
                }
            }
        }

        ready
    }

    /// Marks a step as completed and checks if whole workflow is finished
    pub fn complete_step(&mut self, step_id: &str) {
        if let Some(step) = self.steps.get_mut(step_id) {
            step.status = StepStatus::Completed;
        }

        self.is_completed = self.steps.values().all(|s| s.status == StepStatus::Completed);
    }

    /// Handles a step failure with automatic retry logic or cascade rollback
    pub fn fail_step(&mut self, step_id: &str, error: &str) {
        if let Some(step) = self.steps.get_mut(step_id) {
            if step.retry_count < step.max_retries {
                step.retry_count += 1;
                step.status = StepStatus::Pending; // Retry
            } else {
                step.status = StepStatus::Failed(error.to_string());
                self.is_failed = true;
                self.trigger_rollback();
            }
        }
    }

    /// Rolls back any running or pending steps when a fatal failure occurs
    fn trigger_rollback(&mut self) {
        for step in self.steps.values_mut() {
            if step.status == StepStatus::Running || step.status == StepStatus::Pending {
                step.status = StepStatus::RolledBack;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_dag_dependency_resolution() {
        let mut workflow = WorkflowGraph::new("wf_code_adaptation");

        // Step 1: Merlin decompiles intent
        workflow.add_step("step_1", "Merlin", "DecompileIntent", "{}", vec![], 2);

        // Step 2: Hephaestus forges AST patch (depends on step 1)
        workflow.add_step("step_2", "Hephaestus", "ForgePatch", "{}", vec!["step_1".to_string()], 2);

        // Step 3: Argus audits security (depends on step 2)
        workflow.add_step("step_3", "Argus", "SecurityAudit", "{}", vec!["step_2".to_string()], 1);

        // Initially only step 1 should be ready
        let ready = workflow.get_ready_steps();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].step_id, "step_1");

        // Complete step 1 -> step 2 becomes ready
        workflow.complete_step("step_1");
        let ready2 = workflow.get_ready_steps();
        assert_eq!(ready2.len(), 1);
        assert_eq!(ready2[0].step_id, "step_2");

        // Complete step 2 -> step 3 becomes ready
        workflow.complete_step("step_2");
        let ready3 = workflow.get_ready_steps();
        assert_eq!(ready3.len(), 1);
        assert_eq!(ready3[0].step_id, "step_3");

        // Complete step 3 -> workflow finishes
        workflow.complete_step("step_3");
        assert!(workflow.is_completed);
        assert!(!workflow.is_failed);
    }

    #[test]
    fn test_workflow_retry_and_rollback() {
        let mut workflow = WorkflowGraph::new("wf_failing");
        workflow.add_step("step_1", "Hephaestus", "Compile", "{}", vec![], 1);
        workflow.add_step("step_2", "Argus", "Audit", "{}", vec!["step_1".to_string()], 1);

        // Fail once -> retry
        workflow.fail_step("step_1", "Compile Error 1");
        assert_eq!(workflow.steps.get("step_1").unwrap().retry_count, 1);
        assert_eq!(workflow.steps.get("step_1").unwrap().status, StepStatus::Pending);

        // Fail second time (exceeds max_retries = 1) -> fatal failure + rollback
        workflow.fail_step("step_1", "Compile Error 2");
        assert!(workflow.is_failed);
        assert_eq!(workflow.steps.get("step_2").unwrap().status, StepStatus::RolledBack);
    }
}
