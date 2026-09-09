// Plugin API for Aaroneous hypervisor

use anyhow::Result;
use serde::{Deserialize, Serialize};
use ipc_bus::MachinePacket;

/// Minimal representation of a capability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub name: String,
    // Additional metadata can be added as needed.
}

/// Description of a plugin, including provided and required capabilities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub name: String,
    pub provides: Vec<CapabilityDescriptor>,
    pub requires: Vec<CapabilityDescriptor>,
}

/// Trait that every dynamically loaded plugin must implement.
pub trait Plugin: Send + Sync {
    /// Return the descriptor describing the plugin.
    fn descriptor(&self) -> PluginDescriptor;
    /// Handle an incoming IPC packet.
    fn handle(&self, msg: MachinePacket) -> Result<()>;
}
