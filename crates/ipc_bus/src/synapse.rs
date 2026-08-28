use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct UniversalSensoryState {
    // Flattened 128x128 grid processing any active game interface or Win32 OS layout
    pub spatial_matrix_grid: Vec<f32>,
    // Real-time environmental rewards (e.g., scoring trends, pixel variance metrics, health indicators)
    pub global_reward_telemetry: Vec<f32>,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy)]
pub struct UniversalMotorIntent {
    pub delta_x: f32,
    pub delta_y: f32,
    // Universal 64-bit flag tracking keyboard and absolute click states
    pub binary_action_register: u64,
}
