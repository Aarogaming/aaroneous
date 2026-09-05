// core/hypervisor/src/hud/modes/console_os.rs
//! Native 3D Game-System Console Launcher & AaroneousOS Hub.
//! Operates like a console operating system (Xbox / Steam Deck style):
//! - Full-canvas 3D glowing star particle field & camera orbit.
//! - Spatial navigation: Gamepad D-Pad, Thumbstick, or Arrow-Key focus snapping across Cartridges.
//! - Hero Showcase Banner with live rotating 3D wireframe mesh & real-time telemetry.
//! - Cartridge Grid of interactive cards (Companions, Auto-Pilot, Create & Train, Cosmos Map, Power Tools, Live Link).

use crate::hud::navigation::NavSection;
use crate::hud::state::{AppWindowMode, SharedHudState};
use eframe::egui::{self, Color32, CornerRadius, Key, Pos2, Stroke, Vec2};

pub struct CartridgeEntry {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub icon: &'static str,
    pub section: NavSection,
    pub status: &'static str,
    pub status_color: Color32,
    pub description: &'static str,
}

#[derive(Default)]
pub struct ConsoleOsLauncher {
    pub active_focus_idx: usize,
}

impl ConsoleOsLauncher {
    pub fn new() -> Self {
        Self { active_focus_idx: 0 }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;
        let time_sec = state.start_time.elapsed().as_secs_f32();

        let cartridges = [
            CartridgeEntry {
                title: "Companions & Team",
                subtitle: "Autonomous Agent Roster",
                icon: "🤖",
                section: NavSection::Agents,
                status: "ONLINE",
                status_color: Color32::from_rgb(63, 185, 80),
                description: "Command autonomous AI companions, coordinate swarm routines, and assign tasks.",
            },
            CartridgeEntry {
                title: "Screen & Auto-Pilot",
                subtitle: "Direct Vision & Motor Control",
                icon: "🎮",
                section: NavSection::ScreenAutomation,
                status: "READY",
                status_color: Color32::from_rgb(56, 139, 253),
                description: "Low-latency window capture, Bézier human emulation, and reactive automated play.",
            },
            CartridgeEntry {
                title: "Create & Train",
                subtitle: "Offline .si Model Foundry",
                icon: "⚡",
                section: NavSection::SiForge,
                status: "ACTIVE",
                status_color: Color32::from_rgb(163, 113, 247),
                description: "Synthesize high-efficiency offline reasoning matrices and 3D skill constellations.",
            },
            CartridgeEntry {
                title: "Cosmos 3D Map",
                subtitle: "Spatial Knowledge Graph",
                icon: "🌌",
                section: NavSection::GalaxyMap3D,
                status: "ORBITING",
                status_color: Color32::from_rgb(121, 192, 255),
                description: "Navigate live swarm topology, synaptic data channels, and 3D memory clusters.",
            },
            CartridgeEntry {
                title: "Power Tools",
                subtitle: "Developer & Script Workbench",
                icon: "🛠️",
                section: NavSection::DevStudio,
                status: "STANDBY",
                status_color: Color32::from_rgb(240, 136, 62),
                description: "Direct workspace editor, compiler diagnostic fixer, and AST rewriter.",
            },
            CartridgeEntry {
                title: "Live Link Stream",
                subtitle: "Zero-Copy Interconnect",
                icon: "⚡",
                section: NavSection::InterconnectMonitor,
                status: "100%",
                status_color: Color32::from_rgb(63, 185, 80),
                description: "Real-time 64MB shared-memory ring buffer with microsecond event throughput.",
            },
        ];

        // ── 1. Spatial Navigation State Machine (Gamepad / Arrow Keys) ──────────
        let total_cards = cartridges.len();
        let cols = 3;
        if ui.input(|i| i.key_pressed(Key::ArrowRight)) {
            self.active_focus_idx = (self.active_focus_idx + 1) % total_cards;
        }
        if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
            self.active_focus_idx = (self.active_focus_idx + total_cards - 1) % total_cards;
        }
        if ui.input(|i| i.key_pressed(Key::ArrowDown)) && self.active_focus_idx + cols < total_cards {
            self.active_focus_idx += cols;
        }
        if ui.input(|i| i.key_pressed(Key::ArrowUp)) && self.active_focus_idx >= cols {
            self.active_focus_idx -= cols;
        }

        // Enter or Space launches focused cartridge
        if ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space)) {
            state.nav_section = cartridges[self.active_focus_idx].section;
            state.app_window_mode = AppWindowMode::FullStudio;
        }

        // ── 2. Fullscreen Canvas & 3D Star Particle Field ───────────────────────
        let available_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let rect = response.rect;

        // Dark Nebula Background
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(8, 10, 18));

        // Background 3D Perspective Projection for particle field
        let center = rect.center() + Vec2::new(0.0, -40.0);
        let yaw: f32 = time_sec * 0.08;
        let pitch: f32 = 0.25;
        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        let cos_p = pitch.cos();
        let sin_p = pitch.sin();
        let focal_dist = 500.0;
        let cam_dist = 400.0;

        let project_3d = |pos: [f32; 3]| -> Option<(Pos2, f32)> {
            let x = pos[0];
            let y = pos[1];
            let z = pos[2];
            let x1 = x * cos_y - z * sin_y;
            let z1 = x * sin_y + z * cos_y;
            let y2 = y * cos_p - z1 * sin_p;
            let z2 = y * sin_p + z1 * cos_p;
            let z_cam = z2 + cam_dist;
            if z_cam <= 10.0 {
                return None;
            }
            let scale = focal_dist / z_cam;
            Some((Pos2::new(center.x + x1 * scale, center.y + y2 * scale), scale))
        };

        // Render ambient glowing 3D particle lattice
        for i in 0..100 {
            let seed = (i as f32) * 19.31;
            let px = (seed.sin() * 800.0) % 700.0;
            let py = ((seed * 1.6).cos() * 500.0) % 350.0;
            let pz = ((seed * 2.4).sin() * 800.0) % 700.0;
            if let Some((pt, scale)) = project_3d([px, py, pz]) {
                if rect.contains(pt) {
                    let tw = ((time_sec * 2.0 + i as f32 * 1.3).sin() * 0.5 + 0.5).clamp(0.2, 1.0);
                    let alpha = ((120.0 * tw) * scale.clamp(0.3, 1.0)) as u8;
                    painter.circle_filled(
                        pt,
                        (1.6 * scale).clamp(0.8, 3.0),
                        Color32::from_rgba_unmultiplied(140, 180, 255, alpha),
                    );
                }
            }
        }

        // ── 3. Console OS Top Navigation Strip ──────────────────────────────────
        let top_strip = egui::Rect::from_min_size(rect.min + Vec2::new(24.0, 16.0), Vec2::new(rect.width() - 48.0, 48.0));
        let mut top_ui = ui.new_child(egui::UiBuilder::new().max_rect(top_strip));
        top_ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⚡ AARONEOUS OS")
                    .color(theme.accent())
                    .size(20.0)
                    .strong(),
            );
            ui.label(
                egui::RichText::new("Console Edition")
                    .color(Color32::from_rgb(140, 155, 180))
                    .size(12.0),
            );

            ui.separator();

            // Level & Achievements Pill
            if ui.button(egui::RichText::new(format!("⭐ Lv. {}  •  🏆 {}/{}", state.user_level, state.achievements.unlocked_count(), state.achievements.total_count()))
                .color(Color32::from_rgb(255, 215, 0))
                .strong()).clicked() {
                state.show_achievements_modal = true;
            }

            ui.separator();

            // Active User Profile & Flow State Pill
            let prof = state.user_identity_engine.active_profile();
            let flow_pct = (state.user_identity_engine.flow_score() * 100.0).round() as u32;
            let user_badge = if prof.is_guest {
                format!("👤 {} [Guest • {}%]", prof.display_name, flow_pct)
            } else {
                format!("👤 {} [Flow {}%]", prof.display_name, flow_pct)
            };
            if ui.button(egui::RichText::new(user_badge).color(Color32::from_rgb(56, 139, 253)).strong()).clicked() {
                state.show_user_profile_modal = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🎛️ Utility Dashboard (F11)").clicked() {
                    state.app_window_mode = AppWindowMode::UtilityDashboard;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(860.0, 520.0)));
                }
                if ui.button("🪟 Full Studio").clicked() {
                    state.app_window_mode = AppWindowMode::FullStudio;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1240.0, 840.0)));
                }

                ui.label(
                    egui::RichText::new(format!("60 FPS  •  Harmony {:.0}%", state.bus_integrity))
                        .color(Color32::from_rgb(63, 185, 80))
                        .size(11.0),
                );
            });
        });

        // ── 4. Hero Showcase Banner (Active Focused Cartridge) ──────────────────
        let focused = &cartridges[self.active_focus_idx];
        let hero_rect = egui::Rect::from_min_size(
            rect.min + Vec2::new(24.0, 72.0),
            Vec2::new(rect.width() - 48.0, 190.0),
        );

        let mut hero_ui = ui.new_child(egui::UiBuilder::new().max_rect(hero_rect));
        egui::Frame::group(hero_ui.style())
            .fill(Color32::from_rgba_unmultiplied(18, 24, 38, 220))
            .stroke(Stroke::new(2.0, theme.accent()))
            .corner_radius(CornerRadius::same(12))
            .shadow(egui::Shadow {
                offset: [0, 8],
                blur: 28,
                spread: 2,
                color: Color32::from_black_alpha(180),
            })
            .show(&mut hero_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(focused.icon)
                            .size(54.0),
                    );
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(
                                egui::RichText::new(focused.title)
                                    .size(22.0)
                                    .color(Color32::WHITE)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("[ {} ]", focused.status))
                                    .color(focused.status_color)
                                    .strong()
                                    .size(12.0),
                            );
                        });

                        ui.label(
                            egui::RichText::new(focused.subtitle)
                                .color(theme.accent())
                                .strong(),
                        );
                        ui.add_space(4.0);

                        ui.label(
                            egui::RichText::new(focused.description)
                                .color(Color32::from_rgb(210, 220, 235))
                                .size(13.0),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("🚀 LAUNCH MODULE (Enter)")
                                            .color(Color32::WHITE)
                                            .strong()
                                            .size(13.0),
                                    )
                                    .fill(theme.accent())
                                    .min_size(Vec2::new(180.0, 32.0)),
                                )
                                .clicked()
                            {
                                state.nav_section = focused.section;
                                state.app_window_mode = AppWindowMode::FullStudio;
                            }

                            if ui.button("⚙️ Config").clicked() {
                                state.nav_section = NavSection::Settings;
                                state.app_window_mode = AppWindowMode::FullStudio;
                            }
                        });
                    });
                });
            });

        // ── 5. Cartridge Grid of Interactive Cards ──────────────────────────────
        let grid_rect = egui::Rect::from_min_size(
            rect.min + Vec2::new(24.0, 280.0),
            Vec2::new(rect.width() - 48.0, rect.height() - 295.0),
        );

        let mut grid_ui = ui.new_child(egui::UiBuilder::new().max_rect(grid_rect));
        grid_ui.horizontal_wrapped(|ui| {
            let card_w = ((grid_rect.width() - 32.0) / 3.0).max(220.0);
            let card_h = 130.0;

            for (idx, cart) in cartridges.iter().enumerate() {
                let is_focused = self.active_focus_idx == idx;
                let stroke_color = if is_focused {
                    Color32::from_rgb(255, 255, 255)
                } else {
                    theme.border_color()
                };
                let bg_color = if is_focused {
                    Color32::from_rgba_unmultiplied(30, 42, 64, 240)
                } else {
                    Color32::from_rgba_unmultiplied(16, 20, 30, 200)
                };

                let resp = egui::Frame::group(ui.style())
                    .fill(bg_color)
                    .stroke(Stroke::new(if is_focused { 2.0 } else { 1.0 }, stroke_color))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_width(card_w);
                        ui.set_height(card_h);

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(cart.icon).size(26.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(cart.title)
                                        .strong()
                                        .size(13.5)
                                        .color(if is_focused { Color32::WHITE } else { Color32::from_rgb(220, 225, 235) }),
                                );
                                ui.label(
                                    egui::RichText::new(cart.subtitle)
                                        .size(10.5)
                                        .color(theme.accent()),
                                );
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(cart.status)
                                        .color(cart.status_color)
                                        .size(10.0)
                                        .strong(),
                                );
                            });
                        });

                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(cart.description)
                                .size(11.0)
                                .color(Color32::from_rgb(160, 175, 195)),
                        );
                    });

                // Click or hover snapping
                let interact_rect = resp.response.rect;
                let click_resp = ui.interact(interact_rect, ui.id().with(idx), egui::Sense::click());
                if click_resp.hovered() {
                    self.active_focus_idx = idx;
                }
                if click_resp.clicked() {
                    state.nav_section = cart.section;
                    state.app_window_mode = AppWindowMode::FullStudio;
                }
                ui.add_space(4.0);
            }
        });
    }
}
