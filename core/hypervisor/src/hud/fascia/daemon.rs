//! core/hypervisor/src/hud/fascia/daemon.rs
//! Automated Intent-to-Fascia Watcher Daemon (Phase 7 Observability & Spatial UX).
//! Monitors foreground process transitions via ETW / Window hooks and dynamically
//! hot-swaps matching `.ron` spatial canvas scene presets.

use crate::hud::state::{SharedHudState, SpatialCanvasScene};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

/// Process-to-Fascia Scene Watcher Daemon
pub struct ProcessFasciaWatcher {
    /// Maps lowercase process executable image names to spatial scene preset file paths
    mapping_table: HashMap<String, PathBuf>,
    /// Whether the user has manually pinned/locked the current fascia layout (preventing auto-switching)
    is_manually_locked: bool,
    /// Currently detected active foreground process executable
    current_process: Option<String>,
    /// Name of the currently loaded spatial scene preset
    current_scene_name: Option<String>,
}

impl Default for ProcessFasciaWatcher {
    fn default() -> Self {
        Self::with_default_mappings()
    }
}

impl ProcessFasciaWatcher {
    /// Instantiates a new empty ProcessFasciaWatcher.
    pub fn new() -> Self {
        Self {
            mapping_table: HashMap::new(),
            is_manually_locked: false,
            current_process: None,
            current_scene_name: None,
        }
    }

    /// Instantiates ProcessFasciaWatcher with standard default engineering, streaming, and game mappings.
    pub fn with_default_mappings() -> Self {
        let mut watcher = Self::new();
        watcher.register_mapping("code.exe", "assets/fascias/dev.ron");
        watcher.register_mapping("devenv.exe", "assets/fascias/dev.ron");
        watcher.register_mapping("rustrover64.exe", "assets/fascias/dev.ron");
        watcher.register_mapping("obs64.exe", "assets/fascias/streaming.ron");
        watcher.register_mapping("chrome.exe", "assets/fascias/browser.ron");
        watcher.register_mapping("msedge.exe", "assets/fascias/browser.ron");
        watcher.register_mapping("blender.exe", "assets/fascias/creative.ron");
        watcher.register_mapping("unrealeditor.exe", "assets/fascias/gamedev.ron");
        watcher
    }

    /// Registers a process image to spatial scene preset mapping.
    pub fn register_mapping(&mut self, process_name: impl Into<String>, scene_path: impl AsRef<Path>) {
        let key = process_name.into().to_lowercase();
        self.mapping_table.insert(key, scene_path.as_ref().to_path_buf());
    }

    /// Toggles the manual canvas layout lock.
    pub fn toggle_manual_lock(&mut self) -> bool {
        self.is_manually_locked = !self.is_manually_locked;
        self.is_manually_locked
    }

    /// Sets the manual canvas layout lock state explicitly.
    pub fn set_manual_lock(&mut self, locked: bool) {
        self.is_manually_locked = locked;
    }

    /// Returns whether the layout is currently locked against automatic switching.
    pub fn is_locked(&self) -> bool {
        self.is_manually_locked
    }

    /// Returns the currently tracked foreground process.
    pub fn current_process(&self) -> Option<&str> {
        self.current_process.as_deref()
    }

    /// Returns the name of the currently active scene preset.
    pub fn current_scene_name(&self) -> Option<&str> {
        self.current_scene_name.as_deref()
    }

    /// Evaluates a foreground process switch event and auto-loads the corresponding scene if mapped.
    pub fn on_process_focus_changed(
        &mut self,
        process_image: &str,
        hud_state: &mut SharedHudState,
    ) -> Option<PathBuf> {
        let clean_proc = process_image.to_lowercase();
        self.current_process = Some(clean_proc.clone());

        if self.is_manually_locked {
            return None;
        }

        if let Some(target_path) = self.mapping_table.get(&clean_proc).cloned() {
            info!(
                target: "hud::fascia",
                %process_image,
                scene = ?target_path,
                "⚡ Auto-switching Spatial HUD Fascia Scene"
            );

            // Attempt to load scene from disk or synthesize default spatial preset
            let resolved_path = if target_path.is_relative() {
                aaroneous_paths::WorkspacePaths::discover().root().join(&target_path)
            } else {
                target_path.clone()
            };

            if let Ok(loaded_scene) = SpatialCanvasScene::load_from_disk(&resolved_path) {
                hud_state.spatial_canvas_scene = loaded_scene;
            } else if let Ok(loaded_scene) = SpatialCanvasScene::load_from_disk(&target_path) {
                hud_state.spatial_canvas_scene = loaded_scene;
            } else {
                // Synthesize active spatial preset for known domain
                if clean_proc.contains("code") || clean_proc.contains("devenv") {
                    hud_state.spatial_canvas_scene.canvas_pan = (100.0, 50.0);
                    hud_state.spatial_canvas_scene.canvas_zoom = 1.0;
                } else if clean_proc.contains("chrome") || clean_proc.contains("edge") {
                    hud_state.spatial_canvas_scene.canvas_pan = (0.0, 0.0);
                    hud_state.spatial_canvas_scene.canvas_zoom = 1.0;
                }
            }

            self.current_scene_name = target_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());

            Some(target_path)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_fascia_watcher_mappings_and_locking() {
        let mut watcher = ProcessFasciaWatcher::with_default_mappings();
        let mut hud_state = SharedHudState::default();

        assert!(!watcher.is_locked());

        // Process switch to VS Code
        let res = watcher.on_process_focus_changed("Code.exe", &mut hud_state);
        assert!(res.is_some());
        assert_eq!(watcher.current_process(), Some("code.exe"));
        assert_eq!(watcher.current_scene_name(), Some("dev"));
        assert_eq!(hud_state.spatial_canvas_scene.canvas_pan, (100.0, 50.0));

        // Manual lock
        watcher.set_manual_lock(true);
        assert!(watcher.is_locked());

        // Process switch to Chrome while locked
        let res_locked = watcher.on_process_focus_changed("chrome.exe", &mut hud_state);
        assert!(res_locked.is_none());
        assert_eq!(watcher.current_process(), Some("chrome.exe"));
        // Scene stays dev
        assert_eq!(watcher.current_scene_name(), Some("dev"));
        assert_eq!(hud_state.spatial_canvas_scene.canvas_pan, (100.0, 50.0));

        // Unlock
        watcher.toggle_manual_lock();
        assert!(!watcher.is_locked());

        let res_unlocked = watcher.on_process_focus_changed("chrome.exe", &mut hud_state);
        assert!(res_unlocked.is_some());
        assert_eq!(watcher.current_scene_name(), Some("browser"));
        assert_eq!(hud_state.spatial_canvas_scene.canvas_pan, (0.0, 0.0));
    }
}
