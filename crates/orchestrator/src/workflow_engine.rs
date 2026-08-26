//! crates/orchestrator/src/workflow_engine.rs
//! Resilient Task Workflow DAG & Specialist Consensus State Machine
//! inspired by Temporal.io, Kubernetes Workflow Controllers, and LangGraph.
//!
//! Supports crash-recoverable persistence via JSON serialization.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
    /// Optional file path for persistence
    #[serde(skip)]
    pub persist_path: Option<PathBuf>,
}

impl WorkflowGraph {
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            steps: HashMap::new(),
            is_completed: false,
            is_failed: false,
            persist_path: None,
        }
    }

    /// Create a new workflow with a persistence path
    pub fn new_with_persist(workflow_id: impl Into<String>, path: PathBuf) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            steps: HashMap::new(),
            is_completed: false,
            is_failed: false,
            persist_path: Some(path),
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

    // ─── Persistence ───────────────────────────────────────────────

    /// Serialize the workflow to a JSON string
    pub fn serialize(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize workflow")
    }

    /// Deserialize a workflow from a JSON string
    pub fn deserialize(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize workflow")
    }

    /// Save the workflow to disk at the configured persist_path
    pub fn save(&self) -> Result<()> {
        let path = self
            .persist_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No persist_path configured for workflow {}", self.workflow_id))?;
        self.save_to(path)
    }

    /// Save the workflow to a specific file path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let json = self.serialize()?;

        // Write to temp file first, then rename for atomicity
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)
            .with_context(|| format!("Failed to write workflow to {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to rename workflow file to {}", path.display()))?;

        Ok(())
    }

    /// Load a workflow from a file path
    pub fn load_from(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read workflow from {}", path.display()))?;
        let mut workflow = Self::deserialize(&json)?;
        workflow.persist_path = Some(path.to_path_buf());
        Ok(workflow)
    }

    /// Save to the default location: .aaroneous/workflows/{workflow_id}.json
    pub fn save_default(&self, workspace_root: &Path) -> Result<()> {
        let dir = workspace_root.join(".aaroneous").join("workflows");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.workflow_id));
        self.save_to(&path)
    }

    /// Load from the default location
    pub fn load_default(workspace_root: &Path, workflow_id: &str) -> Result<Self> {
        let path = workspace_root
            .join(".aaroneous")
            .join("workflows")
            .join(format!("{}.json", workflow_id));
        Self::load_from(&path)
    }

    /// List all persisted workflows in a directory
    pub fn list_persisted(workspace_root: &Path) -> Result<Vec<String>> {
        let dir = workspace_root.join(".aaroneous").join("workflows");
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_string());
                }
            }
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_workflow_dag_dependency_resolution() {
        let mut workflow = WorkflowGraph::new("wf_code_adaptation");

        workflow.add_step("step_1", "Merlin", "DecompileIntent", "{}", vec![], 2);
        workflow.add_step("step_2", "Hephaestus", "ForgePatch", "{}", vec!["step_1".to_string()], 2);
        workflow.add_step("step_3", "Argus", "SecurityAudit", "{}", vec!["step_2".to_string()], 1);

        let ready = workflow.get_ready_steps();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].step_id, "step_1");

        workflow.complete_step("step_1");
        let ready2 = workflow.get_ready_steps();
        assert_eq!(ready2.len(), 1);
        assert_eq!(ready2[0].step_id, "step_2");

        workflow.complete_step("step_2");
        let ready3 = workflow.get_ready_steps();
        assert_eq!(ready3.len(), 1);
        assert_eq!(ready3[0].step_id, "step_3");

        workflow.complete_step("step_3");
        assert!(workflow.is_completed);
        assert!(!workflow.is_failed);
    }

    #[test]
    fn test_workflow_retry_and_rollback() {
        let mut workflow = WorkflowGraph::new("wf_failing");
        workflow.add_step("step_1", "Hephaestus", "Compile", "{}", vec![], 1);
        workflow.add_step("step_2", "Argus", "Audit", "{}", vec!["step_1".to_string()], 1);

        workflow.fail_step("step_1", "Compile Error 1");
        assert_eq!(workflow.steps.get("step_1").unwrap().retry_count, 1);
        assert_eq!(workflow.steps.get("step_1").unwrap().status, StepStatus::Pending);

        workflow.fail_step("step_1", "Compile Error 2");
        assert!(workflow.is_failed);
        assert_eq!(workflow.steps.get("step_2").unwrap().status, StepStatus::RolledBack);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut workflow = WorkflowGraph::new("wf_roundtrip");
        workflow.add_step("s1", "Merlin", "Analyze", "data", vec![], 1);
        workflow.complete_step("s1");

        let json = workflow.serialize().unwrap();
        let loaded = WorkflowGraph::deserialize(&json).unwrap();

        assert_eq!(loaded.workflow_id, "wf_roundtrip");
        assert_eq!(loaded.steps.len(), 1);
        assert!(loaded.is_completed);
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("aaroneous_test_workflows");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = dir.join("test_wf.json");
        let mut workflow = WorkflowGraph::new("test_wf");
        workflow.add_step("s1", "Odin", "Plan", "{}", vec![], 0);
        workflow.save_to(&path).unwrap();

        let loaded = WorkflowGraph::load_from(&path).unwrap();
        assert_eq!(loaded.workflow_id, "test_wf");
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.persist_path, Some(path.clone()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_persisted() {
        let dir = std::env::temp_dir().join("aaroneous_test_list");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".aaroneous/workflows")).unwrap();

        // Create two workflow files
        fs::write(
            dir.join(".aaroneous/workflows/wf_a.json"),
            "{\"workflow_id\":\"wf_a\",\"steps\":{},\"is_completed\":false,\"is_failed\":false}",
        )
        .unwrap();
        fs::write(
            dir.join(".aaroneous/workflows/wf_b.json"),
            "{\"workflow_id\":\"wf_b\",\"steps\":{},\"is_completed\":false,\"is_failed\":false}",
        )
        .unwrap();

        let mut ids = WorkflowGraph::list_persisted(&dir).unwrap();
        ids.sort();
        assert_eq!(ids, vec!["wf_a", "wf_b"]);

        let _ = fs::remove_dir_all(&dir);
    }
}
