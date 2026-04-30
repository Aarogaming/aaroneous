/// Action executor: Converts WASM marionette actions to O3DE effects
/// 
/// Responsibility: Deserialize action bytes, execute via appropriate system, return status

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json;

/// Marionette action types (WASM → O3DE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarionetteAction {
    /// Mouse movement
    MouseMove { x: i32, y: i32 },
    
    /// Mouse click
    MouseClick { button: MouseButton, x: i32, y: i32 },
    
    /// Mouse release
    MouseRelease { button: MouseButton },
    
    /// Keyboard key press
    KeyPress { key: u32 },
    
    /// Keyboard key release
    KeyRelease { key: u32 },
    
    /// Mouse scroll
    Scroll { delta: i32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Action execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStatus {
    Success,
    InvalidAction,
    ExecutionError { reason: String },
    PlatformNotSupported,
}

impl Default for ActionStatus {
    fn default() -> Self {
        ActionStatus::Success
    }
}

/// Executes marionette actions
pub struct ActionExecutor {
    /// Enable/disable action execution (for safety)
    enabled: std::sync::atomic::AtomicBool,
}

impl ActionExecutor {
    /// Create new action executor
    pub fn new() -> Self {
        Self {
            enabled: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    /// Enable/disable execution
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Execute action (WASM → O3DE)
    pub async fn execute(&self, action_bytes: &[u8]) -> Result<Bytes, String> {
        // Check if enabled
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(Bytes::from_static(b"\x00")); // Status: disabled
        }
        
        // Deserialize action
        let action: MarionetteAction = serde_json::from_slice(action_bytes)
            .map_err(|e| format!("Failed to deserialize action: {}", e))?;
        
        // Execute based on action type
        let status = self.execute_action(&action).await?;
        
        // Serialize status back
        let response = serde_json::to_vec(&status)
            .map_err(|e| format!("Failed to encode response: {}", e))?;
        
        Ok(Bytes::from(response))
    }
    
    async fn execute_action(&self, action: &MarionetteAction) -> Result<ActionStatus, String> {
        match action {
            MarionetteAction::MouseMove { x, y } => {
                self.execute_mouse_move(*x, *y).await
            }
            MarionetteAction::MouseClick { button, x, y } => {
                self.execute_mouse_click(*button, *x, *y).await
            }
            MarionetteAction::MouseRelease { button } => {
                self.execute_mouse_release(*button).await
            }
            MarionetteAction::KeyPress { key } => {
                self.execute_key_press(*key).await
            }
            MarionetteAction::KeyRelease { key } => {
                self.execute_key_release(*key).await
            }
            MarionetteAction::Scroll { delta } => {
                self.execute_scroll(*delta).await
            }
        }
    }
    
    async fn execute_mouse_move(&self, x: i32, y: i32) -> Result<ActionStatus, String> {
        // TODO: Implementation
        // On Windows: Use SetCursorPos
        // On Linux: Use uinput or xdotool
        // For now: simulate success
        
        log::debug!("MouseMove({}, {})", x, y);
        Ok(ActionStatus::Success)
    }
    
    async fn execute_mouse_click(
        &self,
        _button: MouseButton,
        x: i32,
        y: i32,
    ) -> Result<ActionStatus, String> {
        // TODO: Implementation
        log::debug!("MouseClick({:?}, {}, {})", _button, x, y);
        Ok(ActionStatus::Success)
    }
    
    async fn execute_mouse_release(&self, _button: MouseButton) -> Result<ActionStatus, String> {
        // TODO: Implementation
        log::debug!("MouseRelease({:?})", _button);
        Ok(ActionStatus::Success)
    }
    
    async fn execute_key_press(&self, key: u32) -> Result<ActionStatus, String> {
        // TODO: Implementation
        log::debug!("KeyPress({})", key);
        Ok(ActionStatus::Success)
    }
    
    async fn execute_key_release(&self, key: u32) -> Result<ActionStatus, String> {
        // TODO: Implementation
        log::debug!("KeyRelease({})", key);
        Ok(ActionStatus::Success)
    }
    
    async fn execute_scroll(&self, delta: i32) -> Result<ActionStatus, String> {
        // TODO: Implementation
        log::debug!("Scroll({})", delta);
        Ok(ActionStatus::Success)
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_action_executor_disabled() {
        let executor = ActionExecutor::new();
        
        let action = MarionetteAction::MouseMove { x: 100, y: 200 };
        let action_bytes = serde_json::to_vec(&action)
            .expect("Encode failed");
        
        let result = executor.execute(&action_bytes).await;
        
        // Should return success even though disabled (returns 0x00)
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_action_executor_enabled() {
        let executor = ActionExecutor::new();
        executor.set_enabled(true);
        
        let action = MarionetteAction::KeyPress { key: 65 };
        let action_bytes = serde_json::to_vec(&action)
            .expect("Encode failed");
        
        let result = executor.execute(&action_bytes).await;
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_marionette_action_serialization() {
        let action = MarionetteAction::MouseMove { x: 100, y: 200 };
        
        let bytes = serde_json::to_vec(&action)
            .expect("Encode failed");
        
        let decoded: MarionetteAction = serde_json::from_slice(&bytes)
            .expect("Decode failed");
        
        match decoded {
            MarionetteAction::MouseMove { x, y } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Wrong action type"),
        }
    }
}
