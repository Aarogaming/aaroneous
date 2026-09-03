// core/hypervisor/src/hud/navigation/shortcuts.rs
//! Keyboard Shortcuts Reference Modal (`?` / `Ctrl+/`).

use crate::hud::theme::HudTheme;
use eframe::egui::{self, CornerRadius, Stroke, Vec2};

#[derive(Default)]
pub struct ShortcutsModal {
    pub is_open: bool,
}

impl ShortcutsModal {
    pub fn new() -> Self {
        Self { is_open: false }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn render(&mut self, ctx: &egui::Context, theme: HudTheme) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let screen_rect = ctx.content_rect();
        let modal_width = 520.0f32.min(screen_rect.width() - 40.0);

        egui::Window::new("⌨️ Keyboard Shortcuts Reference")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_size(Vec2::new(modal_width, 360.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(theme.panel_bg())
                    .stroke(Stroke::new(1.5, theme.accent()))
                    .corner_radius(CornerRadius::same(10)),
            )
            .show(ctx, |ui| {
                ui.heading(
                    egui::RichText::new("Global Keyboard Shortcuts")
                        .color(theme.accent())
                        .strong(),
                );
                ui.separator();

                let shortcuts = [
                    ("Ctrl + K  /  Ctrl + P", "Open Command Palette with fuzzy action search"),
                    ("Middle-Click / Space + Drag", "Pan the Spatial Window Canvas"),
                    ("Ctrl + Scroll Wheel", "Zoom Spatial Canvas (0.5x – 2.0x)"),
                    ("Home  /  0", "Reset Spatial Canvas Pan & Zoom to Origin"),
                    ("F9", "Start / Stop Macro Action Recording"),
                    ("F10", "Toggle Compact Floating Mini-Recorder HUD"),
                    ("F11", "Minimize Studio to System Tray"),
                    ("F12  /  Win + G", "Launch In-Game Transparent Overlay HUD"),
                    ("?  /  Ctrl + /", "Open this Keyboard Shortcuts Reference"),
                    ("Esc", "Close any active modal dialog or command palette"),
                ];

                for (keys, desc) in &shortcuts {
                    ui.horizontal(|ui| {
                        egui::Frame::group(ui.style())
                            .fill(theme.card_bg())
                            .stroke(Stroke::new(1.0, theme.border_color()))
                            .corner_radius(CornerRadius::same(4))
                            .show(ui, |ui| {
                                ui.set_min_width(160.0);
                                ui.label(
                                    egui::RichText::new(*keys)
                                        .color(theme.accent())
                                        .strong()
                                        .monospace(),
                                );
                            });
                        ui.label(*desc);
                    });
                    ui.add_space(4.0);
                }

                ui.add_space(8.0);
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.is_open = false;
                    }
                });
            });

        self.is_open = open;
    }
}
