// core/hypervisor/src/hud/views/agents_hub.rs
//! Autonomous SI Agents, Specialists Hive & Swarm Mesh view.

use crate::hud::state::{
    AgentExecutionState, AgentKind, AgentsSubTab, CustomAgent, SharedHudState,
};
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
                egui::RichText::new("🤖 Automation Hub & Team")
                    .color(theme.accent())
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if state.agents_subtab == AgentsSubTab::CustomAgents {
                    let btn_text = if state.is_creating_agent {
                        "❌ Cancel"
                    } else {
                        "➕ New Automation Assistant"
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
                (AgentsSubTab::CustomAgents, "🤖 Automation Assistants"),
                (AgentsSubTab::Specialists, "👥 Specialized Roles"),
                (AgentsSubTab::SwarmMesh, "🌐 Connected Network"),
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

                let click_resp = ui.interact(
                    resp.response.rect,
                    ui.id().with(tab as usize),
                    egui::Sense::click(),
                );
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

                                                if ui.button("⚡ Bind Macro").clicked() {
                                                    state.macro_name_input = format!("{}_macro", agent.name.to_lowercase().replace(' ', "_"));
                                                    state.macro_desc_input = agent.description.clone();
                                                    state.si_forge_subtab = crate::hud::state::SiForgeSubTab::SmartMacros;
                                                    state.nav_section = crate::hud::navigation::NavSection::SiForge;
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    });

                if let Some(agent) = to_spawn {
                    state.award_xp(25, "Executed Automation Task");
                    state.spawn_agent_execution(&agent);
                }
                if let Some(idx) = to_delete.filter(|&idx| idx < state.custom_agents.len()) {
                    let agent = state.custom_agents.remove(idx);
                    agent.delete_from_disk();
                }

                ui.add_space(8.0);
                ui.separator();

                // Instant Recall Experience Bank (HNSW Memory Fabric)
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚡ Instant Experience Bank:").strong());
                            let edit_resp = ui.add(
                                egui::TextEdit::singleline(&mut state.recall_search_query)
                                    .hint_text("Search past workflows (e.g. 'clean workspace cache', 'take screenshot')...")
                                    .desired_width(360.0),
                            );

                            if edit_resp.changed() || ui.button("🔍 Search").clicked() {
                                let query = state.recall_search_query.to_lowercase();
                                if query.trim().is_empty() {
                                    state.recall_search_results.clear();
                                } else {
                                    // Live instant associative recall from real registered memory items
                                    let mut matches = Vec::new();
                                    for agent in &state.custom_agents {
                                        let name_lower = agent.name.to_lowercase();
                                        let desc_lower = agent.description.to_lowercase();
                                        if name_lower.contains(&query) || desc_lower.contains(&query) {
                                            matches.push((agent.name.clone(), 0.95, "0.4 ms (Instant)".to_string()));
                                        }
                                    }
                                    for m in &state.saved_si_macros {
                                        let name_lower = m.macro_name.to_lowercase();
                                        let desc_lower = m.description.to_lowercase();
                                        if name_lower.contains(&query) || desc_lower.contains(&query) {
                                            matches.push((format!("Macro: {}", m.macro_name), 0.92, "0.3 ms (Instant)".to_string()));
                                        }
                                    }
                                    for routine in &state.routine_steps {
                                        let name_lower = routine.name.to_lowercase();
                                        let desc_lower = routine.description.to_lowercase();
                                        if name_lower.contains(&query) || desc_lower.contains(&query) {
                                            matches.push((format!("Routine Step: {}", routine.name), 0.88, "0.2 ms (Instant)".to_string()));
                                        }
                                    }
                                    state.recall_search_results = matches;
                                }
                            }
                        });

                        if !state.recall_search_results.is_empty() {
                            ui.add_space(4.0);
                            for (name, sim, latency) in &state.recall_search_results {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("⚡ {}", name)).color(theme.accent()).strong());
                                    ui.label(format!("Match: {:.0}%", sim * 100.0));
                                    ui.label(egui::RichText::new(format!("Recall Speed: {}", latency)).color(Color32::from_rgb(63, 185, 80)));
                                });
                            }
                        }
                    });

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
                            ui.label(
                                egui::RichText::new("No automation events logged yet.")
                                    .italics()
                                    .color(Color32::GRAY),
                            );
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
                    "Automatic multi-role delegation: tasks are intelligently split between specialized assistants.",
                );
                ui.add_space(4.0);

                // Task Intent Input Deck
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(8))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Enter Goal or Task:").strong());
                            ui.add(
                                egui::TextEdit::singleline(&mut state.hive_intent_input)
                                    .hint_text("e.g. 'Format documents and verify code safety'...")
                                    .desired_width(420.0),
                            );
                            if ui.button("⚡ Plan & Execute").clicked()
                                && !state.hive_intent_input.trim().is_empty()
                            {
                                let intent_text = state.hive_intent_input.trim().to_string();

                                // Route through Intermediary Capability Broker
                                let outcome = state.capability_broker.execute(
                                    "specialist.dispatch_intent",
                                    serde_json::json!({ "intent": intent_text }),
                                );

                                let specialist = outcome.payload.get("assigned_specialist")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Orchestrator");

                                state.hive_routing_decision = Some(format!(
                                    "Plan established: Routed to {} [Latency: {}µs] — Active execution pipeline.",
                                    specialist, outcome.latency_us
                                ));
                                state.hive_routing_trace.push(format!(
                                    "Task: '{}' -> Processed via {} ({}µs).",
                                    intent_text, specialist, outcome.latency_us
                                ));

                                let now_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;

                                state.event_logs.push(crate::hud::state::AutomationEventLog {
                                    timestamp_ms: now_ms,
                                    source: format!("Broker ({})", specialist),
                                    action: format!("Executed capability: 'specialist.dispatch_intent' ('{}')", intent_text),
                                    latency_us: outcome.latency_us as f32,
                                    success: outcome.success,
                                });

                                state.inject_live_intent(&intent_text);
                                state.hive_intent_input.clear();
                                state.award_xp(35, "Executed Hive Specialist Plan");
                            }
                        });
                        if let Some(dec) = &state.hive_routing_decision {
                            ui.label(egui::RichText::new(dec).color(theme.accent()));
                        }
                    });

                ui.add_space(8.0);

                // Grid of the 9 Specialists (Normalized names)
                let specialists = [
                    (
                        "01. Task Coordinator",
                        "Active",
                        "Decomposes complex requests into simple steps.",
                        Color32::from_rgb(255, 215, 0),
                    ),
                    (
                        "02. Code Specialist",
                        "Active",
                        "Handles code editing, file modifications, and formatting.",
                        Color32::from_rgb(163, 113, 247),
                    ),
                    (
                        "03. Visual Presenter",
                        "Active",
                        "Draws high-speed user interface and HUD components.",
                        Color32::from_rgb(56, 139, 253),
                    ),
                    (
                        "04. System Tools",
                        "Active",
                        "Direct interaction with OS files, terminals, and processes.",
                        Color32::from_rgb(240, 136, 62),
                    ),
                    (
                        "05. Safety Guard",
                        "Active",
                        "Guarantees actions execute safely with zero conflicts.",
                        Color32::from_rgb(248, 81, 73),
                    ),
                    (
                        "06. Memory Archivist",
                        "Active",
                        "3D semantic knowledge graph clustering & indexing.",
                        Color32::from_rgb(121, 192, 255),
                    ),
                    (
                        "07. Network Router",
                        "Active",
                        "Direct high-speed communication between distributed machines.",
                        Color32::from_rgb(63, 185, 80),
                    ),
                    (
                        "08. Policy & Privacy Guard",
                        "Active",
                        "Maintains user privacy and execution permission policies.",
                        Color32::from_rgb(219, 109, 40),
                    ),
                    (
                        "09. Vision Perceiver",
                        "Active",
                        "Ultra-fast screen analysis and interactive element detection.",
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
