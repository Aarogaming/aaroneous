//! In-process Rust plugin contracts for a single compiled dependency graph.
//! Descriptors are control-plane data and may allocate. This trait is not a
//! stable dynamic-library ABI: do not pass its trait objects across DLL boundaries.
//! Dynamic entry points, version negotiation and ownership require a separate ABI.
#![deny(unsafe_code)]

use anyhow::Result;
use ipc_bus::MachinePacket;
use serde::{Deserialize, Serialize};

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

/// Contract for in-process plugins built with the host.
pub trait Plugin: Send + Sync {
    /// Return the descriptor describing the plugin.
    fn descriptor(&self) -> PluginDescriptor;
    /// Handle an incoming IPC packet.
    fn handle(&self, msg: MachinePacket) -> Result<()>;
}
