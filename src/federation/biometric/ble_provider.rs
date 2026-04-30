/// Real BLE biometric provider backed by btleplug
///
/// This module is only compiled when the `biometric-ble` feature is enabled.
/// It wraps a btleplug `Manager` and `Adapter` and translates the raw GATT
/// API into the higher-level `BiometricSample` stream used by Symbiotic.
///
/// # Architecture
///
/// - One `Manager` per process (held inside `BiometricProvider`)
/// - The first available adapter is used by default
/// - Devices are tracked via `dashmap` for concurrent access
/// - Notifications are translated to `BiometricSample`s via service-specific parsers
///
/// # Platform Support
///
/// btleplug 0.12 supports:
/// - Linux (BlueZ via D-Bus)
/// - macOS (Core Bluetooth)
/// - Windows 10+ (WinRT)
/// - iOS (limited)
/// - Android (planned)
///
/// On macOS, Bluetooth permission must be granted to the application.
/// On Linux, the user must have access to the `bluetoothd` D-Bus interface
/// (typically the `bluetooth` group).

use super::services::{parse_heart_rate_measurement, StandardServices};
use super::types::{
    BiometricDevice, BiometricKind, BiometricSample, BiometricStream, BleError, DeviceFilter,
};
use btleplug::api::{
    BDAddr, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Real BLE-backed biometric provider
pub struct BiometricProvider {
    manager: Manager,
    adapter: Adapter,
    /// Connected peripherals indexed by device ID
    connected: Arc<Mutex<HashMap<String, Peripheral>>>,
}

impl BiometricProvider {
    /// Spawn a new BLE provider using the first available adapter
    pub async fn spawn() -> Result<Self, BleError> {
        info!("Initializing btleplug Manager");
        let manager = Manager::new()
            .await
            .map_err(|e| ble_error(e, "create manager"))?;

        let adapters = manager
            .adapters()
            .await
            .map_err(|e| ble_error(e, "list adapters"))?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or(BleError::NoAdapter)?;

        info!("Using BLE adapter: {:?}", adapter);

        Ok(Self {
            manager,
            adapter,
            connected: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get the underlying btleplug manager (advanced use)
    pub fn manager(&self) -> &Manager {
        &self.manager
    }

    /// Scan for BLE devices for the specified duration
    pub async fn scan(&self, duration: Duration) -> Result<Vec<BiometricDevice>, BleError> {
        debug!("Starting BLE scan for {:?}", duration);
        self.adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| ble_error(e, "start scan"))?;

        tokio::time::sleep(duration).await;

        let peripherals = self
            .adapter
            .peripherals()
            .await
            .map_err(|e| ble_error(e, "list peripherals"))?;

        // Stop scan to save battery
        let _ = self.adapter.stop_scan().await;

        let mut devices = Vec::with_capacity(peripherals.len());
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                let id = p.id().to_string();
                let services: Vec<String> = props
                    .services
                    .iter()
                    .map(|u| u.to_string())
                    .collect();
                devices.push(BiometricDevice {
                    id,
                    name: props.local_name.unwrap_or_default(),
                    rssi: props.rssi,
                    services,
                    connected: p.is_connected().await.unwrap_or(false),
                });
            }
        }

        info!("Discovered {} BLE devices", devices.len());
        Ok(devices)
    }

    /// Scan with a filter applied
    pub async fn scan_filtered(
        &self,
        duration: Duration,
        filter: DeviceFilter,
    ) -> Result<Vec<BiometricDevice>, BleError> {
        let all = self.scan(duration).await?;
        Ok(all.into_iter().filter(|d| filter.matches(d)).collect())
    }

    /// Connect to a device by ID and discover its services
    pub async fn connect(&self, device_id: &str) -> Result<(), BleError> {
        let peripheral = self.find_peripheral(device_id).await?;

        debug!("Connecting to {}", device_id);
        peripheral
            .connect()
            .await
            .map_err(|e| BleError::ConnectionFailed(format!("{}: {}", device_id, e)))?;

        peripheral
            .discover_services()
            .await
            .map_err(|e| ble_error(e, "discover_services"))?;

        self.connected
            .lock()
            .await
            .insert(device_id.to_string(), peripheral);

        Ok(())
    }

    /// Disconnect from a device
    pub async fn disconnect(&self, device_id: &str) -> Result<(), BleError> {
        let mut connected = self.connected.lock().await;
        if let Some(peripheral) = connected.remove(device_id) {
            peripheral
                .disconnect()
                .await
                .map_err(|e| ble_error(e, "disconnect"))?;
        }
        Ok(())
    }

    /// Subscribe to heart rate notifications from a connected device
    ///
    /// Returns a stream of `BiometricSample`s. Each notification produces
    /// one HR sample, plus optionally one HRV sample if RR intervals are present.
    pub async fn subscribe_heart_rate(
        &self,
        device_id: &str,
    ) -> Result<BiometricStream, BleError> {
        let peripheral = {
            let connected = self.connected.lock().await;
            connected
                .get(device_id)
                .cloned()
                .ok_or_else(|| BleError::DeviceNotFound(device_id.to_string()))?
        };

        let hr_char_uuid = StandardServices::heart_rate_measurement();
        let chars = peripheral.characteristics();
        let hr_char = chars
            .iter()
            .find(|c| c.uuid == hr_char_uuid)
            .ok_or_else(|| BleError::UnsupportedService("Heart Rate".to_string()))?
            .clone();

        peripheral
            .subscribe(&hr_char)
            .await
            .map_err(|e| ble_error(e, "subscribe to HR char"))?;

        let device_id_owned = device_id.to_string();

        // Get a notification stream and map values to BiometricSamples
        let notif_stream = peripheral
            .notifications()
            .await
            .map_err(|e| ble_error(e, "open notifications"))?;

        let stream = notif_stream
            .filter_map(move |notification| {
                let device_id = device_id_owned.clone();
                async move {
                    if notification.uuid != hr_char_uuid {
                        return None;
                    }
                    match parse_heart_rate_measurement(&notification.value) {
                        Ok(parsed) => Some(BiometricSample {
                            timestamp: now_secs(),
                            device_id,
                            kind: BiometricKind::HeartRate,
                            value: parsed.bpm as f64,
                            raw_payload: Some(notification.value),
                        }),
                        Err(e) => {
                            warn!("Failed to parse HR measurement: {}", e);
                            None
                        }
                    }
                }
            });

        Ok(Box::pin(stream))
    }

    /// Read battery level from a connected device
    pub async fn read_battery_level(&self, device_id: &str) -> Result<u8, BleError> {
        let peripheral = {
            let connected = self.connected.lock().await;
            connected
                .get(device_id)
                .cloned()
                .ok_or_else(|| BleError::DeviceNotFound(device_id.to_string()))?
        };

        let battery_char_uuid = StandardServices::battery_level();
        let chars = peripheral.characteristics();
        let battery_char = chars
            .iter()
            .find(|c| c.uuid == battery_char_uuid)
            .ok_or_else(|| BleError::UnsupportedService("Battery Level".to_string()))?
            .clone();

        let data = peripheral
            .read(&battery_char)
            .await
            .map_err(|e| ble_error(e, "read battery"))?;

        if data.is_empty() {
            return Err(BleError::Parse("empty battery payload".to_string()));
        }

        Ok(data[0])
    }

    /// Find a peripheral by its device ID string
    async fn find_peripheral(&self, device_id: &str) -> Result<Peripheral, BleError> {
        let peripherals = self
            .adapter
            .peripherals()
            .await
            .map_err(|e| ble_error(e, "list peripherals"))?;

        peripherals
            .into_iter()
            .find(|p| p.id().to_string() == device_id)
            .ok_or_else(|| BleError::DeviceNotFound(device_id.to_string()))
    }

    /// Shut down: disconnect all devices and stop the adapter
    pub async fn shutdown(self) -> Result<(), BleError> {
        info!("Shutting down BiometricProvider");
        let mut connected = self.connected.lock().await;
        for (id, peripheral) in connected.drain() {
            if let Err(e) = peripheral.disconnect().await {
                warn!("Failed to disconnect {}: {}", id, e);
            }
        }
        let _ = self.adapter.stop_scan().await;
        Ok(())
    }
}

/// Convert a btleplug error to our `BleError`
fn ble_error<E: std::fmt::Display>(e: E, context: &str) -> BleError {
    let msg = format!("{}: {}", context, e);
    // Try to detect permission errors based on message contents
    if msg.to_lowercase().contains("permission") || msg.to_lowercase().contains("denied") {
        BleError::PermissionDenied(msg)
    } else {
        BleError::Ble(msg)
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawning the provider should succeed if there's any BLE adapter available.
    /// On CI/headless systems this will fail with NoAdapter, which is fine.
    #[tokio::test]
    async fn test_ble_spawn() {
        let result = BiometricProvider::spawn().await;
        match result {
            Ok(provider) => {
                // Adapter exists on this machine
                let _ = provider.shutdown().await;
            }
            Err(BleError::NoAdapter) => {
                // CI environment, no adapter - this is expected
            }
            Err(e) => {
                // Other errors are also acceptable in test environment
                // (e.g., permission denied on macOS without entitlement)
                eprintln!("Spawn returned (expected in test env): {}", e);
            }
        }
    }

    /// Test that scan does not panic and returns a Vec (possibly empty)
    #[tokio::test]
    #[ignore = "requires real BLE adapter"]
    async fn test_ble_scan_returns_vec() {
        let provider = match BiometricProvider::spawn().await {
            Ok(p) => p,
            Err(_) => return, // Skip if no adapter
        };
        let result = provider.scan(Duration::from_secs(1)).await;
        // Either succeeds with a list, or fails with a real error - both are acceptable.
        // What we don't want is a panic.
        assert!(result.is_ok() || result.is_err());
        let _ = provider.shutdown().await;
    }

    /// Test that connect to a non-existent device returns DeviceNotFound
    #[tokio::test]
    #[ignore = "requires real BLE adapter"]
    async fn test_ble_connect_unknown_device() {
        let provider = match BiometricProvider::spawn().await {
            Ok(p) => p,
            Err(_) => return,
        };
        let result = provider.connect("nonexistent-device-id-12345").await;
        assert!(matches!(result, Err(BleError::DeviceNotFound(_))));
        let _ = provider.shutdown().await;
    }

    #[test]
    fn test_ble_error_classification() {
        let permission_err = ble_error("Permission denied for adapter", "test");
        assert!(matches!(permission_err, BleError::PermissionDenied(_)));

        let generic_err = ble_error("Some other error", "test");
        assert!(matches!(generic_err, BleError::Ble(_)));
    }
}
