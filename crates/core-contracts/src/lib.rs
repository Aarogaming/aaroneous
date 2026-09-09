// Core contracts for Aaroneous microkernel

use bytemuck::{Pod, Zeroable};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Hierarchy tier of a component.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HierarchyTier {
    Core = 0,
    System = 1,
    SubordinateModule = 2,
    Extension = 3,
}

/// Execution methodology flags.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMethodology {
    LocalQuantizedModel = 0x01,
    RemoteApiProxy = 0x02,
    HardwareAcceleratedDirect = 0x03,
    DeterministicSimulation = 0x04,
}

bitflags! {
    /// Operational intent bitflags.
    #[derive(Serialize, Deserialize)]
    pub struct OperationalIntent: u16 {
        const RealtimeLowLatency = 0b0000_0000_0000_0001;
        const BatchProcessing = 0b0000_0000_0000_0010;
        const HighThroughput = 0b0000_0000_0000_0100;
        const SecureEnclave = 0b0000_0000_0000_1000;
    }
}

bitflags! {
    /// Capability bitmask for components.
    #[derive(Serialize, Deserialize)]
    pub struct Capability: u64 {
        const INFERENCE_OUT = 0b0000_0001;
        const AUDIO_OUT = 0b0000_0010;
        const VIDEO_OUT = 0b0000_0100;
        const SENSOR_INPUT = 0b0000_1000;
    }
}

/// Zero‑copy IPC header used by transport frames.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct IpcHeader {
    pub dst: u32,
    pub src: u32,
    pub len: u32,
    pub flags: u32,
}

/// Component manifest exchanged during bootstrap.
#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ComponentManifest {
    pub name: [u8; 32],
    pub version: u32,
    pub tier: u8,
    pub capabilities: u64,
    pub methodology: u8,
    pub supported_intents: u16,
    pub priority_weight: u8,
    pub address: u32,
}

/// Query descriptor used for capability routing.
#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct QueryDescriptor {
    pub required_capabilities: u64,
    pub methodology: u8,
    pub intent: u16,
    pub priority: u8,
}

/// Helper to pack version numbers into a single u32 (major.minor.patch).
pub const fn pack_version(major: u8, minor: u8, patch: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (patch as u32)
}

// Unit tests verify struct sizes are pod‑compatible.
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn manifest_size() {
        assert_eq!(size_of::<ComponentManifest>(), 64);
    }

    #[test]
    fn query_size() {
        assert_eq!(size_of::<QueryDescriptor>(), 16);
    }
}
