//! crates/ipc_bus/src/synapse.rs
//! High-speed zero-copy sensory and motor neural state IPC primitives.

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct UniversalSensoryState {
    /// Flattened 128x128 grid processing active game interface or Win32 OS layout
    pub spatial_matrix_grid: Vec<f32>,
    /// Real-time environmental rewards (e.g. scoring trends, pixel variance metrics, health indicators)
    pub global_reward_telemetry: Vec<f32>,
}

impl UniversalSensoryState {
    pub const DEFAULT_GRID_DIM: usize = 128;
    pub const TOTAL_CELLS: usize = Self::DEFAULT_GRID_DIM * Self::DEFAULT_GRID_DIM;

    pub fn new_empty(grid_cells: usize) -> Self {
        Self {
            spatial_matrix_grid: vec![0.0; grid_cells],
            global_reward_telemetry: Vec::new(),
        }
    }

    pub fn from_grid(grid: Vec<f32>, rewards: Vec<f32>) -> Self {
        Self {
            spatial_matrix_grid: grid,
            global_reward_telemetry: rewards,
        }
    }

    /// Retrieves pixel at (x, y) given the square dimension
    pub fn get_pixel(&self, x: usize, y: usize, dim: usize) -> Option<f32> {
        let idx = y * dim + x;
        self.spatial_matrix_grid.get(idx).copied()
    }

    /// Sets pixel value at (x, y) given the square dimension
    pub fn set_pixel(&mut self, x: usize, y: usize, dim: usize, val: f32) -> bool {
        let idx = y * dim + x;
        if idx < self.spatial_matrix_grid.len() {
            self.spatial_matrix_grid[idx] = val;
            true
        } else {
            false
        }
    }

    /// Mean luminance / activation across the spatial grid
    pub fn mean_luminance(&self) -> f32 {
        if self.spatial_matrix_grid.is_empty() {
            return 0.0;
        }
        self.spatial_matrix_grid.iter().sum::<f32>() / self.spatial_matrix_grid.len() as f32
    }

    /// Spatial activation variance across the grid
    pub fn spatial_variance(&self) -> f32 {
        if self.spatial_matrix_grid.is_empty() {
            return 0.0;
        }
        let mean = self.mean_luminance();
        self.spatial_matrix_grid
            .iter()
            .map(|&x| (x - mean) * (x - mean))
            .sum::<f32>()
            / self.spatial_matrix_grid.len() as f32
    }

    /// Mean of reward telemetry indicators
    pub fn mean_reward(&self) -> f32 {
        if self.global_reward_telemetry.is_empty() {
            return 0.0;
        }
        self.global_reward_telemetry.iter().sum::<f32>() / self.global_reward_telemetry.len() as f32
    }

    /// Downsamples any square grid into an 8x8 latent observation feature vector
    pub fn downsample_to_8x8(&self, source_dim: usize) -> [f32; 64] {
        let mut out = [0.0f32; 64];
        if source_dim < 8 || self.spatial_matrix_grid.len() < source_dim * source_dim {
            return out;
        }

        let block_size = source_dim / 8;
        let block_area = (block_size * block_size) as f32;

        for out_y in 0..8 {
            for out_x in 0..8 {
                let mut sum = 0.0f32;
                for by in 0..block_size {
                    for bx in 0..block_size {
                        let sx = out_x * block_size + bx;
                        let sy = out_y * block_size + by;
                        sum += self.spatial_matrix_grid[sy * source_dim + sx];
                    }
                }
                out[out_y * 8 + out_x] = sum / block_area;
            }
        }
        out
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct UniversalMotorIntent {
    pub delta_x: f32,
    pub delta_y: f32,
    /// Universal 64-bit flag tracking keyboard and absolute click states
    pub binary_action_register: u64,
}

impl UniversalMotorIntent {
    pub const MOUSE_LEFT_DOWN: u64   = 1 << 0;
    pub const MOUSE_RIGHT_DOWN: u64  = 1 << 1;
    pub const MOUSE_MIDDLE_DOWN: u64 = 1 << 2;
    pub const KEY_SHIFT: u64         = 1 << 3;
    pub const KEY_CTRL: u64          = 1 << 4;
    pub const KEY_ALT: u64           = 1 << 5;
    pub const KEY_ENTER: u64         = 1 << 6;
    pub const KEY_ESCAPE: u64        = 1 << 7;
    pub const KEY_SPACE: u64         = 1 << 8;

    pub fn new(delta_x: f32, delta_y: f32) -> Self {
        Self {
            delta_x,
            delta_y,
            binary_action_register: 0,
        }
    }

    pub fn with_flag(mut self, flag: u64) -> Self {
        self.binary_action_register |= flag;
        self
    }

    pub fn without_flag(mut self, flag: u64) -> Self {
        self.binary_action_register &= !flag;
        self
    }

    pub fn has_flag(&self, flag: u64) -> bool {
        (self.binary_action_register & flag) != 0
    }

    /// Displacement magnitude (hypotenuse)
    pub fn magnitude(&self) -> f32 {
        (self.delta_x * self.delta_x + self.delta_y * self.delta_y).sqrt()
    }

    /// Clamps movement delta within maximum radial speed limit
    pub fn clamp_delta(&mut self, max_radius: f32) {
        let mag = self.magnitude();
        if mag > max_radius && mag > 1e-6 {
            let scale = max_radius / mag;
            self.delta_x *= scale;
            self.delta_y *= scale;
        }
    }

    /// True if there is no movement and no buttons active
    pub fn is_idle(&self) -> bool {
        self.delta_x.abs() < 1e-6 && self.delta_y.abs() < 1e-6 && self.binary_action_register == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensory_metrics_and_downsample() {
        let mut state = UniversalSensoryState::new_empty(16 * 16);
        assert_eq!(state.mean_luminance(), 0.0);

        // Set high activation on top left quadrant
        for y in 0..8 {
            for x in 0..8 {
                state.set_pixel(x, y, 16, 1.0);
            }
        }

        assert_eq!(state.get_pixel(0, 0, 16), Some(1.0));
        assert_eq!(state.get_pixel(10, 10, 16), Some(0.0));
        assert!(state.mean_luminance() > 0.0);
        assert!(state.spatial_variance() > 0.0);

        let downsampled = state.downsample_to_8x8(16);
        assert_eq!(downsampled.len(), 64);
        assert_eq!(downsampled[0], 1.0); // (0, 0) block is active
        assert_eq!(downsampled[63], 0.0); // (7, 7) block is inactive
    }

    #[test]
    fn test_motor_intent_flags_and_clamping() {
        let mut intent = UniversalMotorIntent::new(30.0, 40.0)
            .with_flag(UniversalMotorIntent::MOUSE_LEFT_DOWN);

        assert_eq!(intent.magnitude(), 50.0);
        assert!(intent.has_flag(UniversalMotorIntent::MOUSE_LEFT_DOWN));
        assert!(!intent.has_flag(UniversalMotorIntent::KEY_SHIFT));

        intent.clamp_delta(10.0);
        assert!((intent.magnitude() - 10.0).abs() < 1e-4);
        assert_eq!(intent.delta_x, 6.0);
        assert_eq!(intent.delta_y, 8.0);
    }
}
