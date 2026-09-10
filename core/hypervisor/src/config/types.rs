// POD structs for compile‑time JSON transpilation
use bytemuck::{Pod, Zeroable};
use serde::Deserialize;

#[repr(C)]
#[derive(Copy, Clone, Debug, Deserialize, Pod, Zeroable)]
pub struct AgentHabitats {
    // Placeholder fields – adjust to match the actual JSON schema
    pub count: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Deserialize, Pod, Zeroable)]
pub struct SpatialLayout {
    // Placeholder fields – adjust to match the actual JSON schema
    pub width: u16,
    pub height: u16,
    pub reserved: u32,
}
