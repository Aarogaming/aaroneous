// core/hypervisor/src/hud/views/workbench.rs
//! Developer workbench, source tree explorer, code editor, and compiler diagnostics diff view.

use crate::hud::state::{DevStudioTab, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32};

#[derive(Default)]
pub struct WorkbenchView;

impl HudView for WorkbenchView {
    fn id(&self) -> &'static str {
        "workbench"
    }

    fn title(&self) -> &'static str {
        "🛠️ Developer Workbench"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🛠️ Developer Studio & Workbench")
                    .color(theme.accent())
                    .strong(),
            );

            ui.separator();
            ui.selectable_value(&mut state.dev_tab, DevStudioTab::Workbench, "📁 Workbench");
            ui.selectable_value(&mut state.dev_tab, DevStudioTab::SiDistillation, "💎 SI Distillation");
            ui.selectable_value(&mut state.dev_tab, DevStudioTab::SiMacroHub, "🔄 Smart Macros");
        });

        ui.separator();

        match state.dev_tab {
            DevStudioTab::Workbench => {
                ui.horizontal(|ui| {
                    ui.label(format!("Active File: {}", state.workbench_active_file));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔍 Scan Diagnostics").clicked() {
                            match state.dev_tools_engine.run_cargo_diagnostic_check() {
                                Ok(diags) => {
                                    state.workbench_status_msg = format!("Found {} diagnostic items.", diags.len());
                                    state.workbench_diagnostics = diags;
                                }
                                Err(e) => {
                                    state.workbench_status_msg = format!("Diagnostic error: {e}");
                                }
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                ui.columns(3, |cols| {
                    // Col 0: File Tree
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("📁 Workspace Files").strong().color(theme.accent()));
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                            for (i, item) in state.workspace_tree_items.iter().enumerate() {
                                let icon = if item.is_dir { "📁" } else { "📄" };
                                let text = format!("{} {} ({} lines)", icon, item.relative_path, item.line_count);
                                if ui.selectable_label(state.selected_tree_idx == i, text).clicked() {
                                    state.selected_tree_idx = i;
                                    if !item.is_dir {
                                        state.workbench_active_file = item.relative_path.clone();
                                        if let Ok(content) = std::fs::read_to_string(&item.path) {
                                            state.workbench_file_content = content;
                                        }
                                    }
                                }
                            }
                        });
                    });

                    // Col 1: Editor
                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new(format!("📄 Code: {}", state.workbench_active_file)).strong().color(theme.accent()));
                        ui.separator();
                        ui.add(egui::TextEdit::multiline(&mut state.workbench_file_content).desired_rows(22).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                    });

                    // Col 2: Diagnostics & Diffs
                    cols[2].vertical(|ui| {
                        ui.label(egui::RichText::new("🔍 Diagnostics & Unified Diff").strong().color(theme.accent()));
                        ui.separator();

                        if !state.workbench_diagnostics.is_empty() {
                            ui.label(egui::RichText::new(format!("Found {} Diagnostics", state.workbench_diagnostics.len())).color(Color32::from_rgb(255, 120, 0)));
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                for diag in &state.workbench_diagnostics {
                                    ui.label(format!("[{}] {}: {}", diag.level, diag.code.as_deref().unwrap_or(""), diag.message));
                                }
                            });
                            ui.separator();
                        }

                        ui.label(egui::RichText::new("Myers Unified Diff Preview").italics());
                        ui.add(egui::TextEdit::multiline(&mut state.workbench_diff_preview).desired_rows(10).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                        ui.label(egui::RichText::new(&state.workbench_status_msg).color(theme.accent()));
                    });
                });
            }

            DevStudioTab::SiDistillation => {
                ui.label(egui::RichText::new("💎 SI Distillation Corpus Miner").strong().color(theme.accent()));
                ui.label(format!("Corpus Samples: {} | Bytes: {} | Energy: {:.4}", state.si_corpus_count, state.si_corpus_bytes, state.si_corpus_avg_energy));
                if ui.button("⚡ Mine New Traces").clicked() {
                    let _ = state.si_miner.mine_starter_distillation_corpus();
                    if let Ok((c, b, e)) = state.si_miner.get_live_metrics() {
                        state.si_corpus_count = c;
                        state.si_corpus_bytes = b;
                        state.si_corpus_avg_energy = e;
                    }
                }
            }

            DevStudioTab::SiMacroHub => {
                ui.label(egui::RichText::new("🔄 Machine-Native Smart Macros").strong().color(theme.accent()));
                for m in &state.saved_si_macros {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&m.macro_name).strong());
                        ui.label(format!("(Hotkey: {})", m.hotkey.as_deref().unwrap_or("None")));
                    });
                }
            }

            _ => {}
        }
    }
}
