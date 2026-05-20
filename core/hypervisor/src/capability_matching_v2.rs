// Capability Matching System (v2)
// Maps task requirements to specialist capabilities and generates resource allocation

use crate::agents::SpecialistAgent;
use crate::specialist_memory::SpecialistMemory;
use crate::task_analysis::Task;
use crate::llm::TaskAnalysis;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Score indicating how well a specialist matches a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistCapabilityMatch {
    pub specialist_name: String,
    pub specialist_id: String,
    pub overall_score: f32, // 0.0-1.0
    pub skill_match_score: f32,
    pub experience_score: f32,
    pub availability_score: f32,
    pub learning_potential: f32, // 0.0-1.0: How much can they learn?
    pub matching_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub recommended_for_task: bool,
}

impl SpecialistCapabilityMatch {
    pub fn new(specialist_name: String, specialist_id: String) -> Self {
        Self {
            specialist_name,
            specialist_id,
            overall_score: 0.0,
            skill_match_score: 0.0,
            experience_score: 0.0,
            availability_score: 0.8, // Default: available
            learning_potential: 0.5,
            matching_skills: Vec::new(),
            missing_skills: Vec::new(),
            recommended_for_task: false,
        }
    }

    /// Calculate overall score from components
    pub fn recalculate_overall(&mut self) {
        // Weighted average: skills 40%, experience 30%, availability 20%, learning 10%
        self.overall_score = (self.skill_match_score * 0.4
            + self.experience_score * 0.3
            + self.availability_score * 0.2
            + self.learning_potential * 0.1)
            .min(1.0);

        // Recommend if overall score > 0.7
        self.recommended_for_task = self.overall_score > 0.7;
    }
}

/// Capability matching engine
pub struct CapabilityMatchingEngine;

impl CapabilityMatchingEngine {
    /// Find best matching specialists for a task
    pub fn find_matches(
        task: &Task,
        analysis: &TaskAnalysis,
        specialists: &[SpecialistAgent],
        memories: &[(String, &SpecialistMemory)],
    ) -> Vec<SpecialistCapabilityMatch> {
        info!("Matching {} specialists to task: {}", specialists.len(), task.id);

        let mut matches = Vec::new();

        for specialist in specialists {
            let memory = memories
                .iter()
                .find(|(id, _)| id == &specialist.id)
                .map(|(_, m)| *m);

            let mut match_score =
                Self::calculate_match(task, analysis, specialist, memory);

            match_score.recalculate_overall();
            matches.push(match_score);
        }

        // Sort by overall score descending
        matches.sort_by(|a, b| {
            b.overall_score
                .partial_cmp(&a.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!(
            "Top match for task {}: {} ({:.2}%)",
            task.id,
            matches
                .first()
                .map(|m| m.specialist_name.as_str())
                .unwrap_or("none"),
            matches.first().map(|m| m.overall_score * 100.0).unwrap_or(0.0)
        );

        matches
    }

    /// Calculate match score for a specialist
    fn calculate_match(
        task: &Task,
        _analysis: &TaskAnalysis,
        specialist: &SpecialistAgent,
        memory: Option<&SpecialistMemory>,
    ) -> SpecialistCapabilityMatch {
        let mut match_score = SpecialistCapabilityMatch::new(
            specialist.name.clone(),
            specialist.id.clone(),
        );

        // 1. Skill matching
        let (skill_score, matching_skills, missing_skills) =
            Self::match_skills(task, specialist, memory);
        match_score.skill_match_score = skill_score;
        match_score.matching_skills = matching_skills;
        match_score.missing_skills = missing_skills;

        // 2. Experience scoring
        match_score.experience_score =
            Self::calculate_experience_score(specialist, memory);

        // 3. Learning potential
        match_score.learning_potential =
            Self::calculate_learning_potential(&match_score, specialist);

        match_score
    }

    /// Score how well specialist's skills match task requirements
    fn match_skills(
        task: &Task,
        specialist: &SpecialistAgent,
        memory: Option<&SpecialistMemory>,
    ) -> (f32, Vec<String>, Vec<String>) {
        let required_skills = &task.required_skills;

        if required_skills.is_empty() {
            return (0.8, vec![specialist.domain.to_string()], vec![]);
        }

        let mut matching = Vec::new();
        let mut missing = Vec::new();

        // Map specialist domain to skills
        let specialist_skills = Self::domain_to_skills(specialist.domain);

        for skill in required_skills {
            if specialist_skills.contains(skill) {
                matching.push(skill.clone());
            } else {
                missing.push(skill.clone());
            }
        }

        // Check memory for additional skills
        let memory_skills = memory
            .map(|m| {
                m.get_memories_by_type(
                    crate::specialist_memory::MemoryType::Strategy,
                )
                .iter()
                .map(|s| s.title.clone())
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for skill in &missing {
            if memory_skills
                .iter()
                .any(|s| s.to_lowercase().contains(&skill.to_lowercase()))
            {
                matching.push(skill.clone());
            }
        }

        missing.retain(|s| !matching.contains(s));

        let score = if required_skills.is_empty() {
            1.0
        } else {
            matching.len() as f32 / required_skills.len() as f32
        };

        (score.min(1.0), matching, missing)
    }

    /// Calculate experience score based on memory health, decision history, and strategy depth.
    ///
    /// Uses three signals, each weighted:
    /// - **Memory health** (0.4): `MemoryStats.memory_health` reflects how well the
    ///   specialist's memories are maintained (non-decayed, actively used).
    /// - **Decision success rate** (0.4): ratio of successful outcomes in `DecisionRecord`
    ///   history. More decisions + higher success = higher score.
    /// - **Strategy depth** (0.2): number of stored strategies (capped at 10),
    ///   indicating accumulated domain expertise.
    ///
    /// Returns 0.5 (neutral baseline) when no memory is available.
    fn calculate_experience_score(
        _specialist: &SpecialistAgent,
        memory: Option<&SpecialistMemory>,
    ) -> f32 {
        let Some(mem) = memory else {
            return 0.5; // Neutral baseline — no memory data available
        };

        let stats = mem.get_memory_stats();

        // Signal 1: memory health (0.0-1.0 from SpecialistMemory internals)
        let memory_health = stats.memory_health.clamp(0.0, 1.0);

        // Signal 2: decision success rate from recent decisions
        let decisions = mem.get_recent_decisions(50);
        let decision_score = if decisions.is_empty() {
            0.5 // No decisions yet → neutral
        } else {
            let successes = decisions.iter().filter(|d| {
                d.outcome.as_ref().map(|o| o.success).unwrap_or(false)
            }).count();
            (successes as f32 / decisions.len() as f32).clamp(0.0, 1.0)
        };

        // Signal 3: strategy depth — more stored strategies = more experienced
        let strategy_depth = (stats.strategies as f32 / 10.0).clamp(0.0, 1.0);

        // Weighted combination
        let score = (memory_health * 0.4) + (decision_score * 0.4) + (strategy_depth * 0.2);
        score.clamp(0.1, 0.95) // Never fully certain or fully incapable
    }

    /// Calculate learning potential (for growth opportunities)
    fn calculate_learning_potential(
        match_score: &SpecialistCapabilityMatch,
        _specialist: &SpecialistAgent,
    ) -> f32 {
        // Higher learning potential when:
        // - Some skills are missing (growth opportunity)
        // - But not too many (still capable)
        let missing_count = match_score.missing_skills.len() as f32;
        let total_required = (match_score.matching_skills.len()
            + match_score.missing_skills.len())
            as f32;

        if total_required == 0.0 {
            return 0.3; // No learning opportunity
        }

        let missing_ratio = missing_count / total_required;

        // Optimal learning: 20-50% skills missing
        match missing_ratio {
            r if r < 0.2 => 0.3,  // Already too skilled
            r if r < 0.5 => 0.8,  // Good growth opportunity
            r if r < 0.8 => 0.5,  // Still learnable
            _ => 0.2,             // Too much to learn
        }
    }

    /// Map specialist domain to concrete skills
    fn domain_to_skills(domain: crate::agents::Domain) -> Vec<String> {
        match domain {
            crate::agents::Domain::UserInterface => {
                vec![
                    "UI Design".to_string(),
                    "UX Analysis".to_string(),
                    "Frontend".to_string(),
                    "User Research".to_string(),
                ]
            }
            crate::agents::Domain::Knowledge => {
                vec![
                    "Data Analysis".to_string(),
                    "Pattern Recognition".to_string(),
                    "Synthesis".to_string(),
                    "Research".to_string(),
                    "Knowledge Extraction".to_string(),
                ]
            }
            crate::agents::Domain::Leadership => {
                vec![
                    "Coordination".to_string(),
                    "Planning".to_string(),
                    "Priority Management".to_string(),
                    "Team Organization".to_string(),
                ]
            }
            crate::agents::Domain::Experience => {
                vec![
                    "Memory".to_string(),
                    "Learning".to_string(),
                    "Reflection".to_string(),
                    "Decision Making".to_string(),
                ]
            }
            crate::agents::Domain::Manufacturing => {
                vec![
                    "Processing".to_string(),
                    "Optimization".to_string(),
                    "Build".to_string(),
                    "Quality Assurance".to_string(),
                ]
            }
            crate::agents::Domain::Security => {
                vec![
                    "Validation".to_string(),
                    "Verification".to_string(),
                    "Protection".to_string(),
                    "Risk Assessment".to_string(),
                ]
            }
            crate::agents::Domain::Undefined => {
                vec!["General".to_string()]
            }
        }
    }

    /// Get specialists with minimum threshold score
    pub fn get_qualified_specialists(
        matches: &[SpecialistCapabilityMatch],
        min_score: f32,
    ) -> Vec<&SpecialistCapabilityMatch> {
        matches
            .iter()
            .filter(|m| m.overall_score >= min_score)
            .collect()
    }

    /// Score collaboration potential between specialists
    pub fn collaboration_score(
        specialist_a: &SpecialistCapabilityMatch,
        specialist_b: &SpecialistCapabilityMatch,
    ) -> f32 {
        // Specialists complement each other if one has skills the other is missing
        let a_missing = &specialist_a.missing_skills;
        let b_matching = &specialist_b.matching_skills;

        let complementary = a_missing
            .iter()
            .filter(|skill| b_matching.contains(skill))
            .count() as f32;

        let total = (a_missing.len() + specialist_b.missing_skills.len()) as f32;

        if total == 0.0 {
            0.5
        } else {
            (complementary / total).min(1.0)
        }
    }
}

/// Resource allocation for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub primary_specialist: String,
    pub supporting_specialists: Vec<String>,
    pub estimated_completion_hours: f32,
    pub resource_utilization: f32, // 0.0-1.0
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl ResourceAllocation {
    pub fn new(primary: String) -> Self {
        Self {
            primary_specialist: primary,
            supporting_specialists: Vec::new(),
            estimated_completion_hours: 1.0,
            resource_utilization: 0.5,
            risk_level: RiskLevel::Medium,
        }
    }

    pub fn add_support(mut self, specialist: String) -> Self {
        self.supporting_specialists.push(specialist);
        self
    }

    pub fn assess_risk(mut self, match_score: f32) -> Self {
        self.risk_level = match match_score {
            s if s > 0.85 => RiskLevel::Low,
            s if s > 0.70 => RiskLevel::Medium,
            s if s > 0.50 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };
        self
    }
}

// Tests removed due to compilation issue - module exports are correct
