// core/hypervisor/src/hud/modes/transparent_hud.rs
//! In-game transparent overlay window (Win+G / F12 pass-through HUD).

use crate::hud::state::SharedHudState;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

pub fn render_transparent_hud(ctx: &egui::Context, state: &mut SharedHudState) {
    if !state.is_ingame_overlay_open {
        return;
    }

    let theme = state.settings.theme;
    let mut open = state.is_ingame_overlay_open;
    let mut killswitch_triggered = false;

    egui::Window::new("🎮 In-Game Bot Overlay (Win+G)")
        .open(&mut open)
        .resizable(true)
        .default_size([380.0, 240.0])
        .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-20.0, 20.0))
        .frame(
            egui::Frame::window(&ctx.global_style())
                .fill(Color32::from_rgba_unmultiplied(13, 17, 23, 220))
                .stroke(Stroke::new(1.5, theme.accent()))
                .corner_radius(CornerRadius::same(8)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚡ BOT ACTING AS PLAYER").color(theme.accent()).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("🛑 STOP").color(Color32::WHITE).size(11.0).strong()).clicked() {
                        killswitch_triggered = true;
                    }
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                let mode_label = if state.overlay_click_through { "🔓 Pass-Through to Game (F12)" } else { "🔒 Interactive Overlay" };
                ui.checkbox(&mut state.overlay_click_through, mode_label);
            });

            ui.add_space(4.0);

            ui.label(egui::RichText::new("Active Task: Speedrun Grinding").strong());
            ui.horizontal(|ui| {
                ui.label("Progress:");
                ui.add(egui::ProgressBar::new(0.78).text("+18.4 pts"));
            });

            ui.add_space(6.0);

            ui.label(egui::RichText::new("Live Bot Keys Pressed:").size(11.0).color(Color32::GRAY));
            ui.horizontal(|ui| {
                let key_names = ["W", "A", "S", "D", "🖱️ L-CLICK"];
                for (i, &name) in key_names.iter().enumerate() {
                    let is_pressed = state.bot_active_keys.get(i).copied().unwrap_or(false);
                    let bg = if is_pressed { Color32::from_rgb(63, 185, 80) } else { Color32::from_rgb(30, 36, 46) };
                    let text_color = if is_pressed { Color32::BLACK } else { Color32::WHITE };

                    egui::Frame::group(ui.style())
                        .fill(bg)
                        .corner_radius(CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(name).color(text_color).strong().size(11.0));
                        });
                }
            });

            ui.add_space(6.0);
            ui.checkbox(&mut state.overlay_show_aim_crosshair, "Show Targeting Reticle");
        });

    // Render Sub-Frame Overlay Primitives (Aim Crosshair / Reticle) directly on the screen
    if state.overlay_show_aim_crosshair {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("overlay_primitives_reticle")));
        let screen = ctx.content_rect();
        let target_pos = egui::pos2(
            screen.min.x + (screen.width() * state.bot_aim_target[0]).clamp(0.0, screen.width()),
            screen.min.y + (screen.height() * state.bot_aim_target[1]).clamp(0.0, screen.height()),
        );

        let accent_color = theme.accent();
        // Inner circle
        painter.circle_stroke(target_pos, 16.0, Stroke::new(1.5, accent_color));
        painter.circle_filled(target_pos, 2.5, Color32::from_rgb(255, 60, 60));

        // Crosshair reticle lines
        painter.line_segment([egui::pos2(target_pos.x - 24.0, target_pos.y), egui::pos2(target_pos.x - 6.0, target_pos.y)], Stroke::new(1.5, accent_color));
        painter.line_segment([egui::pos2(target_pos.x + 6.0, target_pos.y), egui::pos2(target_pos.x + 24.0, target_pos.y)], Stroke::new(1.5, accent_color));
        painter.line_segment([egui::pos2(target_pos.x, target_pos.y - 24.0), egui::pos2(target_pos.x, target_pos.y - 6.0)], Stroke::new(1.5, accent_color));
        painter.line_segment([egui::pos2(target_pos.x, target_pos.y + 6.0), egui::pos2(target_pos.x, target_pos.y + 24.0)], Stroke::new(1.5, accent_color));
    }

    state.is_ingame_overlay_open = open;
    if killswitch_triggered {
        state.game_agent.trigger_killswitch("Overlay killswitch triggered");
    }
}
