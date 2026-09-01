//! core/hypervisor/src/hypervisor_hud.rs
//! Unified Hypervisor Telemetry HUD & Visualizer Subsystem (egui / eframe).
//!
//! Consolidates 4 Core Visual Telemetry Viewports:
//! 1. 🌌 3D Omni Galaxy View: Star-Nodes, Gravitational Clustering, Semantic Cosine Distance.
//! 2. ⚡ SPMC SignalBridge & Sentinel SVDD: 256-Bar Latent Vector Signal Analyzer & Threat Gauge.
//! 3. 👁️ Spatial Delta Vision & Sensory Grid: 16x16 Motion Saliency Mask & Compute Savings.
//! 4. 🧬 System Thermodynamics: 4-Channel Feedback Signals, Curiosity Impulses & Token Pool.

use std::sync::Arc;
use eframe::egui::{self, Color32, RichText, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};

use evolution::{NeurochemicalHomeostasisEngine, NeurochemicalLevels};
use platform_bridge::SensoryMotorPipeline;
use nervous_system::specialist_bus::{SpecialistSynapseBus, TENSOR_DIM};
use omni::{OmniEngine, SpatialCoord, StarNode, StarNodeType};

use crate::bus_visualizer::BusVisualizer;

/// Navigation tabs in the Unified Hypervisor HUD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudTab {
    Galaxy3D,
    SignalAnalyzer,
    SpatialDeltaSensory,
    SystemThermodynamics,
}

/// The Master Unified Hypervisor HUD Desktop App
pub struct HypervisorHudApp {
    pub active_tab: HudTab,
    pub omni_engine: Arc<OmniEngine>,
    pub bus_visualizer: BusVisualizer,
    pub sensory_pipeline: SensoryMotorPipeline,
    pub neurochemistry: NeurochemicalHomeostasisEngine,
    pub tick_counter: u64,
    pub is_simulating: bool,
    pub last_frame_savings_pct: f32,
    pub last_frame_active_sectors: usize,
    pub last_active_mask: [bool; 256],
    pub simulated_frame: Vec<f32>,
}

impl Default for HypervisorHudApp {
    fn default() -> Self {
        Self::new()
    }
}

fn block_on_future<F: std::future::Future>(f: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(f))
    } else {
        tokio::runtime::Runtime::new().unwrap().block_on(f)
    }
}

impl HypervisorHudApp {
    /// Initializes the Unified Hypervisor HUD with live subsystems
    pub fn new() -> Self {
        let omni = Arc::new(OmniEngine::default());
        let bus = Arc::new(SpecialistSynapseBus::new_federation());
        let centroid = [0.05f32; TENSOR_DIM];
        let radius = 14.5f32;

        let bus_visualizer = BusVisualizer::new(bus, centroid, radius);
        let sensory_pipeline = SensoryMotorPipeline::new("Aaroneous_Hypervisor_HUD");
        let neurochemistry = NeurochemicalHomeostasisEngine::new(NeurochemicalLevels::new(0.85, 0.50, 0.35, 0.90));

        let mut app = Self {
            active_tab: HudTab::Galaxy3D,
            omni_engine: omni,
            bus_visualizer,
            sensory_pipeline,
            neurochemistry,
            tick_counter: 0,
            is_simulating: true,
            last_frame_savings_pct: 0.0,
            last_frame_active_sectors: 256,
            last_active_mask: [false; 256],
            simulated_frame: vec![0.0f32; 128 * 128],
        };

        app.bootstrap_sample_data();
        app
    }

    /// Seeds initial mock data for live rendering
    pub fn bootstrap_sample_data(&mut self) {
        // Seed standard specialists into Omni Galaxy if empty
        let specs = [
            ("orchestrator", "Orchestrator (Orchestration)", "Orchestration", SpatialCoord::new(-450.0, 300.0, 800.0)),
            ("synthesizer", "Synthesizer (Knowledge)", "Knowledge", SpatialCoord::new(200.0, -150.0, 600.0)),
            ("presenter", "Presenter (Presentation)", "UI", SpatialCoord::new(500.0, 400.0, 400.0)),
            ("fabricator", "Fabricator (Forge)", "Fabrication", SpatialCoord::new(-200.0, -500.0, 700.0)),
            ("sentinel", "Sentinel (Security)", "Security", SpatialCoord::new(0.0, 0.0, 950.0)),
            ("archivist", "Archivist (Memory)", "Memory", SpatialCoord::new(350.0, -350.0, 500.0)),
            ("router", "Router (Router)", "Network", SpatialCoord::new(-600.0, 100.0, 450.0)),
            ("aligner", "Aligner (Symbiosis)", "Symbiosis", SpatialCoord::new(150.0, 550.0, 300.0)),
            ("perceiver", "Perceiver (Perception)", "Vision", SpatialCoord::new(-350.0, 600.0, 650.0)),
        ];

        for (id, title, domain, coord) in specs {
            let star = StarNode::new(id, title, StarNodeType::Specialist, domain, coord, "omni://specialist");
            let engine = self.omni_engine.clone();
            block_on_future(async move {
                engine.insert_node(star).await;
            });
        }
    }

    /// Step background simulation tick
    pub fn step_simulation(&mut self) {
        if !self.is_simulating {
            return;
        }

        self.tick_counter += 1;

        // 1. Step Omni Galaxy gravitational physics relaxation
        let engine = self.omni_engine.clone();
        block_on_future(async move {
            engine.step_gravitational_physics(0.05).await;
        });

        // 2. Step Sensory-Motor pipeline with dynamic moving target
        let center_x = 32 + (self.tick_counter as usize * 4) % 64;
        let center_y = 32 + (self.tick_counter as usize * 3) % 64;
        self.simulated_frame.fill(0.0);
        for dy in 0..10 {
            for dx in 0..10 {
                let idx = (center_y + dy) * 128 + (center_x + dx);
                if idx < self.simulated_frame.len() {
                    self.simulated_frame[idx] = 0.90;
                }
            }
        }

        let frame_clone = self.simulated_frame.clone();
        let report = block_on_future(self.sensory_pipeline.step_cycle(&frame_clone));
        if let Ok(rep) = report {
            self.last_frame_savings_pct = rep.compute_savings_pct;
            self.last_frame_active_sectors = rep.active_sectors;
        }

        // 3. Step Neurochemical homeostatic decay
        self.neurochemistry.step_homeostasis(0.016);
    }

    /// Render 3D Omni Galaxy Viewport
    pub fn render_galaxy_tab(&mut self, ui: &mut Ui) {
        ui.heading("🌌 3D Omni Galaxy Spatial Star-Graph");
        ui.label("Real-time N-body gravitational physics, Coulomb repulsion, and semantic cosine attraction.");
        ui.add_space(8.0);

        let engine = self.omni_engine.clone();
        let nodes = block_on_future(async move {
            engine.get_all_nodes().await
        });

        ui.horizontal(|ui| {
            ui.label(format!("Total Star-Nodes: {}", nodes.len()));
            if ui.button("⚡ Step Gravitational Physics").clicked() {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.omni_engine.step_gravitational_physics(0.1).await;
                    });
                });
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("galaxy_grid").striped(true).min_col_width(100.0).show(ui, |ui| {
                ui.label(RichText::new("Node ID").strong());
                ui.label(RichText::new("Type").strong());
                ui.label(RichText::new("Domain").strong());
                ui.label(RichText::new("3D Coordinates (X, Y, Z)").strong());
                ui.label(RichText::new("Mass").strong());
                ui.end_row();

                for (_, node) in nodes {
                    ui.label(&node.id);
                    ui.label(format!("{:?}", node.node_type));
                    ui.label(&node.domain);
                    ui.label(format!("({:+.1}, {:+.1}, {:+.1})", node.spatial_coord.x, node.spatial_coord.y, node.spatial_coord.z));
                    ui.label(format!("{:.2}", node.activity_pulse));
                    ui.end_row();
                }
            });
        });
    }

    /// Render Epigenetic Vision Viewport
    pub fn render_epigenetic_tab(&mut self, ui: &mut Ui) {
        ui.heading("👁️ Epigenetic Visual Motion Gating (16x16 Grid)");
        ui.label("Frame-over-frame delta filtering with 3-frame hysteresis damping and dormant sector skipping.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Active Sectors: {} / 256", self.last_frame_active_sectors)).strong());
            ui.label(RichText::new(format!("Compute Savings: {:.1}%", self.last_frame_savings_pct)).color(Color32::from_rgb(0, 255, 128)));
        });

        ui.add_space(8.0);

        // 16x16 interactive grid canvas
        let (rect, _response) = ui.allocate_exact_size(Vec2::new(320.0, 320.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        let cell_w = rect.width() / 16.0;
        let cell_h = rect.height() / 16.0;

        let active_cnt = self.last_frame_active_sectors;

        for y in 0..16 {
            for x in 0..16 {
                let idx = y * 16 + x;
                let is_active = idx < active_cnt;

                let cell_rect = egui::Rect::from_min_size(
                    rect.min + Vec2::new(x as f32 * cell_w, y as f32 * cell_h),
                    Vec2::new(cell_w - 1.0, cell_h - 1.0),
                );

                let fill_color = if is_active {
                    Color32::from_rgb(0, 230, 110)
                } else {
                    Color32::from_rgb(25, 28, 36)
                };

                painter.rect_filled(cell_rect, 2.0, fill_color);
                painter.rect_stroke(cell_rect, 1.0, Stroke::new(1.0, Color32::from_rgb(45, 48, 60)), egui::StrokeKind::Inside);
            }
        }
    }

    /// Render Neurochemical Homeostasis Viewport
    pub fn render_neurochemistry_tab(&mut self, ui: &mut Ui) {
        ui.heading("🧬 Proactive Neurochemical Homeostatic Drive");
        ui.label("Continuous 4-channel dynamics, curiosity drive impulses, and metabolic token allocation.");
        ui.add_space(8.0);

        let levels = self.neurochemistry.levels;

        ui.group(|ui| {
            ui.label(RichText::new("4-Channel Neurotransmitter State:").strong());
            ui.horizontal(|ui| {
                ui.label(format!("Dopamine (Reward): {:.2}", levels.dopamine));
                ui.label(format!("Serotonin (Harmony): {:.2}", levels.serotonin));
                ui.label(format!("Noradrenaline (Vigilance): {:.2}", levels.noradrenaline));
                ui.label(format!("Acetylcholine (Plasticity): {:.2}", levels.acetylcholine));
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(format!("Boredom: {:.1}%", levels.boredom_index() * 100.0));
                ui.label(format!("Curiosity: {:.1}%", levels.curiosity_drive() * 100.0));
                ui.label(format!("Stress: {:.1}%", levels.stress_index() * 100.0));
                ui.label(format!("Metabolism: {:.2}x", levels.metabolic_multiplier()));
            });
        });

        ui.add_space(8.0);
        ui.heading("⚡ Specialist Federation Token Allocations (900 Pool)");

        let tokens = self.neurochemistry.calculate_token_distribution(900.0);
        egui::Grid::new("tokens_grid").striped(true).min_col_width(120.0).show(ui, |ui| {
            ui.label(RichText::new("Specialist").strong());
            ui.label(RichText::new("Opcode").strong());
            ui.label(RichText::new("Tokens").strong());
            ui.label(RichText::new("Allocation Rationale").strong());
            ui.end_row();

            for alloc in tokens {
                ui.label(&alloc.specialist_name);
                ui.label(format!("0x{:04X}", alloc.domain_opcode));
                ui.label(format!("{:.0}", alloc.allocated_tokens));
                ui.label(&alloc.boost_reason);
                ui.end_row();
            }
        });
    }
}

impl eframe::App for HypervisorHudApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint();
        self.step_simulation();

        // 1. Top Bar Navigation Panel
        ui.horizontal(|ui| {
            ui.heading(RichText::new("⚡ AARONEOUS HYPERVISOR HUD").strong().color(Color32::from_rgb(0, 210, 255)));
            ui.separator();

            ui.selectable_value(&mut self.active_tab, HudTab::Galaxy3D, "🌌 3D Galaxy");
            ui.selectable_value(&mut self.active_tab, HudTab::SignalAnalyzer, "⚡ Bus & SVDD");
            ui.selectable_value(&mut self.active_tab, HudTab::SpatialDeltaSensory, "👁️ Spatial Delta Vision");
            ui.selectable_value(&mut self.active_tab, HudTab::SystemThermodynamics, "🧬 System Thermodynamics");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.is_simulating, "Live 60Hz Sim");
                ui.label(format!("Frame #{:06}", self.tick_counter));
            });
        });

        ui.separator();

        // 2. Central Content Viewport
        let ctx = ui.ctx().clone();
        match self.active_tab {
            HudTab::Galaxy3D => self.render_galaxy_tab(ui),
            HudTab::SignalAnalyzer => self.bus_visualizer.update_ui(&ctx, ui),
            HudTab::SpatialDeltaSensory => self.render_epigenetic_tab(ui),
            HudTab::SystemThermodynamics => self.render_neurochemistry_tab(ui),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hud_app_initialization_and_tabs() {
        let mut app = HypervisorHudApp::new();
        assert_eq!(app.active_tab, HudTab::Galaxy3D);
        assert!(app.is_simulating);

        app.step_simulation();
        assert_eq!(app.tick_counter, 1);

        app.active_tab = HudTab::SystemThermodynamics;
        assert_eq!(app.active_tab, HudTab::SystemThermodynamics);
    }
}
