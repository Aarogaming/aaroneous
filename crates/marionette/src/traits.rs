//! traits.rs
//! Universal trait abstractions for frontend user emulation, vision, and backend probing.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Normalized visual observation captured from the screen or mock buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualObservation {
    /// 128x128 normalized float luminance values (0.0 to 1.0)
    pub grid: Vec<f32>,
    /// Frame width in grid units (typically 128)
    pub width: usize,
    /// Frame height in grid units (typically 128)
    pub height: usize,
    /// Timestamp in microsecond UNIX epoch
    pub timestamp_us: u64,
    /// Active sectors count (0 to 256) computed by the epigenetic gate
    #[serde(default = "default_active_sectors")]
    pub active_sectors_count: usize,
    /// Compute savings percentage achieved by epigenetic skipping (0.0% to 100.0%)
    #[serde(default)]
    pub compute_savings_pct: f32,
    /// Time in microseconds spent computing the epigenetic saliency gate
    #[serde(default)]
    pub gating_latency_us: u64,
}

fn default_active_sectors() -> usize {
    256
}

impl VisualObservation {
    pub fn new(grid: Vec<f32>, width: usize, height: usize, timestamp_us: u64) -> Self {
        Self {
            grid,
            width,
            height,
            timestamp_us,
            active_sectors_count: 256,
            compute_savings_pct: 0.0,
            gating_latency_us: 0,
        }
    }
}

/// Action type flags for peripheral execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HidAction {
    MouseMove { delta_x: i32, delta_y: i32 },
    LeftClick,
    RightClick,
    DoubleClick,
    KeyPress { key_code: u16 },
    KeyRelease { key_code: u16 },
    Scroll { delta: i32 },
}

/// Motor intent packet containing desired peripheral actuation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidCommand {
    pub actions: Vec<HidAction>,
    pub sequence_id: u64,
    pub timestamp_us: u64,
}

/// Execution probe trace recorded from target process inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbingTrace {
    pub target_process: String,
    pub event_type: String,
    pub payload: String,
    pub timestamp_us: u64,
}

/// The core asynchronous trait for Marionette host backends
#[async_trait]
pub trait MarionetteHost: Send + Sync {
    /// Ingest a visual frame (128x128 normalized float grid)
    async fn pull_visual_perception(&mut self) -> Result<VisualObservation>;

    /// Ingest a visual frame masked by an epigenetic gate (256 sectors)
    async fn pull_visual_perception_gated(&mut self, gate_mask: &[bool; 256]) -> Result<VisualObservation>;

    /// Submit a motor action command (safe mock logging or guarded live execution)
    async fn inject_hid_event(&mut self, command: HidCommand) -> Result<()>;

    /// Record a backend probe trace event into the datalogger
    async fn log_probe_trace(&mut self, trace: ProbingTrace) -> Result<()>;

    /// Check if live hardware emulation is active and permitted
    fn is_live_emulation_active(&self) -> bool;
}
