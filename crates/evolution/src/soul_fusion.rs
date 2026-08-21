//! crates/evolution/src/soul_fusion.rs
//! 5-Layer Soul Vector Extraction & Autonomous Skill Fusion Engine
//! inspired by Llama.cpp, Transformer Attention Extraction, and Rimuru Soul Rank Evolution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Layer 1: Functional Specialist Soul
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistSoulLayer {
    pub domain_name: String,
    pub reasoning_capacity: f32, // 0.0 to 1.0
    pub context_window: usize,
    pub primary_skills: Vec<String>,
}

/// Layer 2: Narrative Voice & Style Soul
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeSoulLayer {
    pub voice_archetype: String, // e.g. "Oracle", "Architect", "Sentinel", "Rebel"
    pub vocabulary_richness: f32,
    pub conciseness_bias: f32,
    pub formality_level: f32,
}

/// Layer 3: Psychological Personality Soul (Big-5 OCEAN Model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySoulLayer {
    pub openness: f32,          // Curiosity / Exploration
    pub conscientiousness: f32, // Rigor / Verification
    pub extraversion: f32,      // Broadcast / Proactivity
    pub agreeableness: f32,     // Consensus / Alignment
    pub neuroticism: f32,       // Risk sensitivity / Paranoia
}

/// Layer 4: Inter-Specialist Relational Soul
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalSoulLayer {
    pub trust_affinities: HashMap<String, f32>, // Specialist name -> Trust (0.0 to 1.0)
    pub leadership_deference: f32,
}

/// Layer 5: Historical Experience Soul
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceSoulLayer {
    pub total_tasks_digested: usize,
    pub successful_adaptations: usize,
    pub domain_embeddings: Vec<Vec<f32>>,
}

/// Complete 5-Layer Composite Soul Vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSoulVector {
    pub specialist_id: String,
    pub soul_rank: String, // "Rank E", "Rank D", "Rank C", "Rank B", "Rank A", "Rank S", "Ultimate"
    pub functional: SpecialistSoulLayer,
    pub narrative: NarrativeSoulLayer,
    pub personality: PersonalitySoulLayer,
    pub relational: RelationalSoulLayer,
    pub experience: ExperienceSoulLayer,
}

/// An Emergent Fused Skill created from synthesizing two foundational skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedEmergentSkill {
    pub skill_id: String,
    pub parent_skills: (String, String),
    pub synergy_multiplier: f32,
    pub power_rating: f32,
    pub rank_requirement: String,
}

/// Soul Fusion and Skill Synthesis Engine
pub struct SoulFusionEngine;

impl SoulFusionEngine {
    /// Constructs a standard 5-Layer Soul Vector for a Federated Specialist
    pub fn build_specialist_soul(
        specialist_id: &str,
        domain: &str,
        voice: &str,
        openness: f32,
        conscientiousness: f32,
    ) -> CompositeSoulVector {
        CompositeSoulVector {
            specialist_id: specialist_id.to_string(),
            soul_rank: "Rank S".to_string(),
            functional: SpecialistSoulLayer {
                domain_name: domain.to_string(),
                reasoning_capacity: 0.95,
                context_window: 131072,
                primary_skills: vec!["autonomous_reasoning".to_string()],
            },
            narrative: NarrativeSoulLayer {
                voice_archetype: voice.to_string(),
                vocabulary_richness: 0.9,
                conciseness_bias: 0.8,
                formality_level: 0.85,
            },
            personality: PersonalitySoulLayer {
                openness,
                conscientiousness,
                extraversion: 0.75,
                agreeableness: 0.85,
                neuroticism: 0.15,
            },
            relational: RelationalSoulLayer {
                trust_affinities: HashMap::new(),
                leadership_deference: 0.9,
            },
            experience: ExperienceSoulLayer {
                total_tasks_digested: 0,
                successful_adaptations: 0,
                domain_embeddings: Vec::new(),
            },
        }
    }

    /// Legacy method forwarding to `build_specialist_soul`
    #[inline]
    pub fn build_olympian_soul(
        specialist_id: &str,
        domain: &str,
        voice: &str,
        openness: f32,
        conscientiousness: f32,
    ) -> CompositeSoulVector {
        Self::build_specialist_soul(specialist_id, domain, voice, openness, conscientiousness)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_soul_vector_generation() {
        let soul = SoulFusionEngine::build_specialist_soul(
            "Hephaestus",
            "Fabrication",
            "Architect",
            0.95,
            0.99,
        );
        assert_eq!(soul.specialist_id, "Hephaestus");
        assert_eq!(soul.functional.domain_name, "Fabrication");
        assert_eq!(soul.personality.conscientiousness, 0.99);
    }

    #[test]
    fn test_olympian_soul_vector_generation() {
        let soul = SoulFusionEngine::build_olympian_soul(
            "Hephaestus",
            "Fabrication",
            "Architect",
            0.95,
            0.99,
        );

        assert_eq!(soul.specialist_id, "Hephaestus");
        assert_eq!(soul.soul_rank, "Rank S");
        assert_eq!(soul.functional.domain_name, "Fabrication");
        assert_eq!(soul.personality.conscientiousness, 0.99);
    }

    #[test]
    fn test_skill_fusion_synergy() {
        let fused = SoulFusionEngine::fuse_skills("ast_mutation", 120.0, "shadow_sandbox", 100.0);

        assert_eq!(fused.parent_skills.0, "ast_mutation");
        assert_eq!(fused.parent_skills.1, "shadow_sandbox");
        // (120 + 100) * 1.35 = 297.0 -> Rank S
        assert!((fused.power_rating - 297.0).abs() < 1e-4);
        assert_eq!(fused.rank_requirement, "Rank S");
    }
}
