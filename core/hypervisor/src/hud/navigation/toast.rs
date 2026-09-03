// core/hypervisor/src/hud/navigation/toast.rs
//! Toast notification engine for auto-fading alerts.

use crate::hud::theme::HudTheme;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Stroke, Vec2};
use std::time::Instant;

/// Toast Notification Level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A Toast Notification Item
#[derive(Debug, Clone)]
pub struct ToastNotification {
    pub id: u64,
    pub title: String,
    pub message: String,
    pub level: ToastLevel,
    pub created: Instant,
    pub duration_secs: f32,
}

/// Manages and renders toast notifications
#[derive(Default)]
pub struct ToastNotificationManager {
    toasts: Vec<ToastNotification>,
    counter: u64,
}

impl ToastNotificationManager {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            counter: 0,
        }
    }

    pub fn push(&mut self, title: impl Into<String>, message: impl Into<String>, level: ToastLevel) {
        self.counter += 1;
        self.toasts.push(ToastNotification {
            id: self.counter,
            title: title.into(),
            message: message.into(),
            level,
            created: Instant::now(),
            duration_secs: 4.0,
        });
    }

    pub fn render(&mut self, ctx: &egui::Context, theme: HudTheme) {
        let now = Instant::now();
        self.toasts.retain(|t| now.duration_since(t.created).as_secs_f32() < t.duration_secs);

        if self.toasts.is_empty() {
            return;
        }

        let screen_rect = ctx.content_rect();
        let mut y_offset = screen_rect.max.y - 45.0;

        for toast in self.toasts.iter().rev() {
            let age_secs = now.duration_since(toast.created).as_secs_f32();
            let alpha = if age_secs > (toast.duration_secs - 0.5) {
                ((toast.duration_secs - age_secs) / 0.5).clamp(0.0, 1.0)
            } else if age_secs < 0.25 {
                (age_secs / 0.25).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let (accent_color, icon) = match toast.level {
                ToastLevel::Info => (Color32::from_rgb(56, 139, 253), "ℹ️"),
                ToastLevel::Success => (Color32::from_rgb(63, 185, 80), "✅"),
                ToastLevel::Warning => (Color32::from_rgb(210, 153, 34), "⚠️"),
                ToastLevel::Error => (Color32::from_rgb(248, 81, 73), "❌"),
            };

            let toast_rect = egui::Rect::from_min_size(
                Pos2::new(screen_rect.max.x - 360.0, y_offset - 64.0),
                Vec2::new(340.0, 58.0),
            );

            let bg = Color32::from_rgba_unmultiplied(
                theme.card_bg().r(),
                theme.card_bg().g(),
                theme.card_bg().b(),
                (240.0 * alpha) as u8,
            );
            let border = Color32::from_rgba_unmultiplied(
                accent_color.r(),
                accent_color.g(),
                accent_color.b(),
                (220.0 * alpha) as u8,
            );

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new(format!("toast_{}", toast.id)),
            ));

            painter.rect(
                toast_rect,
                CornerRadius::same(6),
                bg,
                Stroke::new(1.2, border),
                egui::StrokeKind::Outside,
            );

            let text_color = Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * alpha) as u8);
            let sub_color = Color32::from_rgba_unmultiplied(180, 190, 205, (220.0 * alpha) as u8);

            painter.text(
                toast_rect.min + Vec2::new(10.0, 8.0),
                egui::Align2::LEFT_TOP,
                format!("{} {}", icon, toast.title),
                egui::FontId::proportional(13.0),
                text_color,
            );

            painter.text(
                toast_rect.min + Vec2::new(10.0, 28.0),
                egui::Align2::LEFT_TOP,
                &toast.message,
                egui::FontId::proportional(11.0),
                sub_color,
            );

            y_offset -= 68.0;
        }
    }
}
