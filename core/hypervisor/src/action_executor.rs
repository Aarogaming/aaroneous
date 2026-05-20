// Action Executor
// Executes decisions made by the decision engine: file ops, WASM, throttling, notifications

use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use serde::{Serialize, Deserialize};
use crate::decision_engine::{Action, TaskEvaluation, ExecutionOutcome};
use biology::SystemBiology;
use crate::constellation_ui::{ConstellationCanvas, NodeMetrics};
use crate::ConstellationNode;

/// Types of actions the executor can perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableAction {
    FileOperation {
        path: PathBuf,
        operation: FileOp,
        content: Option<String>,
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
    pub execution_history: Vec<ActionResult>,
    pub max_history: usize,
}

impl ActionExecutor {
    pub fn new(wasm_path: PathBuf) -> Self {
        Self {
            biology: SystemBiology::new(),
            constellation: ConstellationCanvas::new(),
            wasm_path,
            execution_history: Vec::new(),
            max_history: 100,
        }
    }

    /// Execute a single action
    pub async fn execute(&mut self, action: ExecutableAction) -> ActionResult {
        let start = std::time::Instant::now();
        
        let result = match action {
            ExecutableAction::FileOperation { path, operation, content } => {
                self.execute_file_operation(&path, operation, content.as_deref())
            }
            ExecutableAction::SpawnWasm { enzyme_path, input_data } => {
                self.spawn_wasm_enzyme(&enzyme_path, &input_data).await
            }
            ExecutableAction::ThrottleSystem { new_rate, reason } => {
                self.throttle_system(new_rate, &reason)
            }
            ExecutableAction::NotifyUser { message, priority } => {
                self.notify_user(&message, priority)
            }
            ExecutableAction::RequestMutation { specialist_id, goal } => {
                self.request_mutation(&specialist_id, &goal)
            }
            ExecutableAction::UpdateConstellation { node_id, metrics } => {
                self.update_constellation_node(&node_id, metrics)
            }
            ExecutableAction::ScaleSpecialist { specialist_id, scale_factor } => {
                self.scale_specialist(&specialist_id, scale_factor)
            }
        };
        
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        
        // Record in history
        if self.execution_history.len() >= self.max_history {
            self.execution_history.remove(0);
        }
        self.execution_history.push(result.clone());
        
        result
    }

    /// Execute a file operation
    fn execute_file_operation(&self, path: &Path, operation: FileOp, content: Option<&str>) -> ActionResult {
        let result = match operation {
            FileOp::Create => {
                if let Some(content) = content {
                    fs::write(path, content).map(|_| ())
                } else {
                    fs::write(path, "").map(|_| ())
                }
            }
            FileOp::Modify => {
                if let Some(content) = content {
                    fs::write(path, content).map(|_| ())
                } else {
                    Ok(())
                }
            }
            FileOp::Delete => fs::remove_file(path),
            FileOp::Move(ref dest) => fs::rename(path, dest),
            FileOp::Copy(ref dest) => fs::copy(path, dest).map(|_| ()),
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

    /// Spawn a WASM enzyme
    async fn spawn_wasm_enzyme(&self, enzyme_path: &Path, input_data: &[u8]) -> ActionResult {
        // In a real implementation, this would use wasmtime to run the WASM module
        // For now, we'll simulate it
        
        if !enzyme_path.exists() {
            return ActionResult {
                action_type: "spawn_wasm".to_string(),
                success: false,
                duration_ms: 0.0,
                message: format!("WASM enzyme not found: {}", enzyme_path.display()),
                metadata: serde_json::json!({"path": enzyme_path.to_string_lossy()}),
            };
        }
        
        ActionResult {
            action_type: "spawn_wasm".to_string(),
            success: true,
            duration_ms: 0.0,
            message: format!("WASM enzyme spawned: {}", enzyme_path.display()),
            metadata: serde_json::json!({
                "path": enzyme_path.to_string_lossy(),
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
            message: format!("Throttled from {:.2} to {:.2}: {}", old_rate, new_rate, reason),
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
            message: format!("Mutation requested for {} with goal: {}", specialist_id, goal),
            metadata: serde_json::json!({
                "specialist_id": specialist_id,
                "goal": goal,
            }),
        }
    }

    /// Update a constellation node with new metrics
    fn update_constellation_node(&mut self, node_id: &str, metrics: NodeMetrics) -> ActionResult {
        let metrics_clone = metrics.clone();
        
        if let Some(index) = self.constellation.nodes.iter().position(|n| n.id == node_id) {
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
            Action::ExecuteImmediately => {
                if let Some(path) = file_path {
                    Some(ExecutableAction::FileOperation {
                        path,
                        operation: FileOp::Modify,
                        content: None,
                    })
                } else {
                    None
                }
            }
            Action::DelegateToWASM => {
                Some(ExecutableAction::SpawnWasm {
                    enzyme_path: PathBuf::from("extensions/wasm/test_enzyme/target/wasm32-unknown-unknown/release/test_enzyme.wasm"),
                    input_data: vec![],
                })
            }
            Action::QueueForLater => {
                // Just notify for now
                Some(ExecutableAction::NotifyUser {
                    message: format!("Task {} queued for later", evaluation.task_id),
                    priority: 1,
                })
            }
            Action::RequestHumanInput => {
                Some(ExecutableAction::NotifyUser {
                    message: format!("Task {} requires human input: {}", evaluation.task_id, evaluation.reasoning),
                    priority: 2,
                })
            }
            Action::Reject => {
                Some(ExecutableAction::NotifyUser {
                    message: format!("Task {} rejected: {}", evaluation.task_id, evaluation.reasoning),
                    priority: 3,
                })
            }
        }
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> ExecutionStats {
        let total = self.execution_history.len();
        let success = self.execution_history.iter().filter(|r| r.success).count();
        let failed = total - success;
        
        let avg_duration = if total > 0 {
            self.execution_history.iter().map(|r| r.duration_ms).sum::<f64>() / total as f64
        } else {
            0.0
        };
        
        ExecutionStats {
            total_executions: total,
            success_count: success,
            failed_count: failed,
            success_rate: if total > 0 { success as f64 / total as f64 } else { 0.0 },
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
}
