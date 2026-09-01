// core/hypervisor/src/hud/views/system_thermo.rs
//! Real-time hardware telemetry plots, frametime curves, and memory counters.

use crate::hud::state::SharedHudState;
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Stroke, Vec2};

#[derive(Default)]
pub struct SystemThermoView;

impl SystemThermoView {
    pub fn render_plot_canvas(
        ui: &mut egui::Ui,
        title: &str,
        points: &[f32],
        color: Color32,
        y_range: (f32, f32),
    ) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).strong().size(12.0));
            let height = 90.0;
            let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::hover());
            let rect = response.rect;

            painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(10, 14, 20));
            painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, Color32::from_rgb(30, 40, 55)), egui::StrokeKind::Outside);

            if points.len() >= 2 {
                let dx = rect.width() / (points.len() - 1) as f32;
                let (min_y, max_y) = y_range;
                let y_span = (max_y - min_y).max(0.001);

                let mut screen_pts = Vec::with_capacity(points.len());
                for (i, &val) in points.iter().enumerate() {
                    let norm_y = ((val - min_y) / y_span).clamp(0.0, 1.0);
                    let px = rect.min.x + (i as f32 * dx);
                    let py = rect.max.y - (norm_y * (rect.height() - 8.0)) - 4.0;
                    screen_pts.push(Pos2::new(px, py));
                }

                for w in screen_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.8, color));
                }

                if let Some(last) = points.last() {
                    painter.text(
                        rect.max - Vec2::new(8.0, 18.0),
                        egui::Align2::RIGHT_BOTTOM,
                        format!("{:.2}", last),
                        egui::FontId::proportional(11.0),
                        color,
                    );
                }
            }
        });
    }
}

impl HudView for SystemThermoView {
    fn id(&self) -> &'static str {
        "system_thermo"
    }

    fn title(&self) -> &'static str {
        "📈 System Telemetry"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        ui.heading(
            egui::RichText::new("📈 System Thermodynamics & Hardware Telemetry")
                .color(theme.accent())
                .strong(),
        );
        ui.label("Real-time measured frame time deltas, neural memory occupancy, and reward trajectories.");
        ui.separator();

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                Self::render_plot_canvas(ui, "⚡ Live Framerate (FPS)", &state.telemetry_fps_history, theme.accent(), (100.0, 140.0));
                ui.add_space(8.0);
                Self::render_plot_canvas(ui, "⏱️ Execution Latency (ms)", &state.telemetry_latency_history, Color32::from_rgb(255, 120, 0), (0.0, 1.0));
            });

            cols[1].vertical(|ui| {
                Self::render_plot_canvas(ui, "📈 Cumulative Reward Curve", &state.telemetry_reward_history, Color32::from_rgb(63, 185, 80), (0.0, 80.0));
                ui.add_space(8.0);

                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(egui::RichText::new("Hardware Acceleration & Subsystems").strong());
                        ui.horizontal(|ui| {
                            ui.label("DirectX 12 Duplication:");
                            ui.label(egui::RichText::new("ONLINE").color(Color32::from_rgb(63, 185, 80)).strong());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Epigenetic Delta Gating:");
                            ui.label(egui::RichText::new("ACTIVE (16x16)").color(theme.accent()).strong());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Shared Memory Interconnect:");
                            ui.label(egui::RichText::new("64 MB MMAP").color(Color32::from_rgb(210, 153, 34)).strong());
                        });
                    });
            });
        });
    }
}
