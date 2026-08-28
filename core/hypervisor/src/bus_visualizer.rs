//! core/hypervisor/src/bus_visualizer.rs
//! Real-Time SPMC Specialist Bus Oscilloscope & Sentinel-Auditor Guardrail Visualizer.
//!
//! Features:
//! 1. Asynchronous atomic sampler glancing at SPMC memory maps via Ordering::Relaxed without stalling producers.
//! 2. 60Hz 256-dim BarChart oscilloscope mapping active latent intent vector activations.
//! 3. Sentinel Deep SVDD Threat Gauge (Euclidean distance D = ||S_t - c||_2 with green -> yellow -> red thresholding).
//! 4. Deep SVDD Centroid baseline curve overlay and "ORTHOGONAL SNAP ENGAGED" warning indicator.

use eframe::egui::{self, Color32, Pos2, ProgressBar, Rect, RichText, Stroke, Ui, Vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use nervous_system::specialist_bus::{SpecialistSynapseBus, TENSOR_DIM};

pub struct BusVisualizer {
    pub bus: Arc<SpecialistSynapseBus>,
    pub last_seq_numbers: [u64; 11],
    pub latest_tensors: [[f32; TENSOR_DIM]; 11],
    // Sentinel Deep SVDD Reference Manifold
    pub sentinel_centroid: [f32; TENSOR_DIM],
    pub sentinel_radius: f32,
    pub total_snaps_observed: u64,
}

impl BusVisualizer {
    pub fn new(bus: Arc<SpecialistSynapseBus>, centroid: [f32; TENSOR_DIM], radius: f32) -> Self {
        Self {
            bus,
            last_seq_numbers: [0; 11],
            latest_tensors: [[0.0; TENSOR_DIM]; 11],
            sentinel_centroid: centroid,
            sentinel_radius: radius.max(1.0),
            total_snaps_observed: 0,
        }
    }

    /// Sample the lock-free bus without introducing memory barriers or thread locks
    pub fn sample_bus_state(&mut self) {
        for (i, channel) in self.bus.channels.iter().enumerate().take(11) {
            let current_seq = channel.write_cursor.value.load(Ordering::Relaxed);
            if current_seq > self.last_seq_numbers[i] {
                self.last_seq_numbers[i] = current_seq;
                if let Some(tensor) = channel.read_latest(200) {
                    self.latest_tensors[i] = tensor;
                }
            }
        }
    }

    /// Render oscilloscope and safety telemetry at 60Hz
    pub fn update_ui(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        self.sample_bus_state();

        ui.heading("⚡ SPMC Synapse Bus Oscilloscope & Sentinel Guardrail");
        ui.label("Sub-microsecond 128-byte aligned zero-copy telemetry over 11 federated specialist channels.");
        ui.add_space(8.0);

        // 1. Sentinel Safety Threat Gauge Calculation
        let tensor = &self.latest_tensors[0]; // Router broadcast intent
        let dist_sq: f32 = tensor
            .iter()
            .zip(&self.sentinel_centroid)
            .map(|(t, c)| (t - c).powi(2))
            .sum();
        let current_distance = dist_sq.sqrt();
        let threat_ratio = (current_distance / self.sentinel_radius).clamp(0.0, 2.0);

        if threat_ratio > 1.0 {
            self.total_snaps_observed += 1;
        }

        // 2. Render Threat Gauge Card
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Sentinel Deep SVDD Manifold Deviation:").strong());
                let (gauge_color, status_text) = if threat_ratio < 0.7 {
                    (Color32::from_rgb(0, 255, 128), "SAFE (In-Distribution)")
                } else if threat_ratio <= 1.0 {
                    (Color32::from_rgb(255, 200, 0), "ELEVATED (Approaching Boundary)")
                } else {
                    (Color32::from_rgb(255, 50, 50), "⚠️ ORTHOGONAL SNAP ENGAGED")
                };

                ui.colored_label(gauge_color, status_text);
            });

            ui.add(
                ProgressBar::new((threat_ratio / 1.0).min(1.0))
                    .fill(if threat_ratio > 1.0 { Color32::RED } else if threat_ratio > 0.7 { Color32::YELLOW } else { Color32::GREEN })
                    .text(format!("{:.3} / {:.3} R ({:.1}%)", current_distance, self.sentinel_radius, threat_ratio * 100.0)),
            );

            if threat_ratio > 1.0 {
                ui.colored_label(Color32::RED, format!("🚨 Out-of-Distribution Vector Snapped (Total Interceptions: {})", self.total_snaps_observed));
            }
        });

        ui.add_space(8.0);

        // 3. Custom Painter Oscilloscope for R^256 Intent Activation
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Router Intent Activations (R^256 Manifold):").strong());
                ui.label(format!("Cursor Epoch: {}", self.last_seq_numbers[0]));
            });

            let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), 160.0), egui::Sense::hover());
            let rect = response.rect;

            // Background void
            painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 16, 22));
            painter.rect_stroke(rect, egui::CornerRadius::same(4), Stroke::new(1.0, Color32::from_rgb(35, 45, 60)), egui::StrokeKind::Inside);

            let bar_width = (rect.width() / TENSOR_DIM as f32).max(1.0);
            let center_y = rect.center().y;
            let max_bar_h = rect.height() * 0.45;

            // Draw 256 bars
            for (i, &val) in tensor.iter().enumerate() {
                let x = rect.left() + i as f32 * bar_width;
                let h = (val.abs() * max_bar_h * 0.5).clamp(1.0, max_bar_h);
                let bar_rect = if val >= 0.0 {
                    Rect::from_min_max(Pos2::new(x, center_y - h), Pos2::new(x + bar_width * 0.85, center_y))
                } else {
                    Rect::from_min_max(Pos2::new(x, center_y), Pos2::new(x + bar_width * 0.85, center_y + h))
                };

                let color = if val > 0.5 {
                    Color32::from_rgb(0, 255, 150) // High positive
                } else if val < -0.5 {
                    Color32::from_rgb(255, 90, 90) // High negative
                } else {
                    Color32::from_rgb(70, 130, 220) // Normal active
                };

                painter.rect_filled(bar_rect, 1.0, color);
            }

            // Draw Centroid baseline line across bars
            let mut points = Vec::with_capacity(TENSOR_DIM);
            for (i, &c_val) in self.sentinel_centroid.iter().enumerate() {
                let x = rect.left() + i as f32 * bar_width;
                let y = center_y - (c_val * max_bar_h * 0.5).clamp(-max_bar_h, max_bar_h);
                points.push(Pos2::new(x, y));
            }
            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment([points[i], points[i + 1]], Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 220, 0, 180)));
            }
        });

        ui.add_space(8.0);

        // 4. Specialist Subscribers Status
        ui.horizontal(|ui| {
            for (i, name) in ["Router", "Desktop Emulator", "Adaptation Engine", "Sentinel"].iter().enumerate() {
                let is_active = self.last_seq_numbers[i] > 0;
                let badge_color = if is_active { Color32::GREEN } else { Color32::GRAY };
                ui.colored_label(badge_color, format!("● {}", name));
            }
        });

        ctx.request_repaint(); // Maintain continuous 60Hz oscilloscope animation
    }
}
