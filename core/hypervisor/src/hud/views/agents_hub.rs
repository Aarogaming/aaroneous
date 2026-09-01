// core/hypervisor/src/hud/views/agents_hub.rs
//! Autonomous SI Agents & Workflows, pipeline builder, and disk persistence view.

use crate::hud::state::{AgentExecutionState, AgentKind, CustomAgent, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Stroke};
use uuid::Uuid;

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

        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🤖 Autonomous SI Agents & Smart Workflows")
                    .color(theme.accent())
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn_text = if state.is_creating_agent { "❌ Cancel New Agent" } else { "➕ Create New SI Agent" };
                if ui.button(btn_text).clicked() {
                    state.is_creating_agent = !state.is_creating_agent;
                }
            });
        });

        ui.label("Self-contained synthetic intelligence automation routines persistent to `{data}/agents/*.json`.");
        ui.separator();

        // ── Agent Creation Form ─────────────────────────────────────────────────
        if state.is_creating_agent {
            egui::Frame::group(ui.style())
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.5, theme.accent()))
                .corner_radius(CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.heading(egui::RichText::new("✨ Create New SI Automation Agent").strong().size(15.0));
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Agent Name:");
                        ui.add(egui::TextEdit::singleline(&mut state.new_agent_name).hint_text("e.g. Code Review Bot"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.add(egui::TextEdit::singleline(&mut state.new_agent_desc).hint_text("What this agent accomplishes"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Agent Kind:");
                        ui.selectable_value(&mut state.new_agent_kind, AgentKind::SingleUseTask, AgentKind::SingleUseTask.name());
                        ui.selectable_value(&mut state.new_agent_kind, AgentKind::SmartMacroLoop, AgentKind::SmartMacroLoop.name());
                        ui.selectable_value(&mut state.new_agent_kind, AgentKind::Assistant, AgentKind::Assistant.name());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Target Application / Path:");
                        ui.text_edit_singleline(&mut state.new_agent_target_app);
                    });

                    ui.label("Instructions / Goal Prompt:");
                    ui.add(egui::TextEdit::multiline(&mut state.new_agent_instructions).desired_rows(3));

                    ui.add_space(6.0);
                    if ui.button("💾 Save & Spawn Agent to Disk").clicked() && !state.new_agent_name.trim().is_empty() {
                        let new_agent = CustomAgent {
                            id: format!("agent_{}", Uuid::new_v4().simple()),
                            name: state.new_agent_name.clone(),
                            description: state.new_agent_desc.clone(),
                            kind: state.new_agent_kind,
                            instructions: state.new_agent_instructions.clone(),
                            target_app: state.new_agent_target_app.clone(),
                            tasks_completed: 0,
                            state: AgentExecutionState::Idle,
                            color: [56, 139, 253],
                            soul_model: Some("⚡ Solid-State Native Engine".to_string()),
                        };
                        new_agent.save_to_disk();
                        state.custom_agents.push(new_agent);
                        state.is_creating_agent = false;
                        state.new_agent_name.clear();
                        state.new_agent_desc.clear();
                        state.new_agent_instructions.clear();
                    }
                });
            ui.add_space(8.0);
        }

        // ── Active Agents Grid ──────────────────────────────────────────────────
        let mut to_spawn = None;
        let mut to_delete = None;

        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            for (idx, agent) in state.custom_agents.iter_mut().enumerate() {
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0, theme.border_color()))
                    .corner_radius(CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let [r, g, b] = agent.color;
                            ui.label(egui::RichText::new("🤖").color(Color32::from_rgb(r, g, b)).size(18.0));
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&agent.name).strong().size(13.0));
                                ui.label(egui::RichText::new(&agent.description).size(11.0).color(Color32::GRAY));
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗑️").clicked() {
                                    to_delete = Some(idx);
                                }

                                match agent.state {
                                    AgentExecutionState::Idle | AgentExecutionState::Paused | AgentExecutionState::Completed => {
                                        if ui.button("▶️ Run Task").clicked() {
                                            to_spawn = Some(agent.clone());
                                        }
                                    }
                                    AgentExecutionState::Running => {
                                        ui.label(egui::RichText::new("⚡ ACTIVE").color(Color32::from_rgb(63, 185, 80)).strong());
                                    }
                                }

                                ui.label(format!("Tasks Done: {}", agent.tasks_completed));
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });

        if let Some(agent) = to_spawn {
            state.spawn_agent_execution(&agent);
        }
        if let Some(idx) = to_delete {
            if idx < state.custom_agents.len() {
                let agent = state.custom_agents.remove(idx);
                agent.delete_from_disk();
            }
        }

        ui.add_space(8.0);
        ui.separator();

        // ── Real-Time Event Stream Log ──────────────────────────────────────────
        ui.label(egui::RichText::new("📊 Live Automation Event Stream").strong().color(theme.accent()));
        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
            for log in state.event_logs.iter().rev() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("[{}ms]", log.timestamp_ms)).monospace().color(Color32::GRAY));
                    ui.label(egui::RichText::new(&log.source).strong());
                    ui.label(&log.action);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.0}µs", log.latency_us)).color(Color32::from_rgb(63, 185, 80)));
                    });
                });
            }
        });
    }
}
