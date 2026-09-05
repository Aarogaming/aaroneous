// core/hypervisor/src/hud/views/agents_hub.rs
//! Autonomous SI Agents, Specialists Hive & Swarm Mesh view.

use crate::hud::state::{AgentExecutionState, AgentKind, AgentsSubTab, CustomAgent, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

#[derive(Default)]
pub struct AgentsHubView;

impl HudView for AgentsHubView {
    fn id(&self) -> &'static str {
        "agents_hub"
    }

    fn title(&self) -> &'static str {
        "🤖 Agents Hub"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        // ── Top Navigation / Tab Selector ───────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🤖 Agents & Autonomous Swarm Mesh")
                    .color(theme.accent())
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.agents_subtab == AgentsSubTab::CustomAgents {
                    let btn_text = if state.is_creating_agent {
                        "❌ Cancel New Agent"
                    } else {
                        "➕ Create New SI Agent"
                    };
                    if ui.button(btn_text).clicked() {
                        state.is_creating_agent = !state.is_creating_agent;
                    }
                }
            });
        });

        ui.add_space(4.0);

        // Sub-Tab Switcher
        ui.horizontal(|ui| {
            let tabs = [
                (AgentsSubTab::CustomAgents, "🤖 Custom Bots"),
                (AgentsSubTab::Specialists, "👥 9 Specialists & Hive"),
                (AgentsSubTab::SwarmMesh, "🌐 Swarm Mesh"),
            ];

            for (tab, label) in tabs {
                let is_selected = state.agents_subtab == tab;
                let bg_color = if is_selected {
                    theme.card_bg()
                } else {
                    Color32::TRANSPARENT
                };
                let stroke = if is_selected {
                    Stroke::new(1.0, theme.accent())
                } else {
                    Stroke::NONE
                };

                let resp = egui::Frame::NONE
                    .fill(bg_color)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(5))
                    .inner_margin(egui::Margin::symmetric(10, 5))
                    .show(ui, |ui| {
                        let text_color = if is_selected {
                            theme.accent()
                        } else {
                            Color32::from_rgb(200, 205, 215)
                        };
                        ui.label(egui::RichText::new(label).strong().color(text_color));
                    });

                let click_resp = ui.interact(resp.response.rect, ui.id().with(tab as usize), egui::Sense::click());
                if click_resp.clicked() {
                    state.agents_subtab = tab;
                }
            }
        });

        ui.separator();

        // ── Render Selected Sub-Tab ─────────────────────────────────────────────
        match state.agents_subtab {
            AgentsSubTab::CustomAgents => {
                ui.label("Self-contained synthetic intelligence automation routines persistent to `{data}/agents/*.json`.");
                ui.add_space(4.0);

                // Agent Creation Form
                if state.is_creating_agent {
                    egui::Frame::group(ui.style())
                        .fill(theme.card_bg())
                        .stroke(Stroke::new(1.5, theme.accent()))
                        .corner_radius(CornerRadius::same(8))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.heading(
                                egui::RichText::new("✨ Create New SI Automation Agent")
                                    .strong()
                                    .size(15.0),
                            );
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                ui.label("Agent Name:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.new_agent_name)
                                        .hint_text("e.g. Code Review Bot"),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Description:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.new_agent_desc)
                                        .hint_text("What this agent accomplishes"),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Agent Kind:");
                                ui.selectable_value(
                                    &mut state.new_agent_kind,
                                    AgentKind::SingleUseTask,
                                    AgentKind::SingleUseTask.name(),
                                );
                                ui.selectable_value(
                                    &mut state.new_agent_kind,
                                    AgentKind::SmartMacroLoop,
                                    AgentKind::SmartMacroLoop.name(),
                                );
                                ui.selectable_value(
                                    &mut state.new_agent_kind,
                                    AgentKind::Assistant,
                                    AgentKind::Assistant.name(),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Target Application / Path:");
                                ui.text_edit_singleline(&mut state.new_agent_target_app);
                            });

                            ui.horizontal(|ui| {
                                ui.label("System Instructions:");
                                ui.add(
                                    egui::TextEdit::multiline(&mut state.new_agent_instructions)
                                        .desired_rows(3),
                                );
                            });

                            ui.add_space(4.0);
                            if ui
                                .button(
                                    egui::RichText::new("💾 Save & Spawn SI Agent")
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .clicked()
                                && !state.new_agent_name.trim().is_empty()
                            {
                                let new_agent = CustomAgent {
                                    id: format!("agent_{}", uuid::Uuid::new_v4().simple()),
                                    name: state.new_agent_name.clone(),
                                    description: state.new_agent_desc.clone(),
                                    kind: state.new_agent_kind,
                                    instructions: state.new_agent_instructions.clone(),
                                    target_app: state.new_agent_target_app.clone(),
                                    tasks_completed: 0,
                                    state: AgentExecutionState::Idle,
                                    color: [56, 139, 253],
                                    soul_model: None,
                                };
                                new_agent.save_to_disk();
                                state.custom_agents.push(new_agent);

                                state.new_agent_name.clear();
                                state.new_agent_desc.clear();
                                state.new_agent_instructions.clear();
                                state.is_creating_agent = false;
                            }
                        });
                    ui.add_space(6.0);
                }

                // Active Agents Grid
                let mut to_spawn = None;
                let mut to_delete = None;

                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if state.custom_agents.is_empty() {
                            ui.label(egui::RichText::new("No agents created yet. Click 'Create New SI Agent' to build one.").italics());
                        }

                        for (idx, agent) in state.custom_agents.iter_mut().enumerate() {
                            egui::Frame::group(ui.style())
                                .fill(theme.card_bg())
                                .stroke(Stroke::new(1.0, theme.border_color()))
                                .corner_radius(CornerRadius::same(6))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let [r, g, b] = agent.color;
                                        ui.label(
                                            egui::RichText::new("🤖")
                                                .color(Color32::from_rgb(r, g, b))
                                                .size(18.0),
                                        );
                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&agent.name).strong().size(13.0),
                                            );
                                            ui.label(
                                                egui::RichText::new(&agent.description)
                                                    .size(11.0)
                                                    .color(Color32::GRAY),
                                            );
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("🗑️").clicked() {
                                                    to_delete = Some(idx);
                                                }

                                                match agent.state {
                                                    AgentExecutionState::Idle
                                                    | AgentExecutionState::Paused
                                                    | AgentExecutionState::Completed => {
                                                        if ui.button("▶️ Run Task").clicked() {
                                                            to_spawn = Some(agent.clone());
                                                        }
                                                    }
                                                    AgentExecutionState::Running => {
                                                        ui.label(
                                                            egui::RichText::new("⚡ ACTIVE")
                                                                .color(Color32::from_rgb(63, 185, 80))
                                                                .strong(),
                                                        );
                                                    }
                                                }

                                                ui.label(format!(
                                                    "Tasks Done: {}",
                                                    agent.tasks_completed
                                                ));
                                            },
                                        );
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    });

                if let Some(agent) = to_spawn {
                    state.spawn_agent_execution(&agent);
                }
                if let Some(idx) = to_delete.filter(|&idx| idx < state.custom_agents.len()) {
                    let agent = state.custom_agents.remove(idx);
                    agent.delete_from_disk();
                }

                ui.add_space(8.0);
                ui.separator();

                // Live Automation Event Stream
                ui.label(
                    egui::RichText::new("📊 Live Automation Event Stream")
                        .strong()
                        .color(theme.accent()),
                );
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        if state.event_logs.is_empty() {
                            ui.label(egui::RichText::new("No automation events logged yet.").italics().color(Color32::GRAY));
                        } else {
                            for log in state.event_logs.iter().rev() {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("[{}ms]", log.timestamp_ms))
                                            .monospace()
                                            .color(Color32::GRAY),
                                    );
                                    ui.label(egui::RichText::new(&log.source).strong());
                                    ui.label(&log.action);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}µs",
                                                    log.latency_us
                                                ))
                                                .color(Color32::from_rgb(63, 185, 80)),
                                            );
                                        },
                                    );
                                });
                            }
                        }
                    });
            }

            AgentsSubTab::Specialists => {
                ui.label(
                    "Deterministic domain specialist delegation with non-linguistic continuous state coordination.",
                );
                ui.add_space(4.0);

                // Hive Intent Input Deck
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(8))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Submit Hive Task Intent:").strong());
                            ui.add(
                                egui::TextEdit::singleline(&mut state.hive_intent_input)
                                    .desired_width(420.0),
                            );
                            if ui.button("⚡ Dispatch Intent").clicked()
                                && !state.hive_intent_input.trim().is_empty()
                            {
                                state.hive_routing_decision = Some(
                                    "Routed intent to Synthesizer (0x0200) & DevTools (0x0400)"
                                        .to_string(),
                                );
                                state.hive_routing_trace.push(format!(
                                    "Task: '{}' -> Completed.",
                                    state.hive_intent_input
                                ));
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
                    (
                        "01. Orchestrator",
                        "0x0100",
                        "Central task decomposition & dynamic DAG execution.",
                        Color32::from_rgb(255, 215, 0),
                    ),
                    (
                        "02. Synthesizer",
                        "0x0200",
                        "Polyglot code synthesis, AST rewrite & compilation.",
                        Color32::from_rgb(163, 113, 247),
                    ),
                    (
                        "03. Presenter",
                        "0x0300",
                        "DirectX 12/Vulkan frame composition & interactive HUD.",
                        Color32::from_rgb(56, 139, 253),
                    ),
                    (
                        "04. DevTools",
                        "0x0400",
                        "Automated FFI wrapper synthesis & live memory repair.",
                        Color32::from_rgb(240, 136, 62),
                    ),
                    (
                        "05. Sentinel",
                        "0x0500",
                        "SVDD latent security manifold & containment checks.",
                        Color32::from_rgb(248, 81, 73),
                    ),
                    (
                        "06. Archivist",
                        "0x0600",
                        "3D semantic knowledge graph clustering & indexing.",
                        Color32::from_rgb(121, 192, 255),
                    ),
                    (
                        "07. Router",
                        "0x0700",
                        "P2P streaming TCP mesh multiplexer & gossip consensus.",
                        Color32::from_rgb(63, 185, 80),
                    ),
                    (
                        "08. Aligner",
                        "0x0800",
                        "Federation policy alignment & safety arbitration.",
                        Color32::from_rgb(219, 109, 40),
                    ),
                    (
                        "09. Perceiver",
                        "0x0900",
                        "DXGI screen capture & low-latency perceptual gating.",
                        Color32::from_rgb(88, 166, 255),
                    ),
                ];

                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
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
                                        ui.label(
                                            egui::RichText::new(format!("({})", opcode))
                                                .color(Color32::GRAY)
                                                .monospace(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new("ONLINE")
                                                        .color(Color32::from_rgb(63, 185, 80))
                                                        .strong()
                                                        .size(10.0),
                                                );
                                            },
                                        );
                                    });
                                    ui.label(
                                        egui::RichText::new(*desc)
                                            .size(11.0)
                                            .color(Color32::from_rgb(200, 210, 225)),
                                    );
                                });
                            ui.add_space(4.0);
                        }
                    });
            }

            AgentsSubTab::SwarmMesh => {
                ui.label("Distributed work stealing, Byzantine fault tolerance & P2P .si cartridge sync.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    egui::Frame::group(ui.style())
                        .fill(theme.card_bg())
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, theme.border_color()))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(180.0, 70.0));
                            ui.label("Active Quorums:");
                            ui.heading(format!("{}", state.swarm_live_quorums));
                        });
                    egui::Frame::group(ui.style())
                        .fill(theme.card_bg())
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, theme.border_color()))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(180.0, 70.0));
                            ui.label("Tasks Offloaded:");
                            ui.heading(format!("{}", state.swarm_offload_count));
                        });
                });
            }
        }
    }
}
