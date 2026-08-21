//! ariel.rs
//! Ariel (The Visionary / Experience Designer) & Glass (Optical Telemetry & HUD Streamer).
//! Domain Opcode: 0x0300 (UI_PRESENTATION)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// UI Presentation frame streamed to MaelstromUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPresentationFrame {
    pub active_view: String,
    pub theme: String,
    pub cognitive_load_pct: f32,
    pub active_specialist: String,
    pub status_message: String,
}

/// Glass Relic Engine: Optical HUD Telemetry and visual streamer
#[derive(Debug, Clone)]
pub struct GlassRelic {
    pub frames_streamed: u64,
    pub hud_layers_active: usize,
}

impl Default for GlassRelic {
    fn default() -> Self {
        Self {
            frames_streamed: 0,
            hud_layers_active: 3,
        }
    }
}

impl RelicEngine for GlassRelic {
    fn relic_name(&self) -> &'static str {
        "Glass"
    }

    fn supervisor_name(&self) -> &'static str {
        "Ariel"
    }

    fn relic_status(&self) -> String {
        format!(
            "Glass Optical Streamer: {} frames delivered across {} HUD layers",
            self.frames_streamed, self.hud_layers_active
        )
    }
}

/// Ariel Sovereign Specialist
pub struct ArielSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub glass: GlassRelic,
    pub omni_engine: std::sync::Arc<omni::OmniEngine>,
}

impl Default for ArielSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl ArielSpecialist {
    pub fn new() -> Self {
        let omni_engine = std::sync::Arc::new(omni::OmniEngine::default());
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            glass: GlassRelic::default(),
            omni_engine,
        }
    }

    /// Composes a UI frame for frontend rendering
    pub fn compose_ui_frame(&mut self, active_view: &str, status: &str) -> UiPresentationFrame {
        self.glass.frames_streamed += 1;
        info!(target: "specialist::ariel", %active_view, "Composing visual UI presentation frame");

        UiPresentationFrame {
            active_view: active_view.to_string(),
            theme: "Aaroneous_Obsidian_Cyan".to_string(),
            cognitive_load_pct: 24.5,
            active_specialist: "Ariel".to_string(),
            status_message: status.to_string(),
        }
    }

    /// Ingests workspace topology and specialist federation into the 3D Omni Galaxy
    pub async fn populate_omni_galaxy(&self) -> Result<(usize, usize)> {
        let spec_count = self.omni_engine.ingest_standard_specialists().await;
        let crate_count = self.omni_engine.ingest_workspace_crates(&[
            "nervous_system", "compute", "evolution", "biology",
            "orchestrator", "chimera", "marionette", "specialists",
            "paths", "transpiler", "omni", "a_run"
        ]).await;

        self.omni_engine.step_gravitational_physics(0.1).await;
        Ok((spec_count, crate_count))
    }
}

#[async_trait]
impl SovereignSpecialist for ArielSpecialist {
    fn name(&self) -> &'static str {
        "Ariel"
    }

    fn domain_opcode(&self) -> u16 {
        0x0300
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let req_str = String::from_utf8_lossy(&packet.payload);
        
        if req_str.contains("galaxy") || req_str.contains("snapshot") {
            let _ = self.populate_omni_galaxy().await;
            let snapshot = self.omni_engine.export_snapshot().await?;
            let payload = serde_json::to_vec(&snapshot)?;

            return Ok(MnlpResponse {
                success: true,
                opcode: self.domain_opcode(),
                correlation_id: packet.correlation_id,
                message: format!("Ariel delivered 3D Omni Galaxy snapshot ({} stars, {} galaxies)", snapshot.total_stars, snapshot.total_galaxies),
                payload,
            });
        }

        let frame = self.compose_ui_frame(&req_str, "System Operational");
        let payload = serde_json::to_vec(&frame)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!("Ariel composed UI frame for '{}'", req_str),
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

    #[test]
    fn test_ariel_frame_composition() {
        let mut ariel = ArielSpecialist::new();
        let frame = ariel.compose_ui_frame("OmniGalaxyView", "Exploring sector 4");
        assert_eq!(frame.active_view, "OmniGalaxyView");
        assert_eq!(ariel.glass.frames_streamed, 1);
    }
}
