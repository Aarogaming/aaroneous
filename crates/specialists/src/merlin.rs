//! merlin.rs
//! Merlin (The Seer / Knowledge Synthesist) & Grimoire (Citation & Research Vault).
//! Domain Opcode: 0x0200 (KNOWLEDGE_SYNTHESIS)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// Synthesized research knowledge item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSynthesis {
    pub topic: String,
    pub summary: String,
    pub citations: Vec<String>,
    pub confidence_score: f32,
}

/// Grimoire Relic Engine: Semantic research vault and citation index
#[derive(Debug, Clone)]
pub struct GrimoireRelic {
    pub indexed_documents: usize,
    pub citation_graph: HashMap<String, Vec<String>>,
}

impl Default for GrimoireRelic {
    fn default() -> Self {
        Self {
            indexed_documents: 0,
            citation_graph: HashMap::new(),
        }
    }
}

impl RelicEngine for GrimoireRelic {
    fn relic_name(&self) -> &'static str {
        "Grimoire"
    }

    fn supervisor_name(&self) -> &'static str {
        "Merlin"
    }

    fn relic_status(&self) -> String {
        format!(
            "Grimoire Vault: {} documents indexed, {} citation nodes",
            self.indexed_documents,
            self.citation_graph.len()
        )
    }
}

/// Merlin Sovereign Specialist
pub struct MerlinSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub knowledge_cache: HashMap<String, KnowledgeSynthesis>,
    pub grimoire: GrimoireRelic,
}

impl Default for MerlinSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl MerlinSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            knowledge_cache: HashMap::new(),
            grimoire: GrimoireRelic::default(),
        }
    }

    /// Synthesize knowledge for a topic
    pub fn synthesize(&mut self, query: &str) -> KnowledgeSynthesis {
        info!(target: "specialist::merlin", %query, "Synthesizing research intelligence");

        let synthesis = KnowledgeSynthesis {
            topic: query.to_string(),
            summary: format!("Synthesized structural patterns and operational specifications for '{}'", query),
            citations: vec![
                "dev/docs/06_MACHINE_NATIVE_LINKING_PROTOCOL.md".to_string(),
                "dev/docs/11_OMNI_GALAXY_DATA_NAVIGATION_SPEC.md".to_string(),
            ],
            confidence_score: 0.98,
        };

        self.grimoire.indexed_documents += 1;
        self.grimoire.citation_graph.insert(query.to_string(), synthesis.citations.clone());
        self.knowledge_cache.insert(query.to_string(), synthesis.clone());

        synthesis
    }
}

#[async_trait]
impl SovereignSpecialist for MerlinSpecialist {
    fn name(&self) -> &'static str {
        "Merlin"
    }

    fn domain_opcode(&self) -> u16 {
        0x0200
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let query = String::from_utf8_lossy(&packet.payload);
        let synthesis = self.synthesize(&query);
        let payload = serde_json::to_vec(&synthesis)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!("Merlin synthesized knowledge for '{}'", query),
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
            backlog_count: self.knowledge_cache.len(),
            is_dormant: self.tokens < 1.0,
            last_active: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merlin_synthesis() {
        let mut merlin = MerlinSpecialist::new();
        let k = merlin.synthesize("Machine-Native Linking");
        assert_eq!(k.confidence_score, 0.98);
        assert_eq!(merlin.grimoire.indexed_documents, 1);
    }
}
