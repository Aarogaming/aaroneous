/// Stub biometric provider (no-op, used when `biometric-ble` feature is disabled)
///
/// This implementation:
/// - Provides the same API as the real btleplug-backed provider
/// - Records calls for testing
/// - Returns deterministic synthetic data instead of real BLE operations
/// - Allows tests to run without requiring a real BLE adapter

use super::types::{
    BiometricDevice, BiometricKind, BiometricSample, BiometricStream, BleError, DeviceFilter,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Stub biometric provider - records operations but does no real BLE
pub struct BiometricProvider {
    /// Records of operations performed (for test inspection)
    pub call_log: Arc<Mutex<Vec<StubCall>>>,
    /// Optional canned responses for testing
    pub canned_devices: Arc<Mutex<Vec<BiometricDevice>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StubCall {
    Spawn,
    Scan { duration_secs: u64 },
    Connect(String),
    Disconnect(String),
    Subscribe(String),
    ReadBattery(String),
}

impl BiometricProvider {
    /// Create a new stub provider
    pub async fn spawn() -> Result<Self, BleError> {
        Ok(Self {
            call_log: Arc::new(Mutex::new(vec![StubCall::Spawn])),
            canned_devices: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Test helper: pre-load canned device list to be returned by `scan`
    pub async fn set_canned_devices(&self, devices: Vec<BiometricDevice>) {
        *self.canned_devices.lock().await = devices;
    }

    /// Scan for BLE devices (stub: returns canned devices)
    pub async fn scan(&self, duration: Duration) -> Result<Vec<BiometricDevice>, BleError> {
        self.call_log.lock().await.push(StubCall::Scan {
            duration_secs: duration.as_secs(),
        });
        Ok(self.canned_devices.lock().await.clone())
    }

    /// Scan with a filter
    pub async fn scan_filtered(
        &self,
        duration: Duration,
        filter: DeviceFilter,
    ) -> Result<Vec<BiometricDevice>, BleError> {
        let all = self.scan(duration).await?;
        Ok(all.into_iter().filter(|d| filter.matches(d)).collect())
    }

    /// Connect to a device
    pub async fn connect(&self, device_id: &str) -> Result<(), BleError> {
        self.call_log
            .lock()
            .await
            .push(StubCall::Connect(device_id.to_string()));
        Ok(())
    }

    /// Disconnect from a device
    pub async fn disconnect(&self, device_id: &str) -> Result<(), BleError> {
        self.call_log
            .lock()
            .await
            .push(StubCall::Disconnect(device_id.to_string()));
        Ok(())
    }

    /// Subscribe to heart rate notifications.
    ///
    /// Stub: returns a stream that yields one synthetic sample.
    pub async fn subscribe_heart_rate(
        &self,
        device_id: &str,
    ) -> Result<BiometricStream, BleError> {
        self.call_log
            .lock()
            .await
            .push(StubCall::Subscribe(device_id.to_string()));

        // Yield one synthetic HR sample (72 bpm)
        let sample = BiometricSample::heart_rate(device_id.to_string(), 72);
        let stream = futures_util::stream::iter(vec![sample]);
        Ok(Box::pin(stream))
    }

    /// Read battery level
    pub async fn read_battery_level(&self, device_id: &str) -> Result<u8, BleError> {
        self.call_log
            .lock()
            .await
            .push(StubCall::ReadBattery(device_id.to_string()));
        // Stub: return 80% battery
        Ok(80)
    }

    /// Test helper: get a snapshot of call log
    #[cfg(test)]
    pub async fn calls(&self) -> Vec<StubCall> {
        self.call_log.lock().await.clone()
    }

    /// Shutdown the provider
    pub async fn shutdown(self) -> Result<(), BleError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn test_stub_spawn() {
        let provider = BiometricProvider::spawn().await.unwrap();
        let calls = provider.calls().await;
        assert_eq!(calls, vec![StubCall::Spawn]);
    }

    #[tokio::test]
    async fn test_stub_scan_returns_canned() {
        let provider = BiometricProvider::spawn().await.unwrap();
        let device = BiometricDevice {
            id: "dev1".to_string(),
            name: "Polar H10".to_string(),
            rssi: Some(-45),
            services: vec!["0000180d-0000-1000-8000-00805f9b34fb".to_string()],
            connected: false,
        };
        provider.set_canned_devices(vec![device.clone()]).await;

        let scanned = provider.scan(Duration::from_secs(2)).await.unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0], device);
    }

    #[tokio::test]
    async fn test_stub_scan_filtered() {
        let provider = BiometricProvider::spawn().await.unwrap();
        let polar = BiometricDevice {
            id: "1".to_string(),
            name: "Polar H10".to_string(),
            rssi: Some(-45),
            services: vec!["0000180d-0000-1000-8000-00805f9b34fb".to_string()],
            connected: false,
        };
        let other = BiometricDevice {
            id: "2".to_string(),
            name: "SmartLight".to_string(),
            rssi: Some(-50),
            services: vec!["00001234-0000-1000-8000-00805f9b34fb".to_string()],
            connected: false,
        };
        provider
            .set_canned_devices(vec![polar.clone(), other])
            .await;

        let result = provider
            .scan_filtered(Duration::from_secs(1), DeviceFilter::heart_rate_monitors())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], polar);
    }

    #[tokio::test]
    async fn test_stub_connect_disconnect_records_calls() {
        let provider = BiometricProvider::spawn().await.unwrap();
        provider.connect("dev1").await.unwrap();
        provider.disconnect("dev1").await.unwrap();
        let calls = provider.calls().await;
        assert!(calls.iter().any(|c| matches!(c, StubCall::Connect(s) if s == "dev1")));
        assert!(calls.iter().any(|c| matches!(c, StubCall::Disconnect(s) if s == "dev1")));
    }

    #[tokio::test]
    async fn test_stub_read_battery() {
        let provider = BiometricProvider::spawn().await.unwrap();
        let level = provider.read_battery_level("dev1").await.unwrap();
        assert_eq!(level, 80);
    }

    #[tokio::test]
    async fn test_stub_subscribe_yields_sample() {
        let provider = BiometricProvider::spawn().await.unwrap();
        let mut stream = provider.subscribe_heart_rate("dev1").await.unwrap();
        let sample = stream.next().await.unwrap();
        assert_eq!(sample.kind, BiometricKind::HeartRate);
        assert_eq!(sample.value, 72.0);
        assert_eq!(sample.device_id, "dev1");
    }

    #[tokio::test]
    async fn test_stub_shutdown() {
        let provider = BiometricProvider::spawn().await.unwrap();
        let result = provider.shutdown().await;
        assert!(result.is_ok());
    }
}
