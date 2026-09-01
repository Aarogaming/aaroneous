// core/hypervisor/src/hud/theme/mod.rs
//! High-Contrast Professional Themes for the Aaroneous Desktop Studio.

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

/// High-Contrast Professional Themes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudTheme {
    CobaltDark,     // Dark slate with electric cobalt blue accents
    ObsidianSlate,  // Minimalist deep carbon with crisp silver & ice blue
    EmeraldMatrix,  // Terminal slate with vivid emerald green accents
    AmberSovereign, // Dark charcoal with radiant gold & amber accents
}

impl HudTheme {
    pub fn name(&self) -> &'static str {
        match self {
            HudTheme::CobaltDark => "⚡ Cobalt Dark",
            HudTheme::ObsidianSlate => "🌑 Obsidian Slate",
            HudTheme::EmeraldMatrix => "📟 Emerald Matrix",
            HudTheme::AmberSovereign => "👑 Amber Sovereign",
        }
    }

    pub fn bg_color(&self) -> Color32 {
        match self {
            HudTheme::CobaltDark => Color32::from_rgb(13, 17, 23),
            HudTheme::ObsidianSlate => Color32::from_rgb(10, 10, 12),
            HudTheme::EmeraldMatrix => Color32::from_rgb(10, 16, 13),
            HudTheme::AmberSovereign => Color32::from_rgb(18, 16, 14),
        }
    }

    pub fn panel_bg(&self) -> Color32 {
        match self {
            HudTheme::CobaltDark => Color32::from_rgb(22, 27, 34),
            HudTheme::ObsidianSlate => Color32::from_rgb(20, 20, 24),
            HudTheme::EmeraldMatrix => Color32::from_rgb(16, 26, 20),
            HudTheme::AmberSovereign => Color32::from_rgb(28, 24, 20),
        }
    }

    pub fn card_bg(&self) -> Color32 {
        match self {
            HudTheme::CobaltDark => Color32::from_rgb(30, 36, 46),
            HudTheme::ObsidianSlate => Color32::from_rgb(28, 28, 34),
            HudTheme::EmeraldMatrix => Color32::from_rgb(22, 36, 28),
            HudTheme::AmberSovereign => Color32::from_rgb(38, 32, 26),
        }
    }

    pub fn accent(&self) -> Color32 {
        match self {
            HudTheme::CobaltDark => Color32::from_rgb(56, 139, 253),     // Electric Cobalt
            HudTheme::ObsidianSlate => Color32::from_rgb(121, 192, 255),  // Ice Blue
            HudTheme::EmeraldMatrix => Color32::from_rgb(63, 185, 80),    // Emerald
            HudTheme::AmberSovereign => Color32::from_rgb(210, 153, 34),  // Amber Gold
        }
    }

    pub fn border_color(&self) -> Color32 {
        match self {
            HudTheme::CobaltDark => Color32::from_rgb(48, 54, 61),
            HudTheme::ObsidianSlate => Color32::from_rgb(45, 45, 55),
            HudTheme::EmeraldMatrix => Color32::from_rgb(35, 60, 45),
            HudTheme::AmberSovereign => Color32::from_rgb(60, 50, 40),
        }
    }
}
