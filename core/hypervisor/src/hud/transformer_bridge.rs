// core/hypervisor/src/hud/transformer_bridge.rs
//! Bidirectional Frontend-Backend Signal Transformer Bridge.
//!
//! Enforces clean architectural separation between presentation and execution:
//! 1. `FrontendCommandSignal`: Serialized intent emitted by the presentation layer
//!    (clicks, intent text, cartridge mounts, emergency stops).
//! 2. `BackendTelemetryFrame`: Immutable execution state emitted by the core engine
//!    (cycle latencies, thermodynamic equilibrium, active modules, response feedback).
//! 3. `FrontendTransformerBridge`: Crossbeam channel pair insulating UI from core runtime.

use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Signals emitted from any presentation layer (Desktop GUI, CLI, Web, Mobile)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrontendCommandSignal {
    TransduceLinguisticIntent { prompt: String },
    MountCartridgePackage { slot_index: usize, file_path: PathBuf },
    SetExecutionDomain { domain_id: u8 },
    EmergencyCutoff,
}

/// Immutable execution telemetry emitted by the backend core engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendTelemetryFrame {
    pub timestamp_us: u64,
    pub free_energy_delta: f32,
    pub cycle_latency_us: u64,
    pub active_modules: Vec<String>,
    pub response_feedback: String,
}

impl Default for BackendTelemetryFrame {
    fn default() -> Self {
        Self {
            timestamp_us: 0,
            free_energy_delta: 0.012,
            cycle_latency_us: 14,
            active_modules: vec!["OpticalPerception".to_string(), "KineticDispatch".to_string()],
            response_feedback: "Aaroneous Core nominal. Signal transformer active.".to_string(),
        }
    }
}

/// The Bidirectional Transformer Pipe
pub struct FrontendTransformerBridge {
    pub cmd_sender: Sender<FrontendCommandSignal>,
    pub cmd_receiver: Receiver<FrontendCommandSignal>,
    pub telemetry_sender: Sender<BackendTelemetryFrame>,
    pub telemetry_receiver: Receiver<BackendTelemetryFrame>,
}

impl Default for FrontendTransformerBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl FrontendTransformerBridge {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (telem_tx, telem_rx) = unbounded();

        Self {
            cmd_sender: cmd_tx,
            cmd_receiver: cmd_rx,
            telemetry_sender: telem_tx,
            telemetry_receiver: telem_rx,
        }
    }

    /// Presentation layer helper: dispatches a command to the core engine
    pub fn dispatch_command(&self, cmd: FrontendCommandSignal) {
        let _ = self.cmd_sender.send(cmd);
    }

    /// Presentation layer helper: drains the latest telemetry frame from the core
    pub fn poll_latest_telemetry(&self) -> Option<BackendTelemetryFrame> {
        let mut latest = None;
        while let Ok(frame) = self.telemetry_receiver.try_recv() {
            latest = Some(frame);
        }
        latest
    }

    /// Backend engine helper: drains queued frontend commands
    pub fn poll_commands(&self) -> Vec<FrontendCommandSignal> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = self.cmd_receiver.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }

    /// Backend engine helper: broadcasts an updated telemetry frame to the presentation layer
    pub fn publish_telemetry(&self, frame: BackendTelemetryFrame) {
        let _ = self.telemetry_sender.send(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_bridge_bidirectional_flow() {
        let bridge = FrontendTransformerBridge::new();

        // 1. Frontend emits command signal
        let cmd = FrontendCommandSignal::TransduceLinguisticIntent {
            prompt: "Format active workspace".to_string(),
        };
        bridge.dispatch_command(cmd);

        // 2. Backend polls command
        let received_cmds = bridge.poll_commands();
        assert_eq!(received_cmds.len(), 1);
        if let FrontendCommandSignal::TransduceLinguisticIntent { prompt } = &received_cmds[0] {
            assert_eq!(prompt, "Format active workspace");
        } else {
            panic!("Expected TransduceLinguisticIntent");
        }

        // 3. Backend emits telemetry frame
        let frame = BackendTelemetryFrame {
            timestamp_us: 1000,
            free_energy_delta: 0.015,
            cycle_latency_us: 18,
            active_modules: vec!["CompilerAST".to_string()],
            response_feedback: "Workspace formatted.".to_string(),
        };
        bridge.publish_telemetry(frame);

        // 4. Frontend polls latest telemetry
        let latest = bridge.poll_latest_telemetry().unwrap();
        assert_eq!(latest.cycle_latency_us, 18);
        assert_eq!(latest.response_feedback, "Workspace formatted.");
    }
}
