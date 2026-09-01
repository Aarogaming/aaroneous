// core/hypervisor/src/hud/views/screen_automation.rs
//! Screen & Audio capture view, Discord-style window picker, and vision feed preview.

use crate::hud::state::{ScreenShareTab, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, TextureOptions, Vec2};

#[derive(Default)]
pub struct ScreenAutomationView;

impl HudView for ScreenAutomationView {
    fn id(&self) -> &'static str {
        "screen_automation"
    }

    fn title(&self) -> &'static str {
        "👁️ Screen & Motor"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("👁️ Epigenetic Vision & Sandboxed Motor Engine")
                    .color(theme.accent())
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh Windows").clicked() {
                    state.discovered_windows = platform_bridge::WindowDiscoveryEngine::enumerate_available_targets().unwrap_or_default();
                }
            });
        });

        ui.label(
            "DirectX 12 / DXGI desktop duplication and window-specific capture with epigenetic delta gating.",
        );
        ui.separator();

        // ── Window & Screen Picker ──────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.screen_share_tab, ScreenShareTab::Applications, "🪟 Applications");
            ui.selectable_value(&mut state.screen_share_tab, ScreenShareTab::Screens, "🖥️ Entire Screen");
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            // Left Column: Discovered Target Windows
            cols[0].vertical(|ui| {
                ui.label(egui::RichText::new("Select Capture Target:").strong());
                egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                    if state.discovered_windows.is_empty() {
                        ui.label(egui::RichText::new("No active application windows detected.").italics());
                    } else {
                        for (i, win) in state.discovered_windows.iter().enumerate() {
                            let is_selected = state.selected_window_idx == i;
                            let title_text = if win.title.is_empty() { "[Untitled Window]" } else { &win.title };
                            let label = format!("{} ({})", title_text, win.process_name);

                            if ui.selectable_label(is_selected, label).clicked() {
                                state.selected_window_idx = i;
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Capture Modifiers & Filters:").strong());
                ui.horizontal(|ui| {
                    ui.label("Target FPS:");
                    ui.add(egui::Slider::new(&mut state.capture_modifiers.target_fps, 15..=120).text("FPS"));
                });
                ui.horizontal(|ui| {
                    ui.label("Entropy Threshold:");
                    ui.add(egui::Slider::new(&mut state.capture_modifiers.entropy_threshold, 0.01..=0.20).text("Threshold"));
                });
            });

            // Right Column: Live Viewport Preview
            cols[1].vertical(|ui| {
                ui.label(egui::RichText::new("Live Perceptual Stream (128x128 Gated Grid)").strong());

                if state.viewport_texture.is_none() {
                    let mut dummy_rgba = vec![0u8; 128 * 128 * 4];
                    for i in 0..(128 * 128) {
                        dummy_rgba[i * 4] = (i % 256) as u8;
                        dummy_rgba[i * 4 + 1] = 100;
                        dummy_rgba[i * 4 + 2] = 200;
                        dummy_rgba[i * 4 + 3] = 255;
                    }
                    let color_img = egui::ColorImage::from_rgba_unmultiplied([128, 128], &dummy_rgba);
                    state.viewport_texture = Some(ui.ctx().load_texture("vision_stream_tex", color_img, TextureOptions::NEAREST));
                }

                if let Some(texture) = &state.viewport_texture {
                    ui.image((texture.id(), Vec2::new(260.0, 260.0)));
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(format!("Framerate: {:.1} FPS", state.vision_fps));
                    ui.separator();
                    ui.label(format!("Entropy: {:.2} bits", state.vision_entropy));
                });
            });
        });
    }
}
