// Aaroneous Rank Evolution System
// Automatic soul rank progression with achievement milestones and capability unlocks
// Detects when specialists are ready for rank-ups and handles transitions

use crate::skill_system::{SpecialistSkillSet, SoulRank};
use crate::event_loop::RankEvolutionEvent;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Requirements for each rank level
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankRequirements {
    pub rank: SoulRank,
    pub min_skills_level3: usize,
    pub min_skills_level5: usize,
    pub min_skills_level10: usize,
    pub min_awakened_skills: usize,
    pub min_fused_skills: usize,
    pub min_cascade_fusions: usize,
    pub requires_teaching: bool,
    pub min_mentees: usize,
    pub min_total_xp: u32,
    pub requires_unique_form: bool,
}

impl RankRequirements {
    pub fn rank1() -> Self {
        Self {
            rank: SoulRank::Rank1NovellyDigested,
            min_skills_level3: 0,
            min_skills_level5: 0,
            min_skills_level10: 0,
            min_awakened_skills: 0,
            min_fused_skills: 0,
            min_cascade_fusions: 0,
            requires_teaching: false,
            min_mentees: 0,
            min_total_xp: 0,
            requires_unique_form: false,
        }
    }

    pub fn rank2() -> Self {
        Self {
            rank: SoulRank::Rank2IntegratedSpecialist,
            min_skills_level3: 5,
            min_skills_level5: 0,
            min_skills_level10: 0,
            min_awakened_skills: 0,
            min_fused_skills: 0,
            min_cascade_fusions: 0,
            requires_teaching: false,
            min_mentees: 0,
            min_total_xp: 1000,
            requires_unique_form: false,
        }
    }

    pub fn rank3() -> Self {
        Self {
            rank: SoulRank::Rank3Journeyman,
            min_skills_level3: 10,
            min_skills_level5: 1,
            min_skills_level10: 0,
            min_awakened_skills: 0,
            min_fused_skills: 1, // At least 1 fusion
            min_cascade_fusions: 0,
            requires_teaching: false,
            min_mentees: 0,
            min_total_xp: 5000,
            requires_unique_form: false,
        }
    }

    pub fn rank4() -> Self {
        Self {
            rank: SoulRank::Rank4Master,
            min_skills_level3: 15,
            min_skills_level5: 5,
            min_skills_level10: 3, // 3 skills at level 10+
            min_awakened_skills: 1, // At least 1 awakened skill
            min_fused_skills: 2,
            min_cascade_fusions: 0,
            requires_teaching: true, // Must be ready to teach
            min_mentees: 0, // Can teach but doesn't need mentees yet
            min_total_xp: 15000,
            requires_unique_form: false,
        }
    }

    pub fn rank5() -> Self {
        Self {
            rank: SoulRank::Rank5Transcendent,
            min_skills_level3: 20,
            min_skills_level5: 10,
            min_skills_level10: 5,
            min_awakened_skills: 2, // 2+ awakened skills
            min_fused_skills: 3,
            min_cascade_fusions: 1, // At least 1 cascade fusion
            requires_teaching: true,
            min_mentees: 1, // Must have taught at least 1
            min_total_xp: 50000,
            requires_unique_form: true, // Must have unique form
        }
    }

    pub fn for_rank(rank: SoulRank) -> Self {
        match rank {
            SoulRank::Rank1NovellyDigested => Self::rank1(),
            SoulRank::Rank2IntegratedSpecialist => Self::rank2(),
            SoulRank::Rank3Journeyman => Self::rank3(),
            SoulRank::Rank4Master => Self::rank4(),
            SoulRank::Rank5Transcendent => Self::rank5(),
        }
    }
}

/// Achievement milestone for rank progression
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankMilestone {
    pub name: String,
    pub description: String,
    pub requirement: String,
    pub progress: f64,          // 0.0-1.0
    pub achieved: bool,
    pub achieved_at: Option<DateTime<Utc>>,
}

impl RankMilestone {
    pub fn new(name: String, description: String, requirement: String) -> Self {
        Self {
            name,
            description,
            requirement,
            progress: 0.0,
            achieved: false,
            achieved_at: None,
        }
    }

    pub fn mark_achieved(&mut self) {
        self.achieved = true;
        self.progress = 1.0;
        self.achieved_at = Some(Utc::now());
    }
}

/// Rank progression tracker for a specialist
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankProgressionTracker {
    pub specialist_id: String,
    pub current_rank: SoulRank,
    pub next_rank: SoulRank,
    pub next_rank_requirements: RankRequirements,
    pub milestones: Vec<RankMilestone>,
    pub progress_percentage: f64,  // 0.0-100.0
    pub estimated_days_to_rank_up: Option<u32>,
}

impl RankProgressionTracker {
    pub fn new(specialist_id: String, current_rank: SoulRank) -> Self {
        let next_rank = match current_rank {
            SoulRank::Rank1NovellyDigested => SoulRank::Rank2IntegratedSpecialist,
            SoulRank::Rank2IntegratedSpecialist => SoulRank::Rank3Journeyman,
            SoulRank::Rank3Journeyman => SoulRank::Rank4Master,
            SoulRank::Rank4Master => SoulRank::Rank5Transcendent,
            SoulRank::Rank5Transcendent => SoulRank::Rank5Transcendent,
        };

        let next_reqs = RankRequirements::for_rank(next_rank);

        let milestones = vec![
            RankMilestone::new(
                "Acquire Base Skills".to_string(),
                "Reach minimum skill count".to_string(),
                format!("{} skills at level 3+", next_reqs.min_skills_level3),
            ),
            RankMilestone::new(
                "Intermediate Mastery".to_string(),
                "Get some skills to level 5".to_string(),
                format!("{} skills at level 5+", next_reqs.min_skills_level5),
            ),
            RankMilestone::new(
                "Advanced Specialization".to_string(),
                "Master key skills".to_string(),
                format!("{} skills at level 10+", next_reqs.min_skills_level10),
            ),
            RankMilestone::new(
                "Awakening Breakthrough".to_string(),
                "Achieve skill awakenings".to_string(),
                format!("{} awakened skill(s)", next_reqs.min_awakened_skills),
            ),
            RankMilestone::new(
                "Skill Fusion Mastery".to_string(),
                "Create and master fusions".to_string(),
                format!("{} fusions", next_reqs.min_fused_skills),
            ),
            RankMilestone::new(
                "Total Experience".to_string(),
                "Accumulate XP across skills".to_string(),
                format!("{} total XP", next_reqs.min_total_xp),
            ),
        ];

        Self {
            specialist_id,
            current_rank,
            next_rank,
            next_rank_requirements: next_reqs,
            milestones,
            progress_percentage: 0.0,
            estimated_days_to_rank_up: None,
        }
    }

    /// Update progress based on current skillset
    pub fn update_progress(&mut self, skillset: &SpecialistSkillSet) {
        let reqs = &self.next_rank_requirements;

        let skills_l3_progress = (skillset.skills.values().filter(|s| s.level >= 3).count() as f64)
            / (reqs.min_skills_level3 as f64).max(1.0);
        let skills_l5_progress = (skillset.skills.values().filter(|s| s.level >= 5).count() as f64)
            / (reqs.min_skills_level5 as f64).max(1.0);
        let skills_l10_progress =
            (skillset.skills.values().filter(|s| s.level >= 10).count() as f64)
                / (reqs.min_skills_level10 as f64).max(1.0);
        let awakened_progress = (skillset.skills.values().filter(|s| s.is_awakened).count() as f64)
            / (reqs.min_awakened_skills as f64).max(1.0);
        let fused_progress = (skillset.fused_skills.len() as f64)
            / (reqs.min_fused_skills as f64).max(1.0);
        let xp_progress = (skillset.total_experience as f64) / (reqs.min_total_xp as f64).max(1.0);

        // Update individual milestones
        if skillset.skills.values().filter(|s| s.level >= 3).count() >= reqs.min_skills_level3 {
            self.milestones[0].progress = 1.0;
            if !self.milestones[0].achieved {
                self.milestones[0].mark_achieved();
            }
        } else {
            self.milestones[0].progress = skills_l3_progress.min(1.0);
        }

        if skillset.skills.values().filter(|s| s.level >= 5).count() >= reqs.min_skills_level5 {
            self.milestones[1].progress = 1.0;
            if !self.milestones[1].achieved {
                self.milestones[1].mark_achieved();
            }
        } else {
            self.milestones[1].progress = skills_l5_progress.min(1.0);
        }

        if skillset.skills.values().filter(|s| s.level >= 10).count() >= reqs.min_skills_level10 {
            self.milestones[2].progress = 1.0;
            if !self.milestones[2].achieved {
                self.milestones[2].mark_achieved();
            }
        } else {
            self.milestones[2].progress = skills_l10_progress.min(1.0);
        }

        if skillset.skills.values().filter(|s| s.is_awakened).count() >= reqs.min_awakened_skills {
            self.milestones[3].progress = 1.0;
            if !self.milestones[3].achieved {
                self.milestones[3].mark_achieved();
            }
        } else {
            self.milestones[3].progress = awakened_progress.min(1.0);
        }

        if skillset.fused_skills.len() >= reqs.min_fused_skills {
            self.milestones[4].progress = 1.0;
            if !self.milestones[4].achieved {
                self.milestones[4].mark_achieved();
            }
        } else {
            self.milestones[4].progress = fused_progress.min(1.0);
        }

        if skillset.total_experience >= reqs.min_total_xp {
            self.milestones[5].progress = 1.0;
            if !self.milestones[5].achieved {
                self.milestones[5].mark_achieved();
            }
        } else {
            self.milestones[5].progress = xp_progress.min(1.0);
        }

        // Calculate overall progress
        let achieved_count = self.milestones.iter().filter(|m| m.achieved).count();
        let total_progress: f64 = self.milestones.iter().map(|m| m.progress).sum::<f64>();
        self.progress_percentage = (total_progress / self.milestones.len() as f64) * 100.0;
    }

    /// Check if all requirements are met
    pub fn is_rank_up_ready(&self, skillset: &SpecialistSkillSet) -> bool {
        let reqs = &self.next_rank_requirements;

        let has_skills_l3 = skillset.skills.values().filter(|s| s.level >= 3).count()
            >= reqs.min_skills_level3;
        let has_skills_l5 = skillset.skills.values().filter(|s| s.level >= 5).count()
            >= reqs.min_skills_level5;
        let has_skills_l10 = skillset.skills.values().filter(|s| s.level >= 10).count()
            >= reqs.min_skills_level10;
        let has_awakened = skillset.skills.values().filter(|s| s.is_awakened).count()
            >= reqs.min_awakened_skills;
        let has_fusions = skillset.fused_skills.len() >= reqs.min_fused_skills;
        let has_xp = skillset.total_experience >= reqs.min_total_xp;
        let can_teach = !reqs.requires_teaching || skillset.mentors.len() > 0;

        has_skills_l3 && has_skills_l5 && has_skills_l10 && has_awakened && has_fusions && has_xp
            && can_teach
    }
}

/// Rank evolution coordinator
pub struct RankEvolutionCoordinator {
    progression_trackers: HashMap<String, RankProgressionTracker>,
    promotion_history: Vec<(String, DateTime<Utc>, SoulRank, SoulRank)>, // specialist, time, old, new
}

impl RankEvolutionCoordinator {
    pub fn new() -> Self {
        Self {
            progression_trackers: HashMap::new(),
            promotion_history: Vec::new(),
        }
    }

    /// Initialize tracker for a specialist
    pub fn track_specialist(&mut self, specialist_id: String, current_rank: SoulRank) {
        let tracker = RankProgressionTracker::new(specialist_id.clone(), current_rank);
        self.progression_trackers.insert(specialist_id, tracker);
    }

    /// Update progress for all tracked specialists
    pub fn update_all_progress(&mut self, specialists: &[(String, &SpecialistSkillSet)]) {
        for (spec_id, skillset) in specialists {
            if let Some(tracker) = self.progression_trackers.get_mut(spec_id) {
                tracker.update_progress(skillset);
            }
        }
    }

    /// Check which specialists are ready for rank-up
    pub fn find_rank_up_candidates(&self, specialists: &[(String, &SpecialistSkillSet)]) -> Vec<String> {
        specialists
            .iter()
            .filter_map(|(spec_id, skillset)| {
                if let Some(tracker) = self.progression_trackers.get(spec_id) {
                    if tracker.is_rank_up_ready(skillset) {
                        Some(spec_id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Process rank-up for a specialist
    pub fn promote_specialist(&mut self, specialist_id: &str, skillset: &mut SpecialistSkillSet) -> Option<RankEvolutionEvent> {
        if let Some(tracker) = self.progression_trackers.get(specialist_id) {
            if tracker.is_rank_up_ready(skillset) {
                let old_rank = skillset.soul_rank;
                let new_rank = tracker.next_rank;

                // Update skillset rank
                skillset.soul_rank = new_rank;

                // Record promotion
                let now = Utc::now();
                self.promotion_history
                    .push((specialist_id.to_string(), now, old_rank, new_rank));

                // Get milestone skills
                let milestone_skills: Vec<String> = skillset
                    .skills
                    .values()
                    .filter(|s| s.level >= 5)
                    .map(|s| s.skill_name.clone())
                    .collect();

                let summary = format!(
                    "Promoted from {} to {}. Demonstrated mastery with {} skills.",
                    old_rank.name(),
                    new_rank.name(),
                    milestone_skills.len()
                );

                // Create new tracker for next rank
                let new_tracker = RankProgressionTracker::new(specialist_id.to_string(), new_rank);
                self.progression_trackers.insert(specialist_id.to_string(), new_tracker);

                return Some(RankEvolutionEvent::new(
                    specialist_id.to_string(),
                    old_rank,
                    new_rank,
                    summary,
                    milestone_skills,
                ));
            }
        }

        None
    }

    /// Get progression tracker for specialist
    pub fn get_progression(&self, specialist_id: &str) -> Option<&RankProgressionTracker> {
        self.progression_trackers.get(specialist_id)
    }

    /// Get promotion history
    pub fn get_promotion_history(&self) -> &[(String, DateTime<Utc>, SoulRank, SoulRank)] {
        &self.promotion_history
    }

    /// Get specialists promoted in last N days
    pub fn get_recent_promotions(&self, days: i64) -> Vec<&(String, DateTime<Utc>, SoulRank, SoulRank)> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.promotion_history
            .iter()
            .filter(|(_, time, _, _)| *time > cutoff)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_system::{Skill, SkillOrigin, SkillType};

    #[test]
    fn test_rank_requirements() {
        let rank1 = RankRequirements::rank1();
        let rank2 = RankRequirements::rank2();
        let rank5 = RankRequirements::rank5();

        assert_eq!(rank1.rank, SoulRank::Rank1NovellyDigested);
        assert!(rank2.min_skills_level3 > rank1.min_skills_level3);
        assert!(rank5.min_total_xp > rank2.min_total_xp);
        assert!(rank5.requires_unique_form);
    }

    #[test]
    fn test_rank_progression_tracker() {
        let mut tracker = RankProgressionTracker::new(
            "spec_1".to_string(),
            SoulRank::Rank1NovellyDigested,
        );

        let mut skillset = SpecialistSkillSet::new("spec_1".to_string());

        // Add some skills at level 3
        for i in 0..5 {
            let mut skill = Skill::new(
                format!("skill_{}", i),
                format!("Skill {}", i),
                SkillType::DAG,
                "spec_1".to_string(),
                SkillOrigin::Genetic,
                "Test".to_string(),
                "Test".to_string(),
            );
            skill.level = 3;
            skillset.add_skill(skill);
        }

        skillset.total_experience = 1000;

        tracker.update_progress(&skillset);
        assert!(tracker.milestones[0].achieved); // Should achieve "Acquire Base Skills"
        assert!(tracker.progress_percentage > 0.0);
    }

    #[test]
    fn test_rank_evolution_coordinator() {
        let mut coordinator = RankEvolutionCoordinator::new();
        coordinator.track_specialist("spec_1".to_string(), SoulRank::Rank1NovellyDigested);

        assert!(coordinator.get_progression("spec_1").is_some());

        let tracker = coordinator.get_progression("spec_1").unwrap();
        assert_eq!(tracker.current_rank, SoulRank::Rank1NovellyDigested);
        assert_eq!(tracker.next_rank, SoulRank::Rank2IntegratedSpecialist);
    }

    #[test]
    fn test_rank_up_ready() {
        let mut tracker = RankProgressionTracker::new(
            "spec_1".to_string(),
            SoulRank::Rank1NovellyDigested,
        );

        let mut skillset = SpecialistSkillSet::new("spec_1".to_string());

        // Not ready yet
        assert!(!tracker.is_rank_up_ready(&skillset));

        // Add skills to meet Rank 2 requirements
        for i in 0..5 {
            let mut skill = Skill::new(
                format!("skill_{}", i),
                format!("Skill {}", i),
                SkillType::DAG,
                "spec_1".to_string(),
                SkillOrigin::Genetic,
                "Test".to_string(),
                "Test".to_string(),
            );
            skill.level = 3;
            skillset.add_skill(skill);
        }
        skillset.total_experience = 1500;

        assert!(tracker.is_rank_up_ready(&skillset));
    }
}
