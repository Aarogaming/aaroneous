// Action Executor moved from core/hypervisor
// This file is identical to the original src/action_executor.rs

use crate::decision_engine::{Action, TaskEvaluation};
use biology::SystemBiology;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Execution statistics for tracking action outcomes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful: u64,
    pub failed: u64,
}

/// Types of actions the executor can perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableAction {
    FileOperation {
        path: PathBuf,
        operation: FileOp,
        content: Option<String>,
    },
    ExecuteMicroBytecode {
        program: crate::micro_vm::VmProgram,
        gas_limit: Option<u64>,
    },
    SpawnWasm {
        enzyme_path: PathBuf,
        input_data: Vec<u8>,
    },
    ThrottleSystem {
        new_rate: f32,
        reason: String,
    },
    NotifyUser {
        message: String,
        priority: u8,
    },
    RequestMutation {
        specialist_id: String,
        goal: String,
    },
    ScaleSpecialist {
        specialist_id: String,
        scale_factor: f32,
    },
}

/// File operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOp {
    Create,
    Modify,
    Delete,
    Move(PathBuf),
    Copy(PathBuf),
}

/// Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_type: String,
    pub success: bool,
    pub duration_ms: f64,
    pub message: String,
    pub metadata: serde_json::Value,
}

/// Action Executor - executes decisions and tracks outcomes
pub struct ActionExecutor {
    pub biology: SystemBiology,
    pub wasm_path: PathBuf,
    pub allowed_roots: Vec<PathBuf>,
    pub execution_history: Vec<ActionResult>,
    pub max_history: usize,
}

impl ActionExecutor {
    pub fn new(wasm_path: PathBuf) -> Self {
        let mut allowed_roots = Vec::new();
        let ws = aaroneous_paths::WorkspacePaths::discover();
        allowed_roots.push(ws.root().clone());
        allowed_roots.push(std::env::temp_dir());
        Self {
            biology: SystemBiology::new(),
            wasm_path,
            allowed_roots,
            execution_history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn from_decision(_decision: &Action) -> Self {
        Self::new(PathBuf::from("/tmp"))
    }

    pub fn execute(&self, _action: &ExecutableAction) -> ActionResult {
        let stats = self.get_stats();
        ActionResult {
            action_type: "stub".to_string(),
            success: true,
            duration_ms: 0.0,
            message: "Execution stub".to_string(),
            metadata: serde_json::json!({ "stats": stats }),
        }
    }

    pub fn get_stats(&self) -> ExecutionStats {
        let total = self.execution_history.len() as u64;
        let successful = self.execution_history.iter().filter(|r| r.success).count() as u64;
        ExecutionStats {
            total_executions: total,
            successful,
            failed: total.saturating_sub(successful),
        }
    }
}
