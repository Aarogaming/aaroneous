use crate::wasm_ebus_bridge::action_executor::ActionExecutor;
use crate::wasm_ebus_bridge::marionette::MarionetteAction;
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct MarionetteBridge {
    executor: Arc<ActionExecutor>,
}

impl MarionetteBridge {
    pub fn new(executor: Arc<ActionExecutor>) -> Self {
        Self { executor }
    }

    pub async fn send_action(&self, action: MarionetteAction) -> Result<(), String> {
        let bytes = serde_json::to_vec(&action)
            .map_err(|e| format!("Failed to serialize action: {}", e))?;
        self.executor.execute(&bytes).await?;
        Ok(())
    }
}
