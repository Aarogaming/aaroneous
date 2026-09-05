use aaroneous_api::UiCartridge;
use eframe::egui;

pub struct ScratchpadCartridge {
    input_text: String,
    history: String,
}

impl ScratchpadCartridge {
    pub fn new() -> Self {
        Self {
            input_text: String::new(),
            history: "# Aaroneous OS Intercom\n\nI am connected to your Local Brain. How can I help you today?".to_string(),
        }
    }
}

impl UiCartridge for ScratchpadCartridge {
    fn name(&self) -> &str {
        "Native Scratchpad"
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("AI Intercom & Scratchpad");
        ui.separator();
        
        egui::Panel::bottom("editor_panel")
            .resizable(true)
            .min_size(100.0)
            .show_inside(ui, |panel_ui| {
                panel_ui.add_space(4.0);
                panel_ui.horizontal(|ui| {
                    if !self.input_text.is_empty() {
                        if ui.button("🚀 Submit").clicked() {
                            let prompt = self.input_text.clone();
                            self.history.push_str(&format!("\n\n---\n**🧑 You:**\n{}", prompt));
                            self.input_text.clear();
                        }
                    } else {
                        ui.label("Type to begin...");
                    }
                    if ui.button("📋 Clear").clicked() {
                        self.history = "# Aaroneous OS Intercom\n\nI am connected to your Local Brain.".to_string();
                        self.input_text.clear();
                    }
                });
                panel_ui.add_space(4.0);
                panel_ui.add(
                    egui::TextEdit::multiline(&mut self.input_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(5)
                        .hint_text("Type prompt here...")
                );
            });

        egui::CentralPanel::default().show_inside(ui, |inner_ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(inner_ui, |scroll_ui| {
                    scroll_ui.label(
                        egui::RichText::new(&self.history)
                            .family(egui::FontFamily::Proportional)
                            .size(14.0),
                    );
                });
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_plugin() -> *mut dyn UiCartridge {
    let plugin: Box<dyn UiCartridge> = Box::new(ScratchpadCartridge::new());
    Box::into_raw(plugin)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_plugin(ptr: *mut dyn UiCartridge) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}