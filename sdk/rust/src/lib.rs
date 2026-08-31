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
}
