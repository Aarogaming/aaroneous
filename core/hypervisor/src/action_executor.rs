// Action Executor
// Executes decisions made by the decision engine: file ops, throttling, notifications

use crate::constellation_ui::{ConstellationCanvas, NodeMetrics};
use crate::decision_engine::{Action, TaskEvaluation};
use biology::SystemBiology;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    UpdateConstellation {
        node_id: String,
        metrics: NodeMetrics,
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
    pub constellation: ConstellationCanvas,
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
            constellation: ConstellationCanvas::new(),
            wasm_path,
            allowed_roots,
            execution_history: Vec::new(),
            max_history: 100,
        }
    }

    /// Verifies that a target file path is safely confined within allowed workspace roots
    pub fn validate_sandbox_path(&self, path: &Path) -> Result<PathBuf, String> {
        // Disallow relative parent directory traversal
        if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(format!("Parent directory traversal ('..') is disallowed: {}", path.display()));
        }

        // Canonicalize or normalize path
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            aaroneous_paths::WorkspacePaths::discover().root().join(path)
        };

        // If path exists, check canonical path against allowed roots
        let canonical_check = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        let is_allowed = self.allowed_roots.iter().any(|root| {
            let root_canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
            canonical_check.starts_with(&root_canonical) || resolved.starts_with(root)
        });

        if !is_allowed {
            return Err(format!(
                "Security violation: path '{}' is outside allowed workspace boundaries",
                path.display()
            ));
        }

        Ok(resolved)
    }

    /// Execute a single action
    pub async fn execute(&mut self, action: ExecutableAction) -> ActionResult {
        let start = std::time::Instant::now();

        let result = match action {
            ExecutableAction::FileOperation {
                path,
                operation,
                content,
            } => self.execute_file_operation(&path, operation, content.as_deref()),
            ExecutableAction::ExecuteMicroBytecode { program, gas_limit } => {
                self.execute_micro_bytecode(&program, gas_limit)
            }
            ExecutableAction::SpawnWasm {
                enzyme_path,
                input_data,
            } => self.spawn_wasm_enzyme(&enzyme_path, &input_data).await,
            ExecutableAction::ThrottleSystem { new_rate, reason } => {
                self.throttle_system(new_rate, &reason)
            }
            ExecutableAction::NotifyUser { message, priority } => {
                self.notify_user(&message, priority)
            }
            ExecutableAction::RequestMutation {
                specialist_id,
                goal,
            } => self.request_mutation(&specialist_id, &goal),
            ExecutableAction::UpdateConstellation { node_id, metrics } => {
                self.update_constellation_node(&node_id, metrics)
            }
            ExecutableAction::ScaleSpecialist {
                specialist_id,
                scale_factor,
            } => self.scale_specialist(&specialist_id, scale_factor),
        };

        let _duration = start.elapsed().as_secs_f64() * 1000.0;

        // Record in history
        if self.execution_history.len() >= self.max_history {
            self.execution_history.remove(0);
        }
        self.execution_history.push(result.clone());

        result
    }

    /// Execute a file operation with strict sandbox confinement validation
    fn execute_file_operation(
        &self,
        path: &Path,
        operation: FileOp,
        content: Option<&str>,
    ) -> ActionResult {
        // Validate source path
        let target_path = match self.validate_sandbox_path(path) {
            Ok(p) => p,
            Err(err) => {
                return ActionResult {
                    action_type: "file_operation".to_string(),
                    success: false,
                    duration_ms: 0.0,
                    message: format!("Sandbox security violation: {}", err),
                    metadata: serde_json::json!({"path": path.to_string_lossy(), "security_error": true}),
                };
            }
        };

        // Validate destination path for move/copy
        if let FileOp::Move(ref dest) | FileOp::Copy(ref dest) = operation
            && let Err(err) = self.validate_sandbox_path(dest)
        {
            return ActionResult {
                action_type: "file_operation".to_string(),
                success: false,
                duration_ms: 0.0,
                message: format!("Sandbox security violation on destination: {}", err),
                metadata: serde_json::json!({"dest": dest.to_string_lossy(), "security_error": true}),
            };
        }

        let result = match operation {
            FileOp::Create => {
                if let Some(content) = content {
                    fs::write(&target_path, content).map(|_| ())
                } else {
                    fs::write(&target_path, "").map(|_| ())
                }
            }
            FileOp::Modify => {
                if let Some(content) = content {
                    fs::write(&target_path, content).map(|_| ())
                } else {
                    Ok(())
                }
            }
            FileOp::Delete => fs::remove_file(&target_path),
            FileOp::Move(ref dest) => fs::rename(&target_path, dest),
            FileOp::Copy(ref dest) => fs::copy(&target_path, dest).map(|_| ()),
        };

        match result {
            Ok(_) => ActionResult {
                action_type: "file_operation".to_string(),
                success: true,
                duration_ms: 0.0,
                message: format!("File operation completed: {:?}", operation),
                metadata: serde_json::json!({"path": path.to_string_lossy()}),
            },
            Err(e) => ActionResult {
                action_type: "file_operation".to_string(),
                success: false,
                duration_ms: 0.0,
                message: format!("File operation failed: {}", e),
                metadata: serde_json::json!({"path": path.to_string_lossy(), "error": e.to_string()}),
            },
        }
    }

    /// Executes a sandboxed micro-worker bytecode program with strict gas and memory limits
    fn execute_micro_bytecode(
        &self,
        program: &crate::micro_vm::VmProgram,
        gas_limit: Option<u64>,
    ) -> ActionResult {
        let gas = gas_limit.unwrap_or(crate::micro_vm::DEFAULT_GAS_LIMIT);
        let mut vm = crate::micro_vm::MicroBytecodeVm::with_limits(
            gas,
            crate::micro_vm::DEFAULT_MEMORY_LIMIT,
        );
        let start = std::time::Instant::now();
        match vm.execute(program) {
            Ok(result) => ActionResult {
                action_type: "execute_micro_bytecode".to_string(),
                success: true,
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                message: format!(
                    "Micro-bytecode execution succeeded: {} instructions executed, {} gas consumed",
                    result.instructions_executed, result.gas_consumed
                ),
                metadata: serde_json::json!({
                    "instructions_executed": result.instructions_executed,
                    "gas_consumed": result.gas_consumed,
                    "gas_remaining": result.gas_remaining,
                    "r0": result.registers[0],
                }),
            },
            Err(e) => ActionResult {
                action_type: "execute_micro_bytecode".to_string(),
                success: false,
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                message: format!("Micro-bytecode execution error: {}", e),
                metadata: serde_json::json!({
                    "error": e.to_string(),
                }),
            },
        }
    }

    /// WASM enzyme spawning is no longer supported (wasmtime removed).
    async fn spawn_wasm_enzyme(&self, enzyme_path: &Path, input_data: &[u8]) -> ActionResult {
        ActionResult {
            action_type: "spawn_wasm".to_string(),
            success: false,
            duration_ms: 0.0,
            message: format!(
                "WASM enzyme execution is not available (wasmtime removed). \
                 Enzyme path: {}, input size: {} bytes",
                enzyme_path.display(),
                input_data.len()
            ),
            metadata: serde_json::json!({
                "path": enzyme_path.display().to_string(),
                "input_size": input_data.len(),
            }),
        }
    }

    /// Throttle the system
    fn throttle_system(&mut self, new_rate: f32, reason: &str) -> ActionResult {
        let old_rate = self.biology.expression_rate;
        self.biology.set_expression_rate(new_rate);

        ActionResult {
            action_type: "throttle_system".to_string(),
            success: true,
            duration_ms: 0.0,
            message: format!(
                "Throttled from {:.2} to {:.2}: {}",
                old_rate, new_rate, reason
            ),
            metadata: serde_json::json!({
                "old_rate": old_rate,
                "new_rate": new_rate,
                "reason": reason,
            }),
        }
    }

    /// Notify the user
    fn notify_user(&self, message: &str, priority: u8) -> ActionResult {
        println!("[NOTIFY] (priority: {}) {}", priority, message);

        ActionResult {
            action_type: "notify_user".to_string(),
            success: true,
            duration_ms: 0.0,
            message: message.to_string(),
            metadata: serde_json::json!({"priority": priority}),
        }
    }

    /// Request a mutation for a specialist
    fn request_mutation(&self, specialist_id: &str, goal: &str) -> ActionResult {
        self.biology.request_mutation(specialist_id, goal);

        ActionResult {
            action_type: "request_mutation".to_string(),
            success: true,
            duration_ms: 0.0,
            message: format!(
                "Mutation requested for {} with goal: {}",
                specialist_id, goal
            ),
            metadata: serde_json::json!({
                "specialist_id": specialist_id,
                "goal": goal,
            }),
        }
    }

    /// Update a constellation node with new metrics
    fn update_constellation_node(&mut self, node_id: &str, metrics: NodeMetrics) -> ActionResult {
        let metrics_clone = metrics.clone();

        if let Some(index) = self
            .constellation
            .nodes
            .iter()
            .position(|n| n.id == node_id)
        {
            self.constellation.update_node_metrics(index, metrics);

            ActionResult {
                action_type: "update_constellation".to_string(),
                success: true,
                duration_ms: 0.0,
                message: format!("Updated metrics for node {}", node_id),
                metadata: serde_json::json!({
                    "node_id": node_id,
                    "entropy": metrics_clone.entropy,
                    "confidence": metrics_clone.confidence,
                }),
            }
        } else {
            ActionResult {
                action_type: "update_constellation".to_string(),
                success: false,
                duration_ms: 0.0,
                message: format!("Node not found: {}", node_id),
                metadata: serde_json::json!({"node_id": node_id}),
            }
        }
    }

    /// Scale a specialist's capacity
    fn scale_specialist(&mut self, specialist_id: &str, scale_factor: f32) -> ActionResult {
        if let Some(metabolism) = self.biology.specialist_metabolism.get_mut(specialist_id) {
            metabolism.max_tokens *= scale_factor;
            metabolism.regen_rate *= scale_factor;

            ActionResult {
                action_type: "scale_specialist".to_string(),
                success: true,
                duration_ms: 0.0,
                message: format!("Scaled {} by {:.2}x", specialist_id, scale_factor),
                metadata: serde_json::json!({
                    "specialist_id": specialist_id,
                    "scale_factor": scale_factor,
                }),
            }
        } else {
            ActionResult {
                action_type: "scale_specialist".to_string(),
                success: false,
                duration_ms: 0.0,
                message: format!("Specialist not found: {}", specialist_id),
                metadata: serde_json::json!({"specialist_id": specialist_id}),
            }
        }
    }

    /// Convert a decision engine action to an executable action
    pub fn from_decision(
        evaluation: &TaskEvaluation,
        action: &Action,
        file_path: Option<PathBuf>,
    ) -> Option<ExecutableAction> {
        match action {
            Action::ExecuteImmediately => file_path.map(|path| ExecutableAction::FileOperation {
                path,
                operation: FileOp::Modify,
                content: None,
            }),
            Action::DelegateToWASM => Some(ExecutableAction::SpawnWasm {
                enzyme_path: PathBuf::from(
                    "extensions/wasm/test_enzyme/target/wasm32-unknown-unknown/release/test_enzyme.wasm",
                ),
                input_data: vec![],
            }),
            Action::QueueForLater => {
                // Just notify for now
                Some(ExecutableAction::NotifyUser {
                    message: format!("Task {} queued for later", evaluation.task_id),
                    priority: 1,
                })
            }
            Action::RequestHumanInput => Some(ExecutableAction::NotifyUser {
                message: format!(
                    "Task {} requires human input: {}",
                    evaluation.task_id, evaluation.reasoning
                ),
                priority: 2,
            }),
            Action::Reject => Some(ExecutableAction::NotifyUser {
                message: format!(
                    "Task {} rejected: {}",
                    evaluation.task_id, evaluation.reasoning
                ),
                priority: 3,
            }),
        }
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> ExecutionStats {
        let total = self.execution_history.len();
        let success = self.execution_history.iter().filter(|r| r.success).count();
        let failed = total - success;

        let avg_duration = if total > 0 {
            self.execution_history
                .iter()
                .map(|r| r.duration_ms)
                .sum::<f64>()
                / total as f64
        } else {
            0.0
        };

        ExecutionStats {
            total_executions: total,
            success_count: success,
            failed_count: failed,
            success_rate: if total > 0 {
                success as f64 / total as f64
            } else {
                0.0
            },
            avg_duration_ms: avg_duration,
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executions: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = ActionExecutor::new(PathBuf::from("test.wasm"));
        assert!(executor.execution_history.is_empty());
    }

    #[test]
    fn test_throttle_system() {
        let mut executor = ActionExecutor::new(PathBuf::from("test.wasm"));
        let result = executor.throttle_system(0.5, "test reason");

        assert!(result.success);
        assert_eq!(executor.biology.expression_rate, 0.5);
    }

    #[test]
    fn test_notify_user() {
        let executor = ActionExecutor::new(PathBuf::from("test.wasm"));
        let result = executor.notify_user("Test message", 1);

        assert!(result.success);
        assert_eq!(result.message, "Test message");
    }

    #[test]
    fn test_execution_stats() {
        let executor = ActionExecutor::new(PathBuf::from("test.wasm"));
        let stats = executor.get_stats();

        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_sandbox_path_containment_and_rejection() {
        let executor = ActionExecutor::new(PathBuf::from("test.wasm"));

        // Path traversal should be rejected
        let traversal_path = PathBuf::from("../../../etc/passwd");
        assert!(executor.validate_sandbox_path(&traversal_path).is_err());

        // File operation with path escape returns failure action result
        let action = ExecutableAction::FileOperation {
            path: traversal_path,
            operation: FileOp::Create,
            content: Some("malicious content".to_string()),
        };
        let mut exec = ActionExecutor::new(PathBuf::from("test.wasm"));
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let res = rt.block_on(exec.execute(action));
        assert!(!res.success);
        assert!(res.message.contains("Sandbox security violation"));
    }

    #[test]
    fn test_micro_bytecode_action_execution() {
        let mut executor = ActionExecutor::new(PathBuf::from("test.wasm"));
        let program = crate::micro_vm::VmProgram::new(vec![
            crate::micro_vm::VmInstruction::MovImm { dst: 0, val: 100 },
            crate::micro_vm::VmInstruction::MovImm { dst: 1, val: 250 },
            crate::micro_vm::VmInstruction::Add { dst: 0, src: 1 },
            crate::micro_vm::VmInstruction::Halt,
        ]);

        let action = ExecutableAction::ExecuteMicroBytecode {
            program,
            gas_limit: Some(10_000),
        };

        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let res = rt.block_on(executor.execute(action));
        assert!(res.success);
        assert!(res.message.contains("Micro-bytecode execution succeeded"));
        assert_eq!(res.metadata["r0"], 350);
    }
}
