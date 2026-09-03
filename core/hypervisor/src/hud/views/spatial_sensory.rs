// core/hypervisor/src/hud/views/spatial_sensory.rs
//! Cognitive Specialists, Neurochemistry, Swarm Mesh, and Skill Tree views.

use crate::hud::navigation::NavSection;
use crate::hud::state::SharedHudState;
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

#[derive(Default)]
pub struct SpatialSensoryView;

impl HudView for SpatialSensoryView {
    fn id(&self) -> &'static str {
        "spatial_sensory"
    }

    fn title(&self) -> &'static str {
        "👥 Specialists & Cognition"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        match state.nav_section {
            NavSection::Specialists => {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("👥 9 Domain Specialists & Autonomous Hive Intent")
                            .color(theme.accent())
                            .strong(),
                    );
                });
                ui.label(
                    "Deterministic domain specialist delegation with non-linguistic continuous state coordination.",
                );
                ui.separator();

                // Hive Intent Input Deck
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(8))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Submit Hive Task Intent:").strong());
                            ui.add(egui::TextEdit::singleline(&mut state.hive_intent_input).desired_width(420.0));
                            if ui.button("⚡ Dispatch Intent").clicked() && !state.hive_intent_input.trim().is_empty() {
                                state.hive_routing_decision = Some("Routed intent to Synthesizer (0x0200) & DevTools (0x0400)".to_string());
                                state.hive_routing_trace.push(format!("Task: '{}' -> Completed.", state.hive_intent_input));
                                state.hive_intent_input.clear();
                            }
                        });
                        if let Some(dec) = &state.hive_routing_decision {
                            ui.label(egui::RichText::new(dec).color(theme.accent()));
                        }
                    });

                ui.add_space(8.0);

                // Grid of the 9 Specialists
                let specialists = [
                    ("01. Orchestrator", "0x0100", "Central task decomposition & dynamic DAG execution.", Color32::from_rgb(255, 215, 0)),
                    ("02. Synthesizer", "0x0200", "Polyglot code synthesis, AST rewrite & compilation.", Color32::from_rgb(163, 113, 247)),
                    ("03. Presenter", "0x0300", "DirectX 12/Vulkan frame composition & interactive HUD.", Color32::from_rgb(56, 139, 253)),
                    ("04. DevTools", "0x0400", "Automated FFI wrapper synthesis & live memory repair.", Color32::from_rgb(240, 136, 62)),
                    ("05. Sentinel", "0x0500", "SVDD latent security manifold & containment checks.", Color32::from_rgb(248, 81, 73)),
                    ("06. Archivist", "0x0600", "3D semantic knowledge graph clustering & indexing.", Color32::from_rgb(121, 192, 255)),
                    ("07. Router", "0x0700", "P2P streaming TCP mesh multiplexer & gossip consensus.", Color32::from_rgb(63, 185, 80)),
                    ("08. Aligner", "0x0800", "Federation policy alignment & safety arbitration.", Color32::from_rgb(219, 109, 40)),
                    ("09. Perceiver", "0x0900", "DXGI screen capture & low-latency perceptual gating.", Color32::from_rgb(88, 166, 255)),
                ];

                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for (name, opcode, desc, color) in &specialists {
                        egui::Frame::group(ui.style())
                            .fill(theme.card_bg())
                            .stroke(Stroke::new(1.0, theme.border_color()))
                            .corner_radius(CornerRadius::same(6))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("✦").color(*color).size(16.0));
                                    ui.label(egui::RichText::new(*name).strong().color(*color));
                                    ui.label(egui::RichText::new(format!("({})", opcode)).color(Color32::GRAY).monospace());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new("ONLINE").color(Color32::from_rgb(63, 185, 80)).strong().size(10.0));
                                    });
                                });
                                ui.label(egui::RichText::new(*desc).size(11.0).color(Color32::from_rgb(200, 210, 225)));
                            });
                        ui.add_space(4.0);
                    }
                });
            }

            NavSection::LearningAndSelfPlay | NavSection::LivingMind => {
                ui.heading(
                    egui::RichText::new("🧬 Neurochemistry & Self-Play Learning")
                        .color(theme.accent())
                        .strong(),
                );
                ui.label("Real-time neurochemical modulator concentrations and Alice vs Bob adversarial simulations.");
                ui.separator();

                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(egui::RichText::new("Neurochemical Modulator Concentrations").strong());
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Dopamine (Reward/Salience):");
                            ui.add(egui::ProgressBar::new(state.living_mind_dopamine).text(format!("{:.0}%", state.living_mind_dopamine * 100.0)));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Acetylcholine (Focus/Rate):");
                            ui.add(egui::ProgressBar::new(state.living_mind_acetylcholine).text(format!("{:.0}%", state.living_mind_acetylcholine * 100.0)));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Serotonin (Stability/Equil):");
                            ui.add(egui::ProgressBar::new(state.living_mind_serotonin).text(format!("{:.0}%", state.living_mind_serotonin * 100.0)));
                        });
                    });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Adversarial Self-Play Dream Duels (Alice vs Bob)").strong().color(theme.accent()));
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for entry in &state.dream_duel_history {
                        ui.label(egui::RichText::new(entry).size(11.0));
                    }
                });
            }

            NavSection::SwarmMesh | NavSection::GhostStation => {
                ui.heading(
                    egui::RichText::new("🌐 FederationBus Multi-Hive P2P Swarm Mesh")
                        .color(theme.accent())
                        .strong(),
                );
                ui.label("Distributed work stealing, Byzantine fault tolerance & P2P .si cartridge sync.");
                ui.separator();

                ui.horizontal(|ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(Vec2::new(180.0, 70.0));
                        ui.label("Active Quorums:");
                        ui.heading(format!("{}", state.swarm_live_quorums));
                    });
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(Vec2::new(180.0, 70.0));
                        ui.label("Tasks Offloaded:");
                        ui.heading(format!("{}", state.swarm_offload_count));
                    });
                });
            }

            _ => {}
        }
    }
}
