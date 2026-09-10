pub const CONFIG_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

pub mod types;
pub mod predictive_models_config;

pub fn agent_habitats() -> &'static types::AgentHabitats {
    let size = std::mem::size_of::<types::AgentHabitats>();
    let bytes = &CONFIG_BIN[0..size];
    bytemuck::from_bytes(bytes)
}

pub fn spatial_layout() -> &'static types::SpatialLayout {
    let offset = std::mem::size_of::<types::AgentHabitats>();
    let size = std::mem::size_of::<types::SpatialLayout>();
    let bytes = &CONFIG_BIN[offset..offset + size];
    bytemuck::from_bytes(bytes)
}
