pub struct ActionExecutor;
impl Default for ActionExecutor {
    fn default() -> Self {
        Self
    }
}
impl ActionExecutor {
    pub async fn execute(&self, _action: &str) -> Result<ActionResult, anyhow::Error> {
        Ok(ActionResult::Success)
    }
}
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success,
    Failure(String),
    Pending,
}
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    pub success_count: u64,
    pub failure_count: u64,
}
