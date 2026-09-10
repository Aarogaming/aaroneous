//! Compile-time typed command registry for strongly-typed navigation & actions.

use std::rc::Rc;
use serde::{Deserialize, Serialize};

use crate::hud::navigation::NavSection;
use crate::hud::state::SharedHudState;


/// Capability-based dynamic command for runtime capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCommand {
    pub id: String,
    pub params: serde_json::Value,
}

impl CapabilityCommand {
    pub fn new(id: impl Into<String>, params: serde_json::Value) -> Self {
        Self { id: id.into(), params }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn params(&self) -> &serde_json::Value { &self.params }
}

pub type CommandExecutor = Rc<dyn Fn() + Send + Sync>;

/// TypedCommand enum with compile-time exhaustiveness for all nav commands.
#[derive(Clone)]
pub enum TypedCommand {
    Navigate(NavSection),
    ToggleRecording,
    ToggleCompactOverlay,
    MinimizeToTray,
    ToggleInGameOverlay,
    ToggleDevMode,
    RunDiagnostics,
    RescanModels,
    MineSiDistillation,
    RunSiMacro(String, std::path::PathBuf),
    TileWindowsGrid,
    ExecuteCapability(CapabilityCommand),
}

impl TypedCommand {
    pub fn id(&self) -> String {
        match self {
            Self::Navigate(section) => format!("nav.{}", section.as_str()),
            Self::ToggleRecording => "rec.toggle".to_string(),
            Self::ToggleCompactOverlay => "overlay.toggle_compact".to_string(),
            Self::MinimizeToTray => "window.minimize_tray".to_string(),
            Self::ToggleInGameOverlay => "overlay.toggle_ingame".to_string(),
            Self::ToggleDevMode => "system.toggle_dev".to_string(),
            Self::RunDiagnostics => "diag.run_cargo".to_string(),
            Self::RescanModels => "model.rescan".to_string(),
            Self::MineSiDistillation => "si.mine_distillation".to_string(),
            Self::RunSiMacro(_, _) => "si.run_macro".to_string(),
            Self::TileWindowsGrid => "window.tile_grid".to_string(),
            Self::ExecuteCapability(cmd) => format!("cap.{}", cmd.id()),
        }
    }

    pub fn execute(&self, state: &mut SharedHudState) {
        match self {
            Self::Navigate(section) => {
                state.nav_section = *section;
            }
            Self::ToggleRecording => {
                state.toggle_recording();
            }
            Self::ToggleCompactOverlay => {}
            Self::MinimizeToTray => {
                state.is_minimized_to_tray = true;
            }
            Self::ToggleInGameOverlay => {
                state.is_ingame_overlay_open = !state.is_ingame_overlay_open;
            }
            Self::ToggleDevMode => {
                state.settings.dev_mode = !state.settings.dev_mode;
                state.settings.save_to_disk();
            }
            Self::RunDiagnostics => {
                if let Ok(diags) = state.dev_tools_engine.run_cargo_diagnostic_check() {
                    state.workbench_diagnostics = diags;
                }
            }
            Self::RescanModels => {
                state.rescan_workspace_files();
            }
            Self::MineSiDistillation => {}
            Self::RunSiMacro(_, _path) => {}
            Self::TileWindowsGrid => {}
            Self::ExecuteCapability(cmd) => {
                if let Some(params) = cmd.params.as_object() {
                    for (k, v) in params {
                        println!("Capability {} param {}: {:?}", cmd.id(), k, v);
                    }
                }
            }
        }
    }

    pub fn from_nav_section(section: NavSection) -> Self {
        Self::Navigate(section)
    }
}

/// Compile-time command registry with ID-to-executor mapping.
pub struct CommandRegistry {
    executors: Vec<(String, CommandExecutor)>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
        }
    }

    /// Register a command ID with its executor.
    pub fn register(&mut self, id: &str, executor: CommandExecutor) {
        self.executors.push((id.to_string(), executor));
    }

    /// Look up executor by ID.
    pub fn get(&self, id: &str) -> Option<&CommandExecutor> {
        self.executors.iter().find(|(k, _)| k == id).map(|(_, v)| v)
    }

    /// Execute command by ID.
    pub fn execute(&self, id: &str) -> Option<()> {
        if let Some(executor) = self.get(id) {
            executor();
            Some(())
        } else {
            None
        }
    }

    /// Get all registered IDs.
    pub fn ids(&self) -> Vec<String> {
        self.executors.iter().map(|(id, _)| id.clone()).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
