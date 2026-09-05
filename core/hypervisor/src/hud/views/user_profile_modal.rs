// core/hypervisor/src/hud/views/user_profile_modal.rs
//! User Profile, Kinematic Calibration & JARVIS Companion Modal.
//!
//! Visualizes:
//! 1. Active User Profile, Primary vs. Guest tag, and rapid profile switcher.
//! 2. Live Operator Flow State gauge (Deep Flow, Deliberating, Skimming, Distracted, Fatigued).
//! 3. Real-time Kinematic Radar / Biomarkers (Speed, Tremor, Dwell, Flight, Correction Rate).
//! 4. Conversational JARVIS companion interaction drawer.

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use crate::hud::state::SharedHudState;
use compute::AttentionState;

pub fn render_user_profile_modal(ctx: &egui::Context, state: &mut SharedHudState) {
    if !state.show_user_profile_modal {
        return;
    }

    let screen_rect = ctx.content_rect();
    let modal_width = 560.0f32.min(screen_rect.width() - 40.0);
    let modal_height = 620.0f32.min(screen_rect.height() - 40.0);

    let mut is_open = true;
    let mut should_close = false;
    let mut switched_target_id: Option<String> = None;

    egui::Window::new("👤 Operator Identity & Companion Sync")
        .open(&mut is_open)
        .fixed_size(Vec2::new(modal_width, modal_height))
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(screen_rect.center())
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);

            // Top Header: Active Profile & Flow Metric
            ui.horizontal(|ui| {
                let is_guest = state.user_identity_engine.active_profile().is_guest;
                let (badge_text, badge_color) = if is_guest {
                    ("🛡️ GUEST OPERATOR", Color32::from_rgb(210, 153, 34))
                } else {
                    ("⚡ PRIMARY OPERATOR", Color32::from_rgb(56, 139, 253))
                };

                ui.label(
                    RichText::new(badge_text)
                        .color(badge_color)
                        .strong()
                        .size(13.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let flow_score = state.user_identity_engine.flow_score();
                    let flow_pct = (flow_score * 100.0).round() as u32;
                    let flow_color = if flow_pct >= 85 {
                        Color32::from_rgb(63, 185, 80)
                    } else if flow_pct >= 60 {
                        Color32::from_rgb(210, 153, 34)
                    } else {
                        Color32::from_rgb(248, 81, 73)
                    };

                    ui.label(
                        RichText::new(format!("Harmony: {}%", flow_pct))
                            .color(flow_color)
                            .strong()
                            .size(13.0),
                    );
                });
            });

            ui.separator();

            // Profile Info & Switching
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Active Identity:").strong());
                    ui.label(
                        RichText::new(&state.user_identity_engine.active_profile().display_name)
                            .color(Color32::WHITE)
                            .strong()
                            .size(15.0),
                    );
                });

                ui.add_space(4.0);
                ui.label(RichText::new("Registered Operators:").size(11.0).color(Color32::from_gray(160)));

                let profiles_meta: Vec<(String, String, bool)> = state
                    .user_identity_engine
                    .all_profiles()
                    .into_iter()
                    .map(|p| (p.id.clone(), p.display_name.clone(), p.is_primary))
                    .collect();
                let active_id = state.user_identity_engine.active_profile().id.clone();

                ui.horizontal_wrapped(|ui| {
                    for (prof_id, prof_name, is_primary) in profiles_meta {
                        let is_active = prof_id == active_id;
                        let btn_label = if is_primary {
                            format!("👑 {prof_name}")
                        } else {
                            format!("👤 {prof_name}")
                        };

                        let btn = egui::Button::new(RichText::new(btn_label).size(12.0))
                            .stroke(Stroke::new(
                                1.0,
                                if is_active {
                                    Color32::from_rgb(56, 139, 253)
                                } else {
                                    Color32::from_gray(60)
                                },
                            ));

                        if ui.add(btn).clicked() && !is_active {
                            switched_target_id = Some(prof_id);
                        }
                    }
                });
            });

            // Flow State & Mental Cadence
            ui.group(|ui| {
                ui.label(RichText::new("Attentional State & Cognitive Cadence").strong().size(13.0));
                ui.add_space(2.0);

                let (state_title, state_desc, state_color) = match state.user_identity_engine.attention_state() {
                    AttentionState::DeepFlow => (
                        "🌊 Deep Flow State",
                        "Kinematic inputs match baseline with high velocity smoothness. Training ingestion enabled.",
                        Color32::from_rgb(63, 185, 80),
                    ),
                    AttentionState::Deliberating => (
                        "🤔 Deliberating / Precision Focus",
                        "Operator is calculating next action before execution. Pauses are intentional.",
                        Color32::from_rgb(56, 139, 253),
                    ),
                    AttentionState::Skimming => (
                        "👀 Skimming & Overview",
                        "Rapid cursor movement with low click density. Sensory inputs held in staging.",
                        Color32::from_rgb(210, 153, 34),
                    ),
                    AttentionState::Distracted => (
                        "⚡ Context Shift / Distracted",
                        "Out-of-domain focus or irregular rhythm detected. Training quarantine engaged.",
                        Color32::from_rgb(248, 81, 73),
                    ),
                    AttentionState::Fatigued => (
                        "☕ Fatigue Tolerance Warning",
                        "Elevated flight times and micro-correction spikes. Autonomous assist ratio increased.",
                        Color32::from_rgb(219, 109, 40),
                    ),
                };

                ui.label(RichText::new(state_title).color(state_color).strong().size(14.0));
                ui.label(RichText::new(state_desc).size(11.0).color(Color32::from_gray(180)));
            });

            // Kinematic Biomarkers Telemetry
            ui.group(|ui| {
                ui.label(RichText::new("Kinematic Biomarkers (Live vs. Baseline)").strong().size(13.0));
                ui.add_space(4.0);

                let bio = state.user_identity_engine.active_profile().baseline_biomarkers.clone();

                egui::Grid::new("biomarkers_grid").num_columns(2).spacing([20.0, 6.0]).show(ui, |ui| {
                    ui.label("Cursor Velocity:");
                    ui.label(RichText::new(format!("{:.0} px/s (±{:.0})", bio.mean_cursor_speed, bio.cursor_speed_stddev)).strong());
                    ui.end_row();

                    ui.label("Trajectory Smoothness:");
                    ui.label(RichText::new(format!("{:.1}% jitter tolerance", bio.trajectory_jitter * 100.0)).strong());
                    ui.end_row();

                    ui.label("Keystroke Dynamics:");
                    ui.label(RichText::new(format!("{:.0}ms dwell / {:.0}ms flight", bio.key_dwell_ms, bio.key_flight_ms)).strong());
                    ui.end_row();

                    ui.label("Target Deceleration:");
                    ui.label(RichText::new(format!("{:.2} Fitts index", bio.target_landing_deceleration)).strong());
                    ui.end_row();
                });
            });

            // Companion Interaction & Speech
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎙️ Companion Presence (JARVIS Persona)").strong().size(13.0));
                });
                ui.add_space(4.0);

                let active_name = state.user_identity_engine.active_profile().display_name.clone();
                let is_guest = state.user_identity_engine.active_profile().is_guest;
                let flow = state.user_identity_engine.flow_score();

                let greeting = state.intercom.companion.generate_greeting(&active_name, is_guest, flow);
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Aaroneous:").color(Color32::from_rgb(56, 139, 253)).strong());
                    ui.label(RichText::new(format!("\"{}\"", greeting)).italics().color(Color32::from_gray(210)));
                });
            });

            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("Close").strong()).clicked() {
                    should_close = true;
                }
            });
        });

    if should_close {
        is_open = false;
    }
    if let Some(target_id) = switched_target_id {
        let _ = state.user_identity_engine.switch_user(&target_id);
    }
    state.show_user_profile_modal = is_open;
}
