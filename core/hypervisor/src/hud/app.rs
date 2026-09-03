// core/hypervisor/src/hud/app.rs
//! Main `StudioApp` struct, event loop, and render dispatcher.

use crate::hud::fascia::ProcessFasciaWatcher;
use crate::hud::modes::{render_compact_recorder_overlay, render_full_studio, render_transparent_hud};
use crate::hud::navigation::{CommandAction, CommandPalette, ShortcutsModal, ToastLevel, ToastNotificationManager};
use crate::hud::state::{AppWindowMode, SharedHudState};
use crate::hud::views::{
    AgentsHubView, Galaxy3DView, HudView, ScreenAutomationView, SettingsView, SiForgeView,
    SignalAnalyzerView, SpatialSensoryView, SystemThermoView, WorkbenchView,
};
use eframe::egui::{self, Key};

/// The primary Aaroneous Desktop Studio application
pub struct StudioApp {
    pub state: SharedHudState,
    pub views: Vec<Box<dyn HudView>>,
    pub palette: CommandPalette,
    pub toasts: ToastNotificationManager,
    pub shortcuts: ShortcutsModal,
    pub fascia_watcher: ProcessFasciaWatcher,
}

impl Default for StudioApp {
    fn default() -> Self {
        let state = SharedHudState::default();
        let mut toasts = ToastNotificationManager::new();
        toasts.push(
            "Aaroneous Online",
            format!("Discovered {} local GGUF models across local hubs.", state.discovered_gguf_models.len()),
            ToastLevel::Success,
        );

        let views: Vec<Box<dyn HudView>> = vec![
            Box::new(SpatialSensoryView),
            Box::new(Galaxy3DView),
            Box::new(SiForgeView),
            Box::new(ScreenAutomationView),
            Box::new(SignalAnalyzerView),
            Box::new(SystemThermoView),
            Box::new(AgentsHubView),
            Box::new(SettingsView),
            Box::new(WorkbenchView),
        ];

        Self {
            state,
            views,
            palette: CommandPalette::new(),
            toasts,
            shortcuts: ShortcutsModal::new(),
            fascia_watcher: ProcessFasciaWatcher::default(),
        }
    }
}

impl StudioApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute_command(&mut self, action: CommandAction, ctx: &egui::Context) {
        match action {
            CommandAction::Navigate(section) => {
                self.state.nav_section = section;
            }
            CommandAction::ToggleRecording => {
                self.state.toggle_recording();
            }
            CommandAction::ToggleCompactOverlay => {
                self.state.app_window_mode = match self.state.app_window_mode {
                    AppWindowMode::FullStudio => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(340.0, 60.0)));
                        AppWindowMode::CompactRecorderOverlay
                    }
                    AppWindowMode::CompactRecorderOverlay => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1240.0, 840.0)));
                        AppWindowMode::FullStudio
                    }
                };
            }
            CommandAction::MinimizeToTray => {
                self.state.is_minimized_to_tray = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            CommandAction::ToggleInGameOverlay => {
                self.state.is_ingame_overlay_open = !self.state.is_ingame_overlay_open;
            }
            CommandAction::ToggleDevMode => {
                self.state.settings.dev_mode = !self.state.settings.dev_mode;
                self.state.settings.save_to_disk();
            }
            CommandAction::RunDiagnostics => {
                if let Ok(diags) = self.state.dev_tools_engine.run_cargo_diagnostic_check() {
                    self.state.workbench_diagnostics = diags;
                }
            }
            CommandAction::RescanModels => {
                self.state.rescan_local_models();
                self.toasts.push("Hubs Rescanned", format!("Discovered {} models.", self.state.discovered_gguf_models.len()), ToastLevel::Info);
            }
            CommandAction::MineSiDistillation => {
                let _ = self.state.si_miner.mine_starter_distillation_corpus();
                self.toasts.push("Mining Complete", "Mined starter synthetic reasoning traces.", ToastLevel::Success);
            }
            CommandAction::RunSiMacro(name, _path) => {
                self.toasts.push("Macro Executed", format!("Ran macro '{name}'."), ToastLevel::Info);
            }
            CommandAction::TileWindowsGrid => {
                let rect = ctx.content_rect();
                self.state.spatial_canvas_scene.arrange_tiled_grid(rect.width(), rect.height(), 20.0);
                self.toasts.push("Layout Applied", "Arranged tool windows in zero-overlap grid.", ToastLevel::Info);
            }
            CommandAction::SetTheme(theme) => {
                self.state.settings.theme = theme;
                self.state.settings.save_to_disk();
            }
        }
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Poll asynchronous background worker messages & telemetry
        self.state.poll_background_messages();
        self.state.poll_live_bus();
        self.state.tick_telemetry_plots();

        // ── Global Keyboard Shortcuts ───────────────────────────────────────────
        if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(Key::K) || i.key_pressed(Key::P))) {
            self.palette.toggle();
        }
        if ctx.input(|i| i.key_pressed(Key::F9)) {
            self.state.toggle_recording();
        }
        if ctx.input(|i| i.key_pressed(Key::F10)) {
            self.execute_command(CommandAction::ToggleCompactOverlay, &ctx);
        }
        if ctx.input(|i| i.key_pressed(Key::F12)) {
            self.state.is_ingame_overlay_open = !self.state.is_ingame_overlay_open;
        }
        if ctx.input(|i| (i.modifiers.ctrl && i.key_pressed(Key::Slash)) || i.key_pressed(Key::Questionmark)) {
            self.shortcuts.toggle();
        }

        // ── Spatial Canvas Interaction Shortcuts (Pan, Zoom, Reset) ────────────
        let drag_delta = ctx.input(|i| i.pointer.delta());
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        let is_space_drag = ctx.input(|i| i.key_down(Key::Space) && i.pointer.is_decidedly_dragging());
        let is_middle_drag = ctx.input(|i| i.pointer.middle_down());
        let is_ctrl_zoom = ctx.input(|i| i.modifiers.ctrl);
        let reset_hotkey = ctx.input(|i| i.key_pressed(Key::Home) || (i.modifiers.ctrl && i.key_pressed(Key::Num0)));

        self.state.handle_canvas_pan_zoom(
            drag_delta,
            scroll_delta,
            is_space_drag || is_middle_drag,
            is_ctrl_zoom,
            reset_hotkey,
        );

        let theme = self.state.settings.theme;

        // ── Render Active Window Mode ───────────────────────────────────────────
        match self.state.app_window_mode {
            AppWindowMode::FullStudio => {
                let mut toggle_palette = false;
                let mut toggle_shortcuts = false;

                render_full_studio(
                    ui,
                    &mut self.state,
                    &mut self.views,
                    &mut toggle_palette,
                    &mut toggle_shortcuts,
                );

                if toggle_palette {
                    self.palette.toggle();
                }
                if toggle_shortcuts {
                    self.shortcuts.toggle();
                }
            }
            AppWindowMode::CompactRecorderOverlay => {
                render_compact_recorder_overlay(ui, &mut self.state);
            }
        }

        // ── Floating Windows & Overlays ─────────────────────────────────────────
        render_transparent_hud(&ctx, &mut self.state);

        // Command Palette Modal
        if let Some(action) = self.palette.render(&ctx, theme) {
            self.execute_command(action, &ctx);
        }

        // Shortcuts Modal
        self.shortcuts.render(&ctx, theme);

        // Toast Notifications
        self.toasts.render(&ctx, theme);

        // Repaint request for continuous high-framerate rendering
        ctx.request_repaint();
    }
}
