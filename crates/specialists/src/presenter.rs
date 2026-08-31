//! presenter.rs
//! Presenter (The Visionary / Experience Designer) & DisplayBuffer (Optical Telemetry & HUD Streamer).
//! Domain Opcode: 0x0300 (UI_PRESENTATION)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::traits::{DomainSubEngine, MnlpPacket, MnlpResponse, SovereignSpecialist, SpecialistHealth};

/// UI Presentation frame streamed to BusVisualizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPresentationFrame {
    pub active_view: String,
    pub theme: String,
    pub cognitive_load_pct: f32,
    pub active_specialist: String,
    pub status_message: String,
}

/// DisplayBufferEngine: Optical HUD Telemetry and visual streamer sub-engine
#[derive(Debug, Clone)]
pub struct DisplayBufferEngine {
    pub frames_streamed: u64,
    pub hud_layers_active: usize,
}

/// Backwards-compatible alias
pub type DisplayBufferRelic = DisplayBufferEngine;

impl Default for DisplayBufferEngine {
    fn default() -> Self {
        Self {
            frames_streamed: 0,
            hud_layers_active: 3,
        }
    }
}

impl DomainSubEngine for DisplayBufferEngine {
    fn engine_name(&self) -> &'static str {
        "DisplayBuffer"
    }

    fn supervisor_name(&self) -> &'static str {
        "Presenter"
    }

    fn engine_status(&self) -> String {
        format!(
            "DisplayBuffer Optical Streamer: {} frames delivered across {} HUD layers",
            self.frames_streamed, self.hud_layers_active
        )
    }
}

/// Presenter Specialist
pub struct PresenterSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub display_buffer: DisplayBufferEngine,
    pub glass: DisplayBufferEngine,
    pub omni_engine: std::sync::Arc<omni::OmniEngine>,
}

impl Default for PresenterSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenterSpecialist {
    pub fn new() -> Self {
        let omni_engine = std::sync::Arc::new(omni::OmniEngine::default());
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            display_buffer: DisplayBufferEngine::default(),
            glass: DisplayBufferEngine::default(),
            omni_engine,
        }
    }

    /// Composes a UI frame for frontend rendering
    pub fn compose_ui_frame(&mut self, active_view: &str, status: &str) -> UiPresentationFrame {
        self.display_buffer.frames_streamed += 1;
        self.glass.frames_streamed = self.display_buffer.frames_streamed;
        info!(target: "specialist::presenter", %active_view, "Composing visual UI presentation frame");

        UiPresentationFrame {
            active_view: active_view.to_string(),
            theme: "Aaroneous_Obsidian_Cyan".to_string(),
            cognitive_load_pct: 24.5,
            active_specialist: "Presenter".to_string(),
            status_message: status.to_string(),
        }
    }

    /// Ingests workspace topology and specialist federation into the 3D Omni Galaxy
    pub async fn populate_omni_galaxy(&self) -> Result<(usize, usize)> {
        let spec_count = self.omni_engine.ingest_standard_specialists().await;
        let crate_count = self.omni_engine.ingest_workspace_crates(&[
            "ipc_bus", "compute", "evolution", "biology",
            "orchestrator", "adaptation_engine", "desktop_emulator", "specialists",
            "paths", "transpiler", "omni", "hypervisor"
        ]).await;

        self.omni_engine.step_gravitational_physics(0.1).await;
        Ok((spec_count, crate_count))
    }

    /// Computes an adaptive layout recommendation based on operator interaction heatmap
    pub fn compute_adaptive_layout(&self, heatmap: &InteractionHeatmap) -> serde_json::Value {
        serde_json::json!({
            "primary_view": heatmap.primary_focus_view,
            "total_interactions": heatmap.total_interactions,
            "recommended_scale": if heatmap.total_interactions > 100 { 1.1 } else { 1.0 },
            "high_density_mode": heatmap.total_interactions > 50,
        })
    }
}

/// Operator Interaction Heatmap and Layout Optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionHeatmap {
    pub view_focus_counts: std::collections::HashMap<String, u64>,
    pub total_interactions: u64,
    pub primary_focus_view: String,
}

impl InteractionHeatmap {
    pub fn record_focus(&mut self, view: &str) {
        *self.view_focus_counts.entry(view.to_string()).or_insert(0) += 1;
        self.total_interactions += 1;
        if let Some((top_view, _)) = self.view_focus_counts.iter().max_by_key(|(_, &c)| c) {
            self.primary_focus_view = top_view.clone();
        }
    }
}

#[async_trait]
impl SovereignSpecialist for PresenterSpecialist {
    fn name(&self) -> &'static str {
        "Presenter"
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
                message: format!("Presenter delivered 3D Omni Galaxy snapshot ({} stars, {} galaxies)", snapshot.total_stars, snapshot.total_galaxies),
                payload,
            });
        }

        let frame = self.compose_ui_frame(&req_str, "System Operational");
        let payload = serde_json::to_vec(&frame)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!("Presenter composed UI frame for '{}'", req_str),
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
    fn test_presenter_frame_composition() {
        let mut presenter = PresenterSpecialist::new();
        let frame = presenter.compose_ui_frame("OmniGalaxyView", "Exploring sector 4");
        assert_eq!(frame.active_view, "OmniGalaxyView");
        assert_eq!(presenter.glass.frames_streamed, 1);
    }

    #[test]
    fn test_interaction_heatmap_adaptive_layout() {
        let presenter = PresenterSpecialist::new();
        let mut heatmap = InteractionHeatmap::default();
        heatmap.record_focus("GalaxyMap3D");
        heatmap.record_focus("GalaxyMap3D");
        heatmap.record_focus("ScreenAutomation");

        assert_eq!(heatmap.primary_focus_view, "GalaxyMap3D");
        assert_eq!(heatmap.total_interactions, 3);

        let layout = presenter.compute_adaptive_layout(&heatmap);
        assert_eq!(layout["primary_view"], "GalaxyMap3D");
        assert_eq!(layout["total_interactions"], 3);
    }
}
