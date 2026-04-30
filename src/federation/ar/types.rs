/// Shared types for the AR/VR module
///
/// These types are used regardless of whether the `ar-openxr` feature is
/// enabled, so the Phygital specialist has a stable API.

use serde::{Deserialize, Serialize};

/// Errors that can occur during AR/VR operations
#[derive(Debug, thiserror::Error)]
pub enum ArError {
    #[error("OpenXR error: {0}")]
    OpenXr(String),

    #[error("No OpenXR runtime installed on this system")]
    NoRuntime,

    #[error("HMD not connected")]
    NoHmd,

    #[error("Form factor not supported by runtime: {0:?}")]
    UnsupportedFormFactor(FormFactor),

    #[error("Session in wrong state: expected {expected}, got {actual}")]
    InvalidSessionState { expected: String, actual: String },

    #[error("AR feature not enabled (compile with --features ar-openxr)")]
    FeatureNotEnabled,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// What kind of HMD/AR device is being targeted
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormFactor {
    /// Standard VR/AR HMD (Quest, Index, Vive, HoloLens, etc.)
    HeadMountedDisplay,
    /// Smartphone/tablet hand-held AR (ARKit/ARCore)
    HandheldDisplay,
}

/// View configuration: how many views (eyes) the device supports
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViewConfiguration {
    /// Single view (handheld AR, monoscopic)
    Mono,
    /// Two views (stereoscopic VR/AR)
    Stereo,
    /// Quad view (some Varjo headsets - 2 high-res + 2 wide-FoV)
    Quad,
}

/// Information about the connected AR/VR system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArSystemInfo {
    /// Human-readable runtime name (e.g., "Monado", "SteamVR", "Meta Link")
    pub runtime_name: String,
    /// Runtime version
    pub runtime_version: String,
    /// HMD/system name (e.g., "Meta Quest 3", "Valve Index")
    pub system_name: String,
    /// Vendor ID (PCI/USB style)
    pub vendor_id: u32,
    /// What form factor this device is
    pub form_factor: FormFactor,
    /// Supported view configuration
    pub view_configuration: ViewConfiguration,
    /// Whether the device tracks orientation only or full 6-DoF
    pub tracks_position: bool,
    /// Whether the device supports passthrough (mixed reality)
    pub supports_passthrough: bool,
}

impl ArSystemInfo {
    /// Map this OpenXR system to the Phygital `SpatialDevice` enum
    /// based on heuristics on system_name and runtime_name.
    pub fn classify_spatial_device(&self) -> Option<&'static str> {
        let system_lower = self.system_name.to_lowercase();
        let runtime_lower = self.runtime_name.to_lowercase();

        if system_lower.contains("hololens") {
            if system_lower.contains("3") {
                Some("HoloLens3")
            } else {
                Some("HoloLens2")
            }
        } else if system_lower.contains("magic leap") || system_lower.contains("magicleap") {
            Some("MagicLeap")
        } else if system_lower.contains("vision pro") || runtime_lower.contains("visionos") {
            Some("AppleVisionPro")
        } else if system_lower.contains("quest") {
            Some("MetaQuest3")
        } else if runtime_lower.contains("arkit") {
            Some("ARKit")
        } else if runtime_lower.contains("arcore") {
            Some("ARCore")
        } else {
            None
        }
    }
}

/// Lifecycle state of an OpenXR session
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArSessionState {
    /// Session not yet created
    Idle,
    /// Session created but not begun
    Ready,
    /// Session is running and rendering
    Running,
    /// Session is paused (visible but not focused)
    Visible,
    /// Session is focused (user is interacting)
    Focused,
    /// Session is shutting down
    Stopping,
    /// Session has exited
    Exited,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_quest() {
        let info = ArSystemInfo {
            runtime_name: "Meta Link".to_string(),
            runtime_version: "1.0".to_string(),
            system_name: "Meta Quest 3".to_string(),
            vendor_id: 0x2833,
            form_factor: FormFactor::HeadMountedDisplay,
            view_configuration: ViewConfiguration::Stereo,
            tracks_position: true,
            supports_passthrough: true,
        };
        assert_eq!(info.classify_spatial_device(), Some("MetaQuest3"));
    }

    #[test]
    fn test_classify_hololens() {
        let info = ArSystemInfo {
            runtime_name: "Windows Mixed Reality".to_string(),
            runtime_version: "1.0".to_string(),
            system_name: "Microsoft HoloLens 3".to_string(),
            vendor_id: 0,
            form_factor: FormFactor::HeadMountedDisplay,
            view_configuration: ViewConfiguration::Stereo,
            tracks_position: true,
            supports_passthrough: true,
        };
        assert_eq!(info.classify_spatial_device(), Some("HoloLens3"));
    }

    #[test]
    fn test_classify_arkit() {
        let info = ArSystemInfo {
            runtime_name: "ARKit Bridge".to_string(),
            runtime_version: "1.0".to_string(),
            system_name: "iPhone 15 Pro".to_string(),
            vendor_id: 0,
            form_factor: FormFactor::HandheldDisplay,
            view_configuration: ViewConfiguration::Mono,
            tracks_position: true,
            supports_passthrough: true,
        };
        assert_eq!(info.classify_spatial_device(), Some("ARKit"));
    }

    #[test]
    fn test_classify_unknown() {
        let info = ArSystemInfo {
            runtime_name: "Custom Runtime".to_string(),
            runtime_version: "1.0".to_string(),
            system_name: "Generic HMD".to_string(),
            vendor_id: 0,
            form_factor: FormFactor::HeadMountedDisplay,
            view_configuration: ViewConfiguration::Stereo,
            tracks_position: true,
            supports_passthrough: false,
        };
        assert_eq!(info.classify_spatial_device(), None);
    }

    #[test]
    fn test_session_state_progression() {
        // Just exercising the variants for coverage
        let states = [
            ArSessionState::Idle,
            ArSessionState::Ready,
            ArSessionState::Running,
            ArSessionState::Visible,
            ArSessionState::Focused,
            ArSessionState::Stopping,
            ArSessionState::Exited,
        ];
        // Each state should be unique
        for (i, s1) in states.iter().enumerate() {
            for (j, s2) in states.iter().enumerate() {
                if i == j {
                    assert_eq!(s1, s2);
                } else {
                    assert_ne!(s1, s2);
                }
            }
        }
    }

    #[test]
    fn test_ar_error_display() {
        let e = ArError::NoRuntime;
        assert!(e.to_string().contains("OpenXR"));

        let e = ArError::FeatureNotEnabled;
        assert!(e.to_string().contains("ar-openxr"));

        let e = ArError::InvalidSessionState {
            expected: "Running".to_string(),
            actual: "Idle".to_string(),
        };
        assert!(e.to_string().contains("Running"));
        assert!(e.to_string().contains("Idle"));
    }
}
