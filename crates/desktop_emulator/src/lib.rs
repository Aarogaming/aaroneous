//! crates/desktop_emulator
//! Unified frontend user emulation, visual perception, backend probing, and datalogging organ for Aaroneous.

pub mod epigenetic_vision;
pub mod event_recorder;
pub mod game_player;
pub mod mock;
pub mod native_win32;
pub mod probing;
pub mod protocol_bridge;
pub mod sensory_motor_loop;
pub mod traits;
pub mod window_target;

pub use sensory_motor_loop::{SensoryMotorCycleReport, SensoryMotorPipeline};
pub use epigenetic_vision::{
    EpigeneticGatingResult, EpigeneticVisionGater, DEFAULT_DELTA_THRESHOLD,
    DEFAULT_HYSTERESIS_FRAMES, GRID_HEIGHT, GRID_SIZE, GRID_WIDTH, SECTORS_PER_COL,
    SECTORS_PER_ROW, SECTOR_SIZE, TOTAL_SECTORS,
};
pub use event_recorder::{FramebufferAnalyzer, RecordedInputEvent, SessionRecording};
pub use game_player::{AutonomousGameAgent, GamePolicyAction, PlaythroughState};
pub use mock::MockMarionette;
pub use native_win32::NativeWin32Marionette;
pub use probing::ProcessProbeLogger;
pub use protocol_bridge::{MarionetteProtocolBridge, MnlpPerceptionPacket};
pub use traits::{HidAction, HidCommand, MarionetteHost, ProbingTrace, VisualObservation};
pub use window_target::{AudioCaptureModifier, CaptureModifiers, CaptureTarget, DiscoveredWindow, WindowDiscoveryEngine};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The primary Desktop Emulator Engine managing active backend, epigenetic vision gater, and probing datalogger
pub struct DesktopEmulator {
    host: Arc<Mutex<dyn MarionetteHost>>,
    probe_logger: Arc<Mutex<ProcessProbeLogger>>,
    gater: Arc<Mutex<EpigeneticVisionGater>>,
}

impl DesktopEmulator {
    /// Creates the default production DesktopEmulator
    pub fn new_default() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::new_native_win32(false)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::new_mock()
        }
    }

    /// Creates a safe sandboxed DesktopEmulator using Mock backend
    pub fn new_mock() -> Self {
        Self {
            host: Arc::new(Mutex::new(MockMarionette::new())),
            probe_logger: Arc::new(Mutex::new(ProcessProbeLogger::default())),
            gater: Arc::new(Mutex::new(EpigeneticVisionGater::new())),
        }
    }

    /// Creates a live Win32 DesktopEmulator (guarded by safety permit)
    pub fn new_native_win32(allow_live_input: bool) -> Self {
        Self {
            host: Arc::new(Mutex::new(NativeWin32Marionette::new(allow_live_input))),
            probe_logger: Arc::new(Mutex::new(ProcessProbeLogger::default())),
            gater: Arc::new(Mutex::new(EpigeneticVisionGater::new())),
        }
    }

    /// Ingest the next visual frame
    pub async fn pull_visual_perception(&self) -> Result<VisualObservation> {
        let mut host = self.host.lock().await;
        host.pull_visual_perception().await
    }

    /// Ingest the next visual frame through the epigenetic motion saliency gate (zeroing static background)
    pub async fn pull_epigenetic_perception(&self) -> Result<(VisualObservation, EpigeneticGatingResult)> {
        // 1. Raw frame capture
        let raw_obs = self.pull_visual_perception().await?;

        // 2. Compute 16x16 epigenetic saliency mask
        let gating_result = {
            let mut gater = self.gater.lock().await;
            gater.process_frame(&raw_obs.grid)
        };

        // 3. Ingest masked perception
        let mut host = self.host.lock().await;
        let mut gated_obs = host.pull_visual_perception_gated(&gating_result.bool_mask).await?;
        gated_obs.active_sectors_count = gating_result.active_sectors_count;
        gated_obs.compute_savings_pct = gating_result.compute_savings_pct;
        gated_obs.gating_latency_us = gating_result.duration_us;

        Ok((gated_obs, gating_result))
    }

    /// Submit a motor action command
    pub async fn inject_hid_event(&self, command: HidCommand) -> Result<()> {
        let mut host = self.host.lock().await;
        host.inject_hid_event(command).await
    }

    /// Log a probe trace
    pub async fn log_probe_trace(&self, trace: ProbingTrace) -> Result<()> {
        let mut logger = self.probe_logger.lock().await;
        logger.record_trace(trace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_desktop_emulator_mock_lifecycle() {
        let engine = DesktopEmulator::new_mock();
        let frame = engine.pull_visual_perception().await.unwrap();
        assert_eq!(frame.grid.len(), 128 * 128);

        let cmd = HidCommand {
            actions: vec![HidAction::LeftClick],
            sequence_id: 42,
            timestamp_us: 1000,
        };
        engine.inject_hid_event(cmd).await.unwrap();

        let trace = ProbingTrace {
            target_process: "target.exe".to_string(),
            event_type: "open_handle".to_string(),
            payload: "ok".to_string(),
            timestamp_us: 2000,
        };
        engine.log_probe_trace(trace).await.unwrap();
    }
}
