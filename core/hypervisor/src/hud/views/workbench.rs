// core/hypervisor/src/hud/views/workbench.rs
//! Developer workbench, source tree explorer, code editor, and compiler diagnostics diff view.

use crate::hud::state::{DevStudioTab, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Vec2};

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
            ui.selectable_value(
                &mut state.dev_tab,
                DevStudioTab::SiDistillation,
                "💎 SI Distillation",
            );
            ui.selectable_value(
                &mut state.dev_tab,
                DevStudioTab::SiMacroHub,
                "🔄 Smart Macros",
            );
            ui.selectable_value(
                &mut state.dev_tab,
                DevStudioTab::OtEdgeGateway,
                "⚡ Industrial / OT Gateway",
            );
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
                                    state.workbench_status_msg =
                                        format!("Found {} diagnostic items.", diags.len());
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
                        ui.label(
                            egui::RichText::new("📁 Workspace Files")
                                .strong()
                                .color(theme.accent()),
                        );
                        ui.separator();
                        let max_h = (ui.available_height() - 20.0).max(300.0);
                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .max_height(max_h)
                            .show(ui, |ui| {
                                for (i, item) in state.workspace_tree_items.iter().enumerate() {
                                    let icon = if item.is_dir { "📁" } else { "📄" };
                                    let text = format!(
                                        "{} {} ({} lines)",
                                        icon, item.relative_path, item.line_count
                                    );
                                    let is_selected = state.selected_tree_idx == i;
                                    let row_w = ui.available_width();
                                    let (row_rect, resp) = ui.allocate_exact_size(Vec2::new(row_w, 22.0), egui::Sense::click());
                                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                                    if resp.clicked() {
                                        state.selected_tree_idx = i;
                                        if !item.is_dir {
                                            state.workbench_active_file =
                                                item.relative_path.clone();
                                            if let Ok(content) = std::fs::read_to_string(&item.path)
                                            {
                                                state.workbench_file_content = content;
                                            }
                                        }
                                    }

                                    let bg = if is_selected {
                                        Color32::from_rgba_unmultiplied(56, 139, 253, 40)
                                    } else if resp.hovered() {
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                                    } else {
                                        Color32::TRANSPARENT
                                    };
                                    ui.painter().rect_filled(row_rect, CornerRadius::same(4), bg);

                                    let text_color = if is_selected {
                                        theme.accent()
                                    } else if resp.hovered() {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(205, 215, 225)
                                    };
                                    ui.painter().text(
                                        Pos2::new(row_rect.min.x + 6.0, row_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        &text,
                                        egui::FontId::proportional(11.5),
                                        text_color,
                                    );
                                }
                            });
                    });

                    // Col 1: Editor
                    cols[1].vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "📄 Code: {}",
                                    state.workbench_active_file
                                ))
                                .strong()
                                .color(theme.accent()),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new("💾 Save").strong().color(Color32::WHITE)).clicked() {
                                    if let Some(item) = state.workspace_tree_items.get(state.selected_tree_idx) {
                                        if !item.is_dir {
                                            match std::fs::write(&item.path, &state.workbench_file_content) {
                                                Ok(_) => {
                                                    state.workbench_status_msg = format!("Saved {} successfully.", state.workbench_active_file);
                                                    state.award_xp(15, "Saved Source Code File");
                                                }
                                                Err(e) => {
                                                    state.workbench_status_msg = format!("Failed to save file: {e}");
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        });
                        ui.separator();
                        let editor_h = (ui.available_height() - 20.0).max(300.0);
                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .max_height(editor_h)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut state.workbench_file_content)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY),
                                );
                            });
                    });

                    // Col 2: Diagnostics & Diffs
                    cols[2].vertical(|ui| {
                        ui.label(
                            egui::RichText::new("🔍 Diagnostics & Unified Diff")
                                .strong()
                                .color(theme.accent()),
                        );
                        ui.separator();

                        if !state.workbench_diagnostics.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Found {} Diagnostics",
                                    state.workbench_diagnostics.len()
                                ))
                                .color(Color32::from_rgb(255, 120, 0)),
                            );
                            egui::ScrollArea::both()
                                .auto_shrink([false, false])
                                .max_height(160.0)
                                .show(ui, |ui| {
                                    for diag in &state.workbench_diagnostics {
                                        ui.label(format!(
                                            "[{}] {}: {}",
                                            diag.level,
                                            diag.code.as_deref().unwrap_or(""),
                                            diag.message
                                        ));
                                    }
                                });
                            ui.separator();
                        }

                        ui.label(egui::RichText::new("Myers Unified Diff Preview").italics());
                        ui.add(
                            egui::TextEdit::multiline(&mut state.workbench_diff_preview)
                                .desired_rows(10)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                        ui.label(
                            egui::RichText::new(&state.workbench_status_msg).color(theme.accent()),
                        );
                    });
                });
            }

            DevStudioTab::SiDistillation => {
                ui.label(
                    egui::RichText::new("💎 SI Distillation Corpus Miner")
                        .strong()
                        .color(theme.accent()),
                );
                ui.label(format!(
                    "Corpus Samples: {} | Bytes: {} | Energy: {:.4}",
                    state.si_corpus_count, state.si_corpus_bytes, state.si_corpus_avg_energy
                ));
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
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("🔄 Machine-Native Smart Macros")
                            .strong()
                            .color(theme.accent()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚡ Create New Macro in Forge").clicked() {
                            state.si_forge_subtab = crate::hud::state::SiForgeSubTab::SmartMacros;
                            state.nav_section = crate::hud::navigation::NavSection::SiForge;
                        }
                    });
                });
                ui.separator();
                ui.add_space(4.0);

                let mut to_run = None;
                if state.saved_si_macros.is_empty() {
                    ui.label(egui::RichText::new("No registered macros found.").italics().color(Color32::GRAY));
                } else {
                    for m in &state.saved_si_macros {
                        egui::Frame::group(ui.style())
                            .fill(theme.card_bg())
                            .stroke(eframe::egui::Stroke::new(1.0, theme.border_color()))
                            .corner_radius(eframe::egui::CornerRadius::same(6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&m.macro_name).strong());
                                    ui.label(format!(
                                        "(Hotkey: {})",
                                        m.hotkey.as_deref().unwrap_or("None")
                                    ));
                                    ui.label(
                                        egui::RichText::new(format!("{}µs", m.latency_us))
                                            .color(Color32::from_rgb(63, 185, 80))
                                            .size(11.0),
                                    );

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("▶ Execute").clicked() {
                                            to_run = Some(m.file_path.clone());
                                        }
                                        ui.label(
                                            egui::RichText::new(&m.description)
                                                .size(11.0)
                                                .color(Color32::from_rgb(180, 190, 205)),
                                        );
                                    });
                                });
                            });
                        ui.add_space(2.0);
                    }
                }

                if let Some(path) = to_run {
                    let _ = state.si_macro_engine.execute_macro_mmap(&path);
                    state.award_xp(10, "Executed Smart Macro");
                }
            }

            DevStudioTab::OtEdgeGateway => {
                ui.label(
                    egui::RichText::new("⚡ Industrial OT Edge Gateway & Physical Register Bank")
                        .strong()
                        .color(theme.accent()),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Selected Serial Port:");
                    egui::ComboBox::from_id_salt("ot_port_select")
                        .selected_text(&state.ot_selected_port)
                        .show_ui(ui, |ui| {
                            for port in &state.ot_available_ports {
                                ui.selectable_value(
                                    &mut state.ot_selected_port,
                                    port.clone(),
                                    port,
                                );
                            }
                        });

                    if ui.button("🔄 Rescan Ports").clicked() {
                        state.ot_available_ports = tokio_serial::available_ports()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|p| p.port_name)
                            .collect();
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.columns(2, |cols| {
                    // Col 0: Modbus Holding Registers
                    cols[0].vertical(|ui| {
                        ui.label(
                            egui::RichText::new("📊 Modbus Holding Registers (40001..40016)")
                                .strong()
                                .color(theme.accent()),
                        );
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                egui::Grid::new("holding_regs_grid")
                                    .striped(true)
                                    .spacing([20.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Address").strong());
                                        ui.label(egui::RichText::new("Value (u16)").strong());
                                        ui.end_row();

                                        for (i, &val) in state
                                            .industrial_registers
                                            .holding_registers
                                            .iter()
                                            .take(16)
                                            .enumerate()
                                        {
                                            ui.label(format!("40{:03}", i + 1));
                                            ui.label(
                                                egui::RichText::new(format!("{val}")).monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    });

                    // Col 1: Discrete I/O Coils & Safety Interlocks
                    cols[1].vertical(|ui| {
                        ui.label(
                            egui::RichText::new("🔌 Discrete I/O States & Physical Coils")
                                .strong()
                                .color(theme.accent()),
                        );
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                egui::Grid::new("discrete_io_grid")
                                    .striped(true)
                                    .spacing([20.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Channel").strong());
                                        ui.label(egui::RichText::new("State").strong());
                                        ui.end_row();

                                        for (i, &state) in state
                                            .industrial_registers
                                            .discrete_inputs
                                            .iter()
                                            .take(16)
                                            .enumerate()
                                        {
                                            ui.label(format!("DI / Coil #{:02}", i));
                                            let status_text = if state {
                                                egui::RichText::new("HIGH [1]")
                                                    .color(Color32::from_rgb(46, 160, 67))
                                                    .strong()
                                            } else {
                                                egui::RichText::new("LOW  [0]")
                                                    .color(Color32::from_rgb(139, 148, 158))
                                            };
                                            ui.label(status_text);
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                });
            }

            _ => {}
        }
    }
}
