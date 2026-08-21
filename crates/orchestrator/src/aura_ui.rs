//! Aura UI implementation for the Linguistic Transducer

/// Aura UI manifest structure
#[derive(Debug, Clone)]
pub struct AuraUiManifest {
    /// Color scheme for the UI
    pub colors: AuraColors,
    /// Font settings for the UI
    pub font: AuraFont,
    /// Layout configuration
    pub layout: AuraLayout,
}

/// Color scheme for Aura UI
#[derive(Debug, Clone)]
pub struct AuraColors {
    /// Background color
    pub background: String,
    /// Text color
    pub text: String,
}

/// Font settings for Aura UI
#[derive(Debug, Clone)]
pub struct AuraFont {
    /// Font family
    pub family: String,
    /// Font size
    pub size: u32,
}

/// Layout configuration for Aura UI
#[derive(Debug, Clone)]
pub struct AuraLayout {
    /// Width of the UI
    pub width: u32,
    /// Height of the UI
    pub height: u32,
    /// Orientation
    pub orientation: String,
}

impl AuraUiManifest {
    /// Creates a new Aura UI manifest with default settings
    pub fn new() -> Self {
        Self {
            colors: AuraColors {
                background: "#2C2E33".to_string(),
                text: "#E2E4E6".to_string(),
            },
            font: AuraFont {
                family: "Inter".to_string(),
                size: 16,
            },
            layout: AuraLayout {
                width: 1920,
                height: 1080,
                orientation: "landscape".to_string(),
            },
        }
    }

    /// Creates an Aura UI manifest with custom settings
    pub fn with_colors_and_font(background: &str, text: &str, font_family: &str) -> Self {
        Self {
            colors: AuraColors {
                background: background.to_string(),
                text: text.to_string(),
            },
            font: AuraFont {
                family: font_family.to_string(),
                size: 16,
            },
            layout: AuraLayout {
                width: 1920,
                height: 1080,
                orientation: "landscape".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_aura_ui() {
        let aura_ui = AuraUiManifest::new();
        assert_eq!(aura_ui.colors.background, "#2C2E33");
        assert_eq!(aura_ui.colors.text, "#E2E4E6");
        assert_eq!(aura_ui.font.family, "Inter");
    }

    #[test]
    fn test_custom_aura_ui() {
        let aura_ui = AuraUiManifest::with_colors_and_font("#000000", "#FFFFFF", "Arial");
        assert_eq!(aura_ui.colors.background, "#000000");
        assert_eq!(aura_ui.colors.text, "#FFFFFF");
        assert_eq!(aura_ui.font.family, "Arial");
    }
}