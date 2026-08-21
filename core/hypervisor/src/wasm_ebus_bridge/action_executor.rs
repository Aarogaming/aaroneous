use crate::hid_driver::{HidCommand, HidDriver, MouseButton};
/// Action executor: Converts WASM marionette actions to HID driver effects
///
/// Responsibility: Deserialize action bytes, execute via appropriate system, return status
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json;

/// Marionette action types (WASM → HID driver)
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

/// Action execution status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ActionStatus {
    #[default]
    Success,
    InvalidAction,
    ExecutionError {
        reason: String,
    },
    PlatformNotSupported,
}

/// Executes marionette actions
#[derive(Default)]
pub struct ActionExecutor {
    /// Enable/disable action execution (for safety)
    enabled: std::sync::atomic::AtomicBool,

    /// HID driver for input control (lazily initialized)
    hid_driver: parking_lot::Mutex<Option<HidDriver>>,
}

impl ActionExecutor {
    /// Create new action executor
    pub fn new() -> Self {
        Self {
            enabled: std::sync::atomic::AtomicBool::new(false),
            hid_driver: parking_lot::Mutex::new(None),
        }
    }

    /// Initialize HID driver (called on first action or explicitly)
    async fn ensure_hid_driver(&self) -> Result<(), String> {
        // Check if already initialized (brief lock)
        {
            let driver_opt = self.hid_driver.lock();
            if driver_opt.is_some() {
                return Ok(());
            }
        }

        // Initialize HID driver (no lock held during async init)
        let driver = HidDriver::new().await?;

        // Store the initialized driver
        {
            let mut driver_opt = self.hid_driver.lock();
            *driver_opt = Some(driver);
        }

        Ok(())
    }

    /// Enable/disable execution
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
    }

    /// Execute action (WASM → HID driver)
    pub async fn execute(&self, action_bytes: &[u8]) -> Result<Bytes, String> {
        // Deserialize action
        let action: MarionetteAction = serde_json::from_slice(action_bytes)
            .map_err(|e| format!("Failed to deserialize action: {}", e))?;

        // Execute based on action type
        let status = self.execute_action(&action).await?;

        // Serialize status back
        let response =
            serde_json::to_vec(&status).map_err(|e| format!("Failed to encode response: {}", e))?;

        Ok(Bytes::from(response))
    }

    async fn execute_action(&self, action: &MarionetteAction) -> Result<ActionStatus, String> {
        match action {
            MarionetteAction::MouseMove { x, y } => self.execute_mouse_move(*x, *y).await,
            MarionetteAction::MouseClick { button, x, y } => {
                self.execute_mouse_click(*button, *x, *y).await
            }
            MarionetteAction::MouseRelease { button } => self.execute_mouse_release(*button).await,
            MarionetteAction::KeyPress { key } => self.execute_key_press(*key).await,
            MarionetteAction::KeyRelease { key } => self.execute_key_release(*key).await,
            MarionetteAction::Scroll { delta } => self.execute_scroll(*delta).await,
        }
    }

    async fn execute_mouse_move(&self, x: i32, y: i32) -> Result<ActionStatus, String> {
        self.ensure_hid_driver().await?;

        let driver = {
            let driver_opt = self.hid_driver.lock();
            driver_opt.clone()
        };

        if let Some(d) = driver {
            let cmd = HidCommand::MouseMove { x, y };
            match d.execute(cmd).await {
                Ok(_) => Ok(ActionStatus::Success),
                Err(e) => Ok(ActionStatus::ExecutionError { reason: e }),
            }
        } else {
            Ok(ActionStatus::ExecutionError {
                reason: "HID driver not initialized".to_string(),
            })
        }
    }

    async fn execute_mouse_click(
        &self,
        button: MouseButton,
        x: i32,
        y: i32,
    ) -> Result<ActionStatus, String> {
        self.ensure_hid_driver().await?;

        let driver = {
            let driver_opt = self.hid_driver.lock();
            driver_opt.clone()
        };

        if let Some(d) = driver {
            let cmd = HidCommand::MouseClick { button, x, y };
            match d.execute(cmd).await {
                Ok(_) => Ok(ActionStatus::Success),
                Err(e) => Ok(ActionStatus::ExecutionError { reason: e }),
            }
        } else {
            Ok(ActionStatus::ExecutionError {
                reason: "HID driver not initialized".to_string(),
            })
        }
    }

    async fn execute_mouse_release(&self, button: MouseButton) -> Result<ActionStatus, String> {
        self.ensure_hid_driver().await?;

        let driver = {
            let driver_opt = self.hid_driver.lock();
            driver_opt.clone()
        };

        if let Some(d) = driver {
            let cmd = HidCommand::MouseRelease { button };
            match d.execute(cmd).await {
                Ok(_) => Ok(ActionStatus::Success),
                Err(e) => Ok(ActionStatus::ExecutionError { reason: e }),
            }
        } else {
            Ok(ActionStatus::ExecutionError {
                reason: "HID driver not initialized".to_string(),
            })
        }
    }

    async fn execute_key_press(&self, key: u32) -> Result<ActionStatus, String> {
        self.ensure_hid_driver().await?;

        let driver = {
            let driver_opt = self.hid_driver.lock();
            driver_opt.clone()
        };

        if let Some(d) = driver {
            let cmd = HidCommand::KeyPress { key, modifiers: 0 };
            match d.execute(cmd).await {
                Ok(_) => Ok(ActionStatus::Success),
                Err(e) => Ok(ActionStatus::ExecutionError { reason: e }),
            }
        } else {
            Ok(ActionStatus::ExecutionError {
                reason: "HID driver not initialized".to_string(),
            })
        }
    }

    async fn execute_key_release(&self, key: u32) -> Result<ActionStatus, String> {
        self.ensure_hid_driver().await?;

        let driver = {
            let driver_opt = self.hid_driver.lock();
            driver_opt.clone()
        };

        if let Some(d) = driver {
            let cmd = HidCommand::KeyRelease { key };
            match d.execute(cmd).await {
                Ok(_) => Ok(ActionStatus::Success),
                Err(e) => Ok(ActionStatus::ExecutionError { reason: e }),
            }
        } else {
            Ok(ActionStatus::ExecutionError {
                reason: "HID driver not initialized".to_string(),
            })
        }
    }

    async fn execute_scroll(&self, delta: i32) -> Result<ActionStatus, String> {
        self.ensure_hid_driver().await?;

        let driver = {
            let driver_opt = self.hid_driver.lock();
            driver_opt.clone()
        };

        if let Some(d) = driver {
            let cmd = HidCommand::Scroll { delta };
            match d.execute(cmd).await {
                Ok(_) => Ok(ActionStatus::Success),
                Err(e) => Ok(ActionStatus::ExecutionError { reason: e }),
            }
        } else {
            Ok(ActionStatus::ExecutionError {
                reason: "HID driver not initialized".to_string(),
            })
        }
    }
}
