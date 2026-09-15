//! Aura UI — consolidated re-export from aura_ui_manifest.
//! The canonical design system lives in aura_ui_manifest.rs.
//! This module provides backward-compatible type aliases.

pub use crate::aura_ui_manifest::{
    AuraColorScheme as AuraColors, AuraFontConfig as AuraFont,
    AuraUIDesignSystem as AuraUiManifest, default_aura_ui_manifest as default_aura_ui,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_aura_ui() {
        let aura_ui = default_aura_ui();
        assert_eq!(aura_ui.colors.background, "#2C2E33");
        assert_eq!(aura_ui.colors.text, "#E2E4E6");
        assert_eq!(aura_ui.fonts.primary_font, "Inter");
    }
}
