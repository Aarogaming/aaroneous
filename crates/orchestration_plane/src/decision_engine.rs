#[derive(Debug, Clone)]
pub enum DecisionTask {
    ProcessMetadata,
    EvaluateCapability,
    ExecuteAction,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ExecutionError(pub String);

impl ExecutionError {
    pub fn from(err: anyhow::Error) -> Self {
        ExecutionError(err.to_string())
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExecutionError {}

#[derive(Debug, Clone)]
pub enum ExecutionOutcome {
    Success(crate::action_executor::ActionResult),
    Failure(ExecutionError),
}

impl std::fmt::Display for ExecutionOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ExecutionOutcome::Success(_) => write!(f, "success"),
            ExecutionOutcome::Failure(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskEvaluation {
    Complete,
    Pending,
    Failed,
}

pub struct AutonomousDecisionEngine;
impl Default for AutonomousDecisionEngine {
    fn default() -> Self {
        Self
    }
}
