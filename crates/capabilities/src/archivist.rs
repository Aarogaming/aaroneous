//! archivist.rs
//! Archivist (The Memory Keeper / Chronicler) & MemoryIndex (3D Galaxy Semantic Data Access Engine).
//! Powered directly by Omni.
//! Domain Opcode: 0x0600 (MEMORY_CONSOLIDATION)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tracing::info;

use omni::{OmniEngine, SpatialCoord, StarNode, StarNodeType};
use crate::traits::{DomainSubEngine, MnlpPacket, MnlpResponse, SovereignSpecialist, SpecialistHealth};

/// MemoryIndexEngine: 3D Galaxy semantic data access and indexing sub-engine
pub struct MemoryIndexEngine {
    pub omni_engine: Arc<OmniEngine>,
}

/// Backwards-compatible alias
pub type MemoryIndexRelic = MemoryIndexEngine;

impl DomainSubEngine for MemoryIndexEngine {
    fn engine_name(&self) -> &'static str {
        "MemoryIndex"
    }

    fn supervisor_name(&self) -> &'static str {
        "Archivist"
    }

    fn engine_status(&self) -> String {
        "MemoryIndex 3D Galaxy Engine: Online and clustering star-nodes".to_string()
    }
}

/// Archivist Specialist
pub struct ArchivistSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub omni_engine: Arc<OmniEngine>,
    pub memory_index: MemoryIndexEngine,
    pub relic: MemoryIndexEngine,
    pub neurochemistry: evolution::NeurochemicalHomeostasisEngine,
}

impl Default for ArchivistSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchivistSpecialist {
    pub fn new() -> Self {
        let omni = Arc::new(OmniEngine::default());
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            omni_engine: omni.clone(),
            memory_index: MemoryIndexEngine { omni_engine: omni.clone() },
            relic: MemoryIndexEngine { omni_engine: omni },
            neurochemistry: evolution::NeurochemicalHomeostasisEngine::default(),
        }
    }

    /// Consolidates an episodic experience into a permanent 3D star-node
    pub async fn consolidate_memory(&self, node_id: &str, title: &str, domain: &str, payload_uri: &str) -> StarNode {
        info!(target: "specialist::archivist", %node_id, %title, "Consolidating lived experience into Omni Galaxy star-node");

        let star = StarNode::new(
            node_id,
            title,
            StarNodeType::Memory,
            domain,
            SpatialCoord::new(0.0, 0.0, 500.0), // Active memory
            payload_uri,
        );

        self.omni_engine.insert_node(star.clone()).await;
        star
    }
}

#[async_trait]
impl SovereignSpecialist for ArchivistSpecialist {
    fn name(&self) -> &'static str {
        "Archivist"
    }

    fn domain_opcode(&self) -> u16 {
        0x0600
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let payload_str = String::from_utf8_lossy(&packet.payload);
        
        if payload_str.starts_with("drive:") || payload_str.starts_with("neurochemistry:") {
            let impulses = self.neurochemistry.evaluate_autonomic_impulses();
            let payload = serde_json::to_vec(&impulses)?;

            return Ok(MnlpResponse {
                success: true,
                opcode: self.domain_opcode(),
                correlation_id: packet.correlation_id,
                message: format!("Archivist evaluated {} autonomic impulses from neurochemical state", impulses.len()),
                payload,
            });
        }

        let star = self.consolidate_memory("mem_auto", &payload_str, "Cognition", "omni://memory/auto").await;
        let payload = serde_json::to_vec(&star)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!("Archivist consolidated memory into star-node '{}'", payload_str),
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
    async fn test_archivist_memory_consolidation() {
        let archivist = ArchivistSpecialist::new();
        let star = archivist.consolidate_memory("test_mem", "Adapter Success", "Fabrication", "omni://test").await;
        assert_eq!(star.id, "test_mem");

        let snapshot = archivist.omni_engine.export_snapshot().await.unwrap();
        assert_eq!(snapshot.total_stars, 1);
    }
}
