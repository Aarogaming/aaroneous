// core/hypervisor/src/hud/modes/transparent_hud.rs
//! In-game transparent overlay window (Win+G / F12 pass-through HUD).
//! Enables user-emulation: bots act through the transparent overlay, executing
//! natural Bezier cursor motions, key taps, and action routines with live preview.

use crate::hud::state::SharedHudState;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

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
        .default_size([400.0, 260.0])
        .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-20.0, 20.0))
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
                ui.label(egui::RichText::new("Emulation Mode:").strong());
                ui.label(egui::RichText::new("Natural Bezier Curves").color(Color32::from_rgb(63, 185, 80)));
            });

            ui.horizontal(|ui| {
                ui.label("Routine Progress:");
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
                    "Show Cursor Aim Reticle",
                );
                ui.checkbox(
                    &mut state.overlay_show_bot_telemetry,
                    "Human Jitter Emulation",
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
