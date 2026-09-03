// crates/autonomic_adaptation/src/skills.rs
//! Capability registration, leveling, compositional fusion, and maturity progression.
//! Tracks operational capability levels, performance metrics, and skill consolidation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumIter, EnumString};

/// Main skill types available in the system
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumString, EnumIter,
)]
pub enum SkillType {
    #[strum(serialize = "DAG")]
    DAG, // Directed Acyclic Graph reasoning
    #[strum(serialize = "RAG")]
    RAG, // Retrieval Augmented Generation
    #[strum(serialize = "MCP")]
    MCP, // Model Context Protocol
    #[strum(serialize = "API")]
    API, // Direct API integration
    #[strum(serialize = "Fusion")]
    Fusion, // Created by fusing two skills
    #[strum(serialize = "Unique")]
    Unique, // Completely unique form (post-awakening evolution)
}

/// Skill origin - how it was acquired
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumString, EnumIter,
)]
pub enum SkillOrigin {
    #[strum(serialize = "Genetic")]
    Genetic, // Innate from GGUF extraction
    #[strum(serialize = "Earned")]
    Earned, // Learned through task execution
    #[strum(serialize = "Granted")]
    Granted, // Given by system/Omni
    #[strum(serialize = "Fused")]
    Fused, // Created by combining two skills
    #[strum(serialize = "Awakened")]
    Awakened, // Evolved from a mastered skill
}

/// Specialist capability maturity level — represents verified reliability and performance
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Display,
    EnumString,
    EnumIter,
)]
pub enum PersonaRank {
    Rank1NovellyDigested = 1,
    Rank2IntegratedSpecialist = 2,
    Rank3Journeyman = 3,
    Rank4Master = 4,
    Rank5Transcendent = 5,
}

impl PersonaRank {
    pub fn name(&self) -> &str {
        match self {
            PersonaRank::Rank1NovellyDigested => "Level 1: Newly Integrated",
            PersonaRank::Rank2IntegratedSpecialist => "Level 2: Verified Specialist",
            PersonaRank::Rank3Journeyman => "Level 3: Benchmarked Member",
            PersonaRank::Rank4Master => "Level 4: Domain Expert",
            PersonaRank::Rank5Transcendent => "Level 5: Autonomous Reflex",
        }
    }
}

/// A single skill/capability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skill {
    pub skill_id: String,
    pub skill_name: String,
    pub skill_type: SkillType,
    pub specialist_id: String,

    // Progression
    pub level: u8,       // 1-20
    pub experience: u32, // XP toward next level
    pub xp_to_next_level: u32,
    pub mastery_progress: f64, // 0.0-1.0 progress to next level

    // Performance
    pub success_rate: f64,    // 0.0-1.0
    pub usage_count: u32,     // How many times used
    pub average_quality: f64, // 1.0-10.0 scale

    // Evolution
    pub origin: SkillOrigin,
    pub is_awakened: bool,
    pub awakened_form: Option<String>, // Name of awakened form if applicable
    pub parent_skills: Vec<String>,    // Skills this was created from (if fused)
    pub child_skills: Vec<String>,     // Skills created from this (if fused into)

    // Metadata
    pub description: String,
    pub effect_description: String,
    pub prerequisites: Vec<String>,      // Skills required before this
    pub compatible_fusions: Vec<String>, // Skills this can fuse with
    pub created_at: DateTime<Utc>,
    pub first_used: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub awakening_readiness: f64, // 0.0-1.0 how close to awakening threshold
}

impl Skill {
    pub fn new(
        skill_id: String,
        skill_name: String,
        skill_type: SkillType,
        specialist_id: String,
        origin: SkillOrigin,
        description: String,
        effect_description: String,
    ) -> Self {
        Self {
            skill_id,
            skill_name,
            skill_type,
            specialist_id,
            level: 1,
            experience: 0,
            xp_to_next_level: 500,
            mastery_progress: 0.0,
            success_rate: 0.0,
            usage_count: 0,
            average_quality: 0.0,
            origin,
            is_awakened: false,
            awakened_form: None,
            parent_skills: Vec::new(),
            child_skills: Vec::new(),
            description,
            effect_description,
            prerequisites: Vec::new(),
            compatible_fusions: Vec::new(),
            created_at: Utc::now(),
            first_used: None,
            last_used: None,
            awakening_readiness: 0.0,
        }
    }

    /// Record a skill usage and update metrics
    pub fn record_usage(&mut self, success: bool, quality: f64) {
        self.usage_count += 1;
        self.last_used = Some(Utc::now());

        if self.first_used.is_none() {
            self.first_used = Some(Utc::now());
        }

        // Update average quality
        let prev_total = self.average_quality * (self.usage_count - 1) as f64;
        self.average_quality = (prev_total + quality) / self.usage_count as f64;

        // Update success rate
        let prev_success_count = (self.success_rate * (self.usage_count - 1) as f64) as u32;
        let new_success_count = if success {
            prev_success_count + 1
        } else {
            prev_success_count
        };
        self.success_rate = new_success_count as f64 / self.usage_count as f64;

        // Award XP based on quality and success
        let base_xp = if success { 10 } else { 5 };
        let quality_multiplier = (quality / 10.0).clamp(0.5, 2.0);
        let xp_gained = (base_xp as f64 * quality_multiplier) as u32;

        self.add_experience(xp_gained);

        // Update awakening readiness
        if self.level >= 10 && self.success_rate >= 0.9 {
            self.awakening_readiness = (self.level as f64 / 20.0) * self.success_rate;
        }
    }

    /// Add experience and handle leveling
    pub fn add_experience(&mut self, xp: u32) {
        self.experience += xp;

        while self.experience >= self.xp_to_next_level && self.level < 20 {
            self.experience -= self.xp_to_next_level;
            self.level += 1;
            self.xp_to_next_level = (500 * (self.level as u32 - 1)).max(500);
            self.mastery_progress = 0.0;
        }

        self.mastery_progress = self.experience as f64 / self.xp_to_next_level as f64;
    }

    /// Check if skill is ready to awaken
    pub fn can_awaken(&self) -> bool {
        self.level >= 10 && self.success_rate >= 0.9 && self.awakening_readiness >= 0.8
    }

    /// Awaken skill to new form
    pub fn awaken(&mut self, awakened_form_name: String) -> bool {
        if !self.can_awaken() {
            return false;
        }

        self.is_awakened = true;
        self.awakened_form = Some(awakened_form_name);
        self.level = 11; // Keep but mark as awakened
        self.experience = 0;
        self.xp_to_next_level = 3000; // Harder to level post-awakening

        true
    }

    /// Check if this skill can fuse with another
    pub fn can_fuse_with(&self, other: &Skill) -> bool {
        if self.skill_id == other.skill_id {
            return false;
        }

        if self.level < 3 || other.level < 3 {
            return false;
        }

        // Must have semantic relationship or be listed as compatible
        self.compatible_fusions.contains(&other.skill_id)
            || other.compatible_fusions.contains(&self.skill_id)
    }

    /// Get skill power score (0.0-10.0) based on level, quality, and success
    pub fn power_score(&self) -> f64 {
        let level_component = (self.level as f64 / 2.0).min(10.0);
        let quality_component = self.average_quality;
        let success_component = self.success_rate * 10.0;

        ((level_component + quality_component + success_component) / 3.0).min(10.0)
    }
}

/// A fusion of two or more skills into a new ability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusedSkill {
    pub fused_skill_id: String,
    pub fused_skill_name: String,
    pub specialist_id: String,
    pub parent_skill_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub emergent_properties: Vec<String>,
    pub power_multiplier: f64, // Combined skill is more powerful than parts
}

impl FusedSkill {
    pub fn new(
        fused_skill_name: String,
        specialist_id: String,
        parent_skills: Vec<&Skill>,
        emergent_properties: Vec<String>,
    ) -> Self {
        let parent_ids = parent_skills.iter().map(|s| s.skill_id.clone()).collect();
        let base_power =
            parent_skills.iter().map(|s| s.power_score()).sum::<f64>() / parent_skills.len() as f64;

        Self {
            fused_skill_id: format!("fused_{}_{}", specialist_id, uuid::Uuid::new_v4()),
            fused_skill_name,
            specialist_id,
            parent_skill_ids: parent_ids,
            created_at: Utc::now(),
            emergent_properties,
            power_multiplier: (1.5 + (base_power / 10.0)).min(3.0), // 1.5x to 3x power boost
        }
    }
}

/// Specialist's skill inventory and rank
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialistSkillSet {
    pub specialist_id: String,
    pub persona_rank: PersonaRank,
    pub total_experience: u32,
    pub skills: HashMap<String, Skill>,
    pub fused_skills: Vec<FusedSkill>,
    pub mentees: Vec<String>,
    pub mentors: Vec<String>,
}

impl SpecialistSkillSet {
    pub fn new(specialist_id: String) -> Self {
        Self {
            specialist_id,
            persona_rank: PersonaRank::Rank1NovellyDigested,
            total_experience: 0,
            skills: HashMap::new(),
            fused_skills: Vec::new(),
            mentees: Vec::new(),
            mentors: Vec::new(),
        }
    }

    /// Add a skill to this specialist
    pub fn add_skill(&mut self, skill: Skill) {
        self.skills.insert(skill.skill_id.clone(), skill);
    }

    /// Record skill usage
    pub fn use_skill(&mut self, skill_id: &str, success: bool, quality: f64) -> bool {
        if let Some(skill) = self.skills.get_mut(skill_id) {
            skill.record_usage(success, quality);
            self.total_experience += skill.experience;
            return true;
        }
        false
    }

    /// Check if ready for rank evolution
    pub fn check_rank_evolution(&mut self) -> Option<PersonaRank> {
        let skilled_count = self.skills.values().filter(|s| s.level >= 5).count();

        let awakened_count = self.skills.values().filter(|s| s.is_awakened).count();

        let next_rank = match self.persona_rank {
            PersonaRank::Rank1NovellyDigested => {
                if skilled_count >= 5 {
                    Some(PersonaRank::Rank2IntegratedSpecialist)
                } else {
                    None
                }
            }
            PersonaRank::Rank2IntegratedSpecialist => {
                if skilled_count >= 10 && self.skills.values().any(|s| s.level >= 5) {
                    Some(PersonaRank::Rank3Journeyman)
                } else {
                    None
                }
            }
            PersonaRank::Rank3Journeyman => {
                if self.skills.values().filter(|s| s.level >= 10).count() >= 3
                    && awakened_count >= 1
                {
                    Some(PersonaRank::Rank4Master)
                } else {
                    None
                }
            }
            PersonaRank::Rank4Master => {
                if awakened_count >= 2 && !self.fused_skills.is_empty() {
                    Some(PersonaRank::Rank5Transcendent)
                } else {
                    None
                }
            }
            PersonaRank::Rank5Transcendent => None,
        };

        if let Some(new_rank) = next_rank {
            self.persona_rank = new_rank;
        }

        next_rank
    }

    /// Get total power score of specialist
    pub fn total_power_score(&self) -> f64 {
        let skill_power: f64 = self.skills.values().map(|s| s.power_score()).sum();
        let fusion_boost: f64 = self.fused_skills.iter().map(|f| f.power_multiplier).sum();
        let rank_multiplier = (self.persona_rank as u8 as f64) * 0.5;

        skill_power + fusion_boost + rank_multiplier
    }

    /// Get all skills of a specific type
    pub fn get_skills_by_type(&self, skill_type: SkillType) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.skill_type == skill_type)
            .collect()
    }

    /// Suggest optimal skill fusions
    pub fn suggest_fusions(&self) -> Vec<(String, String)> {
        let mut suggestions = Vec::new();

        let skills: Vec<&Skill> = self.skills.values().collect();
        for i in 0..skills.len() {
            for j in (i + 1)..skills.len() {
                if skills[i].can_fuse_with(skills[j]) {
                    suggestions.push((skills[i].skill_id.clone(), skills[j].skill_id.clone()));
                }
            }
        }

        suggestions
    }

    /// Add mentee relationship
    pub fn add_mentee(&mut self, mentee_id: String) {
        if !self.mentees.contains(&mentee_id) {
            self.mentees.push(mentee_id);
        }
    }

    /// Add mentor relationship
    pub fn add_mentor(&mut self, mentor_id: String) {
        if !self.mentors.contains(&mentor_id) {
            self.mentors.push(mentor_id);
        }
    }
}

/// Skill registry - central capability database
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    specialist_skillsets: HashMap<String, SpecialistSkillSet>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            specialist_skillsets: HashMap::new(),
        }
    }

    /// Register a new skill
    pub fn register_skill(&mut self, skill: Skill) {
        let skill_id = skill.skill_id.clone();
        self.skills.insert(skill_id, skill);
    }

    /// Get skill by ID
    pub fn get_skill(&self, skill_id: &str) -> Option<&Skill> {
        self.skills.get(skill_id)
    }

    /// Get specialist's skillset
    pub fn get_skillset(&self, specialist_id: &str) -> Option<&SpecialistSkillSet> {
        self.specialist_skillsets.get(specialist_id)
    }

    /// Get mutable specialist's skillset
    pub fn get_skillset_mut(&mut self, specialist_id: &str) -> Option<&mut SpecialistSkillSet> {
        self.specialist_skillsets.get_mut(specialist_id)
    }

    /// Create skillset for new specialist
    pub fn create_skillset(&mut self, specialist_id: String) {
        self.specialist_skillsets.insert(
            specialist_id.clone(),
            SpecialistSkillSet::new(specialist_id),
        );
    }

    /// Find all specialists with a skill
    pub fn find_specialists_with_skill(&self, skill_type: SkillType) -> Vec<String> {
        self.specialist_skillsets
            .iter()
            .filter_map(|(id, skillset)| {
                if !skillset.get_skills_by_type(skill_type).is_empty() {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all awakened skills in system
    pub fn get_awakened_skills(&self) -> Vec<&Skill> {
        self.skills.values().filter(|s| s.is_awakened).collect()
    }

    /// Get all unique forms (post-rank-5 evolution)
    pub fn get_unique_forms(&self) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.skill_type == SkillType::Unique && s.is_awakened)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new(
            "test_skill_001".to_string(),
            "Test Skill".to_string(),
            SkillType::DAG,
            "specialist_1".to_string(),
            SkillOrigin::Genetic,
            "A test skill".to_string(),
            "Does something".to_string(),
        );

        assert_eq!(skill.level, 1);
        assert_eq!(skill.usage_count, 0);
        assert!(!skill.is_awakened);
    }

    #[test]
    fn test_skill_leveling() {
        let mut skill = Skill::new(
            "test_skill".to_string(),
            "Test".to_string(),
            SkillType::RAG,
            "spec_1".to_string(),
            SkillOrigin::Earned,
            "Test".to_string(),
            "Test".to_string(),
        );

        // Simulate successful uses
        for _ in 0..100 {
            skill.record_usage(true, 8.0);
        }

        assert!(skill.level > 1);
        assert!(skill.success_rate > 0.8);
    }

    #[test]
    fn test_skillset_rank_evolution() {
        let mut skillset = SpecialistSkillSet::new("specialist_1".to_string());

        // Add skills
        for i in 0..6 {
            let mut skill = Skill::new(
                format!("skill_{}", i),
                format!("Skill {}", i),
                SkillType::DAG,
                "specialist_1".to_string(),
                SkillOrigin::Genetic,
                "Test".to_string(),
                "Test".to_string(),
            );
            skill.level = 5;
            skillset.add_skill(skill);
        }

        // Should be ready for rank 2
        let new_rank = skillset.check_rank_evolution();
        assert_eq!(new_rank, Some(PersonaRank::Rank2IntegratedSpecialist));
    }

    #[test]
    fn test_skill_power_score() {
        let mut skill = Skill::new(
            "test".to_string(),
            "Test".to_string(),
            SkillType::MCP,
            "spec".to_string(),
            SkillOrigin::Earned,
            "Test".to_string(),
            "Test".to_string(),
        );

        skill.level = 10;
        skill.average_quality = 8.5;
        skill.success_rate = 0.9; // 90% success rate
        skill.usage_count = 50;

        let power = skill.power_score();
        // (5.0 + 8.5 + 9.0) / 3.0 = 7.5
        assert!(power > 5.0);
        assert!(power <= 10.0);
    }
}
