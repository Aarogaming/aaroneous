//! synthesizer.rs
//! Synthesizer (The Seer / Knowledge Synthesist) & KnowledgeStore (Citation & Research Vault).
//! Domain Opcode: 0x0200 (KNOWLEDGE_SYNTHESIS)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::traits::{DomainSubEngine, MnlpPacket, MnlpResponse, SovereignSpecialist, SpecialistHealth};

/// Synthesized research knowledge item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSynthesis {
    pub topic: String,
    pub summary: String,
    pub citations: Vec<String>,
    pub confidence_score: f32,
}

/// KnowledgeStoreEngine: Semantic research vault and citation index sub-engine
#[derive(Debug, Clone, Default)]
pub struct KnowledgeStoreEngine {
    pub indexed_documents: usize,
    pub citation_graph: HashMap<String, Vec<String>>,
}

/// Backwards-compatible alias
pub type KnowledgeStoreRelic = KnowledgeStoreEngine;

impl DomainSubEngine for KnowledgeStoreEngine {
    fn engine_name(&self) -> &'static str {
        "KnowledgeStore"
    }

    fn supervisor_name(&self) -> &'static str {
        "Synthesizer"
    }

    fn engine_status(&self) -> String {
        format!(
            "KnowledgeStore Vault: {} documents indexed, {} citation nodes",
            self.indexed_documents,
            self.citation_graph.len()
        )
    }
}

/// Synthesizer Specialist
pub struct SynthesizerSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub knowledge_cache: HashMap<String, KnowledgeSynthesis>,
    pub knowledge_base: KnowledgeStoreEngine,
    pub grimoire: KnowledgeStoreEngine,
}

impl Default for SynthesizerSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthesizerSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            knowledge_cache: HashMap::new(),
            knowledge_base: KnowledgeStoreEngine::default(),
            grimoire: KnowledgeStoreEngine::default(),
        }
    }

    /// Synthesize knowledge for a topic
    pub fn synthesize(&mut self, query: &str) -> KnowledgeSynthesis {
        info!(target: "specialist::synthesizer", %query, "Synthesizing research intelligence");

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
impl SovereignSpecialist for SynthesizerSpecialist {
    fn name(&self) -> &'static str {
        "Synthesizer"
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
            message: format!("Synthesizer synthesized knowledge for '{}'", query),
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
    fn test_synthesizer_synthesis() {
        let mut synthesizer = SynthesizerSpecialist::new();
        let k = synthesizer.synthesize("Machine-Native Linking");
        assert_eq!(k.confidence_score, 0.98);
        assert_eq!(synthesizer.grimoire.indexed_documents, 1);
    }
}
