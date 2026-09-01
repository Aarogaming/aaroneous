// core/hypervisor/src/hud/views/settings.rs
//! Preferences, Local GGUF Model Auto-Discovery Hub, safety toggles, and developer mode.

use crate::hud::state::SharedHudState;
use crate::hud::theme::HudTheme;
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Stroke};

#[derive(Default)]
pub struct SettingsView;

impl HudView for SettingsView {
    fn id(&self) -> &'static str {
        "settings"
    }

    fn title(&self) -> &'static str {
        "⚙️ Settings"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        ui.heading(
            egui::RichText::new("⚙️ Preferences, Local Models & Developer Mode")
                .color(theme.accent())
                .strong(),
        );
        ui.separator();

        egui::ScrollArea::vertical().max_height(600.0).show(ui, |ui| {
            // ── Section 1: Themes & Styling ────────────────────────────────────────
            ui.label(egui::RichText::new("🎨 HIGH-CONTRAST THEMES").strong());
            ui.horizontal(|ui| {
                if ui.selectable_value(&mut state.settings.theme, HudTheme::CobaltDark, HudTheme::CobaltDark.name()).clicked() {
                    state.settings.save_to_disk();
                }
                if ui.selectable_value(&mut state.settings.theme, HudTheme::ObsidianSlate, HudTheme::ObsidianSlate.name()).clicked() {
                    state.settings.save_to_disk();
                }
                if ui.selectable_value(&mut state.settings.theme, HudTheme::EmeraldMatrix, HudTheme::EmeraldMatrix.name()).clicked() {
                    state.settings.save_to_disk();
                }
                if ui.selectable_value(&mut state.settings.theme, HudTheme::AmberSovereign, HudTheme::AmberSovereign.name()).clicked() {
                    state.settings.save_to_disk();
                }
            });

            ui.add_space(16.0);

            // ── Section 2: Local GGUF Model Hub Auto-Discovery ──────────────────────
            egui::Frame::group(ui.style())
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.accent()))
                .corner_radius(CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("🧠 Local LLMs & GGUF Model Auto-Discovery").color(theme.accent()).size(16.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🔄 Rescan All Hubs").clicked() {
                                    state.rescan_local_models();
                                }
                                if ui.button("📁 Browse Custom Models Folder...").clicked()
                                    && let Some(folder) = rfd::FileDialog::new().set_title("Select Custom GGUF Models Folder").pick_folder()
                                {
                                    state.settings.custom_models_dir = Some(folder.clone());
                                    state.settings.save_to_disk();
                                    state.rescan_local_models();
                                }
                            });
                        });

                        ui.label("Automatically discovers all downloaded GGUF models across LM Studio, Ollama, HuggingFace, and custom folders.");
                        ui.add_space(6.0);

                        // Hub Badges
                        ui.horizontal_wrapped(|ui| {
                            for hub in &state.model_hubs {
                                let (badge_color, badge_text) = if hub.exists {
                                    (Color32::from_rgb(63, 185, 80), format!("🟢 {}", hub.name))
                                } else {
                                    (Color32::GRAY, format!("⚪ {}", hub.name))
                                };

                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgba_unmultiplied(20, 26, 36, 200))
                                    .stroke(Stroke::new(1.0, theme.border_color()))
                                    .corner_radius(CornerRadius::same(4))
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new(badge_text).color(badge_color).size(11.0).strong());
                                    });
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();

                        let custom_dir_str = state.settings.custom_models_dir.as_ref().map(|p| p.display().to_string());
                        let mut clear_custom_dir = false;
                        let mut selected_model_to_bind = None;

                        if let Some(custom_str) = custom_dir_str {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Custom Folder: {}", custom_str)).size(11.0).color(theme.accent()));
                                if ui.button("❌ Clear").clicked() {
                                    clear_custom_dir = true;
                                }
                            });
                        }

                        ui.add_space(4.0);

                        // Discovered Models List
                        if state.discovered_gguf_models.is_empty() {
                            ui.label(egui::RichText::new("No .gguf models detected in default hubs. Download models via LM Studio or click 'Browse Custom Models Folder...'").color(Color32::from_rgb(210, 153, 34)));
                        } else {
                            ui.label(egui::RichText::new(format!("Discovered {} Models:", state.discovered_gguf_models.len())).strong());
                            let models = state.discovered_gguf_models.clone();
                            let active_model = state.settings.selected_gguf_model.clone();

                            egui::ScrollArea::vertical().id_salt("models_scroll").max_height(160.0).show(ui, |ui| {
                                for (i, m) in models.iter().enumerate() {
                                    let is_selected = active_model.as_deref() == Some(m.file_name.as_str());
                                    let border_color = if is_selected { theme.accent() } else { theme.border_color() };

                                    egui::Frame::group(ui.style())
                                        .fill(if is_selected { theme.panel_bg() } else { Color32::TRANSPARENT })
                                        .stroke(Stroke::new(if is_selected { 1.5 } else { 1.0 }, border_color))
                                        .corner_radius(CornerRadius::same(4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new(&m.file_name).strong().size(12.0));
                                                    ui.label(egui::RichText::new(format!("Hub: {}  |  Size: {}", m.source_hub, m.formatted_size)).color(Color32::GRAY).size(10.0));
                                                });

                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    let btn_text = if is_selected { "✅ Active Persona" } else { "⚡ Bind to Agent Persona" };
                                                    if ui.button(btn_text).clicked() {
                                                        selected_model_to_bind = Some((i, m.file_name.clone()));
                                                    }
                                                });
                                            });
                                        });
                                }
                            });
                        }

                        if clear_custom_dir {
                            state.settings.custom_models_dir = None;
                            state.settings.save_to_disk();
                            state.rescan_local_models();
                        }

                        if let Some((idx, model_name)) = selected_model_to_bind {
                            state.selected_model_idx = idx;
                            state.settings.selected_gguf_model = Some(model_name.clone());
                            state.settings.save_to_disk();
                        }
                    });
                });

            ui.add_space(16.0);

            // ── Section 3: Display Scalability ──────────────────────────────────────
            ui.label(egui::RichText::new("🔍 DISPLAY SCALABILITY").strong());
            ui.horizontal(|ui| {
                ui.label("Display Scale Factor:");
                if ui.add(egui::Slider::new(&mut state.settings.ui_scale, 0.75..=1.5).text("x Scale")).changed() {
                    state.settings.save_to_disk();
                }
            });

            ui.add_space(16.0);

            // ── Section 4: Developer Mode ───────────────────────────────────────────
            egui::Frame::group(ui.style())
                .fill(if state.settings.dev_mode { Color32::from_rgba_unmultiplied(40, 28, 18, 220) } else { theme.card_bg() })
                .stroke(Stroke::new(1.5, if state.settings.dev_mode { Color32::from_rgb(255, 120, 0) } else { theme.border_color() }))
                .corner_radius(CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🛠️ DEVELOPER MODE & INTERNAL DIAGNOSTICS").color(if state.settings.dev_mode { Color32::from_rgb(255, 120, 0) } else { Color32::WHITE }).strong());
                        ui.label("Enables low-level developer tools: Code & AST Forge, Compiler Diagnostics Auto-Fixer, and 64 MB Shared Memory Bus Monitors.");
                        ui.add_space(6.0);

                        if ui.checkbox(&mut state.settings.dev_mode, "Enable Developer Mode (Diagnostics & Code Forge)").changed() {
                            state.settings.save_to_disk();
                        }
                    });
                });

            ui.add_space(16.0);

            // ── Section 5: Safety & Automation ──────────────────────────────────────
            ui.label(egui::RichText::new("🛡️ SAFETY & AUTOMATION").strong());
            if ui.checkbox(&mut state.settings.allow_host_input, "Allow Live Host HID Injection (Hardware Input Safety Permit)").changed() {
                state.settings.save_to_disk();
            }
            if ui.checkbox(&mut state.settings.auto_recompile_on_save, "Auto-Recompile Code on Save").changed() {
                state.settings.save_to_disk();
            }

            ui.add_space(20.0);
            if ui.button("💾 Save Settings to Disk").clicked() {
                state.settings.save_to_disk();
            }
        });
    }
}
