use crate::skill_system::SkillType;
use crate::content_analyzer::{ContentAnalyzer, ContentAnalysis};
use crate::data_ingestion::IngestibleData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of matching data to capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatch {
    /// Matched skill type
    pub skill_type: SkillType,
    /// Specialist ID that should receive this capability
    pub specialist_id: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reason for the match
    pub reason: String,
    /// Difficulty multiplier for XP calculation
    pub difficulty_multiplier: f32,
    /// Quality score for data relevance
    pub quality_score: f32,
}

/// Domain-to-SkillType mapping
const DOMAIN_SKILL_MAPPING: &[(&str, SkillType)] = &[
    // Database domain -> RAG skills (knowledge synthesis)
    ("database", SkillType::RAG),
    
    // Networking domain -> DAG skills (task decomposition)
    ("networking", SkillType::DAG),
    
    // Security domain -> MCP skills (tool integration for security tools)
    ("security", SkillType::MCP),
    
    // Performance domain -> DAG skills (optimization decomposition)
    ("performance", SkillType::DAG),
    
    // Development domain -> DAG skills (code decomposition)
    ("development", SkillType::DAG),
    
    // Operations domain -> MCP skills (tool integration)
    ("operations", SkillType::MCP),
    
    // Crisis domain -> API skills (coordination across specialists)
    ("crisis", SkillType::API),
];

/// Specialist domain expertise (who specializes in what)
#[derive(Debug, Clone)]
pub struct SpecialistProfile {
    pub id: String,
    pub primary_domain: String,
    pub secondary_domains: Vec<String>,
    pub skill_level: u32, // 1-20
}

impl Default for SpecialistProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            primary_domain: String::new(),
            secondary_domains: Vec::new(),
            skill_level: 1,
        }
    }
}

/// Capability Matcher: Routes data to appropriate specialists based on semantic matching
pub struct CapabilityMatcher {
    /// Known specialist profiles
    specialist_profiles: HashMap<String, SpecialistProfile>,
}

impl CapabilityMatcher {
    /// Create a new capability matcher
    pub fn new() -> Self {
        Self {
            specialist_profiles: HashMap::new(),
        }
    }

    /// Register a specialist profile
    pub fn register_specialist(&mut self, profile: SpecialistProfile) {
        self.specialist_profiles.insert(profile.id.clone(), profile);
    }

    /// Load default specialist profiles (Ariel, Merlin, etc.)
    pub fn load_default_specialists() -> Self {
        let mut matcher = Self::new();

        // Ariel: Primary Database specialist
        matcher.register_specialist(SpecialistProfile {
            id: "ariel".to_string(),
            primary_domain: "database".to_string(),
            secondary_domains: vec!["development".to_string(), "security".to_string()],
            skill_level: 15,
        });

        // Merlin: Primary Performance/Development specialist
        matcher.register_specialist(SpecialistProfile {
            id: "merlin".to_string(),
            primary_domain: "development".to_string(),
            secondary_domains: vec!["performance".to_string(), "operations".to_string()],
            skill_level: 16,
        });

        // Odin: Primary Operations/Networking specialist
        matcher.register_specialist(SpecialistProfile {
            id: "odin".to_string(),
            primary_domain: "operations".to_string(),
            secondary_domains: vec!["networking".to_string(), "security".to_string()],
            skill_level: 14,
        });

        // Dionysus: Primary Performance/Crisis specialist
        matcher.register_specialist(SpecialistProfile {
            id: "dionysus".to_string(),
            primary_domain: "performance".to_string(),
            secondary_domains: vec!["crisis".to_string(), "operations".to_string()],
            skill_level: 14,
        });

        // Hephaestus: Primary Development/Security specialist
        matcher.register_specialist(SpecialistProfile {
            id: "hephaestus".to_string(),
            primary_domain: "development".to_string(),
            secondary_domains: vec!["security".to_string(), "performance".to_string()],
            skill_level: 13,
        });

        // Argus: Primary Security/Networking specialist
        matcher.register_specialist(SpecialistProfile {
            id: "argus".to_string(),
            primary_domain: "security".to_string(),
            secondary_domains: vec!["networking".to_string(), "operations".to_string()],
            skill_level: 15,
        });

        matcher
    }

    /// Find matches for ingested data
    pub fn find_matches(&self, _data: &IngestibleData, analysis: &ContentAnalysis, top_n: usize) -> Vec<CapabilityMatch> {
        // Get top detected domains
        let top_domains = ContentAnalyzer::top_domains(analysis, 5);

        if top_domains.is_empty() {
            return vec![];
        }

        let mut all_matches = Vec::new();

        // For each detected domain, find matching specialists
        for (domain, _domain_confidence) in top_domains {
            // Find skill type for this domain
            if let Some((_, skill_type)) = DOMAIN_SKILL_MAPPING
                .iter()
                .find(|(d, _)| d == &domain)
            {
                // Find specialists for this domain
                let specialist_matches = self.find_specialists_for_domain(&domain, *skill_type);
                all_matches.extend(specialist_matches);
            }
        }

        // Sort by confidence and return top N
        all_matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        all_matches.into_iter().take(top_n).collect()
    }

    /// Find specialists that match a specific domain
    fn find_specialists_for_domain(&self, domain: &str, skill_type: SkillType) -> Vec<CapabilityMatch> {
        let mut matches = Vec::new();

        for (spec_id, profile) in &self.specialist_profiles {
            let mut confidence = 0.0;
            let mut reason = String::new();

            // Primary domain match (highest confidence)
            if profile.primary_domain == domain {
                confidence = 0.95;
                reason = format!("Primary domain match: {}", domain);
            }
            // Secondary domain match
            else if profile.secondary_domains.contains(&domain.to_string()) {
                confidence = 0.70;
                reason = format!("Secondary domain match: {}", domain);
            }

            if confidence > 0.0 {
                // Calculate difficulty multiplier based on complexity and skill level
                let difficulty_multiplier = self.calculate_difficulty_multiplier(
                    profile.skill_level,
                    1.0, // Will be updated with actual complexity later
                );

                matches.push(CapabilityMatch {
                    skill_type,
                    specialist_id: spec_id.clone(),
                    confidence,
                    reason,
                    difficulty_multiplier,
                    quality_score: confidence, // Initially same as confidence
                });
            }
        }

        matches
    }

    /// Calculate difficulty multiplier based on specialist skill level
    fn calculate_difficulty_multiplier(&self, skill_level: u32, _complexity: f32) -> f32 {
        match skill_level {
            1..=5 => 1.0,      // Novice: 1x difficulty
            6..=10 => 1.5,     // Apprentice: 1.5x difficulty
            11..=15 => 2.0,    // Journeyman: 2x difficulty
            16..=19 => 2.5,    // Master: 2.5x difficulty
            _ => 3.0,          // Legendary: 3x difficulty
        }
    }

    /// Adjust matches with complexity information
    pub fn apply_complexity_scoring(
        &self,
        mut matches: Vec<CapabilityMatch>,
        complexity: f32,
    ) -> Vec<CapabilityMatch> {
        for m in &mut matches {
            // Adjust difficulty based on data complexity
            m.difficulty_multiplier *= complexity + 1.0;

            // Adjust quality score: lower complexity = higher quality training data
            if complexity < 0.3 {
                m.quality_score *= 1.5; // Simple data is good for beginners
            } else if complexity > 0.7 {
                m.quality_score *= 0.8; // Very complex data is harder to learn from
            }
        }
        matches
    }

    /// Create a skill training example from matched data
    pub fn create_training_example(
        &self,
        data: &IngestibleData,
        m: &CapabilityMatch,
    ) -> SkillTrainingExample {
        SkillTrainingExample {
            data_id: data.id.clone(),
            specialist_id: m.specialist_id.clone(),
            skill_type: m.skill_type.clone(),
            quality_score: m.quality_score,
            difficulty_multiplier: m.difficulty_multiplier,
            description: m.reason.clone(),
        }
    }
}

/// Represents a skill training example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrainingExample {
    pub data_id: String,
    pub specialist_id: String,
    pub skill_type: SkillType,
    pub quality_score: f32,
    pub difficulty_multiplier: f32,
    pub description: String,
}

impl Default for CapabilityMatcher {
    fn default() -> Self {
        Self::load_default_specialists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_registration() {
        let mut matcher = CapabilityMatcher::new();
        let profile = SpecialistProfile {
            id: "test_spec".to_string(),
            primary_domain: "database".to_string(),
            secondary_domains: vec!["development".to_string()],
            skill_level: 10,
        };

        matcher.register_specialist(profile.clone());
        assert!(matcher.specialist_profiles.contains_key("test_spec"));
    }

    #[test]
    fn test_load_default_specialists() {
        let matcher = CapabilityMatcher::load_default_specialists();
        assert_eq!(matcher.specialist_profiles.len(), 6);
        assert!(matcher.specialist_profiles.contains_key("ariel"));
        assert!(matcher.specialist_profiles.contains_key("merlin"));
        assert!(matcher.specialist_profiles.contains_key("odin"));
    }

    #[test]
    fn test_domain_to_skill_mapping() {
        let mapping = DOMAIN_SKILL_MAPPING
            .iter()
            .find(|(d, _)| d == &"database")
            .map(|(_, s)| s.clone());

        assert_eq!(mapping, Some(SkillType::RAG));
    }

    #[test]
    fn test_difficulty_multiplier_calculation() {
        let matcher = CapabilityMatcher::new();

        assert_eq!(matcher.calculate_difficulty_multiplier(1, 0.5), 1.0);
        assert_eq!(matcher.calculate_difficulty_multiplier(7, 0.5), 1.5);
        assert_eq!(matcher.calculate_difficulty_multiplier(12, 0.5), 2.0);
        assert_eq!(matcher.calculate_difficulty_multiplier(20, 0.5), 3.0);
    }

    #[test]
    fn test_find_specialists_for_database_domain() {
        let matcher = CapabilityMatcher::load_default_specialists();
        let matches = matcher.find_specialists_for_domain("database", SkillType::RAG);

        // Ariel should be primary match for database
        let ariel_match = matches.iter().find(|m| m.specialist_id == "ariel");
        assert!(ariel_match.is_some());
        assert_eq!(ariel_match.unwrap().confidence, 0.95);
    }

    #[test]
    fn test_apply_complexity_scoring() {
        let matcher = CapabilityMatcher::new();
        let mut matches = vec![CapabilityMatch {
            skill_type: SkillType::RAG,
            specialist_id: "test".to_string(),
            confidence: 0.9,
            reason: "test".to_string(),
            difficulty_multiplier: 1.0,
            quality_score: 0.9,
        }];

        let original_diff = matches[0].difficulty_multiplier;
        matches = matcher.apply_complexity_scoring(matches, 0.5);
        assert!(matches[0].difficulty_multiplier > original_diff);
    }
}
