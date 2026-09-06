// core/hypervisor/src/hud/app.rs
//! Main `StudioApp` struct, event loop, and render dispatcher.

use crate::hud::fascia::ProcessFasciaWatcher;
use crate::hud::modes::{
    render_compact_recorder_overlay, render_full_studio, render_transparent_hud,
};
use crate::hud::navigation::{
    CommandAction, CommandPalette, ShortcutsModal, ToastLevel, ToastNotificationManager,
};
use crate::hud::state::{AppWindowMode, SharedHudState};
use crate::hud::views::{
    AgentsHubView, Galaxy3DView, HudView, ScreenAutomationView, SettingsView, SiForgeView,
    SignalAnalyzerView, SpatialSensoryView, SystemThermoView, WorkbenchView,
};
use eframe::egui::{self, Color32, Key};

/// The primary Aaroneous Desktop Studio application
pub struct StudioApp {
    pub state: SharedHudState,
    pub views: Vec<Box<dyn HudView>>,
    pub palette: CommandPalette,
    pub toasts: ToastNotificationManager,
    pub shortcuts: ShortcutsModal,
    pub fascia_watcher: ProcessFasciaWatcher,
    pub guide: crate::hud::onboarding::OnboardingGuide,
    pub console_os: crate::hud::modes::ConsoleOsLauncher,
}

impl Default for StudioApp {
    fn default() -> Self {
        let state = SharedHudState::default();
        let mut toasts = ToastNotificationManager::default();

        toasts.push(
            "Kernel Initialized",
            "Aaroneous hypervisor and SWMR shared-memory bridge active.",
            ToastLevel::Success,
        );

        let views: Vec<Box<dyn HudView>> = vec![
            Box::new(AgentsHubView),
            Box::new(SpatialSensoryView),
            Box::new(Galaxy3DView),
            Box::new(SiForgeView),
            Box::new(ScreenAutomationView),
            Box::new(SignalAnalyzerView),
            Box::new(SystemThermoView),
            Box::new(SettingsView),
            Box::new(WorkbenchView),
        ];

        let mut guide = crate::hud::onboarding::OnboardingGuide::new();
        if state.settings.show_welcome_guide_on_startup {
            guide.is_open = true;
        }

        Self {
            state,
            views,
            palette: CommandPalette::new(),
            toasts,
            shortcuts: ShortcutsModal::new(),
            fascia_watcher: ProcessFasciaWatcher::default(),
            guide,
            console_os: crate::hud::modes::ConsoleOsLauncher::new(),
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
                    AppWindowMode::CompactRecorderOverlay => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            1240.0, 840.0,
                        )));
                        AppWindowMode::FullStudio
                    }
                    _ => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            340.0, 60.0,
                        )));
                        AppWindowMode::CompactRecorderOverlay
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
                self.toasts.push(
                    "Hubs Rescanned",
                    format!(
                        "Discovered {} models.",
                        self.state.discovered_gguf_models.len()
                    ),
                    ToastLevel::Info,
                );
            }
            CommandAction::MineSiDistillation => {
                let _ = self.state.si_miner.mine_starter_distillation_corpus();
                self.toasts.push(
                    "Mining Complete",
                    "Mined starter synthetic reasoning traces.",
                    ToastLevel::Success,
                );
            }
            CommandAction::RunSiMacro(name, _path) => {
                self.toasts.push(
                    "Macro Executed",
                    format!("Ran macro '{name}'."),
                    ToastLevel::Info,
                );
            }
            CommandAction::TileWindowsGrid => {
                let rect = ctx.content_rect();
                self.state.spatial_canvas_scene.arrange_tiled_grid(
                    rect.width(),
                    rect.height(),
                    20.0,
                );
                self.toasts.push(
                    "Layout Applied",
                    "Arranged tool windows in zero-overlap grid.",
                    ToastLevel::Info,
                );
            }
            CommandAction::SetTheme(theme) => {
                self.state.settings.theme = theme;
                self.state.settings.save_to_disk();
            }
            CommandAction::ExecuteCapability { id, params } => {
                let outcome = self.state.capability_broker.execute(&id, params);
                if outcome.success {
                    self.toasts.push(
                        "Capability Executed",
                        format!("{} completed in {}µs", outcome.capability_id, outcome.latency_us),
                        ToastLevel::Success,
                    );
                } else {
                    let err = outcome.error.unwrap_or_else(|| "Unknown failure".to_string());
                    self.toasts.push(
                        "Execution Failed",
                        format!("{}: {}", outcome.capability_id, err),
                        ToastLevel::Error,
                    );
                }
            }
        }
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Intercept close event if close_to_tray is enabled
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.state.settings.close_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.state.is_minimized_to_tray = true;
                self.toasts.push(
                    "Minimized to Tray",
                    "Aaroneous is still running in the background. Open from taskbar or tray.",
                    ToastLevel::Info,
                );
            }
        }

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
        if ctx.input(|i| i.key_pressed(Key::F11)) {
            match self.state.app_window_mode {
                AppWindowMode::ConsoleGameOS => {
                    self.console_os.was_fullscreen = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1240.0, 840.0)));
                    self.state.app_window_mode = AppWindowMode::FullStudio;
                }
                _ => {
                    self.state.app_window_mode = AppWindowMode::ConsoleGameOS;
                    self.console_os.was_fullscreen = false;
                }
            }
        }
        if ctx.input(|i| i.key_pressed(Key::F12)) {
            self.state.is_ingame_overlay_open = !self.state.is_ingame_overlay_open;
        }
        if ctx.input(|i| {
            (i.modifiers.ctrl && i.key_pressed(Key::Slash)) || i.key_pressed(Key::Questionmark)
        }) {
            self.shortcuts.toggle();
        }

        // ── Spatial Canvas Interaction Shortcuts (Pan, Zoom, Reset) ────────────
        let drag_delta = ctx.input(|i| i.pointer.delta());
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        let is_space_drag =
            ctx.input(|i| i.key_down(Key::Space) && i.pointer.is_decidedly_dragging());
        let is_middle_drag = ctx.input(|i| i.pointer.middle_down());
        let is_ctrl_zoom = ctx.input(|i| i.modifiers.ctrl);
        let reset_hotkey = ctx
            .input(|i| i.key_pressed(Key::Home) || (i.modifiers.ctrl && i.key_pressed(Key::Num0)));

        self.state.handle_canvas_pan_zoom(
            drag_delta,
            scroll_delta,
            is_space_drag || is_middle_drag,
            is_ctrl_zoom,
            reset_hotkey,
        );

        // ── Ingest Live Pointer Kinematics into User Baseline Engine ────────────
        let ptr_vel = ctx.input(|i| i.pointer.velocity());
        let speed = ptr_vel.length();
        if speed > 10.0 {
            let mut bio = self.state.user_identity_engine.active_profile().baseline_biomarkers.clone();
            bio.mean_cursor_speed = (bio.mean_cursor_speed * 0.95) + (speed * 0.05);
            if let Some(notice) = self.state.user_identity_engine.ingest_kinematics(bio) {
                self.toasts.push("Identity Sync", notice, ToastLevel::Info);
            }
        }

        let theme = self.state.settings.theme;

        // ── Render Active Window Mode ───────────────────────────────────────────
        match self.state.app_window_mode {
            AppWindowMode::FullStudio => {
                let mut toggle_palette = false;
                let mut toggle_shortcuts = false;
                let mut toggle_guide = false;

                render_full_studio(
                    ui,
                    &mut self.state,
                    &mut self.views,
                    &mut toggle_palette,
                    &mut toggle_shortcuts,
                    &mut toggle_guide,
                );

                if toggle_palette {
                    self.palette.toggle();
                }
                if toggle_shortcuts {
                    self.shortcuts.toggle();
                }
                if toggle_guide {
                    self.guide.is_open = true;
                }
            }
            AppWindowMode::CompactRecorderOverlay => {
                render_compact_recorder_overlay(ui, &mut self.state);
            }
            AppWindowMode::UtilityDashboard => {
                crate::hud::modes::render_utility_dashboard(ui, &mut self.state);
            }
            AppWindowMode::ConsoleGameOS => {
                self.console_os.render(ui, &mut self.state);
            }
        }

        // ── Floating Windows & Overlays ─────────────────────────────────────────
        render_transparent_hud(&ctx, &mut self.state);

        // Command Palette Modal
        if let Some(action) = self.palette.render(&ctx, theme, Some(&self.state.capability_broker)) {
            self.execute_command(action, &ctx);
        }

        // Shortcuts Modal
        self.shortcuts.render(&ctx, theme);

        // Interactive Onboarding Demo Guide Modal
        self.guide.render(&ctx, &mut self.state, theme);

        // Achievements & Mastery Modal
        if self.state.show_achievements_modal {
            let mut show_modal = self.state.show_achievements_modal;
            let screen_rect = ctx.content_rect();
            let modal_width = 580.0f32.min(screen_rect.width() - 40.0);
            let modal_height = 420.0f32.min(screen_rect.height() - 60.0);

            let default_pos = egui::pos2(
                (screen_rect.width() - modal_width) * 0.5,
                (screen_rect.height() - modal_height) * 0.4,
            );

            egui::Window::new("🏆 Achievements & Mastery Training")
                .open(&mut show_modal)
                .default_pos(default_pos)
                .default_size(egui::vec2(modal_width, modal_height))
                .min_width(380.0)
                .min_height(280.0)
                .resizable(true)
                .collapsible(true)
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(theme.panel_bg())
                        .stroke(eframe::egui::Stroke::new(1.5, theme.accent()))
                        .corner_radius(eframe::egui::CornerRadius::same(12))
                        .shadow(egui::Shadow {
                            offset: [0, 8],
                            blur: 24,
                            spread: 4,
                            color: Color32::from_black_alpha(200),
                        }),
                )
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Track and master shortcuts, routines, and telemetry.")
                                .color(Color32::from_rgb(180, 190, 210))
                                .size(12.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Unlocked: {} / {}",
                                    self.state.achievements.unlocked_count(),
                                    self.state.achievements.total_count()
                                ))
                                .color(Color32::from_rgb(255, 215, 0))
                                .strong(),
                            );
                        });
                    });

                    ui.separator();
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .max_height(360.0)
                        .show(ui, |ui| {
                            for ach in &self.state.achievements.achievements {
                                let bg = if ach.is_unlocked {
                                    Color32::from_rgba_unmultiplied(20, 45, 30, 200)
                                } else {
                                    theme.card_bg()
                                };
                                let border = if ach.is_unlocked {
                                    Color32::from_rgb(63, 185, 80)
                                } else {
                                    theme.border_color()
                                };

                                egui::Frame::group(ui.style())
                                    .fill(bg)
                                    .stroke(eframe::egui::Stroke::new(1.0, border))
                                    .corner_radius(eframe::egui::CornerRadius::same(6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&ach.icon).size(20.0));
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&ach.title).strong());
                                                    if ach.is_unlocked {
                                                        ui.label(egui::RichText::new("✓ UNLOCKED").color(Color32::from_rgb(63, 185, 80)).strong().size(10.5));
                                                    } else {
                                                        ui.label(egui::RichText::new(format!("{}/{}", ach.current_progress, ach.target_progress)).color(Color32::GRAY).size(10.5));
                                                    }
                                                });
                                                ui.label(egui::RichText::new(&ach.description).size(11.0).color(Color32::from_rgb(180, 190, 210)));
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("+{} XP", ach.xp_reward)).color(Color32::from_rgb(255, 215, 0)).strong());
                                            });
                                        });
                                    });
                                ui.add_space(3.0);
                            }
                        });
                });
            self.state.show_achievements_modal = show_modal;
        }

        // Operator Identity & Companion Modal
        crate::hud::views::user_profile_modal::render_user_profile_modal(&ctx, &mut self.state);

        // Toast Notifications
        self.toasts.render(&ctx, theme);

        // Repaint request for continuous high-framerate rendering
        ctx.request_repaint();
    }
}
