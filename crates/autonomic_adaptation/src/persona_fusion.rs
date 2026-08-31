//! crates/evolution/src/persona_fusion.rs
//! 5-Layer Persona Vector Extraction & Autonomous Skill Fusion Engine
//! inspired by Llama.cpp, Transformer Attention Extraction, and Persona Rank Evolution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Layer 1: Functional Specialist Persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistPersonaLayer {
    pub domain_name: String,
    pub reasoning_capacity: f32, // 0.0 to 1.0
    pub context_window: usize,
    pub primary_skills: Vec<String>,
}

/// Layer 2: Narrative Voice & Style Persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeVoiceLayer {
    pub voice_archetype: String, // e.g. "Oracle", "Architect", "Sentinel", "Rebel"
    pub vocabulary_richness: f32,
    pub conciseness_bias: f32,
    pub formality_level: f32,
}

/// Layer 3: Psychological Personality Persona (Big-5 OCEAN Model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityPersonaLayer {
    pub openness: f32,          // Curiosity / Exploration
    pub conscientiousness: f32, // Rigor / Verification
    pub extraversion: f32,      // Broadcast / Proactivity
    pub agreeableness: f32,     // Consensus / Alignment
    pub neuroticism: f32,       // Risk sensitivity / Paranoia
}

/// Layer 4: Inter-Specialist Relational Persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalPersonaLayer {
    pub trust_affinities: HashMap<String, f32>, // Specialist name -> Trust (0.0 to 1.0)
    pub leadership_deference: f32,
}

/// Layer 5: Historical Experience Persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperiencePersonaLayer {
    pub total_tasks_digested: usize,
    pub successful_adaptations: usize,
    pub domain_embeddings: Vec<Vec<f32>>,
}

/// Complete 5-Layer Composite Persona / Compound Agent Profile Vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositePersonaVector {
    pub specialist_id: String,
    pub persona_rank: String, // "Rank E", "Rank D", "Rank C", "Rank B", "Rank A", "Rank S", "Ultimate"
    pub functional: SpecialistPersonaLayer,
    pub narrative: NarrativeVoiceLayer,
    pub personality: PersonalityPersonaLayer,
    pub relational: RelationalPersonaLayer,
    pub experience: ExperiencePersonaLayer,
}

pub type CompoundAgentProfile = CompositePersonaVector;
pub type OlympianPersonaVector = CompositePersonaVector;

/// An Emergent Fused Skill created from synthesizing two foundational skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedEmergentSkill {
    pub skill_id: String,
    pub parent_skills: (String, String),
    pub synergy_multiplier: f32,
    pub power_rating: f32,
    pub rank_requirement: String,
}

/// Persona Fusion and Skill Synthesis Engine (Capability Fusion)
pub struct PersonaFusionEngine;

pub type CapabilityFusionEngine = PersonaFusionEngine;

impl PersonaFusionEngine {
    /// Constructs a standard 5-Layer Persona Vector for a Federated Specialist
    pub fn build_specialist_persona(
        specialist_id: &str,
        domain: &str,
        voice: &str,
        openness: f32,
        conscientiousness: f32,
    ) -> CompositePersonaVector {
        CompositePersonaVector {
            specialist_id: specialist_id.to_string(),
            persona_rank: "Rank S".to_string(),
            functional: SpecialistPersonaLayer {
                domain_name: domain.to_string(),
                reasoning_capacity: 0.95,
                context_window: 131072,
                primary_skills: vec!["autonomous_reasoning".to_string()],
            },
            narrative: NarrativeVoiceLayer {
                voice_archetype: voice.to_string(),
                vocabulary_richness: 0.9,
                conciseness_bias: 0.8,
                formality_level: 0.85,
            },
            personality: PersonalityPersonaLayer {
                openness,
                conscientiousness,
                extraversion: 0.75,
                agreeableness: 0.85,
                neuroticism: 0.15,
            },
            relational: RelationalPersonaLayer {
                trust_affinities: HashMap::new(),
                leadership_deference: 0.9,
            },
            experience: ExperiencePersonaLayer {
                total_tasks_digested: 0,
                successful_adaptations: 0,
                domain_embeddings: Vec::new(),
            },
        }
    }

    /// Legacy method forwarding to `build_specialist_persona`
    #[inline]
    pub fn build_olympian_persona(
        specialist_id: &str,
        domain: &str,
        voice: &str,
        openness: f32,
        conscientiousness: f32,
    ) -> CompositePersonaVector {
        Self::build_specialist_persona(specialist_id, domain, voice, openness, conscientiousness)
    }

    /// Fuses two complementary skills into an Emergent Fused Skill
    pub fn fuse_skills(
        skill_a: &str,
        power_a: f32,
        skill_b: &str,
        power_b: f32,
    ) -> FusedEmergentSkill {
        let synergy = 1.35f32; // 35% Emergent Synergy Bonus
        let fused_power = (power_a + power_b) * synergy;

        let rank = if fused_power > 500.0 {
            "Ultimate"
        } else if fused_power > 250.0 {
            "Rank S"
        } else if fused_power > 100.0 {
            "Rank A"
        } else {
            "Rank B"
        };

        FusedEmergentSkill {
            skill_id: format!("fused_{}_{}", skill_a, skill_b),
            parent_skills: (skill_a.to_string(), skill_b.to_string()),
            synergy_multiplier: synergy,
            power_rating: fused_power,
            rank_requirement: rank.to_string(),
        }
    }

    /// Automatically mines and synthesizes pairwise cooperative synergies across all 9 Sovereign Specialists
    pub fn mine_all_federation_synergies() -> Vec<FusedEmergentSkill> {
        let pairs = [
            ("orchestrator_intent_decomposition", 150.0, "synthesizer_graph_synthesis", 140.0),
            ("fabricator_ast_mutation", 160.0, "sentinel_svdd_guardrail", 155.0),
            ("perceiver_motion_gating", 130.0, "presenter_hud_compositor", 125.0),
            ("router_mesh_routing", 140.0, "archivist_memory_compaction", 135.0),
            ("aligner_chrono_scheduler", 135.0, "fabricator_jit_synthesis", 150.0),
            ("orchestrator_consensus_quorum", 145.0, "router_gossip_broadcast", 130.0),
            ("synthesizer_semantic_search", 135.0, "sentinel_zero_copy_audit", 140.0),
            ("perceiver_spatial_intent", 140.0, "fabricator_native_optimizer", 145.0),
            ("archivist_homeostasis", 130.0, "aligner_temporal_resonance", 125.0),
            ("presenter_oscilloscope", 120.0, "synthesizer_subgraph_traversal", 135.0),
        ];

        pairs.iter()
            .map(|(skill_a, power_a, skill_b, power_b)| {
                Self::fuse_skills(skill_a, *power_a, skill_b, *power_b)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_persona_vector_generation() {
        let persona = PersonaFusionEngine::build_specialist_persona(
            "Fabricator",
            "Fabrication",
            "Architect",
            0.95,
            0.99,
        );
        assert_eq!(persona.specialist_id, "Fabricator");
        assert_eq!(persona.functional.domain_name, "Fabrication");
        assert_eq!(persona.personality.conscientiousness, 0.99);
    }

    #[test]
    fn test_olympian_persona_vector_generation() {
        let persona = PersonaFusionEngine::build_olympian_persona(
            "Fabricator",
            "Fabrication",
            "Architect",
            0.95,
            0.99,
        );

        assert_eq!(persona.specialist_id, "Fabricator");
        assert_eq!(persona.persona_rank, "Rank S");
        assert_eq!(persona.functional.domain_name, "Fabrication");
        assert_eq!(persona.personality.conscientiousness, 0.99);
    }

    #[test]
    fn test_skill_fusion_synergy() {
        let fused = PersonaFusionEngine::fuse_skills("ast_mutation", 120.0, "shadow_sandbox", 100.0);

        assert_eq!(fused.parent_skills.0, "ast_mutation");
        assert_eq!(fused.parent_skills.1, "shadow_sandbox");
        // (120 + 100) * 1.35 = 297.0 -> Rank S
        assert!((fused.power_rating - 297.0).abs() < 1e-4);
        assert_eq!(fused.rank_requirement, "Rank S");
    }

    #[test]
    fn test_mine_all_federation_synergies() {
        let synergies = PersonaFusionEngine::mine_all_federation_synergies();
        assert_eq!(synergies.len(), 10);
        for syn in &synergies {
            assert!(syn.synergy_multiplier >= 1.35);
            assert!(syn.power_rating > 250.0);
            assert!(syn.rank_requirement == "Rank S" || syn.rank_requirement == "Ultimate");
        }
    }
}
