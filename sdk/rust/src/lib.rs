pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;

/// The Rust SDK for Aaroneous.
/// Provides high-level wrappers for the Synapse and AgentBus.
pub struct SynapseBridge;

impl SynapseBridge {
    pub fn connect() -> Self {
        println!("[SDK] Connected to Aaroneous Synapse");
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synapse_bridge_connect() {
        let bridge = SynapseBridge::connect();
        drop(bridge);
    }

    #[test]
    fn test_sdk_reexports_nervous_system() {
        let _ = nervous_system::SharedMemorySynapse::new_sync("test_sdk", 1024 * 1024);
    }
}
