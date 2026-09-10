use eframe::egui;

/// SEMANTIC-09: Action Macro Recorder UI
/// Provides a visual timeline for mapping recorded .si reflex skills.
pub struct MacroTimelineUi {
    pub is_visible: bool,
    pub frames_recorded: usize,
}

impl MacroTimelineUi {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            frames_recorded: 0,
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, is_recording: bool) {
        if !self.is_visible && !is_recording {
            return;
        }

        if is_recording {
            self.frames_recorded += 1;
        }

        egui::Panel::bottom("macro_timeline_panel")
            .resizable(true)
            .min_size(80.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("🎬 Action Macro Timeline");
                    if is_recording {
                        ui.label(egui::RichText::new("🔴 RECORDING IN PROGRESS").color(egui::Color32::RED).strong());
                        ui.spinner();
                    } else {
                        ui.label(egui::RichText::new("⏹️ IDLE").color(egui::Color32::GRAY));
                    }
                });

                ui.separator();

                // Draw a visual timeline track
                let (rect, _response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 30.0),
                    egui::Sense::hover(),
                );

                let painter = ui.painter();
                painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 30, 30));

                // Draw tick marks for recorded frames
                let width = rect.width();
                let spacing = 5.0;
                let max_ticks = (width / spacing) as usize;
                
                let visible_ticks = self.frames_recorded.min(max_ticks);
                for i in 0..visible_ticks {
                    let x = rect.left() + (i as f32 * spacing);
                    painter.line_segment(
                        [egui::pos2(x, rect.top() + 5.0), egui::pos2(x, rect.bottom() - 5.0)],
                        egui::Stroke::new(1.0, egui::Color32::RED),
                    );
                }
            });
    }
}