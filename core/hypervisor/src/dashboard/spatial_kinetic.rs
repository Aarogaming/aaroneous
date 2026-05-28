// Spatial-Kinetic Engine Telemetry Panel
// Monitors the universal gaming genome, epigenetic gating matrix, and reflex kernel performance.

use eframe::egui;
use std::time::Instant;

pub struct SpatialKineticTelemetry {
    pub gate_matrix_active: u32,
    pub gate_matrix_total: u32,
    pub skip_ratio: f32,
    pub genome_voxels: u64,
    pub genome_tracks: u32,
    pub frame_fps: f32,
    pub compute_latency_us: f32,
    pub vram_usage_mb: f32,
    pub reflex_dispatches: u64,
    pub motor_intents: u64,
    pub last_update: Instant,
    pub frame_history: Vec<f32>,
    pub skip_history: Vec<f32>,
    pub delta_mean: f32,
    pub delta_max: f32,
    pub intent_dx: f32,
    pub intent_dy: f32,
    pub live_connected: bool,
}

impl Default for SpatialKineticTelemetry {
    fn default() -> Self {
        Self {
            gate_matrix_active: 256,
            gate_matrix_total: 256,
            skip_ratio: 0.0,
            genome_voxels: 1_289_158_774,
            genome_tracks: 16,
            frame_fps: 60.0,
            compute_latency_us: 120.0,
            vram_usage_mb: 4917.0,
            reflex_dispatches: 0,
            motor_intents: 0,
            last_update: Instant::now(),
            frame_history: vec![60.0; 60],
            skip_history: vec![0.0; 60],
            delta_mean: 0.0,
            delta_max: 0.0,
            intent_dx: 0.0,
            intent_dy: 0.0,
            live_connected: false,
        }
    }
}

impl SpatialKineticTelemetry {
    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f32();
        if elapsed < 0.1 {
            return;
        }
        self.last_update = now;

        self.frame_history.remove(0);
        self.frame_history.push(self.frame_fps);
        self.skip_history.remove(0);
        self.skip_history.push(self.skip_ratio);
    }

    pub fn render_panel(&mut self, ui: &mut egui::Ui) {
        self.update();

        ui.heading("Spatial-Kinetic Engine");
        ui.add_space(8.0);

        // Live Connection Status
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 120)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Live Bridge:");
                    let status_color = if self.live_connected {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };
                    let status_text = if self.live_connected {
                        "CONNECTED"
                    } else {
                        "DISCONNECTED - Run live_telemetry_bridge.py"
                    };
                    ui.label(egui::RichText::new(status_text).color(status_color).strong());
                });
            });

        ui.add_space(8.0);

        // Genome Status
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 120)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("🧬 Universal Gaming Genome").strong().color(egui::Color32::from_rgb(100, 180, 255)));
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("Source:");
                    ui.label(egui::RichText::new("universal_gaming_core.bin").color(egui::Color32::GREEN));
                });
                ui.horizontal(|ui| {
                    ui.label("Total Voxels:");
                    ui.label(format!("{}", self.genome_voxels));
                });
                ui.horizontal(|ui| {
                    ui.label("Genome Size:");
                    ui.label(format!("{:.0} MB", self.genome_voxels as f32 * 4.0 / 1024.0 / 1024.0 * 1_000_000.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Active Tracks:");
                    ui.label(format!("{}/16", self.genome_tracks));
                });
                ui.horizontal(|ui| {
                    ui.label("Encoding:");
                    ui.label(egui::RichText::new("2-bit (A/T/C/G)").color(egui::Color32::YELLOW));
                });
            });

        ui.add_space(8.0);

        // Epigenetic Gating Matrix
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 120)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("👁 Epigenetic Visual Gating").strong().color(egui::Color32::from_rgb(100, 180, 255)));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Active Sectors:");
                    ui.label(format!("{}/{}", self.gate_matrix_active, self.gate_matrix_total));
                });

                ui.horizontal(|ui| {
                    ui.label("Compute Skip Ratio:");
                    let skip_pct = self.skip_ratio * 100.0;
                    let skip_color = if skip_pct > 70.0 {
                        egui::Color32::GREEN
                    } else if skip_pct > 40.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.label(egui::RichText::new(format!("{:.1}%", skip_pct)).color(skip_color).strong());
                });

                ui.add(egui::ProgressBar::new(self.skip_ratio)
                    .text(format!("{}% zero-compute skip", (self.skip_ratio * 100.0) as u32))
                    .fill(if self.skip_ratio > 0.7 { egui::Color32::GREEN } else { egui::Color32::YELLOW }));

                ui.add_space(4.0);
                ui.label(egui::RichText::new("Skip Ratio History (60 frames)").small().color(egui::Color32::GRAY));
                self.render_skip_graph(ui);
            });

        ui.add_space(8.0);

        // Reflex Kernel Performance
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 120)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("⚡ Reflex Kernel (WebGPU)").strong().color(egui::Color32::from_rgb(100, 180, 255)));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Frame Rate:");
                    let fps_color = if self.frame_fps >= 55.0 {
                        egui::Color32::GREEN
                    } else if self.frame_fps >= 30.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.label(egui::RichText::new(format!("{:.0} FPS", self.frame_fps)).color(fps_color).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("Compute Latency:");
                    ui.label(format!("{:.1} μs", self.compute_latency_us));
                });

                ui.horizontal(|ui| {
                    ui.label("VRAM Usage:");
                    ui.label(format!("{:.0} MB", self.vram_usage_mb));
                });

                ui.horizontal(|ui| {
                    ui.label("Reflex Dispatches:");
                    ui.label(format!("{}", self.reflex_dispatches));
                });

                ui.horizontal(|ui| {
                    ui.label("Motor Intents:");
                    ui.label(format!("{}", self.motor_intents));
                });

                ui.add_space(4.0);
                ui.label(egui::RichText::new("FPS History (60 frames)").small().color(egui::Color32::GRAY));
                self.render_fps_graph(ui);
            });

        ui.add_space(8.0);

        // Live Frame Analysis
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 120)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Live Frame Analysis").strong().color(egui::Color32::from_rgb(100, 180, 255)));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Delta Mean:");
                    ui.label(format!("{:.6}", self.delta_mean));
                });
                ui.horizontal(|ui| {
                    ui.label("Delta Max:");
                    ui.label(format!("{:.4}", self.delta_max));
                });
                ui.horizontal(|ui| {
                    ui.label("Intent X:");
                    ui.label(format!("{:.2}", self.intent_dx));
                });
                ui.horizontal(|ui| {
                    ui.label("Intent Y:");
                    ui.label(format!("{:.2}", self.intent_dy));
                });
            });

        ui.add_space(8.0);

        // 16-Way Track Status
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 120)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("🔬 16-Way Genome Tracks").strong().color(egui::Color32::from_rgb(100, 180, 255)));
                ui.add_space(4.0);

                let track_names = [
                    ("0", "nav_pathfinding", "spatial", egui::Color32::from_rgb(0, 150, 255)),
                    ("1", "nav_collision", "spatial", egui::Color32::from_rgb(0, 150, 255)),
                    ("2", "nav_terrain", "spatial", egui::Color32::from_rgb(0, 150, 255)),
                    ("3", "nav_flowfield", "spatial", egui::Color32::from_rgb(0, 150, 255)),
                    ("4", "state_transitions", "logic", egui::Color32::from_rgb(0, 255, 150)),
                    ("5", "state_hierarchy", "logic", egui::Color32::from_rgb(0, 255, 150)),
                    ("6", "state_rewards", "logic", egui::Color32::from_rgb(0, 255, 150)),
                    ("7", "state_memory", "logic", egui::Color32::from_rgb(0, 255, 150)),
                    ("8", "vis_object_detect", "visual", egui::Color32::from_rgb(255, 150, 0)),
                    ("9", "vis_motion_track", "visual", egui::Color32::from_rgb(255, 150, 0)),
                    ("10", "vis_threat_assess", "visual", egui::Color32::from_rgb(255, 150, 0)),
                    ("11", "vis_hud_parse", "visual", egui::Color32::from_rgb(255, 150, 0)),
                    ("12", "res_budget_alloc", "resource", egui::Color32::from_rgb(200, 100, 255)),
                    ("13", "res_timing_opt", "resource", egui::Color32::from_rgb(200, 100, 255)),
                    ("14", "res_risk_eval", "resource", egui::Color32::from_rgb(200, 100, 255)),
                    ("15", "res_adapt_rate", "resource", egui::Color32::from_rgb(200, 100, 255)),
                ];

                egui::Grid::new("track_grid").striped(true).show(ui, |ui| {
                    ui.label(egui::RichText::new("Track").strong());
                    ui.label(egui::RichText::new("Name").strong());
                    ui.label(egui::RichText::new("Domain").strong());
                    ui.label(egui::RichText::new("Status").strong());
                    ui.end_row();

                    for (id, name, domain, color) in &track_names {
                        ui.label(egui::RichText::new(format!("{}", id)).color(*color));
                        ui.label(name.to_string());
                        ui.label(domain.to_string());
                        ui.label(egui::RichText::new("●").color(egui::Color32::GREEN));
                        ui.end_row();
                    }
                });
            });
    }

    fn render_skip_graph(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 40.0),
            egui::Sense::hover(),
        );
        
        let painter = ui.painter_at(rect);
        let bar_width = rect.width() / self.skip_history.len() as f32;
        
        for (i, &val) in self.skip_history.iter().enumerate() {
            let x = rect.min.x + i as f32 * bar_width;
            let h = val * rect.height();
            let y = rect.max.y - h;
            let color = if val > 0.7 {
                egui::Color32::from_rgb(0, 200, 100)
            } else if val > 0.4 {
                egui::Color32::from_rgb(200, 200, 0)
            } else {
                egui::Color32::from_rgb(200, 50, 50)
            };
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width - 0.5, h)),
                0.0,
                color,
            );
        }
    }

    fn render_fps_graph(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 40.0),
            egui::Sense::hover(),
        );
        
        let painter = ui.painter_at(rect);
        let bar_width = rect.width() / self.frame_history.len() as f32;
        let max_fps = 120.0;
        
        for (i, &fps) in self.frame_history.iter().enumerate() {
            let x = rect.min.x + i as f32 * bar_width;
            let h = (fps / max_fps) * rect.height();
            let y = rect.max.y - h;
            let color = if fps >= 55.0 {
                egui::Color32::from_rgb(0, 200, 100)
            } else if fps >= 30.0 {
                egui::Color32::from_rgb(200, 200, 0)
            } else {
                egui::Color32::from_rgb(200, 50, 50)
            };
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width - 0.5, h)),
                0.0,
                color,
            );
        }
    }
}
