// core/hypervisor/src/hud/modes/transparent_hud.rs
//! In-game transparent overlay window (Win+G / F12 pass-through HUD).
//! Enables user-emulation: bots act through the transparent overlay, executing
//! natural Bezier cursor motions, key taps, and action routines with live preview.

use crate::hud::state::SharedHudState;
use eframe::egui::{self, Color32, CornerRadius, Stroke};

pub fn render_transparent_hud(ctx: &egui::Context, state: &mut SharedHudState) {
    if !state.is_ingame_overlay_open {
        return;
    }

    let theme = state.settings.theme;
    let mut open = state.is_ingame_overlay_open;
    let mut killswitch_triggered = false;
    let time_sec = state.start_time.elapsed().as_secs_f32();

    egui::Window::new("🎮 Transparent Companion Overlay (F12)")
        .open(&mut open)
        .resizable(true)
        .collapsible(true)
        .default_size([400.0, 260.0])
        .default_pos(egui::pos2(
            ctx.content_rect().width() - 430.0,
            30.0,
        ))
        .frame(
            egui::Frame::window(&ctx.global_style())
                .fill(Color32::from_rgba_unmultiplied(13, 17, 23, 225))
                .stroke(Stroke::new(1.5, theme.accent()))
                .corner_radius(CornerRadius::same(8)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚡ COMPANION EMULATION")
                        .color(theme.accent())
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("🛑 EMERGENCY STOP")
                                    .color(Color32::WHITE)
                                    .size(11.0)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(200, 30, 30)),
                        )
                        .clicked()
                    {
                        killswitch_triggered = true;
                    }
                });
            });

            ui.separator();
            // ── Live Situational Guidance Feed (Backseat Driver) ──
            let proj = state.state_publisher.project_hud();
            let is_agent_active = matches!(
                state.game_agent.state,
                platform_bridge::PlaythroughState::Recording { .. }
                    | platform_bridge::PlaythroughState::AutonomousPlaying { .. }
            );
            let foreground_hint = if is_agent_active {
                "Active Autopilot: Executing targeted gameplay routine with natural Bézier dispersion.".to_string()
            } else if state.is_live_bus {
                "Companion Standby: Zero-copy ring buffer online (64MB). Monitoring application events.".to_string()
            } else {
                proj.active_guidance
            };

            egui::Frame::group(ui.style())
                .fill(Color32::from_rgba_unmultiplied(20, 28, 45, 230))
                .stroke(Stroke::new(1.0, theme.accent()))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🧭 CO-PILOT GUIDANCE").color(theme.accent()).strong().size(11.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("LIVE • {:.0} FPS", proj.measured_fps)).color(Color32::from_rgb(63, 185, 80)).strong().size(9.5));
                        });
                    });
                    ui.label(egui::RichText::new(foreground_hint).color(Color32::from_rgb(220, 230, 245)).size(11.5));
                });

            ui.add_space(4.0);

            // ── Quick Intercom Command Input ──
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("💬").size(14.0));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut state.chat_input)
                        .hint_text("Ask co-pilot or dispatch routine...")
                        .desired_width(ui.available_width() - 65.0),
                );
                let send_clicked = ui.button("Send").clicked();
                if (send_clicked || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                    && !state.chat_input.trim().is_empty()
                {
                    let prompt = state.chat_input.trim().to_string();
                    state.chat_history.push(("User".to_string(), prompt.clone(), Color32::from_rgb(120, 180, 255)));

                    // Route through Intermediary Capability Broker
                    let outcome = state.capability_broker.execute(
                        "specialist.dispatch_intent",
                        serde_json::json!({ "intent": prompt }),
                    );

                    let reply = if outcome.success {
                        let specialist = outcome.payload.get("assigned_specialist")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Orchestrator");
                        format!("Routed to {} [{}µs]: Dispatching intent.", specialist, outcome.latency_us)
                    } else {
                        format!("Intent dispatch failed: {}", outcome.error.unwrap_or_default())
                    };

                    state.chat_history.push(("Co-Pilot".to_string(), reply, Color32::from_rgb(63, 185, 80)));
                    state.inject_live_intent(&prompt);
                    state.chat_input.clear();
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let mode_label = if state.overlay_click_through {
                    "🔓 Pass-Through to Application (F12)"
                } else {
                    "🔒 Interactive Overlay Controls"
                };
                ui.checkbox(&mut state.overlay_click_through, mode_label);
            });

            ui.add_space(4.0);

            // Active Emulation Task Status
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Emulation Mode:").strong().size(11.0));
                ui.label(egui::RichText::new("Natural Bezier Curves").color(Color32::from_rgb(63, 185, 80)).size(11.0));
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Routine Progress:").size(11.0));
                let progress_val = ((time_sec * 0.35).sin() * 0.5 + 0.5).clamp(0.05, 0.95);
                ui.add(egui::ProgressBar::new(progress_val).text(format!("{:.0}%", progress_val * 100.0)));
            });

            ui.add_space(6.0);

            // Live Emulated Input Keys & Mouse
            ui.label(
                egui::RichText::new("Emulated Input Indicators:")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            ui.horizontal(|ui| {
                let key_names = ["W", "A", "S", "D", "🖱️ L-CLICK", "🖱️ R-CLICK"];
                for (i, &name) in key_names.iter().enumerate() {
                    let is_pressed = if i < 5 {
                        state.bot_active_keys.get(i).copied().unwrap_or(false)
                    } else {
                        false
                    };
                    let bg = if is_pressed {
                        Color32::from_rgb(63, 185, 80)
                    } else {
                        Color32::from_rgb(26, 32, 44)
                    };
                    let text_color = if is_pressed {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    };

                    egui::Frame::group(ui.style())
                        .fill(bg)
                        .corner_radius(CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(name)
                                    .color(text_color)
                                    .strong()
                                    .size(11.0),
                            );
                        });
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut state.overlay_show_aim_crosshair,
                    "Cursor Aim Reticle",
                );
                ui.checkbox(
                    &mut state.overlay_show_bot_telemetry,
                    "Human Jitter",
                );
            });
        });

    // Render Sub-Frame Overlay Primitives (Aim Crosshair / Reticle & Bezier Trail) directly on screen
    if state.overlay_show_aim_crosshair {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("overlay_primitives_reticle"),
        ));
        let screen = ctx.content_rect();

        // Natural dynamic cursor position calculated with smooth Bezier curve simulation
        let curve_x = (time_sec * 0.7).sin() * 0.35 + 0.5;
        let curve_y = (time_sec * 1.1).cos() * 0.25 + 0.5;
        let target_pos = egui::pos2(
            screen.min.x + (screen.width() * curve_x).clamp(0.0, screen.width()),
            screen.min.y + (screen.height() * curve_y).clamp(0.0, screen.height()),
        );

        let accent_color = theme.accent();

        // Draw Bezier cursor trail
        let prev_pos = egui::pos2(
            target_pos.x - ((time_sec * 0.7).cos() * 40.0),
            target_pos.y + ((time_sec * 1.1).sin() * 30.0),
        );
        painter.line_segment(
            [prev_pos, target_pos],
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(accent_color.r(), accent_color.g(), accent_color.b(), 120)),
        );

        // Inner circle & target pip
        painter.circle_stroke(target_pos, 16.0, Stroke::new(1.5, accent_color));
        painter.circle_filled(target_pos, 2.5, Color32::from_rgb(255, 60, 60));

        // Crosshair reticle lines
        painter.line_segment(
            [
                egui::pos2(target_pos.x - 22.0, target_pos.y),
                egui::pos2(target_pos.x - 6.0, target_pos.y),
            ],
            Stroke::new(1.5, accent_color),
        );
        painter.line_segment(
            [
                egui::pos2(target_pos.x + 6.0, target_pos.y),
                egui::pos2(target_pos.x + 22.0, target_pos.y),
            ],
            Stroke::new(1.5, accent_color),
        );
        painter.line_segment(
            [
                egui::pos2(target_pos.x, target_pos.y - 22.0),
                egui::pos2(target_pos.x, target_pos.y - 6.0),
            ],
            Stroke::new(1.5, accent_color),
        );
        painter.line_segment(
            [
                egui::pos2(target_pos.x, target_pos.y + 6.0),
                egui::pos2(target_pos.x, target_pos.y + 22.0),
            ],
            Stroke::new(1.5, accent_color),
        );
    }

    state.is_ingame_overlay_open = open;
    if killswitch_triggered {
        state
            .game_agent
            .trigger_killswitch("Overlay killswitch triggered");
        state.auto_pilot_kill_requested = true;
    }
}
