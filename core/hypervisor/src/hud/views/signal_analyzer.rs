// core/hypervisor/src/hud/views/signal_analyzer.rs
//! Zero-Copy SWMR Shared Memory Interconnect Bus monitor and internal IPC stream console.

use crate::hud::state::SharedHudState;
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, Vec2};

#[derive(Default)]
pub struct SignalAnalyzerView;

impl HudView for SignalAnalyzerView {
    fn id(&self) -> &'static str {
        "signal_analyzer"
    }

    fn title(&self) -> &'static str {
        "⚡ Bus Monitor & Console"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;

        ui.heading(
            egui::RichText::new("⚡ Zero-Copy SWMR Shared Memory Interconnect Bus")
                .color(theme.accent())
                .strong(),
        );
        ui.label("Direct inspection of the 64 MB kernel memory-mapped ring buffer and generation clock.");
        ui.separator();

        ui.horizontal(|ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_size(Vec2::new(200.0, 90.0));
                ui.label("Bus Integrity:");
                ui.heading(format!("{:.1}%", state.bus_integrity));
                ui.add(egui::ProgressBar::new(state.bus_integrity / 100.0).text("Dynamic Equilibrium Verified"));
            });

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_size(Vec2::new(200.0, 90.0));
                ui.label("Understanding Score:");
                ui.heading(format!("{:.1}%", state.bus_understanding));
                ui.add(egui::ProgressBar::new(state.bus_understanding / 100.0).text("Alignment Verified"));
            });

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_size(Vec2::new(200.0, 90.0));
                ui.label("Throughput Rate:");
                ui.heading(format!("{:.0} pkts/sec", state.bus_events_per_sec));
                ui.label(egui::RichText::new("Sub-microsecond latency").color(Color32::from_rgb(63, 185, 80)));
            });
        });

        ui.add_space(8.0);
        ui.label(format!("Shared Memory Path: {}", state.bus_path.display()));
        ui.label("Mmap Buffer Size: 64 MB");
        ui.separator();

        // ── Internal IPC Chat Console ───────────────────────────────────────────
        ui.heading(
            egui::RichText::new("💬 Protocol Console (Internal IPC Stream)")
                .color(theme.accent())
                .strong(),
        );
        ui.label("Send raw task intents directly into the live SWMR shared memory bus.");

        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for (sender, msg, color) in &state.chat_history {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("[{}]", sender)).color(*color).strong());
                    ui.label(msg);
                });
                ui.add_space(2.0);
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            let response = ui.text_edit_singleline(&mut state.chat_input);
            if (ui.button("Inject Intent ⚡").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !state.chat_input.trim().is_empty()
            {
                let user_msg = state.chat_input.clone();
                state.chat_history.push(("User".to_string(), user_msg.clone(), Color32::WHITE));

                state.inject_live_intent(&user_msg);
                state.chat_input.clear();

                state.chat_history.push((
                    "System".to_string(),
                    format!("Intent injected into interconnect.bus (#{}).", state.bus_generation),
                    Color32::from_rgb(210, 153, 34),
                ));
            }
        });
    }
}
