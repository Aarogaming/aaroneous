//! # Aaroneous Rust SDK
//!
//! High-performance client library and `SynapseBridge` for interacting with the
//! Aaroneous synthetic intelligence runtime, specialist federation, and machine-native IPC bus.
//!
//! ## Example
//! ```rust
//! use aaroneous_sdk::SynapseBridge;
//!
//! let bridge = SynapseBridge::connect();
//! println!("Connected to federation with {} active channels", bridge.channel_count());
//! ```

pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;

pub mod dynamic_plugin;

pub use dynamic_plugin::{
    CreateSpecialistFn, DynamicSpecialistLoader, GetManifestFn, SpecialistEngine,
    SpecialistPluginManifest, SPECIALIST_ABI_VERSION,
};

pub use ipc_bus::{
    MachinePacket, PersistentGrimoireStore, PersistentWalStore, SpecialistSynapseBus,
    SpmcSynapseBus, SynapsePacket, WalRecord,
};
use std::sync::Arc;

/// The Rust SDK for Aaroneous.
/// Provides high-level wrappers for the Machine-Native IPC / Synapse Bus.
pub struct SynapseBridge {
    bus: Arc<SpecialistSynapseBus>,
}

impl SynapseBridge {
    /// Connects to or initializes the Specialist IPC / Synapse Federation Bus.
    pub fn connect() -> Self {
        Self {
            bus: Arc::new(SpecialistSynapseBus::new_federation()),
        }
    }

    /// Returns a reference to the inner SpecialistSynapseBus.
    pub fn bus(&self) -> &Arc<SpecialistSynapseBus> {
        &self.bus
    }

    /// Number of active channels in the federation.
    pub fn channel_count(&self) -> usize {
        self.bus.channels.len()
    }
}

/// Typed event bus client bridge for hypervisor and plugin communication
pub struct EventBusBridge<T: Clone> {
    bus: ipc_bus::UniversalEventBus<T>,
}

impl<T: Clone> Default for EventBusBridge<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> EventBusBridge<T> {
    pub fn new() -> Self {
        Self {
            bus: ipc_bus::UniversalEventBus::new(),
        }
    }

    pub fn publish(&self, topic: &str, payload: T) -> u64 {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);
        self.bus.publish(topic, payload, ts)
    }

    pub fn subscribe(&self, topic: &str, subscriber_id: &str) -> ipc_bus::EventSubscriber<T> {
        self.bus.subscribe(topic, subscriber_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synapse_bridge_connect() {
        let bridge = SynapseBridge::connect();
        assert_eq!(bridge.channel_count(), 11);
    }

    #[test]
    fn test_sdk_reexports_nervous_system() {
        let _ = nervous_system::SharedMemorySynapse::new_sync("test_sdk", 1024 * 1024);
    }

    #[test]
    fn test_sdk_wal_store_reexport() {
        let temp_dir = std::env::temp_dir().join("sdk_wal_test.db");
        let mut store = PersistentWalStore::open(&temp_dir).unwrap();
        store.put("sdk_key", b"sdk_value").unwrap();
        assert_eq!(store.get("sdk_key"), Some(b"sdk_value".as_slice()));
        let _ = std::fs::remove_file(&temp_dir);
    }

    #[test]
    fn test_event_bus_bridge() {
        let bridge = EventBusBridge::<String>::new();
        let sub = bridge.subscribe("sensor.temp", "worker_1");
        let seq = bridge.publish("sensor.temp", "24.5C".to_string());
        assert_eq!(seq, 0);

        let evt = sub.try_recv().unwrap();
        assert_eq!(evt.payload, "24.5C");
    }
}
