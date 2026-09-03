// core/hypervisor/src/hud/modes/compact_recorder.rs
//! Compact Cognitive Execution Node Overlay (F10 mode).

use crate::hud::state::{AppWindowMode, SharedHudState};
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

pub fn render_compact_recorder_overlay(ui: &mut egui::Ui, state: &mut SharedHudState) {
    let theme = state.settings.theme;

    egui::Frame::group(ui.style())
        .fill(theme.panel_bg())
        .stroke(Stroke::new(1.5, theme.accent()))
        .corner_radius(CornerRadius::same(8))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(340.0, 56.0));

            // Top Bar: Brand, Session Status & Window Expansion
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Aaroneous").strong().color(theme.accent()));
                ui.separator();

                // Recording State Status
                match &state.game_agent.state {
                    platform_bridge::PlaythroughState::Recording { frames_recorded, .. } => {
                        let elapsed = state.recording_start_instant.map(|s| s.elapsed().as_secs()).unwrap_or(0);
                        ui.label(
                            egui::RichText::new(format!("🔴 {:02}:{:02} ({} acts)", elapsed / 60, elapsed % 60, frames_recorded))
                                .color(Color32::RED)
                                .strong(),
                        );
                        if ui.button("⏹️ Stop").clicked() {
                            state.toggle_recording();
                        }
                    }
                    _ => {
                        if ui.button("🔴 Rec (F9)").clicked() {
                            state.toggle_recording();
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🪟 Full Studio").clicked() {
                        state.app_window_mode = AppWindowMode::FullStudio;
                    }
                });
            });

            ui.separator();

            // Interactive Cognitive Node & Telemetry Widget
            state.companion_overlay.render(ui);
        });
}
