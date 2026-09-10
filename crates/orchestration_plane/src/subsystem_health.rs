use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct SubsystemHealthReport { pub cpu: f32, pub memory: f32, pub disk: f32 }
impl Default for SubsystemHealthReport { fn default() -> Self { Self { cpu: 0.0, memory: 0.0, disk: 0.0 } } }
