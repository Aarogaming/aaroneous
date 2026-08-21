//! Aura UI Manifest for the HMI system
//! Defines the visual design system for the user interface

use serde::{Deserialize, Serialize};

/// Color scheme for the Aura UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraColorScheme {
    /// Primary background color
    pub background: String,
    /// Secondary background color
    pub secondary_background: String,
    /// Text color
    pub text: String,
    /// Accent color for interactive elements
    pub accent: String,
    /// Highlight color
    pub highlight: String,
}

/// Font configuration for the Aura UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraFontConfig {
    /// Primary font family
    pub primary_font: String,
    /// Font sizes for different UI elements
    pub font_sizes: FontSizes,
    /// Font weights for different text styles
    pub font_weights: FontWeights,
}

/// Font sizes for different UI elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSizes {
    pub tiny: String,
    pub small: String,
    pub medium: String,
    pub large: String,
    pub huge: String,
}

/// Font weights for different text styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontWeights {
    pub light: u16,
    pub regular: u16,
    pub medium: u16,
    pub semibold: u16,
    pub bold: u16,
}

/// Aura UI Design System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraUIDesignSystem {
    /// Color scheme
    pub colors: AuraColorScheme,
    /// Font configuration
    pub fonts: AuraFontConfig,
    /// Spacing configuration
    pub spacing: SpacingConfig,
    /// Component styles
    pub components: ComponentStyles,
}

/// Spacing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingConfig {
    pub xs: String,
    pub s: String,
    pub m: String,
    pub l: String,
    pub xl: String,
}

/// Component styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStyles {
    pub button: ButtonStyles,
    pub input: InputStyles,
    pub card: CardStyles,
    pub modal: ModalStyles,
}

/// Button styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonStyles {
    pub border_radius: String,
    pub padding: String,
    pub font_size: String,
}

/// Input styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputStyles {
    pub border_radius: String,
    pub padding: String,
    pub font_size: String,
}

/// Card styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardStyles {
    pub border_radius: String,
    pub padding: String,
    pub shadow: String,
}

/// Modal styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalStyles {
    pub border_radius: String,
    pub padding: String,
    pub backdrop_filter: String,
}

/// Default Aura UI manifest
pub fn default_aura_ui_manifest() -> AuraUIDesignSystem {
    AuraUIDesignSystem {
        colors: AuraColorScheme {
            background: "#2C2E33".to_string(),
            secondary_background: "#3A3D42".to_string(),
            text: "#E2E4E6".to_string(),
            accent: "#4CAF50".to_string(), // Green for positive actions
            highlight: "#2196F3".to_string(), // Blue for highlights
        },
        fonts: AuraFontConfig {
            primary_font: "Inter".to_string(),
            font_sizes: FontSizes {
                tiny: "12px".to_string(),
                small: "14px".to_string(),
                medium: "16px".to_string(),
                large: "18px".to_string(),
                huge: "24px".to_string(),
            },
            font_weights: FontWeights {
                light: 300,
                regular: 400,
                medium: 500,
                semibold: 600,
                bold: 700,
            },
        },
        spacing: SpacingConfig {
            xs: "4px".to_string(),
            s: "8px".to_string(),
            m: "16px".to_string(),
            l: "24px".to_string(),
            xl: "32px".to_string(),
        },
        components: ComponentStyles {
            button: ButtonStyles {
                border_radius: "8px".to_string(),
                padding: "12px 24px".to_string(),
                font_size: "16px".to_string(),
            },
            input: InputStyles {
                border_radius: "8px".to_string(),
                padding: "12px".to_string(),
                font_size: "16px".to_string(),
            },
            card: CardStyles {
                border_radius: "12px".to_string(),
                padding: "20px".to_string(),
                shadow: "0 4px 6px rgba(0, 0, 0, 0.1)".to_string(),
            },
            modal: ModalStyles {
                border_radius: "16px".to_string(),
                padding: "24px".to_string(),
                backdrop_filter: "blur(10px)".to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_aura_ui_manifest() {
        let manifest = default_aura_ui_manifest();
        assert_eq!(manifest.colors.background, "#2C2E33");
        assert_eq!(manifest.colors.text, "#E2E4E6");
        assert_eq!(manifest.fonts.primary_font, "Inter");
    }
}
