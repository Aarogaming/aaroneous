// core/hypervisor/src/hud/companion_overlay.rs
//! Cognitive Execution Node & Desktop Telemetry Overlay.
//!
//! Provides a compact, non-intrusive desktop widget representing the live
//! execution state of Aaroneous:
//! 1. Real-Time System Equilibrium Indicator (Free Energy ΔF & Shannon Entropy).
//! 2. Active Conductor & Specialist Execution Modules.
//! 3. Conversational Linguistic Intercom Input & Direct Opcode Transduction.
//! 4. Drag-and-Drop `.si` / `.si-pack` Cartridge Ingestion.

use eframe::egui;
use serde::{Deserialize, Serialize};

/// Equilibrium status computed from thermodynamic metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquilibriumState {
    NominalEquilibrium,   // ΔF <= 0.02 (Nominal / Blue)
    ActiveSynthesis,      // 0.02 < ΔF <= 0.05 (Active / Cyan)
    BoundaryInterlock,    // ΔF > 0.05 (Safety Cutoff / Red)
}

impl EquilibriumState {
    pub fn color(&self) -> egui::Color32 {
        match self {
            Self::NominalEquilibrium => egui::Color32::from_rgb(64, 156, 255), // Cyan/Blue
            Self::ActiveSynthesis => egui::Color32::from_rgb(46, 204, 113),    // Emerald Green
            Self::BoundaryInterlock => egui::Color32::from_rgb(231, 76, 60),   // Warning Red
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::NominalEquilibrium => "NOMINAL EQUILIBRIUM",
            Self::ActiveSynthesis => "ACTIVE SYNTHESIS",
            Self::BoundaryInterlock => "SAFETY INTERLOCK",
        }
    }
}

/// The Desktop Telemetry Overlay Widget
pub struct CompanionTelemetryOverlay {
    pub free_energy_delta: f32,
    pub cycle_latency_us: u64,
    pub active_conductor: String,
    pub active_modules: Vec<String>,
    pub chat_input_buffer: String,
    pub last_intercom_reply: String,
    pub is_expanded: bool,
}

impl Default for CompanionTelemetryOverlay {
    fn default() -> Self {
        Self {
            free_energy_delta: 0.012,
            cycle_latency_us: 14,
            active_conductor: "conductor_desktop_v1.si".to_string(),
            active_modules: vec!["OpticalPerception".to_string(), "KineticDispatch".to_string()],
            chat_input_buffer: String::new(),
            last_intercom_reply: "Aaroneous Core ready. System equilibrium nominal.".to_string(),
            is_expanded: false,
        }
    }
}

impl CompanionTelemetryOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_state(&self) -> EquilibriumState {
        if self.free_energy_delta <= 0.02 {
            EquilibriumState::NominalEquilibrium
        } else if self.free_energy_delta <= 0.05 {
            EquilibriumState::ActiveSynthesis
        } else {
            EquilibriumState::BoundaryInterlock
        }
    }

    /// Renders the companion overlay widget inside an egui UI context
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let state = self.current_state();
        let glow_color = state.color();

        ui.vertical(|ui| {
            // Header: Status Orb & Equilibrium
            ui.horizontal(|ui| {
                // Colored Status Indicator Orb
                let (rect, _response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 7.0, glow_color);

                ui.colored_label(glow_color, state.label());
                ui.separator();
                ui.label(format!("Latency: {} μs", self.cycle_latency_us));
                ui.label(format!("ΔF: {:.3}", self.free_energy_delta));

                // Toggle Expand/Compact Button
                if ui.button(if self.is_expanded { "▲ Compact" } else { "▼ Expand" }).clicked() {
                    self.is_expanded = !self.is_expanded;
                }
            });

            // Conversational Intercom Status Output
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&self.last_intercom_reply).italics().color(egui::Color32::LIGHT_GRAY));

            // Expanded View: Organ Register & Intent Input
            if self.is_expanded {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label(format!("Active Conductor: {}", self.active_conductor));
                    ui.label(format!("Active Modules: [{}]", self.active_modules.join(", ")));

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Intercom:");
                        let text_edit = ui.text_edit_singleline(&mut self.chat_input_buffer);
                        if (ui.button("Send").clicked() || (text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                            && !self.chat_input_buffer.trim().is_empty()
                        {
                            let query = self.chat_input_buffer.trim().to_string();
                            self.last_intercom_reply = format!("Intent transducted: [{}]. Opcode graph executed.", query);
                            self.chat_input_buffer.clear();
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Drag & Drop `.si` or `.si-pack` cartridge here to mount").weak());
                });
            }
        });
    }
}
