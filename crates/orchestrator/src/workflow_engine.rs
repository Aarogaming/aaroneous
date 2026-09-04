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
    pub assigned_specialist: String, // e.g. "Fabricator", "Sentinel", "Orchestrator"
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

    /// Dynamically resolves any unassigned steps ("auto", "unassigned", or empty)
    /// by delegating to the optimal Markov Decision Process policy in `TaskRoutingEngine`.
    pub fn resolve_unassigned_steps_via_mdp(&mut self, router: &mut crate::mdps_router::TaskRoutingEngine) -> usize {
        let mut resolved_count = 0;

        for step in self.steps.values_mut() {
            let needs_routing = step.assigned_specialist.is_empty()
                || step.assigned_specialist.eq_ignore_ascii_case("auto")
                || step.assigned_specialist.eq_ignore_ascii_case("unassigned");

            if needs_routing {
                let routable = crate::mdps_router::RoutableTask {
                    id: step.step_id.clone(),
                    task_type: crate::mdps_router::TaskType::Custom(step.action_name.clone()),
                    complexity: 0.5,
                    urgency: 0.5,
                    required_skills: vec![step.action_name.clone()],
                    estimated_cost: 1.0,
                };

                let decision = router.find_optimal_specialist(&routable);
                step.assigned_specialist = decision.specialist_name;
                resolved_count += 1;
            }
        }

        resolved_count
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
    pub fn trigger_rollback(&mut self) {
        for step in self.steps.values_mut() {
            if step.status == StepStatus::Pending || step.status == StepStatus::Running {
                step.status = StepStatus::RolledBack;
            }
        }
    }

    /// Prunes an entire branch of dependent steps starting from a target root step
    pub fn prune_branch(&mut self, root_step_id: &str) -> usize {
        let mut to_prune = HashSet::new();
        to_prune.insert(root_step_id.to_string());

        let mut changed = true;
        while changed {
            changed = false;
            for (id, step) in &self.steps {
                if !to_prune.contains(id) && step.dependencies.iter().any(|dep| to_prune.contains(dep)) {
                    to_prune.insert(id.clone());
                    changed = true;
                }
            }
        }

        let pruned_count = to_prune.len();
        for id in to_prune {
            self.steps.remove(&id);
        }
        pruned_count
    }

    /// Captures a lightweight checkpoint state vector of all completed step IDs
    pub fn checkpoint(&self) -> Vec<String> {
        let mut completed: Vec<String> = self
            .steps
            .iter()
            .filter(|(_, s)| s.status == StepStatus::Completed)
            .map(|(id, _)| id.clone())
            .collect();
        completed.sort();
        completed
    }

    /// Asynchronously executes all currently ready steps concurrently via the provided runner closure.
    /// Ready steps are marked Running, executed in parallel futures, and results applied atomically upon join.
    pub async fn execute_ready_steps_concurrent<F, Fut>(&mut self, runner: F) -> usize
    where
        F: Fn(WorkflowStep) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = (String, std::result::Result<(), String>)> + Send + 'static,
    {
        let ready_steps = self.get_ready_steps();
        if ready_steps.is_empty() {
            return 0;
        }

        let ready_count = ready_steps.len();

        // Mark all ready steps as Running
        for step in &ready_steps {
            if let Some(s) = self.steps.get_mut(&step.step_id) {
                s.status = StepStatus::Running;
            }
        }

        // Spawn parallel tasks using tokio::task::JoinSet
        let mut join_set = tokio::task::JoinSet::new();
        let runner_arc = std::sync::Arc::new(runner);

        for step in ready_steps {
            let r = runner_arc.clone();
            join_set.spawn(async move {
                r(step).await
            });
        }

        // Collect concurrent results and update workflow state
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok((step_id, Ok(()))) => {
                    self.complete_step(&step_id);
                }
                Ok((step_id, Err(err))) => {
                    self.fail_step(&step_id, &err);
                }
                Err(_join_err) => {
                    self.is_failed = true;
                    self.trigger_rollback();
                }
            }
        }

        ready_count
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

    /// Converts the multi-step orchestrator workflow into a machine-native computational DAG (`si_ir::NativeComputationalGraph`).
    /// Maps each workflow step to a deterministic node, mapping step dependencies into topological graph edges.
    pub fn to_computational_graph(&self) -> si_ir::NativeComputationalGraph {
        let mut graph = si_ir::NativeComputationalGraph::new();
        let mut step_to_node_id: HashMap<String, u64> = HashMap::new();

        // 1. Assign deterministic 1-indexed node IDs
        for (idx, step_id) in self.steps.keys().enumerate() {
            step_to_node_id.insert(step_id.clone(), (idx + 1) as u64);
        }

        // 2. Build computation nodes with resolved topological dependencies
        for (step_id, step) in &self.steps {
            let node_id = step_to_node_id[step_id];
            let dep_node_ids: Vec<u64> = step
                .dependencies
                .iter()
                .filter_map(|dep_id| step_to_node_id.get(dep_id).copied())
                .collect();

            let (opcode, lattice, energy) = match step.action_name.as_str() {
                "Alloc" | "Allocate" => (
                    si_ir::MachineOpcode::Alloc {
                        size_bytes: step.payload.len().max(64),
                        align: 64,
                    },
                    si_ir::NativeTypeLattice::LinearMemoryPointer {
                        mutability: true,
                        alignment: 64,
                    },
                    0.005,
                ),
                "TensorDot" | "MatrixMultiply" => (
                    si_ir::MachineOpcode::TensorDot {
                        left_reg: 1,
                        right_reg: 2,
                        dim: 64,
                    },
                    si_ir::NativeTypeLattice::TensorType {
                        shape: vec![64, 64],
                        element_type: Box::new(si_ir::NativeTypeLattice::PrimitiveFloat { bits: 32 }),
                    },
                    0.015,
                ),
                _ => (
                    si_ir::MachineOpcode::Call {
                        function_id: node_id,
                        arg_regs: vec![],
                    },
                    si_ir::NativeTypeLattice::PrimitiveInt {
                        bits: 64,
                        signed: false,
                    },
                    0.001,
                ),
            };

            graph.add_node(si_ir::NativeComputationNode {
                id: node_id,
                opcode,
                type_lattice: lattice,
                energy_cost: energy,
                dependencies: dep_node_ids,
            });
        }

        if !step_to_node_id.is_empty() {
            graph.entry_node = 1;
            graph.exit_node = step_to_node_id.len() as u64;
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_workflow_dag_dependency_resolution() {
        let mut workflow = WorkflowGraph::new("wf_code_adaptation");

        workflow.add_step("step_1", "Synthesizer", "DecompileIntent", "{}", vec![], 2);
        workflow.add_step("step_2", "Fabricator", "ForgePatch", "{}", vec!["step_1".to_string()], 2);
        workflow.add_step("step_3", "Sentinel", "SecurityAudit", "{}", vec!["step_2".to_string()], 1);

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
        workflow.add_step("step_1", "Fabricator", "Compile", "{}", vec![], 1);
        workflow.add_step("step_2", "Sentinel", "Audit", "{}", vec!["step_1".to_string()], 1);

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
        workflow.add_step("s1", "Synthesizer", "Analyze", "data", vec![], 1);
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
        workflow.add_step("s1", "Orchestrator", "Plan", "{}", vec![], 0);
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

    #[test]
    fn test_new_with_persist() {
        let path = std::env::temp_dir().join("test_wf_persist.json");
        let wf = WorkflowGraph::new_with_persist("wf_persist", path.clone());
        assert_eq!(wf.workflow_id, "wf_persist");
        assert_eq!(wf.persist_path, Some(path));
    }

    #[test]
    fn test_save_default_creates_directory() {
        let dir = std::env::temp_dir().join("aaroneous_test_save_default");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut wf = WorkflowGraph::new("wf_default");
        wf.add_step("s1", "Synthesizer", "Analyze", "{}", vec![], 0);
        wf.save_default(&dir).unwrap();

        let loaded = WorkflowGraph::load_default(&dir, "wf_default").unwrap();
        assert_eq!(loaded.workflow_id, "wf_default");
        assert_eq!(loaded.steps.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_default_nonexistent() {
        let dir = std::env::temp_dir().join("aaroneous_test_load_nonexist");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = WorkflowGraph::load_default(&dir, "nonexistent");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_no_persist_path_fails() {
        let wf = WorkflowGraph::new("no_path");
        let result = wf.save();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No persist_path"));
    }

    #[test]
    fn test_parallel_ready_steps() {
        let mut wf = WorkflowGraph::new("parallel");
        wf.add_step("a", "Synthesizer", "Analyze", "{}", vec![], 0);
        wf.add_step("b", "Sentinel", "Audit", "{}", vec![], 0);
        wf.add_step("c", "Fabricator", "Build", "{}", vec!["a".to_string(), "b".to_string()], 0);

        let ready = wf.get_ready_steps();
        assert_eq!(ready.len(), 2); // Both a and b should be ready

        wf.complete_step("a");
        let ready = wf.get_ready_steps();
        assert_eq!(ready.len(), 1); // Only b
        assert_eq!(ready[0].step_id, "b");

        wf.complete_step("b");
        let ready = wf.get_ready_steps();
        assert_eq!(ready.len(), 1); // Now c
        assert_eq!(ready[0].step_id, "c");
    }

    #[test]
    fn test_retry_exhaustion_sets_failed() {
        let mut wf = WorkflowGraph::new("retry");
        wf.add_step("s1", "Fabricator", "Compile", "{}", vec![], 2); // max 2 retries

        wf.fail_step("s1", "Error 1"); // retry_count=1, status=Pending
        assert_eq!(wf.steps.get("s1").unwrap().retry_count, 1);
        assert_eq!(wf.steps.get("s1").unwrap().status, StepStatus::Pending);

        wf.fail_step("s1", "Error 2"); // retry_count=2, status=Pending
        assert_eq!(wf.steps.get("s1").unwrap().retry_count, 2);
        assert_eq!(wf.steps.get("s1").unwrap().status, StepStatus::Pending);

        wf.fail_step("s1", "Error 3"); // retry_count=2 (capped), status=Failed
        assert!(wf.is_failed);
        assert!(matches!(wf.steps.get("s1").unwrap().status, StepStatus::Failed(_)));
    }

    #[test]
    fn test_rollback_preserves_completed() {
        let mut wf = WorkflowGraph::new("rollback_preserve");
        wf.add_step("s1", "Synthesizer", "Plan", "{}", vec![], 0);
        wf.add_step("s2", "Fabricator", "Build", "{}", vec!["s1".to_string()], 0);

        wf.complete_step("s1");
        wf.fail_step("s2", "fatal"); // triggers rollback

        // s1 should remain Completed
        assert_eq!(wf.steps.get("s1").unwrap().status, StepStatus::Completed);
        // s2 should be Failed
        assert!(matches!(wf.steps.get("s2").unwrap().status, StepStatus::Failed(_)));
    }

    #[test]
    fn test_serialize_preserves_all_fields() {
        let mut wf = WorkflowGraph::new("full_serialize");
        wf.add_step("s1", "Orchestrator", "Orchestrate", "payload1", vec!["dep1".to_string()], 3);
        wf.steps.get_mut("s1").unwrap().status = StepStatus::Running;
        wf.steps.get_mut("s1").unwrap().retry_count = 1;

        let json = wf.serialize().unwrap();
        let loaded: WorkflowGraph = serde_json::from_str(&json).unwrap();

        let step = loaded.steps.get("s1").unwrap();
        assert_eq!(step.assigned_specialist, "Orchestrator");
        assert_eq!(step.action_name, "Orchestrate");
        assert_eq!(step.payload, "payload1");
        assert_eq!(step.max_retries, 3);
        assert_eq!(step.retry_count, 1);
        assert_eq!(step.status, StepStatus::Running);
    }

    #[test]
    fn test_list_persisted_empty_dir() {
        let dir = std::env::temp_dir().join("aaroneous_test_empty_wf");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let ids = WorkflowGraph::list_persisted(&dir).unwrap();
        assert!(ids.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_persisted_nonexistent_dir() {
        let dir = std::env::temp_dir().join("aaroneous_test_nonexist_wf_dir");
        let _ = fs::remove_dir_all(&dir);
        // Directory doesn't exist
        let ids = WorkflowGraph::list_persisted(&dir).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_workflow_prune_and_checkpoint() {
        let mut wf = WorkflowGraph::new("test_prune");
        wf.add_step("root", "Planner", "Plan", "{}", vec![], 2);
        wf.add_step("branch_a", "Worker", "Work", "{}", vec!["root".to_string()], 2);
        wf.add_step("branch_b", "Worker", "Work", "{}", vec!["branch_a".to_string()], 2);
        wf.add_step("independent", "Worker", "Work", "{}", vec![], 2);

        wf.complete_step("independent");
        let cp = wf.checkpoint();
        assert_eq!(cp, vec!["independent".to_string()]);

        let pruned = wf.prune_branch("branch_a");
        assert_eq!(pruned, 2); // branch_a and branch_b
        assert!(wf.steps.contains_key("root"));
        assert!(wf.steps.contains_key("independent"));
        assert!(!wf.steps.contains_key("branch_a"));
        assert!(!wf.steps.contains_key("branch_b"));
    }

    #[tokio::test]
    async fn test_concurrent_step_execution() {
        let mut wf = WorkflowGraph::new("test_concurrent");
        wf.add_step("step1", "WorkerA", "ProcessA", "{}", vec![], 2);
        wf.add_step("step2", "WorkerB", "ProcessB", "{}", vec![], 2);
        wf.add_step("step3", "Aggregator", "Aggregate", "{}", vec!["step1".to_string(), "step2".to_string()], 2);

        // First round: step1 and step2 are ready and run concurrently
        let executed = wf
            .execute_ready_steps_concurrent(|step| async move {
                (step.step_id, Ok(()))
            })
            .await;

        assert_eq!(executed, 2);
        assert_eq!(wf.steps["step1"].status, StepStatus::Completed);
        assert_eq!(wf.steps["step2"].status, StepStatus::Completed);
        assert_eq!(wf.steps["step3"].status, StepStatus::Pending);

        // Second round: step3 dependencies now satisfied
        let executed2 = wf
            .execute_ready_steps_concurrent(|step| async move {
                (step.step_id, Ok(()))
            })
            .await;

        assert_eq!(executed2, 1);
        assert!(wf.is_completed);
    }

    #[test]
    fn test_resolve_unassigned_steps_via_mdp() {
        let mut wf = WorkflowGraph::new("test_mdp_routing");
        wf.add_step("step_explicit", "Fabricator", "Compile", "{}", vec![], 2);
        wf.add_step("step_auto", "auto", "AuditSecurity", "{}", vec![], 2);

        let mut router = crate::mdps_router::TaskRoutingEngine::new(vec![
            crate::mdps_router::Specialist {
                id: "spec_sentinel".to_string(),
                name: "Sentinel".to_string(),
                skills: vec!["AuditSecurity".to_string()],
                capacity: 1.0,
                success_rate: 0.95,
                avg_completion_time: 2.0,
            },
        ]);

        let resolved = wf.resolve_unassigned_steps_via_mdp(&mut router);
        assert_eq!(resolved, 1);
        assert_eq!(wf.steps["step_explicit"].assigned_specialist, "Fabricator");
        assert_eq!(wf.steps["step_auto"].assigned_specialist, "Sentinel");
    }

    #[test]
    fn test_to_computational_graph_conversion() {
        let mut wf = WorkflowGraph::new("test_ir_conversion");
        wf.add_step("s1", "Fabricator", "Alloc", "payload_data", vec![], 2);
        wf.add_step("s2", "Synthesizer", "TensorDot", "{}", vec!["s1".to_string()], 2);

        let graph = wf.to_computational_graph();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.entry_node, 1);
        assert_eq!(graph.exit_node, 2);
        assert!(graph.thermodynamic_free_energy > 0.0);

        // Check topological dependency resolution
        let node_s2 = graph.nodes.values().find(|n| matches!(n.opcode, si_ir::MachineOpcode::TensorDot { .. })).unwrap();
        assert_eq!(node_s2.dependencies.len(), 1);
    }
}
