use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentCommandToken {
    AdjustDopamine { specialist: String, delta: f64 },
    SetThrottle { factor: f64 },
    TriggerEpigeneticMask { specialist: String },
    ExecutePatch { target: String, patch_data: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTelemetrySnapshot {
    pub timestamp_ms: u128,
    pub active_agents_count: usize,
    pub cpu_load: f64,
    pub gpu_load: f64,
    pub thermal_status: String,
    pub active_loops: usize,
}

pub struct UiBroker {
    tx_telemetry: Sender<UiTelemetrySnapshot>,
    rx_telemetry: Receiver<UiTelemetrySnapshot>,
    tx_commands: Sender<AgentCommandToken>,
    rx_commands: Receiver<AgentCommandToken>,
}

impl UiBroker {
    pub fn new(bound: usize) -> Self {
        let (tx_telemetry, rx_telemetry) = bounded(bound);
        let (tx_commands, rx_commands) = bounded(bound);
        Self {
            tx_telemetry,
            rx_telemetry,
            tx_commands,
            rx_commands,
        }
    }

    pub fn publish_telemetry(&self, snapshot: UiTelemetrySnapshot) -> Result<()> {
        let _ = self.tx_telemetry.try_send(snapshot);
        Ok(())
    }

    pub fn poll_telemetry(&self) -> Option<UiTelemetrySnapshot> {
        self.rx_telemetry.try_recv().ok()
    }

    pub fn send_command(&self, cmd: AgentCommandToken) -> Result<()> {
        self.tx_commands.send(cmd)?;
        Ok(())
    }

    pub fn poll_commands(&self) -> Vec<AgentCommandToken> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = self.rx_commands.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }
}
