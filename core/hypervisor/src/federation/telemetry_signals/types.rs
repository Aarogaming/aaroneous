/// Shared types for the biometric module
///
/// These types are used regardless of whether the `biometric-ble` feature is
/// enabled, so the Symbiotic specialist has a stable API.

use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

/// Errors that can occur during biometric operations
#[derive(Debug, thiserror::Error)]
pub enum BleError {
    #[error("BLE error: {0}")]
    Ble(String),

    #[error("No BLE adapter found on this system")]
    NoAdapter,

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Service not supported: {0}")]
    UnsupportedService(String),

    #[error("Operation timed out after {0:?}")]
    Timeout(Duration),

    #[error("BLE feature not enabled (compile with --features biometric-ble)")]
    FeatureNotEnabled,

    #[error("Permission denied (Bluetooth permissions on macOS, capability on Linux): {0}")]
    PermissionDenied(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

/// A device discovered during BLE scan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BiometricDevice {
    /// Platform-specific device identifier (MAC on Linux, UUID on macOS, etc.)
    pub id: String,
    /// Human-readable name from advertisement (may be empty)
    pub name: String,
    /// Received signal strength indicator (dBm). None if unknown.
    pub rssi: Option<i16>,
    /// List of advertised service UUIDs (standard GATT or custom)
    pub services: Vec<String>,
    /// Whether the device is currently connected
    pub connected: bool,
}

impl BiometricDevice {
    /// Convenience: does this device advertise the standard Heart Rate Service?
    pub fn supports_heart_rate(&self) -> bool {
        self.services.iter().any(|s| {
            // 0x180D is the standard HR service UUID
            // Full UUID form: 0000180d-0000-1000-8000-00805f9b34fb
            s.contains("180d") || s.contains("180D")
        })
    }

    /// Convenience: does this device advertise the standard Battery Service?
    pub fn has_battery_service(&self) -> bool {
        self.services.iter().any(|s| {
            s.contains("180f") || s.contains("180F")
        })
    }
}

/// What kind of biometric reading this is
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BiometricKind {
    /// Beats per minute
    HeartRate,
    /// Heart rate variability in milliseconds (RR interval std dev)
    HeartRateVariability,
    /// Battery level 0-100%
    BatteryLevel,
    /// Skin temperature in Celsius
    SkinTemperature,
    /// Activity / step count delta
    StepDelta,
    /// SpO2 percentage 0-100
    OxygenSaturation,
    /// Generic numeric reading
    Generic,
}

/// A single biometric reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricSample {
    /// Timestamp (Unix seconds) when the sample was received
    pub timestamp: u64,
    /// Source device ID
    pub device_id: String,
    /// What kind of reading
    pub kind: BiometricKind,
    /// Numeric value (units depend on `kind`)
    pub value: f64,
    /// Optional secondary value (e.g., RR interval list, raw bytes)
    pub raw_payload: Option<Vec<u8>>,
}

impl BiometricSample {
    pub fn heart_rate(device_id: String, bpm: u16) -> Self {
        Self {
            timestamp: now_secs(),
            device_id,
            kind: BiometricKind::HeartRate,
            value: bpm as f64,
            raw_payload: None,
        }
    }

    pub fn battery(device_id: String, percent: u8) -> Self {
        Self {
            timestamp: now_secs(),
            device_id,
            kind: BiometricKind::BatteryLevel,
            value: percent as f64,
            raw_payload: None,
        }
    }
}

/// Filter for BLE scans
#[derive(Debug, Clone, Default)]
pub struct DeviceFilter {
    /// Only return devices whose name contains one of these substrings
    pub name_contains: Vec<String>,
    /// Only return devices advertising at least one of these service UUIDs
    pub required_services: Vec<String>,
    /// Minimum RSSI (e.g., -70 to filter out distant devices)
    pub min_rssi: Option<i16>,
}

impl DeviceFilter {
    /// Filter that matches only heart rate monitors
    pub fn heart_rate_monitors() -> Self {
        Self {
            name_contains: vec![],
            required_services: vec!["0000180d-0000-1000-8000-00805f9b34fb".to_string()],
            min_rssi: None,
        }
    }

    /// Apply this filter to a device
    pub fn matches(&self, device: &BiometricDevice) -> bool {
        if !self.name_contains.is_empty()
            && !self.name_contains.iter().any(|s| device.name.contains(s))
        {
            return false;
        }

        if !self.required_services.is_empty() {
            let has_service = self.required_services.iter().any(|required| {
                device.services.iter().any(|s| s.eq_ignore_ascii_case(required))
            });
            if !has_service {
                return false;
            }
        }

        if let Some(min) = self.min_rssi {
            match device.rssi {
                Some(actual) if actual >= min => {}
                _ => return false,
            }
        }

        true
    }
}

/// Stream of biometric samples (returned by subscribe operations)
pub type BiometricStream = Pin<Box<dyn futures_core::Stream<Item = BiometricSample> + Send>>;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biometric_sample_heart_rate() {
        let sample = BiometricSample::heart_rate("dev1".to_string(), 72);
        assert_eq!(sample.kind, BiometricKind::HeartRate);
        assert_eq!(sample.value, 72.0);
        assert_eq!(sample.device_id, "dev1");
        assert!(sample.raw_payload.is_none());
    }

    #[test]
    fn test_biometric_sample_battery() {
        let sample = BiometricSample::battery("dev1".to_string(), 85);
        assert_eq!(sample.kind, BiometricKind::BatteryLevel);
        assert_eq!(sample.value, 85.0);
    }

    #[test]
    fn test_device_supports_heart_rate() {
        let device = BiometricDevice {
            id: "dev1".to_string(),
            name: "HRMonitor".to_string(),
            rssi: Some(-50),
            services: vec!["0000180d-0000-1000-8000-00805f9b34fb".to_string()],
            connected: false,
        };
        assert!(device.supports_heart_rate());
    }

    #[test]
    fn test_device_has_battery_service() {
        let device = BiometricDevice {
            id: "dev1".to_string(),
            name: "HRMonitor".to_string(),
            rssi: Some(-50),
            services: vec!["0000180f-0000-1000-8000-00805f9b34fb".to_string()],
            connected: false,
        };
        assert!(device.has_battery_service());
        assert!(!device.supports_heart_rate());
    }

    #[test]
    fn test_filter_name_contains() {
        let filter = DeviceFilter {
            name_contains: vec!["Polar".to_string()],
            ..Default::default()
        };
        let polar = BiometricDevice {
            id: "1".to_string(),
            name: "Polar H10".to_string(),
            rssi: None,
            services: vec![],
            connected: false,
        };
        let other = BiometricDevice {
            id: "2".to_string(),
            name: "Random Device".to_string(),
            rssi: None,
            services: vec![],
            connected: false,
        };
        assert!(filter.matches(&polar));
        assert!(!filter.matches(&other));
    }

    #[test]
    fn test_filter_required_services() {
        let filter = DeviceFilter::heart_rate_monitors();
        let hr_device = BiometricDevice {
            id: "1".to_string(),
            name: "HRM".to_string(),
            rssi: None,
            services: vec!["0000180D-0000-1000-8000-00805F9B34FB".to_string()],
            connected: false,
        };
        let other = BiometricDevice {
            id: "2".to_string(),
            name: "Other".to_string(),
            rssi: None,
            services: vec!["00001234-0000-1000-8000-00805f9b34fb".to_string()],
            connected: false,
        };
        assert!(filter.matches(&hr_device), "should match case-insensitively");
        assert!(!filter.matches(&other));
    }

    #[test]
    fn test_filter_min_rssi() {
        let filter = DeviceFilter {
            min_rssi: Some(-60),
            ..Default::default()
        };
        let close = BiometricDevice {
            id: "1".to_string(),
            name: "x".to_string(),
            rssi: Some(-50),
            services: vec![],
            connected: false,
        };
        let far = BiometricDevice {
            id: "2".to_string(),
            name: "x".to_string(),
            rssi: Some(-80),
            services: vec![],
            connected: false,
        };
        let unknown = BiometricDevice {
            id: "3".to_string(),
            name: "x".to_string(),
            rssi: None,
            services: vec![],
            connected: false,
        };
        assert!(filter.matches(&close));
        assert!(!filter.matches(&far));
        assert!(!filter.matches(&unknown), "Unknown RSSI should fail min_rssi filter");
    }

    #[test]
    fn test_ble_error_display() {
        let e = BleError::Timeout(Duration::from_secs(5));
        assert!(e.to_string().contains("5s") || e.to_string().contains("5"));

        let e = BleError::FeatureNotEnabled;
        assert!(e.to_string().contains("biometric-ble"));
    }
}
