//! kami.rs
//! Kami (The Sensory Threshold) & Threshold (Spatial-Kinetic Perception & HID Bridge).
//! Powered directly by Marionette.
//! Domain Opcode: 0x0900 (SPATIAL_SENSORY)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tracing::info;

use marionette::{HidCommand, MarionetteEngine, VisualObservation};
use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// Threshold Relic Engine wrapper for Kami
#[derive(Debug, Clone)]
pub struct ThresholdRelicWrapper {
    pub frames_processed: usize,
    pub avg_compute_savings_pct: f32,
}

impl Default for ThresholdRelicWrapper {
    fn default() -> Self {
        Self {
            frames_processed: 0,
            avg_compute_savings_pct: 0.0,
        }
    }
}

impl RelicEngine for ThresholdRelicWrapper {
    fn relic_name(&self) -> &'static str {
        "Threshold"
    }

    fn supervisor_name(&self) -> &'static str {
        "Kami"
    }

    fn relic_status(&self) -> String {
        format!(
            "Threshold Epigenetic Gating: {} frames processed ({:.1}% avg compute saved)",
            self.frames_processed, self.avg_compute_savings_pct
        )
    }
}

/// Kami Sovereign Specialist
pub struct KamiSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub marionette: Arc<MarionetteEngine>,
    pub relic: ThresholdRelicWrapper,
}

impl Default for KamiSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl KamiSpecialist {
    pub fn new() -> Self {
        let engine = Arc::new(MarionetteEngine::new_mock());
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            marionette: engine,
            relic: ThresholdRelicWrapper::default(),
        }
    }

    /// Captures a raw spatial visual frame from the threshold
    pub async fn capture_frame(&self) -> Result<VisualObservation> {
        info!(target: "specialist::kami", "Capturing spatial visual frame across the physical-digital threshold");
        self.marionette.pull_visual_perception().await
    }

    /// Captures a spatial visual frame gated by the 16x16 epigenetic motion saliency matrix
    pub async fn capture_epigenetic_gated_frame(&mut self) -> Result<(VisualObservation, marionette::EpigeneticGatingResult)> {
        info!(target: "specialist::kami", "Capturing epigenetic gated visual perception across the threshold");
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
impl SovereignSpecialist for KamiSpecialist {
    fn name(&self) -> &'static str {
        "Kami"
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
                "Kami captured epigenetic frame via Marionette ({} active sectors, {:.1}% compute saved in {}µs)",
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
    async fn test_kami_perception() {
        let kami = KamiSpecialist::new();
        let frame = kami.capture_frame().await.unwrap();
        assert_eq!(frame.grid.len(), 128 * 128);
    }
}
