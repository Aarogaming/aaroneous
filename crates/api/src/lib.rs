use eframe::egui;

/// Represents a dynamically loaded UI component
pub trait UiCartridge: Send + Sync {
    fn name(&self) -> &str;
    fn render(&mut self, ui: &mut egui::Ui);
}