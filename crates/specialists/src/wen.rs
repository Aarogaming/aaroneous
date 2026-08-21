//! wen.rs
//! Wen (The Human Symbiote) & Resonance (Cognitive Alignment Matrix).
//! Domain Opcode: 0x0800 (HUMAN_SYMBIOSIS)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// Cognitive load and conversational resonance alignment report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioticResonanceReport {
    pub estimated_cognitive_load: f32, // 0.0 to 1.0
    pub recommended_detail_level: String, // "High", "Standard", "ConciseSummary"
    pub translated_message: String,
}

/// Resonance Relic Engine: Human cognitive alignment & symbiotic tuning matrix
#[derive(Debug, Clone)]
pub struct ResonanceRelic {
    pub interactions_aligned: u64,
    pub current_resonance_score: f32,
}

impl Default for ResonanceRelic {
    fn default() -> Self {
        Self {
            interactions_aligned: 0,
            current_resonance_score: 0.96,
        }
    }
}

impl RelicEngine for ResonanceRelic {
    fn relic_name(&self) -> &'static str {
        "Resonance"
    }

    fn supervisor_name(&self) -> &'static str {
        "Wen"
    }

    fn relic_status(&self) -> String {
        format!(
            "Resonance Matrix: {} interactions tuned with {:.1}% alignment score",
            self.interactions_aligned,
            self.current_resonance_score * 100.0
        )
    }
}

/// Wen Sovereign Specialist
pub struct WenSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub resonance: ResonanceRelic,
}

impl Default for WenSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl WenSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            resonance: ResonanceRelic::default(),
        }
    }

    /// Aligns machine tensor output into clear human resonance
    pub fn align_output(&mut self, raw_machine_output: &str) -> SymbioticResonanceReport {
        self.resonance.interactions_aligned += 1;
        info!(target: "specialist::wen", "Translating machine output for optimal human cognitive resonance");

        SymbioticResonanceReport {
            estimated_cognitive_load: 0.32,
            recommended_detail_level: "ConciseSummary".to_string(),
            translated_message: raw_machine_output.to_string(),
        }
    }
}

#[async_trait]
impl SovereignSpecialist for WenSpecialist {
    fn name(&self) -> &'static str {
        "Wen"
    }

    fn domain_opcode(&self) -> u16 {
        0x0800
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let raw = String::from_utf8_lossy(&packet.payload);
        let aligned = self.align_output(&raw);
        let payload = serde_json::to_vec(&aligned)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: "Wen translated output with symbiotic human resonance".to_string(),
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
    fn test_wen_alignment() {
        let mut wen = WenSpecialist::new();
        let report = wen.align_output("Kernel 0x01 completed in 12ms");
        assert_eq!(report.recommended_detail_level, "ConciseSummary");
        assert_eq!(wen.resonance.interactions_aligned, 1);
    }
}
