// core/hypervisor/src/hud/modes/full_studio.rs
//! Full Studio mode layout (top header, sidebar rail, view container, bottom status bar).

use crate::hud::navigation::NavSection;
use crate::hud::state::{AppWindowMode, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Stroke, Vec2};

pub fn render_full_studio(
    ui: &mut egui::Ui,
    state: &mut SharedHudState,
    views: &mut [Box<dyn HudView>],
    toggle_palette: &mut bool,
    toggle_shortcuts: &mut bool,
    toggle_guide: &mut bool,
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
                    egui::RichText::new("v1.6.0")
                        .size(11.0)
                        .color(Color32::from_rgb(140, 150, 170)),
                );

                ui.separator();

                // Active Persona badge
                let persona_name = state
                    .settings
                    .selected_gguf_model
                    .as_deref()
                    .unwrap_or("⚡ Local Assistant");
                ui.label(
                    egui::RichText::new(format!("🧠 {persona_name}"))
                        .size(11.0)
                        .color(theme.accent()),
                );

                ui.separator();

                // Gamified Productivity Level & XP Badge
                let level = state.user_level;
                let xp = state.user_xp;
                let next_xp = (level as u64) * 250;
                let progress = (xp as f32 / next_xp as f32).clamp(0.0, 1.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(format!("⭐ Lv. {}", level)).strong().color(Color32::from_rgb(255, 215, 0))).clicked() {
                        state.show_achievements_modal = !state.show_achievements_modal;
                    }
                    ui.add(egui::ProgressBar::new(progress).text(format!("{}/{} XP", xp, next_xp)).desired_width(110.0));
                    if ui.button(egui::RichText::new(format!("🏆 {}/{}", state.achievements.unlocked_count(), state.achievements.total_count())).color(Color32::from_rgb(255, 200, 80)).size(11.0)).clicked() {
                        state.show_achievements_modal = !state.show_achievements_modal;
                    }
                    if let Some(notif) = &state.xp_notification {
                        ui.label(egui::RichText::new(notif).color(Color32::from_rgb(63, 185, 80)).strong().size(11.0));
                    }
                });

                ui.separator();

                // Active Operator Identity & Flow Pill
                let prof = state.user_identity_engine.active_profile();
                let flow_pct = (state.user_identity_engine.flow_score() * 100.0).round() as u32;
                let user_badge = if prof.is_guest {
                    format!("👤 {} [Guest • {}%]", prof.display_name, flow_pct)
                } else {
                    format!("👤 {} [Flow {}%]", prof.display_name, flow_pct)
                };
                if ui.button(egui::RichText::new(user_badge).color(Color32::from_rgb(56, 139, 253)).strong().size(11.5)).clicked() {
                    state.show_user_profile_modal = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if state.settings.modular_canvas_mode { "🪟 Standard Mode" } else { "📐 Modular Workspace" }).clicked() {
                        state.settings.modular_canvas_mode = !state.settings.modular_canvas_mode;
                        state.settings.save_to_disk();
                    }

                    if ui.button("🚀 Tour Guide").clicked() {
                        *toggle_guide = true;
                    }

                    if ui.button("❓ Hotkeys (Ctrl+/)").clicked() {
                        *toggle_shortcuts = true;
                    }

                    if ui.button("🔍 Action Palette (Ctrl+K)").clicked() {
                        *toggle_palette = true;
                    }

                    if ui.button("🎮 Console Mode (F11)").clicked() {
                        state.app_window_mode = AppWindowMode::ConsoleGameOS;
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

                    if ui.button("🎮 In-Game HUD (F12)").clicked() {
                        state.is_ingame_overlay_open = !state.is_ingame_overlay_open;
                    }

                    if ui.button("🔴 Rec Macro (F9)").clicked() {
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
                    egui::RichText::new(format!("🤖 Active Companions: {active_count}"))
                        .size(11.0)
                        .color(if active_count > 0 {
                            Color32::from_rgb(63, 185, 80)
                        } else {
                            Color32::GRAY
                        }),
                );

                ui.separator();
                ui.label(
                    egui::RichText::new("⚡ Display Stream: 60 FPS (Ultra-Low Latency)")
                        .size(11.0)
                        .color(Color32::from_rgb(63, 185, 80)),
                );

                ui.separator();
                ui.label(
                    egui::RichText::new(format!("✨ System Harmony: {:.0}%", state.bus_integrity))
                        .size(11.0)
                        .color(theme.accent()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("Framerate: {:.0} FPS", state.measured_fps))
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
                (NavSection::Agents, "🤖 Companions & Team"),
                (NavSection::Specialists, "👥 9 Specialists"),
                (NavSection::ScreenAutomation, "🎮 Screen & Auto-Pilot"),
                (NavSection::SiForge, "⚡ Create & Train"),
                (NavSection::GalaxyMap3D, "🌌 Cosmos Map"),
                (NavSection::Settings, "⚙️ Settings"),
            ];

            let dev_sections = [
                (NavSection::DevStudio, "🛠️ Power Tools"),
                (NavSection::InterconnectMonitor, "⚡ Live Link"),
            ];

            for (sec, label) in sections {
                let is_selected = state.nav_section == sec;
                let available_w = ui.available_width();
                let row_size = Vec2::new(available_w, 28.0);
                let (rect, resp) = ui.allocate_exact_size(row_size, egui::Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                if resp.clicked() {
                    state.nav_section = sec;
                }

                let is_hovered = resp.hovered();
                let bg_color = if is_selected {
                    theme.card_bg()
                } else if is_hovered {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                } else {
                    Color32::TRANSPARENT
                };

                let stroke = if is_selected {
                    Stroke::new(1.0, theme.accent())
                } else if is_hovered {
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24))
                } else {
                    Stroke::NONE
                };

                ui.painter().rect(rect, CornerRadius::same(6), bg_color, stroke, egui::StrokeKind::Inside);

                if is_selected {
                    let bar_rect = egui::Rect::from_min_size(
                        Pos2::new(rect.min.x + 4.0, rect.center().y - 7.0),
                        Vec2::new(3.0, 14.0),
                    );
                    ui.painter().rect_filled(bar_rect, CornerRadius::same(1), theme.accent());
                }

                let text_pos = Pos2::new(rect.min.x + if is_selected { 14.0 } else { 8.0 }, rect.center().y);
                let text_color = if is_selected {
                    theme.accent()
                } else if is_hovered {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(220, 225, 235)
                };

                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(12.5),
                    text_color,
                );

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
                    let available_w = ui.available_width();
                    let row_size = Vec2::new(available_w, 26.0);
                    let (rect, resp) = ui.allocate_exact_size(row_size, egui::Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                    if resp.clicked() {
                        state.nav_section = sec;
                    }

                    let is_hovered = resp.hovered();
                    let bg_color = if is_selected {
                        theme.card_bg()
                    } else if is_hovered {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                    } else {
                        Color32::TRANSPARENT
                    };

                    let stroke = if is_selected {
                        Stroke::new(1.0, theme.accent())
                    } else if is_hovered {
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24))
                    } else {
                        Stroke::NONE
                    };

                    ui.painter().rect(rect, CornerRadius::same(6), bg_color, stroke, egui::StrokeKind::Inside);

                    if is_selected {
                        let bar_rect = egui::Rect::from_min_size(
                            Pos2::new(rect.min.x + 4.0, rect.center().y - 7.0),
                            Vec2::new(3.0, 14.0),
                        );
                        ui.painter().rect_filled(bar_rect, CornerRadius::same(1), theme.accent());
                    }

                    let text_pos = Pos2::new(rect.min.x + if is_selected { 14.0 } else { 8.0 }, rect.center().y);
                    let text_color = if is_selected {
                        theme.accent()
                    } else if is_hovered {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(200, 205, 215)
                    };

                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(12.0),
                        text_color,
                    );

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
                NavSection::Dashboard | NavSection::Specialists => "spatial_sensory",
                NavSection::GalaxyMap3D | NavSection::Galaxy3D | NavSection::Cosmos3D => {
                    "galaxy_map_3d"
                }
                NavSection::SiForge | NavSection::LearningAndSelfPlay | NavSection::LivingMind => {
                    "si_forge"
                }
                NavSection::ScreenAutomation | NavSection::ScreenCapture => "screen_automation",
                NavSection::InterconnectMonitor | NavSection::Console => "signal_analyzer",
                NavSection::Agents
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
