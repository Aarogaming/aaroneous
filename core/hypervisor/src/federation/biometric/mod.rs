// Biometric stub
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiometricKind {
    HeartRate,
    SkinConductance,
    Temperature,
    Motion,
    HeartRateVariability,
    BatteryLevel,
    SkinTemperature,
    StepDelta,
    OxygenSaturation,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricSample {
    pub kind: BiometricKind,
    pub value: f32,
    pub timestamp: u64,
    pub device_id: String,
    pub raw_payload: Vec<u8>,
}

impl BiometricSample {
    pub fn heart_rate(device_id: String, value: u32) -> Self {
        Self {
            kind: BiometricKind::HeartRate,
            value: value as f32,
            timestamp: 0,
            device_id,
            raw_payload: vec![],
        }
    }
    pub fn battery(device_id: String, level: u32) -> Self {
        Self {
            kind: BiometricKind::BatteryLevel,
            value: level as f32,
            timestamp: 0,
            device_id,
            raw_payload: vec![],
        }
    }
}

pub struct BiometricDevice {
    pub id: String,
    pub kind: BiometricKind,
}

pub struct DeviceFilter;
impl Default for DeviceFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceFilter {
    pub fn new() -> Self {
        Self
    }
    pub fn matches(&self, _device: &BiometricDevice) -> bool {
        true
    }
    pub fn heart_rate_monitors() -> Self {
        Self
    }
}

#[derive(Debug)]
pub enum BleError {
    ConnectionFailed,
    Disconnected,
    Timeout,
    FeatureNotEnabled,
}

#[async_trait]
pub trait BiometricProvider: Send + Sync {
    fn connect(&mut self) -> Result<(), BleError>;
    fn disconnect(&mut self);
    fn read_sample(&self) -> Option<BiometricSample>;

    async fn scan_filtered(
        &self,
        _duration: Duration,
        _filter: &DeviceFilter,
    ) -> Result<Vec<BiometricDevice>, BleError> {
        Err(BleError::FeatureNotEnabled)
    }

    async fn connect_by_id(&self, _device_id: &str) -> Result<(), BleError> {
        Err(BleError::FeatureNotEnabled)
    }

    async fn disconnect_by_id(&self, _device_id: &str) -> Result<(), BleError> {
        Err(BleError::FeatureNotEnabled)
    }
}

pub struct BiometricStream;
impl Default for BiometricStream {
    fn default() -> Self {
        Self::new()
    }
}

impl BiometricStream {
    pub fn new() -> Self {
        Self
    }
}

pub mod services;
pub use services::StandardServices;

#[cfg(feature = "biometric-ble")]
pub mod ble_provider;
