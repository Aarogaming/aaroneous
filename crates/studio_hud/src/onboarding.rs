// core/hypervisor/src/hud/onboarding.rs
//! Interactive User Onboarding and Quick Guided Tour.
//! Teaches new users how to operate Aaroneous. Can be dismissed and never shown on startup,
//! while remaining readily accessible anytime via the top header or Guide button.

use crate::hud::state::SharedHudState;
use crate::hud::theme::HudTheme;
use eframe::egui::{self, Color32, CornerRadius, Stroke};

#[derive(Debug, Clone, Default)]
pub struct OnboardingGuide {
    pub is_open: bool,
    pub current_step: usize,
}

impl OnboardingGuide {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self, ctx: &egui::Context, state: &mut SharedHudState, theme: HudTheme) {
        if !self.is_open {
            return;
        }

        let screen_rect = ctx.content_rect();
        let modal_width = 620.0f32.min(screen_rect.width() - 40.0);
        let modal_height = 420.0f32.min(screen_rect.height() - 60.0);

        egui::Area::new(egui::Id::new("interactive_onboarding_modal"))
            .fixed_pos(egui::pos2(
                (screen_rect.width() - modal_width) * 0.5,
                (screen_rect.height() - modal_height) * 0.4,
            ))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.global_style())
                    .fill(theme.panel_bg())
                    .stroke(Stroke::new(1.5, theme.accent()))
                    .corner_radius(CornerRadius::same(12))
                    .shadow(egui::Shadow {
                        offset: [0, 10],
                        blur: 32,
                        spread: 4,
                        color: Color32::from_black_alpha(220),
                    })
                    .show(ui, |ui| {
                        ui.set_width(modal_width);
                        ui.set_height(modal_height);

                        // Header with step indicator & close button
                        ui.horizontal(|ui| {
                            ui.heading(
                                egui::RichText::new("🚀 Aaroneous Quick-Start Guide")
                                    .color(theme.accent())
                                    .strong(),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✕ Close").clicked() {
                                    self.is_open = false;
                                }
                                ui.label(
                                    egui::RichText::new(format!("Step {} of 4", self.current_step + 1))
                                        .color(Color32::GRAY)
                                        .size(11.0),
                                );
                            });
                        });

                        ui.separator();
                        ui.add_space(8.0);

                        // Dynamic step content
                        match self.current_step {
                            0 => {
                                ui.heading("👋 Welcome to Aaroneous!");
                                ui.label(
                                    "Aaroneous is your autonomous synthetic companion and high-performance desktop engine. \
                                     It blends non-linguistic offline AI intelligence, real-time screen auto-pilot, and gamified productivity.",
                                );
                                ui.add_space(8.0);

                                egui::Frame::group(ui.style())
                                    .fill(theme.card_bg())
                                    .corner_radius(CornerRadius::same(6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("⚡ Key Tip:").strong().color(theme.accent()));
                                            ui.label("Everything runs locally and offline with zero cloud dependency.");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("🔍 Action Palette:").strong());
                                            ui.label("Press Ctrl+K or Ctrl+P at any time to navigate anywhere in 1 tap.");
                                        });
                                    });
                            }
                            1 => {
                                ui.heading("🎮 Building Routines & Screen Auto-Pilot");
                                ui.label(
                                    "Use 'Screen & Auto-Pilot' to choose an active game window or full display. \
                                     You can record action playthroughs (F9) or visually assemble multi-step routines with natural Bezier cursor motion.",
                                );
                                ui.add_space(8.0);

                                egui::Frame::group(ui.style())
                                    .fill(theme.card_bg())
                                    .corner_radius(CornerRadius::same(6))
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("🛡️ Safety First:").strong().color(Color32::from_rgb(63, 185, 80)));
                                        ui.label("• An Emergency Stop button and global killswitch are always one key away.");
                                        ui.label("• Floating Mini-HUD (F10) keeps recorder controls accessible without blocking your work.");
                                    });
                            }
                            2 => {
                                ui.heading("⚡ Create & Train Offline Models");
                                ui.label(
                                    "Transform recorded actions or codebase knowledge into compact, verified .si model packages. \
                                     These cartridges load instantly in under 1 microsecond without requiring GPU VRAM.",
                                );
                                ui.add_space(8.0);

                                egui::Frame::group(ui.style())
                                    .fill(theme.card_bg())
                                    .corner_radius(CornerRadius::same(6))
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("🌌 3D Constellation Skills:").strong().color(theme.accent()));
                                        ui.label("• Train skills along celestial constellation branches (Perception, Kinematics, Reflex, Thermodynamics).");
                                        ui.label("• Earn Mastery Points to unlock deeper capabilities and perks.");
                                    });
                            }
                            3 => {
                                ui.heading("🏆 Gamified Productivity & Achievements");
                                ui.label(
                                    "As you complete routines, build models, and use system shortcuts, you earn XP and level up. \
                                     The Achievement system guides your training, rewarding you for mastering each capability.",
                                );
                                ui.add_space(8.0);

                                egui::Frame::group(ui.style())
                                    .fill(theme.card_bg())
                                    .corner_radius(CornerRadius::same(6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("⭐ Current Level:").strong().color(Color32::from_rgb(255, 215, 0)));
                                            ui.label(format!("Level {} ({} XP)", state.user_level, state.user_xp));
                                        });
                                        ui.label("• Unlocking milestones awards XP bursts and unlocks permanent badges.");
                                    });
                            }
                            _ => {}
                        }

                        ui.add_space(14.0);

                        // Footer Navigation & Persistence Toggle
                        ui.separator();
                        ui.horizontal(|ui| {
                            // Persistent "Don't show on launch" checkbox
                            let mut show_on_startup = state.settings.show_welcome_guide_on_startup;
                            if ui.checkbox(&mut show_on_startup, "Show this guide when starting Aaroneous").changed() {
                                state.settings.show_welcome_guide_on_startup = show_on_startup;
                                state.settings.save_to_disk();
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if self.current_step < 3 {
                                    if ui.button(egui::RichText::new("Next ➡️").strong()).clicked() {
                                        self.current_step += 1;
                                    }
                                } else if ui.button(egui::RichText::new("🎉 Get Started").strong().color(Color32::WHITE)).clicked() {
                                    self.is_open = false;
                                    state.settings.show_welcome_guide_on_startup = false;
                                    state.settings.save_to_disk();
                                    state.award_xp(50, "Completed Welcome Tour");
                                }

                                if self.current_step > 0 && ui.button("⬅️ Back").clicked() {
                                    self.current_step -= 1;
                                }
                            });
                        });
                    });
            });
    }
}
