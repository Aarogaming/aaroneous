// core/hypervisor/src/hud/modes/full_studio.rs
//! Full Studio mode layout (top header, sidebar rail, view container, bottom status bar).

use crate::hud::navigation::NavSection;
use crate::hud::state::{AppWindowMode, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

pub fn render_full_studio(
    ui: &mut egui::Ui,
    state: &mut SharedHudState,
    views: &mut [Box<dyn HudView>],
    toggle_palette: &mut bool,
    toggle_shortcuts: &mut bool,
) {
    let theme = state.settings.theme;

    // ── Top Header ──────────────────────────────────────────────────────────
    egui::Panel::top("hud_top_header")
        .frame(
            egui::Frame::side_top_panel(ui.style())
                .fill(theme.panel_bg())
                .stroke(Stroke::new(1.0, theme.border_color())),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚡ AARONEOUS")
                        .size(16.0)
                        .strong()
                        .color(theme.accent()),
                );
                ui.label(
                    egui::RichText::new("v0.5.0")
                        .size(11.0)
                        .color(Color32::from_rgb(140, 150, 170)),
                );

                ui.separator();

                // Active Persona badge
                let persona_name = state
                    .settings
                    .selected_gguf_model
                    .as_deref()
                    .unwrap_or("⚡ Solid-State SI Native");
                ui.label(
                    egui::RichText::new(format!("🧠 {persona_name}"))
                        .size(11.0)
                        .color(theme.accent()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if state.settings.modular_canvas_mode { "🪟 Standard Mode" } else { "📐 Modular Canvas" }).clicked() {
                        state.settings.modular_canvas_mode = !state.settings.modular_canvas_mode;
                        state.settings.save_to_disk();
                    }

                    if ui.button("❓ (Ctrl+/)").clicked() {
                        *toggle_shortcuts = true;
                    }

                    if ui.button("🔍 Command Palette (Ctrl+K)").clicked() {
                        *toggle_palette = true;
                    }

                    if ui.button("🪟 Mini-HUD (F10)").clicked() {
                        state.app_window_mode = AppWindowMode::CompactRecorderOverlay;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            380.0, 180.0,
                        )));
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            egui::viewport::WindowLevel::AlwaysOnTop,
                        ));
                    }

                    if ui.button("🔴 Rec (F9)").clicked() {
                        state.toggle_recording();
                    }
                });
            });
        });

    // ── Bottom Status Bar ───────────────────────────────────────────────────
    egui::Panel::bottom("hud_bottom_status_bar")
        .frame(
            egui::Frame::side_top_panel(ui.style())
                .fill(theme.panel_bg())
                .stroke(Stroke::new(1.0, theme.border_color())),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let active_count = state
                    .custom_agents
                    .iter()
                    .filter(|a| a.state == crate::hud::state::AgentExecutionState::Running)
                    .count();
                ui.label(
                    egui::RichText::new(format!("🤖 Active Bots: {active_count}"))
                        .size(11.0)
                        .color(if active_count > 0 {
                            Color32::from_rgb(63, 185, 80)
                        } else {
                            Color32::GRAY
                        }),
                );

                ui.separator();
                ui.label(
                    egui::RichText::new("⚡ DXGI Desktop Duplication: ACTIVE")
                        .size(11.0)
                        .color(Color32::from_rgb(63, 185, 80)),
                );

                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Bus Integrity: {:.1}%", state.bus_integrity))
                        .size(11.0)
                        .color(theme.accent()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("FPS: {:.0}", state.measured_fps))
                            .size(11.0)
                            .strong()
                            .color(theme.accent()),
                    );
                });
            });
        });

    // ── Left Sidebar Navigation Rail ────────────────────────────────────────
    egui::Panel::left("hud_left_sidebar")
        .frame(
            egui::Frame::side_top_panel(ui.style())
                .fill(theme.panel_bg())
                .stroke(Stroke::new(1.0, theme.border_color())),
        )
        .resizable(false)
        .default_size(170.0)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            let sections = [
                (NavSection::Agents, "🤖 Agents & Swarm"),
                (NavSection::ScreenAutomation, "👁️ Screen & Motor"),
                (NavSection::SiForge, "⚡ SI Forge & Mind"),
                (NavSection::GalaxyMap3D, "🌌 3D Galaxy"),
                (NavSection::Settings, "⚙️ Settings"),
            ];

            let dev_sections = [
                (NavSection::DevStudio, "🛠️ Dev Studio"),
                (NavSection::InterconnectMonitor, "⚡ Bus Monitor"),
            ];

            for (sec, label) in sections {
                let is_selected = state.nav_section == sec;
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
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            if is_selected {
                                // Sleek vertical accent bar on the left
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(3.0, 14.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(1),
                                    theme.accent(),
                                );
                                ui.add_space(3.0);
                            }

                            let text_color = if is_selected {
                                theme.accent()
                            } else {
                                Color32::from_rgb(220, 225, 235)
                            };
                            let rt = egui::RichText::new(label).size(12.5).color(text_color);
                            ui.label(rt);
                        });
                    });

                // Make the entire frame clickable without triggering egui's text selection bounding box
                let interact_rect = resp.response.rect;
                let click_resp =
                    ui.interact(interact_rect, ui.id().with(sec), egui::Sense::click());
                if click_resp.clicked() {
                    state.nav_section = sec;
                }
                ui.add_space(3.0);
            }

            if state.settings.dev_mode {
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("DEVELOPER")
                        .size(9.5)
                        .strong()
                        .color(Color32::from_rgb(140, 150, 165)),
                );
                ui.add_space(2.0);

                for (sec, label) in dev_sections {
                    let is_selected = state.nav_section == sec;
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
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                if is_selected {
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(3.0, 14.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(1),
                                        theme.accent(),
                                    );
                                    ui.add_space(3.0);
                                }
                                let text_color = if is_selected {
                                    theme.accent()
                                } else {
                                    Color32::from_rgb(200, 205, 215)
                                };
                                ui.label(egui::RichText::new(label).size(12.0).color(text_color));
                            });
                        });

                    let interact_rect = resp.response.rect;
                    let click_resp =
                        ui.interact(interact_rect, ui.id().with(sec), egui::Sense::click());
                    if click_resp.clicked() {
                        state.nav_section = sec;
                    }
                    ui.add_space(2.0);
                }
            }
        });

    // ── Central Viewport Container ──────────────────────────────────────────
    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(ui.style()).fill(theme.bg_color()))
        .show_inside(ui, |ui| {
            // Map nav_section to appropriate view
            let target_view_id = match state.nav_section {
                NavSection::GalaxyMap3D | NavSection::Galaxy3D | NavSection::Cosmos3D => {
                    "galaxy_map_3d"
                }
                NavSection::SiForge | NavSection::LearningAndSelfPlay | NavSection::LivingMind => {
                    "si_forge"
                }
                NavSection::ScreenAutomation | NavSection::ScreenCapture => "screen_automation",
                NavSection::InterconnectMonitor | NavSection::Console => "signal_analyzer",
                NavSection::Agents
                | NavSection::Specialists
                | NavSection::SwarmMesh
                | NavSection::GhostStation => "agents_hub",
                NavSection::Settings => "settings",
                NavSection::DevStudio | NavSection::GameStudio | NavSection::CustomTools => {
                    "workbench"
                }
                _ => "agents_hub",
            };

            for view in views.iter_mut() {
                if view.id() == target_view_id {
                    view.render(ui, state);
                    return;
                }
            }

            // Fallback: render first available view if none matched
            if let Some(first) = views.first_mut() {
                first.render(ui, state);
            }
        });
}
