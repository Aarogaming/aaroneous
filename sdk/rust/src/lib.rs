pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;

use ipc_bus::SpecialistSynapseBus;
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
}
