//! perceiver.rs
//! Perceiver (The Sensory Threshold) & GatekeeperEngine (Spatial-Kinetic Perception & HID Bridge).
//! Powered directly by Desktop Emulator.
//! Domain Opcode: 0x0900 (SPATIAL_SENSORY)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tracing::info;

use desktop_emulator::{HidCommand, DesktopEmulator, VisualObservation};
use crate::traits::{DomainSubEngine, MnlpPacket, MnlpResponse, SovereignSpecialist, SpecialistHealth};

/// PerceptionGateEngine: Spatial-Kinetic perception & HID bridge sub-engine
#[derive(Debug, Clone)]
pub struct PerceptionGateEngine {
    pub frames_processed: usize,
    pub avg_compute_savings_pct: f32,
}

/// Backwards-compatible alias
pub type GatekeeperEngineRelic = PerceptionGateEngine;

impl Default for PerceptionGateEngine {
    fn default() -> Self {
        Self {
            frames_processed: 0,
            avg_compute_savings_pct: 0.0,
        }
    }
}

impl DomainSubEngine for PerceptionGateEngine {
    fn engine_name(&self) -> &'static str {
        "PerceptionGate"
    }

    fn supervisor_name(&self) -> &'static str {
        "Perceiver"
    }

    fn engine_status(&self) -> String {
        format!(
            "PerceptionGate Epigenetic Gating: {} frames processed ({:.1}% avg compute saved)",
            self.frames_processed, self.avg_compute_savings_pct
        )
    }
}

/// Perceiver Specialist
pub struct PerceiverSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub marionette: Arc<DesktopEmulator>,
    pub perception_gate: PerceptionGateEngine,
    pub relic: PerceptionGateEngine,
}

impl Default for PerceiverSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl PerceiverSpecialist {
    pub fn new() -> Self {
        let engine = Arc::new(DesktopEmulator::new_mock());
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            marionette: engine,
            perception_gate: PerceptionGateEngine::default(),
            relic: PerceptionGateEngine::default(),
        }
    }

    /// Captures a raw spatial visual frame from the gatekeeper
    pub async fn capture_frame(&self) -> Result<VisualObservation> {
        info!(target: "specialist::perceiver", "Capturing spatial visual frame across the physical-digital gatekeeper");
        self.marionette.pull_visual_perception().await
    }

    /// Captures a spatial visual frame gated by the 16x16 epigenetic motion saliency matrix
    pub async fn capture_epigenetic_gated_frame(&mut self) -> Result<(VisualObservation, desktop_emulator::EpigeneticGatingResult)> {
        info!(target: "specialist::perceiver", "Capturing epigenetic gated visual perception across the gatekeeper");
        let (obs, result) = self.marionette.pull_epigenetic_perception().await?;

        self.relic.frames_processed += 1;
        let count = self.relic.frames_processed as f32;
        self.relic.avg_compute_savings_pct =
            ((self.relic.avg_compute_savings_pct * (count - 1.0)) + obs.compute_savings_pct) / count;

        Ok((obs, result))
    }

    /// Injects a safe motor action
    pub async fn dispatch_motor_action(&self, cmd: HidCommand) -> Result<()> {
        self.marionette.inject_hid_event(cmd).await
    }
}

#[async_trait]
impl SovereignSpecialist for PerceiverSpecialist {
    fn name(&self) -> &'static str {
        "Perceiver"
    }

    fn domain_opcode(&self) -> u16 {
        0x0900
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let (frame, gating) = self.capture_epigenetic_gated_frame().await?;
        let payload = serde_json::to_vec(&frame)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!(
                "Perceiver captured epigenetic frame via Desktop Emulator ({} active sectors, {:.1}% compute saved in {}µs)",
                gating.active_sectors_count, gating.compute_savings_pct, gating.duration_us
            ),
            payload,
        })
    }

    fn recharge_metabolism(&mut self, tokens: f32) {
        self.tokens = (self.tokens + tokens).min(self.max_tokens);
    }

    fn health_report(&self) -> SpecialistHealth {
        SpecialistHealth {
            name: self.name().to_string(),
            domain_opcode: self.domain_opcode(),
            tokens: self.tokens,
            max_tokens: self.max_tokens,
            backlog_count: 0,
            is_dormant: self.tokens < 1.0,
            last_active: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_perceiver_perception() {
        let perceiver = PerceiverSpecialist::new();
        let frame = perceiver.capture_frame().await.unwrap();
        assert_eq!(frame.grid.len(), 128 * 128);
    }
}
