// core/hypervisor/src/hud/modes/utility_dashboard.rs
//! Minimalist Stripped Utility Agent Dashboard & Controls Side-Panel.
//! Zero-distraction mode focused on fast execution, quick companion spawning,
//! live system harmony, and instant safety interlocks.

use crate::hud::state::{AgentExecutionState, AppWindowMode, SharedHudState};
use eframe::egui::{self, Color32, CornerRadius, Stroke};

pub fn render_utility_dashboard(ui: &mut egui::Ui, state: &mut SharedHudState) {
    let theme = state.settings.theme;

    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(ui.style()).fill(theme.bg_color()))
        .show_inside(ui, |ui| {
            // ── Top Header Bar ──────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚡ AARONEOUS UTILITY DASHBOARD")
                        .color(theme.accent())
                        .strong()
                        .size(15.0),
                );

                let prof = state.user_identity_engine.active_profile();
                let flow_pct = (state.user_identity_engine.flow_score() * 100.0).round() as u32;
                let user_badge = if prof.is_guest {
                    format!("👤 {} [Guest • {}%]", prof.display_name, flow_pct)
                } else {
                    format!("👤 {} [Flow {}%]", prof.display_name, flow_pct)
                };
                if ui.button(egui::RichText::new(user_badge).color(Color32::from_rgb(56, 139, 253)).strong()).clicked() {
                    state.show_user_profile_modal = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🎮 Switch to Console Mode (F11)").clicked() {
                        state.app_window_mode = AppWindowMode::ConsoleGameOS;
                    }

                    if ui.button("🪟 Utility Desktop").clicked() {
                        state.app_window_mode = AppWindowMode::FullStudio;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1240.0, 840.0)));
                    }

                    ui.label(
                        egui::RichText::new(format!("Framerate: {:.0} FPS", state.measured_fps))
                            .color(Color32::from_rgb(63, 185, 80))
                            .size(11.0),
                    );
                });
            });

            ui.separator();
            ui.add_space(4.0);

            // ── Emergency Auto-Pilot & Safety Strip ─────────────────────────
            let tele = &state.auto_pilot_telemetry;
            let is_engaged = tele.state == crate::hud::auto_pilot::AutoPilotState::Engaged;

            egui::Frame::group(ui.style())
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.border_color()))
                .corner_radius(CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let status_dot = if is_engaged {
                            egui::RichText::new("● ACTIVE").color(Color32::from_rgb(63, 185, 80)).strong()
                        } else {
                            egui::RichText::new("● STANDBY").color(Color32::GRAY).strong()
                        };
                        ui.label("Auto-Pilot System:");
                        ui.label(status_dot);

                        ui.separator();
                        ui.label(format!("Latency: {:.1}µs", tele.avg_tick_latency_us));
                        ui.separator();
                        ui.label(format!("Actions Dispatched: {}", tele.hid_actions_dispatched));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("🛑 EMERGENCY KILLSWITCH")
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(200, 30, 30)),
                                )
                                .clicked()
                            {
                                state.auto_pilot_kill_requested = true;
                            }

                            let engage_text = if is_engaged { "⏸️ Disengage" } else { "▶️ Engage Auto-Pilot" };
                            if ui.button(engage_text).clicked() {
                                state.auto_pilot_toggle_requested = true;
                            }
                        });
                    });
                });

            ui.add_space(8.0);

            // ── 2-Column Split: Active Agents & Quick Actions ────────────────
            ui.columns(2, |cols| {
                // Col 0: Companion Agents Deck
                cols[0].vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🤖 Active Automation Companions").strong().color(theme.accent()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Count: {}", state.custom_agents.len()));
                        });
                    });
                    ui.add_space(4.0);

                    let mut to_run = None;
                    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                        for agent in &state.custom_agents {
                            egui::Frame::group(ui.style())
                                .fill(theme.card_bg())
                                .stroke(Stroke::new(1.0, theme.border_color()))
                                .corner_radius(CornerRadius::same(5))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let [r, g, b] = agent.color;
                                        ui.label(egui::RichText::new("🤖").color(Color32::from_rgb(r, g, b)));
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(&agent.name).strong().size(12.0));
                                            ui.label(egui::RichText::new(&agent.description).size(10.0).color(Color32::GRAY));
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if agent.state == AgentExecutionState::Running {
                                                ui.label(egui::RichText::new("⚡ RUNNING").color(Color32::from_rgb(63, 185, 80)).strong().size(10.0));
                                            } else if ui.button("▶️ Run").clicked() {
                                                to_run = Some(agent.clone());
                                            }
                                        });
                                    });
                                });
                            ui.add_space(2.0);
                        }
                    });

                    if let Some(agent) = to_run {
                        state.award_xp(25, "Executed Companion Task");
                        state.spawn_agent_execution(&agent);
                    }
                });

                // Col 1: System Health, Instant Shortcuts & Routine Trigger
                cols[1].vertical(|ui| {
                    ui.label(egui::RichText::new("⚡ Fast Controls & System Telemetry").strong().color(theme.accent()));
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style())
                        .fill(theme.card_bg())
                        .stroke(Stroke::new(1.0, theme.border_color()))
                        .corner_radius(CornerRadius::same(6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("System Harmony:");
                                ui.add(egui::ProgressBar::new(state.bus_integrity / 100.0).text(format!("{:.0}%", state.bus_integrity)));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Level & Experience:");
                                ui.label(egui::RichText::new(format!("⭐ Lv. {} ({} XP)", state.user_level, state.user_xp)).color(Color32::from_rgb(255, 215, 0)).strong());
                            });
                        });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Quick Routines & Overlays:").strong());

                    ui.horizontal(|ui| {
                        if ui.button("🔴 Toggle Rec (F9)").clicked() {
                            state.toggle_recording();
                        }
                        if ui.button("🎮 Overlay (F12)").clicked() {
                            state.is_ingame_overlay_open = !state.is_ingame_overlay_open;
                        }
                        if ui.button("🪟 Mini-HUD (F10)").clicked() {
                            state.app_window_mode = AppWindowMode::CompactRecorderOverlay;
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Recent Automation Events:").strong());
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        if state.event_logs.is_empty() {
                            ui.label(egui::RichText::new("No recent logs.").italics().color(Color32::GRAY));
                        } else {
                            for log in state.event_logs.iter().rev().take(6) {
                                ui.label(egui::RichText::new(format!("• [{}] {}", log.source, log.action)).size(10.5));
                            }
                        }
                    });
                });
            });
        });
}
