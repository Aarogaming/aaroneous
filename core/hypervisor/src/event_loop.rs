// Aaroneous Event Loop System
// Real-time skill progression tracking, XP management, and automatic rank evolution
// Handles skill usage recording, level-ups, awakenings, and breakthrough moments

use crate::skill_system::{Skill, SpecialistSkillSet, SoulRank};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Skill execution event - recorded whenever a skill is used
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillExecutionEvent {
    pub event_id: String,
    pub specialist_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub success: bool,
    pub quality_score: f64,           // 1.0-10.0
    pub difficulty_multiplier: f64,   // 1.0-5.0 (crisis severity)
    pub collaboration_bonus: Option<f64>, // 1.0-3.0 (team size bonus)
    pub xp_awarded: u32,
    pub execution_time_ms: u32,
    pub breakthrough: bool,            // Did skill exceed normal limits?
    pub timestamp: DateTime<Utc>,
}

impl SkillExecutionEvent {
    pub fn new(
        specialist_id: String,
        skill_id: String,
        skill_name: String,
        success: bool,
        quality: f64,
    ) -> Self {
        Self {
            event_id: format!("exec_{}", uuid::Uuid::new_v4()),
            specialist_id,
            skill_id,
            skill_name,
            success,
            quality_score: quality.clamp(1.0, 10.0),
            difficulty_multiplier: 1.0,
            collaboration_bonus: None,
            xp_awarded: 0,
            execution_time_ms: 0,
            breakthrough: false,
            timestamp: Utc::now(),
        }
    }

    /// Set difficulty for this execution (crisis severity)
    pub fn set_difficulty(&mut self, difficulty: f64) {
        self.difficulty_multiplier = difficulty.clamp(1.0, 5.0);
    }

    /// Set collaboration bonus (team size)
    pub fn set_collaboration(&mut self, team_size: usize) {
        self.collaboration_bonus = Some((1.0 + (team_size as f64 * 0.5)).min(3.0));
    }

    /// Mark as breakthrough moment
    pub fn mark_breakthrough(&mut self) {
        self.breakthrough = true;
    }
}

/// Level-up event - triggered when skill reaches next level
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelUpEvent {
    pub event_id: String,
    pub specialist_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub old_level: u8,
    pub new_level: u8,
    pub total_usage_count: u32,
    pub success_rate: f64,
    pub timestamp: DateTime<Utc>,
}

impl LevelUpEvent {
    pub fn new(
        specialist_id: String,
        skill_id: String,
        skill_name: String,
        old_level: u8,
        new_level: u8,
        usage_count: u32,
        success_rate: f64,
    ) -> Self {
        Self {
            event_id: format!("levelup_{}", uuid::Uuid::new_v4()),
            specialist_id,
            skill_id,
            skill_name,
            old_level,
            new_level,
            total_usage_count: usage_count,
            success_rate,
            timestamp: Utc::now(),
        }
    }
}

/// Awakening event - skill transcends to new form
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwakeningEvent {
    pub event_id: String,
    pub specialist_id: String,
    pub skill_id: String,
    pub original_name: String,
    pub awakened_form: String,
    pub breakthrough_moment: String, // Description of what triggered awakening
    pub level_at_awakening: u8,
    pub success_rate: f64,
    pub new_abilities: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl AwakeningEvent {
    pub fn new(
        specialist_id: String,
        skill_id: String,
        original_name: String,
        awakened_form: String,
        breakthrough: String,
        level: u8,
        success_rate: f64,
        abilities: Vec<String>,
    ) -> Self {
        Self {
            event_id: format!("awaken_{}", uuid::Uuid::new_v4()),
            specialist_id,
            skill_id,
            original_name,
            awakened_form,
            breakthrough_moment: breakthrough,
            level_at_awakening: level,
            success_rate,
            new_abilities: abilities,
            timestamp: Utc::now(),
        }
    }
}

/// Rank evolution event - specialist promoted to new soul rank
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankEvolutionEvent {
    pub event_id: String,
    pub specialist_id: String,
    pub old_rank: SoulRank,
    pub new_rank: SoulRank,
    pub achievement_summary: String,
    pub milestone_skills: Vec<String>, // Skills that helped achieve this
    pub timestamp: DateTime<Utc>,
}

impl RankEvolutionEvent {
    pub fn new(
        specialist_id: String,
        old_rank: SoulRank,
        new_rank: SoulRank,
        summary: String,
        skills: Vec<String>,
    ) -> Self {
        Self {
            event_id: format!("rankup_{}", uuid::Uuid::new_v4()),
            specialist_id,
            old_rank,
            new_rank,
            achievement_summary: summary,
            milestone_skills: skills,
            timestamp: Utc::now(),
        }
    }
}

/// XP calculation with multipliers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XPCalculation {
    pub base_xp: u32,
    pub quality_multiplier: f64,
    pub difficulty_multiplier: f64,
    pub collaboration_multiplier: f64,
    pub breakthrough_bonus: u32,
    pub teaching_bonus: u32,
    pub total_xp: u32,
}

impl XPCalculation {
    pub fn calculate(
        success: bool,
        quality: f64,
        difficulty: f64,
        collaboration: Option<f64>,
        is_breakthrough: bool,
        is_teaching: bool,
    ) -> Self {
        // Base XP
        let base_xp = if success { 10 } else { 5 };

        // Quality multiplier (1.0-2.0)
        let quality_mult = (quality / 10.0).clamp(0.5, 2.0);

        // Difficulty multiplier (1.0-5.0)
        let diff_mult = difficulty.clamp(1.0, 5.0);

        // Collaboration multiplier (1.0-3.0)
        let collab_mult = collaboration.unwrap_or(1.0).clamp(1.0, 3.0);

        // Breakthrough bonus
        let breakthrough_bonus = if is_breakthrough { 500 } else { 0 };

        // Teaching bonus
        let teaching_bonus = if is_teaching { 50 } else { 0 };

        // Total calculation
        let multiplied = (base_xp as f64) * quality_mult * diff_mult * collab_mult;
        let total_xp = (multiplied as u32) + breakthrough_bonus + teaching_bonus;

        Self {
            base_xp,
            quality_multiplier: quality_mult,
            difficulty_multiplier: diff_mult,
            collaboration_multiplier: collab_mult,
            breakthrough_bonus,
            teaching_bonus,
            total_xp,
        }
    }
}

/// Breakthrough detection - identifies when skill exceeds normal limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BreakthroughDetection {
    pub skill_id: String,
    pub current_level: u8,
    pub current_success_rate: f64,
    pub normal_execution_time: u32,     // Average ms
    pub actual_execution_time: u32,     // This execution
    pub quality_exceeds_average: bool,
    pub speed_exceeds_normal: bool,
    pub success_on_high_difficulty: bool,
    pub is_breakthrough: bool,
    pub breakthrough_magnitude: f64,    // 0.0-1.0 (how far beyond normal)
}

impl BreakthroughDetection {
    pub fn analyze(
        skill_id: String,
        level: u8,
        success_rate: f64,
        quality: f64,
        avg_quality: f64,
        normal_time: u32,
        actual_time: u32,
        difficulty: f64,
    ) -> Self {
        let quality_exceeds = quality > (avg_quality * 1.2);
        let speed_exceeds = actual_time < (normal_time as f64 * 0.7) as u32 && actual_time > 0;
        let success_on_hard = success_rate >= 0.85 && difficulty >= 3.0;

        // Breakthrough if multiple conditions met
        let breakthrough_count = [quality_exceeds, speed_exceeds, success_on_hard]
            .iter()
            .filter(|&&b| b)
            .count();

        let is_breakthrough = breakthrough_count >= 2;
        let magnitude = (breakthrough_count as f64) / 3.0;

        Self {
            skill_id,
            current_level: level,
            current_success_rate: success_rate,
            normal_execution_time: normal_time,
            actual_execution_time: actual_time,
            quality_exceeds_average: quality_exceeds,
            speed_exceeds_normal: speed_exceeds,
            success_on_high_difficulty: success_on_hard,
            is_breakthrough,
            breakthrough_magnitude: magnitude.min(1.0),
        }
    }
}

/// Main Event Loop - processes all skill-related events
pub struct SkillEventLoop {
    execution_events: Vec<SkillExecutionEvent>,
    level_up_events: Vec<LevelUpEvent>,
    awakening_events: Vec<AwakeningEvent>,
    rank_evolution_events: Vec<RankEvolutionEvent>,
    specialist_skill_history: HashMap<String, Vec<String>>, // specialist -> skill_ids used
    last_evolution_check: DateTime<Utc>,
    evolution_check_interval: Duration,
}

impl SkillEventLoop {
    pub fn new() -> Self {
        Self {
            execution_events: Vec::new(),
            level_up_events: Vec::new(),
            awakening_events: Vec::new(),
            rank_evolution_events: Vec::new(),
            specialist_skill_history: HashMap::new(),
            last_evolution_check: Utc::now(),
            evolution_check_interval: Duration::hours(1), // Check rank evolution hourly
        }
    }

    /// Record a skill execution and process XP
    pub fn record_skill_execution(
        &mut self,
        mut event: SkillExecutionEvent,
        skillset: &mut SpecialistSkillSet,
    ) -> Result<ExecutionResult, String> {
        let specialist_id = event.specialist_id.clone();
        let skill_id = event.skill_id.clone();

        // Get skill and record usage
        let skill = skillset
            .skills
            .get_mut(&skill_id)
            .ok_or("Skill not found")?;

        // Record execution metrics
        skill.record_usage(event.success, event.quality_score);

        // Calculate XP
        let xp_calc = XPCalculation::calculate(
            event.success,
            event.quality_score,
            event.difficulty_multiplier,
            event.collaboration_bonus,
            event.breakthrough,
            false, // is_teaching - check separately
        );

        event.xp_awarded = xp_calc.total_xp;
        skill.add_experience(xp_calc.total_xp);

        // Track usage for skill history
        self.specialist_skill_history
            .entry(specialist_id.clone())
            .or_insert_with(Vec::new)
            .push(skill_id.clone());

        // Store event
        self.execution_events.push(event.clone());

        // Check for level-up
        let mut level_up_event = None;
        if skill.level > 1 {
            // Track that level changed by comparing before/after
            // (This is simplified - in real implementation, capture before/after)
            let is_level_up = skill.mastery_progress == 0.0 && skill.level > 1;
            if is_level_up {
                level_up_event = Some(LevelUpEvent::new(
                    specialist_id.clone(),
                    skill_id.clone(),
                    event.skill_name.clone(),
                    skill.level - 1,
                    skill.level,
                    skill.usage_count,
                    skill.success_rate,
                ));
                self.level_up_events
                    .push(level_up_event.clone().unwrap());
            }
        }

        // Check for awakening readiness
        let mut awakening = None;
        if skill.can_awaken() && event.breakthrough {
            let awakened_name = self.generate_awakened_form_name(&event.skill_name);
            skill.awaken(awakened_name.clone());

            awakening = Some(AwakeningEvent::new(
                specialist_id.clone(),
                skill_id.clone(),
                event.skill_name.clone(),
                awakened_name,
                format!("Breakthrough in high-stakes execution (quality: {:.1})", event.quality_score),
                skill.level,
                skill.success_rate,
                vec![
                    "Instant execution".to_string(),
                    "Extended foresight".to_string(),
                    "Teachable ability".to_string(),
                ],
            ));

            self.awakening_events
                .push(awakening.clone().unwrap());
        }

        Ok(ExecutionResult {
            xp_awarded: xp_calc.total_xp,
            xp_calculation: xp_calc,
            level_up: level_up_event,
            awakening,
        })
    }

    /// Check and process rank evolution for a specialist
    pub fn check_rank_evolution(
        &mut self,
        skillset: &mut SpecialistSkillSet,
    ) -> Option<RankEvolutionEvent> {
        if Utc::now() < self.last_evolution_check + self.evolution_check_interval {
            return None; // Not yet time to check
        }

        let old_rank = skillset.soul_rank;
        
        if let Some(new_rank) = skillset.check_rank_evolution() {
            if new_rank != old_rank {
                // Determine milestone skills
                let skilled = skillset
                    .skills
                    .values()
                    .filter(|s| s.level >= 5)
                    .map(|s| s.skill_name.clone())
                    .collect::<Vec<_>>();

                let summary = format!(
                    "Advanced from {} to {}",
                    old_rank.name(),
                    new_rank.name()
                );

                let rank_event = RankEvolutionEvent::new(
                    skillset.specialist_id.clone(),
                    old_rank,
                    new_rank,
                    summary,
                    skilled,
                );

                self.rank_evolution_events.push(rank_event.clone());
                self.last_evolution_check = Utc::now();
                
                return Some(rank_event);
            }
        }

        self.last_evolution_check = Utc::now();
        None
    }

    /// Generate appropriate awakened form name
    fn generate_awakened_form_name(&self, original: &str) -> String {
        match original {
            s if s.contains("Decomposition") => "Adaptive Strategy Mastery".to_string(),
            s if s.contains("Synthesis") => "Emergent Knowledge Integration".to_string(),
            s if s.contains("Code") => "Prophetic Code Architecture".to_string(),
            s if s.contains("Integration") => "Orchestrated System Mastery".to_string(),
            s if s.contains("Pattern") => "Prophetic Pattern Recognition".to_string(),
            _ => format!("Mastered {}", original),
        }
    }

    /// Detect breakthrough moment
    pub fn detect_breakthrough(
        skill: &Skill,
        quality: f64,
        actual_time: u32,
        difficulty: f64,
    ) -> BreakthroughDetection {
        let normal_time = match skill.level {
            1..=5 => 5000,
            6..=10 => 3000,
            11..=15 => 2000,
            _ => 1000,
        };

        BreakthroughDetection::analyze(
            skill.skill_id.clone(),
            skill.level,
            skill.success_rate,
            quality,
            skill.average_quality,
            normal_time,
            actual_time,
            difficulty,
        )
    }

    /// Get execution event history for specialist
    pub fn get_specialist_execution_history(&self, specialist_id: &str) -> Vec<&SkillExecutionEvent> {
        self.execution_events
            .iter()
            .filter(|e| e.specialist_id == specialist_id)
            .collect()
    }

    /// Get level-up history for specialist
    pub fn get_specialist_level_ups(&self, specialist_id: &str) -> Vec<&LevelUpEvent> {
        self.level_up_events
            .iter()
            .filter(|e| e.specialist_id == specialist_id)
            .collect()
    }

    /// Get awakening history for specialist
    pub fn get_specialist_awakenings(&self, specialist_id: &str) -> Vec<&AwakeningEvent> {
        self.awakening_events
            .iter()
            .filter(|e| e.specialist_id == specialist_id)
            .collect()
    }

    /// Get rank evolution history for specialist
    pub fn get_specialist_rank_evolutions(&self, specialist_id: &str) -> Vec<&RankEvolutionEvent> {
        self.rank_evolution_events
            .iter()
            .filter(|e| e.specialist_id == specialist_id)
            .collect()
    }

    /// Get total XP earned by specialist
    pub fn get_specialist_total_xp(&self, specialist_id: &str) -> u32 {
        self.execution_events
            .iter()
            .filter(|e| e.specialist_id == specialist_id)
            .map(|e| e.xp_awarded)
            .sum()
    }

    /// Get skill statistics
    pub fn get_skill_statistics(&self, skill_id: &str) -> SkillStatistics {
        let uses: Vec<_> = self
            .execution_events
            .iter()
            .filter(|e| e.skill_id == skill_id)
            .collect();

        let total_uses = uses.len() as u32;
        let successful = uses.iter().filter(|e| e.success).count() as u32;
        let breakthroughs = uses.iter().filter(|e| e.breakthrough).count() as u32;
        let avg_quality = if uses.is_empty() {
            0.0
        } else {
            uses.iter().map(|e| e.quality_score).sum::<f64>() / uses.len() as f64
        };
        let total_xp: u32 = uses.iter().map(|e| e.xp_awarded).sum();

        SkillStatistics {
            skill_id: skill_id.to_string(),
            total_uses,
            successful_uses: successful,
            success_rate: if total_uses == 0 {
                0.0
            } else {
                successful as f64 / total_uses as f64
            },
            breakthroughs,
            breakthrough_rate: if total_uses == 0 {
                0.0
            } else {
                breakthroughs as f64 / total_uses as f64
            },
            average_quality: avg_quality,
            total_xp_earned: total_xp,
        }
    }
}

/// Result of recording a skill execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub xp_awarded: u32,
    pub xp_calculation: XPCalculation,
    pub level_up: Option<LevelUpEvent>,
    pub awakening: Option<AwakeningEvent>,
}

/// Statistics about a skill's usage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillStatistics {
    pub skill_id: String,
    pub total_uses: u32,
    pub successful_uses: u32,
    pub success_rate: f64,
    pub breakthroughs: u32,
    pub breakthrough_rate: f64,
    pub average_quality: f64,
    pub total_xp_earned: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_system::{Skill, SkillOrigin};

    #[test]
    fn test_xp_calculation_success() {
        let xp = XPCalculation::calculate(true, 8.0, 2.0, Some(1.5), false, false);
        assert!(xp.total_xp > 10); // Should have multipliers applied
        assert_eq!(xp.base_xp, 10);
    }

    #[test]
    fn test_xp_calculation_with_breakthrough() {
        let xp_normal = XPCalculation::calculate(true, 8.0, 2.0, None, false, false);
        let xp_breakthrough = XPCalculation::calculate(true, 8.0, 2.0, None, true, false);
        assert!(xp_breakthrough.total_xp > xp_normal.total_xp);
        assert_eq!(xp_breakthrough.breakthrough_bonus, 500);
    }

    #[test]
    fn test_breakthrough_detection() {
        let breakthrough = BreakthroughDetection::analyze(
            "skill_1".to_string(),
            10,
            0.90,
            9.5,  // High quality
            7.0,  // Above average
            5000, // Normal time
            2000, // Much faster
            3.5,  // High difficulty
        );

        assert!(breakthrough.quality_exceeds_average);
        assert!(breakthrough.speed_exceeds_normal);
        assert!(breakthrough.success_on_high_difficulty);
        assert!(breakthrough.is_breakthrough);
    }

    #[test]
    fn test_event_loop_execution() {
        let mut event_loop = SkillEventLoop::new();
        let mut skillset = SpecialistSkillSet::new("spec_1".to_string());

        let skill = Skill::new(
            "skill_1".to_string(),
            "Test Skill".to_string(),
            SkillType::DAG,
            "spec_1".to_string(),
            SkillOrigin::Genetic,
            "Test".to_string(),
            "Test".to_string(),
        );

        skillset.add_skill(skill);

        let event = SkillExecutionEvent::new(
            "spec_1".to_string(),
            "skill_1".to_string(),
            "Test Skill".to_string(),
            true,
            8.0,
        );

        let result = event_loop.record_skill_execution(event, &mut skillset);
        assert!(result.is_ok());
        assert!(result.unwrap().xp_awarded > 0);
    }

    #[test]
    fn test_skill_statistics() {
        let mut event_loop = SkillEventLoop::new();

        let event1 = SkillExecutionEvent::new(
            "spec_1".to_string(),
            "skill_1".to_string(),
            "Test".to_string(),
            true,
            8.0,
        );

        let event2 = SkillExecutionEvent::new(
            "spec_1".to_string(),
            "skill_1".to_string(),
            "Test".to_string(),
            false,
            5.0,
        );

        event_loop.execution_events.push(event1);
        event_loop.execution_events.push(event2);

        let stats = event_loop.get_skill_statistics("skill_1");
        assert_eq!(stats.total_uses, 2);
        assert_eq!(stats.successful_uses, 1);
        assert!(stats.success_rate > 0.4 && stats.success_rate < 0.6);
    }
}
