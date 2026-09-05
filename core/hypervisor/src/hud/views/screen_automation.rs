// core/hypervisor/src/hud/views/screen_automation.rs
//! Screen & Audio capture view, Discord-style window picker, vision feed preview,
//! and Master Auto-Pilot engagement toggle with live microsecond telemetry.

use crate::hud::state::{ScreenShareTab, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, TextureOptions, Vec2};

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
                egui::RichText::new("🎮 Screen Capture & Auto-Pilot Engine")
                    .color(theme.accent())
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh Targets").clicked() {
                    state.discovered_windows =
                        platform_bridge::WindowDiscoveryEngine::enumerate_available_targets()
                            .unwrap_or_default();
                    state.discovered_screens =
                        platform_bridge::WindowDiscoveryEngine::enumerate_screens()
                            .unwrap_or_default();
                }
            });
        });

        ui.label(
            "Ultra-low latency window-specific capture, direct display mirroring, and automated action execution.",
        );
        ui.separator();

        // ── Auto-Pilot Master Engage Section ────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🤖 Auto-Pilot Control")
                    .color(theme.accent())
                    .strong(),
            );
        });

        ui.horizontal(|ui| {
            let pilot_tele = &state.auto_pilot_telemetry;
            let is_engaged = pilot_tele.state == crate::hud::auto_pilot::AutoPilotState::Engaged;
            let is_emergency =
                pilot_tele.state == crate::hud::auto_pilot::AutoPilotState::EmergencyStop;

            let engage_label = if is_engaged {
                "🔴 DISENGAGE Auto-Pilot"
            } else if is_emergency {
                "⚠️ RESET & Engage"
            } else {
                "🟢 ENGAGE Auto-Pilot (F9)"
            };

            let button_color = if is_engaged {
                egui::Color32::from_rgb(220, 50, 50)
            } else if is_emergency {
                egui::Color32::from_rgb(255, 165, 0)
            } else {
                egui::Color32::from_rgb(50, 200, 50)
            };

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(engage_label)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(button_color)
                    .min_size(Vec2::new(200.0, 32.0)),
                )
                .clicked()
            {
                state.auto_pilot_toggle_requested = true;
            }

            ui.separator();

            // Kill switch
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("🛑 EMERGENCY STOP")
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(180, 0, 0))
                    .min_size(Vec2::new(160.0, 32.0)),
                )
                .clicked()
            {
                state.auto_pilot_kill_requested = true;
            }
        });

        // ── Clean Compact Telemetry Gauge ──────────────────────────────────────
        ui.add_space(2.0);
        let tele = &state.auto_pilot_telemetry;
        ui.horizontal(|ui| {
            let state_color = match tele.state {
                crate::hud::auto_pilot::AutoPilotState::Engaged => Color32::from_rgb(63, 185, 80),
                crate::hud::auto_pilot::AutoPilotState::EmergencyStop => {
                    Color32::from_rgb(248, 81, 73)
                }
                _ => Color32::GRAY,
            };
            ui.label(
                egui::RichText::new(format!("● {:?}", tele.state))
                    .color(state_color)
                    .strong(),
            );
            ui.separator();
            ui.label(format!("Latency: {:.1}μs", tele.avg_tick_latency_us));
            ui.separator();
            ui.label(format!("FPS: {:.0}", tele.active_fps));
            ui.separator();
            ui.label(format!("JIT Execs: {}", tele.jit_executions));
            ui.separator();
            ui.label(format!("Actions: {}", tele.hid_actions_dispatched));
        });

        ui.separator();

        // ── Window & Screen Picker ──────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut state.screen_share_tab,
                ScreenShareTab::Applications,
                "🪟 Applications",
            );
            ui.selectable_value(
                &mut state.screen_share_tab,
                ScreenShareTab::Screens,
                "🖥️ Entire Screen",
            );
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            // Left Column: Discovered Target Windows
            cols[0].vertical(|ui| {
                match state.screen_share_tab {
                    ScreenShareTab::Applications => {
                        ui.label(egui::RichText::new("Select Application Target:").strong());
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                if state.discovered_windows.is_empty() {
                                    ui.label(
                                        egui::RichText::new("No active application windows detected.")
                                            .italics(),
                                    );
                                } else {
                                    for (i, win) in state.discovered_windows.iter().enumerate() {
                                        let is_selected = state.selected_window_idx == i;
                                        let title_text = if win.title.is_empty() {
                                            "[Untitled Window]"
                                        } else {
                                            &win.title
                                        };
                                        let label = format!("{} ({})", title_text, win.process_name);

                                        if ui.selectable_label(is_selected, label).clicked() {
                                            state.selected_window_idx = i;
                                        }
                                    }
                                }
                            });
                    }
                    ScreenShareTab::Screens => {
                        ui.label(egui::RichText::new("Select Physical Display:").strong());
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                if state.discovered_screens.is_empty() {
                                    ui.label(
                                        egui::RichText::new("No active screens detected.")
                                            .italics(),
                                    );
                                } else {
                                    for (i, scr) in state.discovered_screens.iter().enumerate() {
                                        let is_selected = state.selected_screen_idx == i;
                                        let label = format!("🖥️ {} [{}x{}]", scr.name, scr.resolution.0, scr.resolution.1);

                                        if ui.selectable_label(is_selected, label).clicked() {
                                            state.selected_screen_idx = i;
                                            state.capture_modifiers.target = platform_bridge::CaptureTarget::EntireDisplay {
                                                display_id: scr.display_id,
                                                name: scr.name.clone(),
                                            };
                                        }
                                    }
                                }
                            });
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Display Performance & Filters:").strong());
                ui.horizontal(|ui| {
                    ui.label("Display Target Rate:");
                    ui.add(
                        egui::Slider::new(&mut state.capture_modifiers.target_fps, 15..=120)
                            .text("FPS"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Motion Sensitivity:");
                    ui.add(
                        egui::Slider::new(
                            &mut state.capture_modifiers.entropy_threshold,
                            0.01..=0.20,
                        )
                        .text("Sensitivity"),
                    );
                });
            });

            // Right Column: Live Viewport Preview
            cols[1].vertical(|ui| {
                ui.label(
                    egui::RichText::new("Live Perceptual Stream (128x128 Gated Grid)").strong(),
                );

                if state.viewport_texture.is_none() {
                    let mut dummy_rgba = vec![0u8; 128 * 128 * 4];
                    for i in 0..(128 * 128) {
                        dummy_rgba[i * 4] = (i % 256) as u8;
                        dummy_rgba[i * 4 + 1] = 100;
                        dummy_rgba[i * 4 + 2] = 200;
                        dummy_rgba[i * 4 + 3] = 255;
                    }
                    let color_img =
                        egui::ColorImage::from_rgba_unmultiplied([128, 128], &dummy_rgba);
                    state.viewport_texture = Some(ui.ctx().load_texture(
                        "vision_stream_tex",
                        color_img,
                        TextureOptions::NEAREST,
                    ));
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

        ui.add_space(10.0);
        ui.separator();

        // ── Deep Sensory Inputs (Normalized UI) ──────────────────────────────────
        egui::CollapsingHeader::new(
            egui::RichText::new("👁️ Enhanced Sensory Inputs (Deep Window Reader & Audio Monitor)")
                .strong()
                .color(theme.accent()),
        )
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // UI Element Reader Deck
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(eframe::egui::Stroke::new(1.0, theme.border_color()))
                    .corner_radius(eframe::egui::CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width() * 0.48);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("👁️ Deep Window Reader").strong());
                            ui.label("Inspects interactive buttons, input fields, and application controls directly.");
                            ui.add_space(4.0);

                            if ui.checkbox(&mut state.deep_ui_reader_enabled, "Enable Deep Window Reading").changed() {
                                if state.deep_ui_reader_enabled {
                                    let walker = platform_bridge::observability::UiaTreeWalker::default();
                                    if let Ok(tree) = walker.walk_window_tree(0) {
                                        state.deep_ui_discovered_elements = tree.flatten().len();
                                        state.deep_ui_focused_element = Some("Active Input Surface".to_string());
                                    }
                                } else {
                                    state.deep_ui_discovered_elements = 0;
                                    state.deep_ui_focused_element = None;
                                }
                            }

                            if state.deep_ui_reader_enabled {
                                ui.label(egui::RichText::new(format!("Controls Identified: {} interactive elements", state.deep_ui_discovered_elements)).color(Color32::from_rgb(63, 185, 80)));
                                if let Some(focus) = &state.deep_ui_focused_element {
                                    ui.label(format!("Focused Control: {}", focus));
                                }
                            }
                        });
                    });

                // Audio Monitor Deck
                egui::Frame::group(ui.style())
                    .fill(theme.card_bg())
                    .stroke(eframe::egui::Stroke::new(1.0, theme.border_color()))
                    .corner_radius(eframe::egui::CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("🎙️ System Audio & Voice Monitor").strong());
                            ui.label("Listens to application sound cues and audio events to trigger responsive workflows.");
                            ui.add_space(4.0);

                            if ui.checkbox(&mut state.audio_monitor_enabled, "Enable Audio Cue Ingestion").changed() {
                                if state.audio_monitor_enabled {
                                    state.audio_last_event_desc = Some("Audio loopback active (48 kHz 2ch)".to_string());
                                } else {
                                    state.audio_last_event_desc = None;
                                }
                            }

                            if state.audio_monitor_enabled {
                                ui.horizontal(|ui| {
                                    ui.label("Input Level:");
                                    ui.add(egui::ProgressBar::new(0.65).text("-18 dB"));
                                });
                                if let Some(desc) = &state.audio_last_event_desc {
                                    ui.label(egui::RichText::new(desc).color(Color32::from_rgb(63, 185, 80)).size(11.0));
                                }
                            }
                        });
                    });
            });
        });
    }
}
