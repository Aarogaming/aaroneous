/// Biometric Module: Real BLE Wearable Integration via btleplug
///
/// This module bridges the Symbiotic specialist's abstract biometric model
/// to real Bluetooth Low Energy (BLE) wearable devices. Supports standard
/// GATT services for heart rate monitors, fitness trackers, and similar
/// wearables (Apple Watch in HR-broadcast mode, Polar, Garmin, Wahoo, etc.).
///
/// # Feature Gating
///
/// Real BLE support is gated behind the `biometric-ble` feature. Without it,
/// a stub provider is used so tests and offline development continue to work.
///
/// # Usage
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use a_run::federation::biometric::BiometricProvider;
///
/// let provider = BiometricProvider::spawn().await?;
/// let devices = provider.scan(std::time::Duration::from_secs(5)).await?;
/// for device in &devices {
///     println!("Found: {} (RSSI: {:?})", device.name, device.rssi);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Architecture
///
/// - `BiometricProvider`: Async-friendly handle around a BLE manager/adapter
/// - `BleError`: Unified error type covering both stub and real backends
/// - `BiometricDevice`: Discovered device summary
/// - `BiometricSample`: A single reading (heart rate, battery, etc.)
/// - `StandardServices`: UUIDs for common BLE GATT services
///
/// When `biometric-ble` is enabled, `BiometricProvider` wraps a real btleplug
/// `Manager` and `Adapter`. Otherwise it's a stub that returns canned data.
pub mod types;
pub mod services;

#[cfg(feature = "biometric-ble")]
pub mod ble_provider;

#[cfg(not(feature = "biometric-ble"))]
pub mod stub_provider;

pub use types::{
    BleError, BiometricDevice, BiometricSample, BiometricKind, DeviceFilter, BiometricStream,
};
pub use services::StandardServices;

#[cfg(feature = "biometric-ble")]
pub use ble_provider::BiometricProvider;

#[cfg(not(feature = "biometric-ble"))]
pub use stub_provider::BiometricProvider;
