// Aaroneous Capability Dashboard
// Real-time visibility into specialist progress, skills, fusions, and rank evolution
// Provides comprehensive queries and aggregation of system state

use crate::skill_system::{Skill, SkillType, SpecialistSkillSet, SoulRank};
use crate::event_loop::{SkillEventLoop, SkillStatistics};
use crate::rank_evolution::{RankProgressionTracker, RankEvolutionCoordinator};
use crate::fusion_federation::{FusionCapability, FusionFederationBroadcaster};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Real-time specialist status snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialistStatus {
    pub specialist_id: String,
    pub specialist_name: String,
    pub current_rank: SoulRank,
    pub rank_progress: f64,              // 0.0-100.0 toward next rank
    pub total_skills: usize,
    pub skills_by_level: HashMap<u8, usize>, // level -> count
    pub awakened_skills: usize,
    pub fused_skills: usize,
    pub total_xp: u32,
    pub average_quality: f64,
    pub average_success_rate: f64,
    pub total_executions: u32,
    pub breakthroughs: u32,
    pub last_activity: Option<DateTime<Utc>>,
    pub mentees: Vec<String>,
    pub mentors: Vec<String>,
}

impl SpecialistStatus {
    pub fn new(specialist_id: String, specialist_name: String) -> Self {
        Self {
            specialist_id,
            specialist_name,
            current_rank: SoulRank::Rank1NovellyDigested,
            rank_progress: 0.0,
            total_skills: 0,
            skills_by_level: HashMap::new(),
            awakened_skills: 0,
            fused_skills: 0,
            total_xp: 0,
            average_quality: 0.0,
            average_success_rate: 0.0,
            total_executions: 0,
            breakthroughs: 0,
            last_activity: None,
            mentees: Vec::new(),
            mentors: Vec::new(),
        }
    }

    /// Update status from skillset and event loop
    pub fn update(
        &mut self,
        skillset: &SpecialistSkillSet,
        event_loop: &SkillEventLoop,
        tracker: Option<&RankProgressionTracker>,
    ) {
        self.current_rank = skillset.soul_rank;
        self.total_skills = skillset.skills.len();
        self.fused_skills = skillset.fused_skills.len();
        self.awakened_skills = skillset.skills.values().filter(|s| s.is_awakened).count();
        self.total_xp = skillset.total_experience;
        self.mentees = skillset.mentees.clone();
        self.mentors = skillset.mentors.clone();

        // Build skills by level histogram
        self.skills_by_level.clear();
        for skill in skillset.skills.values() {
            *self.skills_by_level.entry(skill.level).or_insert(0) += 1;
        }

        // Calculate averages
        if !skillset.skills.is_empty() {
            let total_quality: f64 = skillset.skills.values().map(|s| s.average_quality).sum();
            self.average_quality = total_quality / skillset.skills.len() as f64;

            let total_success: f64 = skillset.skills.values().map(|s| s.success_rate).sum();
            self.average_success_rate = total_success / skillset.skills.len() as f64;
        }

        // Get execution stats
        let execs = event_loop.get_specialist_execution_history(&self.specialist_id);
        self.total_executions = execs.len() as u32;
        self.breakthroughs = execs.iter().filter(|e| e.breakthrough).count() as u32;
        self.last_activity = execs.last().map(|e| e.timestamp);

        // Update rank progress
        if let Some(tracker) = tracker {
            self.rank_progress = tracker.progress_percentage;
        }
    }
}

/// Skill tree node for visualization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillTreeNode {
    pub skill_id: String,
    pub skill_name: String,
    pub skill_type: SkillType,
    pub level: u8,
    pub experience: u32,
    pub xp_to_next: u32,
    pub success_rate: f64,
    pub is_awakened: bool,
    pub awakened_form: Option<String>,
    pub parent_skills: Vec<String>,
    pub child_skills: Vec<String>,
    pub can_awaken: bool,
    pub awakening_readiness: f64,
}

impl SkillTreeNode {
    pub fn from_skill(skill: &Skill) -> Self {
        Self {
            skill_id: skill.skill_id.clone(),
            skill_name: skill.skill_name.clone(),
            skill_type: skill.skill_type,
            level: skill.level,
            experience: skill.experience,
            xp_to_next: skill.xp_to_next_level,
            success_rate: skill.success_rate,
            is_awakened: skill.is_awakened,
            awakened_form: skill.awakened_form.clone(),
            parent_skills: skill.parent_skills.clone(),
            child_skills: skill.child_skills.clone(),
            can_awaken: skill.can_awaken(),
            awakening_readiness: skill.awakening_readiness,
        }
    }
}

/// Federation-wide capability summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilitySummary {
    pub total_specialists: usize,
    pub total_skills: usize,
    pub total_fusions: usize,
    pub total_awakenings: usize,
    pub specialists_by_rank: HashMap<String, usize>, // rank name -> count
    pub skill_type_distribution: HashMap<String, usize>, // type -> count
    pub most_common_fusions: Vec<(String, usize)>, // (fusion_name, count)
    pub average_specialist_xp: u32,
    pub highest_rank_specialists: Vec<String>,
}

impl CapabilitySummary {
    pub fn new() -> Self {
        Self {
            total_specialists: 0,
            total_skills: 0,
            total_fusions: 0,
            total_awakenings: 0,
            specialists_by_rank: HashMap::new(),
            skill_type_distribution: HashMap::new(),
            most_common_fusions: Vec::new(),
            average_specialist_xp: 0,
            highest_rank_specialists: Vec::new(),
        }
    }
}

/// Crisis response capability snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrisisCapability {
    pub specialist_id: String,
    pub specialist_name: String,
    pub rank: SoulRank,
    pub crisis_skills: Vec<(String, u8, f64)>, // (skill_name, level, success_rate)
    pub breakthrough_rate: f64,
    pub average_crisis_xp: u32,
    pub can_lead_response: bool, // Rank 4+
    pub mentorship_available: bool, // Can teach others
}

impl CrisisCapability {
    pub fn new(specialist_id: String, specialist_name: String) -> Self {
        Self {
            specialist_id,
            specialist_name,
            rank: SoulRank::Rank1NovellyDigested,
            crisis_skills: Vec::new(),
            breakthrough_rate: 0.0,
            average_crisis_xp: 0,
            can_lead_response: false,
            mentorship_available: false,
        }
    }
}

/// Main capability dashboard
pub struct CapabilityDashboard {
    specialist_statuses: HashMap<String, SpecialistStatus>,
    capability_summary: CapabilitySummary,
    skill_trees: HashMap<String, Vec<SkillTreeNode>>, // specialist_id -> skills
    crisis_capabilities: Vec<CrisisCapability>,
    last_update: DateTime<Utc>,
    update_interval_secs: u64,
}

impl CapabilityDashboard {
    pub fn new() -> Self {
        Self {
            specialist_statuses: HashMap::new(),
            capability_summary: CapabilitySummary::new(),
            skill_trees: HashMap::new(),
            crisis_capabilities: Vec::new(),
            last_update: Utc::now(),
            update_interval_secs: 60, // Update every minute
        }
    }

    /// Register a specialist on the dashboard
    pub fn register_specialist(&mut self, specialist_id: String, specialist_name: String) {
        let status = SpecialistStatus::new(specialist_id.clone(), specialist_name);
        self.specialist_statuses.insert(specialist_id, status);
    }

    /// Update all specialist statuses
    pub fn update_all_specialists(
        &mut self,
        specialists: &[(String, SpecialistSkillSet)],
        event_loop: &SkillEventLoop,
        coordinator: &RankEvolutionCoordinator,
    ) {
        self.last_update = Utc::now();

        for (spec_id, skillset) in specialists {
            if let Some(status) = self.specialist_statuses.get_mut(spec_id) {
                let tracker = coordinator.get_progression(spec_id);
                status.update(skillset, event_loop, tracker);

                // Update skill tree
                let skill_nodes: Vec<SkillTreeNode> = skillset
                    .skills
                    .values()
                    .map(SkillTreeNode::from_skill)
                    .collect();
                self.skill_trees.insert(spec_id.clone(), skill_nodes);
            }
        }

        // Recalculate capability summary
        self.recalculate_summary();
    }

    /// Recalculate federation-wide capability summary
    fn recalculate_summary(&mut self) {
        let mut summary = CapabilitySummary::new();

        summary.total_specialists = self.specialist_statuses.len();

        let mut total_xp = 0u64;
        let mut rank_counts: HashMap<String, usize> = HashMap::new();
        let mut skill_counts: HashMap<String, usize> = HashMap::new();

        for status in self.specialist_statuses.values() {
            // Count by rank
            let rank_name = status.current_rank.name().to_string();
            *rank_counts.entry(rank_name).or_insert(0) += 1;

            // Sum XP
            total_xp += status.total_xp as u64;

            // Count skills
            summary.total_skills += status.total_skills;
            summary.total_fusions += status.fused_skills;
            summary.total_awakenings += status.awakened_skills;

            // Track highest ranks
            if status.current_rank == SoulRank::Rank5Transcendent {
                summary.highest_rank_specialists.push(status.specialist_id.clone());
            }
        }

        summary.specialists_by_rank = rank_counts;
        summary.average_specialist_xp = if summary.total_specialists > 0 {
            (total_xp / summary.total_specialists as u64) as u32
        } else {
            0
        };

        self.capability_summary = summary;
    }

    /// Get specialist status
    pub fn get_specialist_status(&self, specialist_id: &str) -> Option<&SpecialistStatus> {
        self.specialist_statuses.get(specialist_id)
    }

    /// Get all specialist statuses
    pub fn get_all_statuses(&self) -> Vec<&SpecialistStatus> {
        self.specialist_statuses.values().collect()
    }

    /// Get skill tree for a specialist
    pub fn get_skill_tree(&self, specialist_id: &str) -> Option<&Vec<SkillTreeNode>> {
        self.skill_trees.get(specialist_id)
    }

    /// Find specialists ready for rank-up
    pub fn find_rank_up_candidates(&self) -> Vec<(String, SoulRank, f64)> {
        self.specialist_statuses
            .values()
            .filter(|s| s.rank_progress >= 90.0)
            .map(|s| (s.specialist_id.clone(), s.current_rank, s.rank_progress))
            .collect()
    }

    /// Find specialists with awakening-ready skills
    pub fn find_awakening_candidates(&self) -> Vec<(String, Vec<String>)> {
        let mut candidates = Vec::new();

        for (spec_id, skill_nodes) in &self.skill_trees {
            let ready_skills: Vec<String> = skill_nodes
                .iter()
                .filter(|node| node.can_awaken && !node.is_awakened)
                .map(|node| node.skill_name.clone())
                .collect();

            if !ready_skills.is_empty() {
                candidates.push((spec_id.clone(), ready_skills));
            }
        }

        candidates
    }

    /// Build crisis response team
    pub fn build_crisis_team(
        &self,
        crisis_difficulty: f64, // 1.0-5.0
        team_size: usize,
    ) -> Vec<CrisisCapability> {
        let mut team = Vec::new();

        for status in self.specialist_statuses.values() {
            // Prefer higher ranks and higher success rates
            let crisis_score = (status.current_rank as u8 as f64) * status.average_success_rate;

            team.push((status.specialist_id.clone(), crisis_score));
        }

        // Sort by crisis score
        team.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Build crisis capabilities for top specialists
        team.into_iter()
            .take(team_size)
            .filter_map(|(spec_id, _)| {
                self.specialist_statuses.get(&spec_id).map(|status| {
                    let mut crisis_cap = CrisisCapability::new(
                        status.specialist_id.clone(),
                        status.specialist_name.clone(),
                    );
                    crisis_cap.rank = status.current_rank;
                    crisis_cap.can_lead_response = status.current_rank as u8 >= 4;
                    crisis_cap.mentorship_available = status.current_rank as u8 >= 4;
                    crisis_cap
                })
            })
            .collect()
    }

    /// Get specialists by skill type
    pub fn get_specialists_by_skill_type(&self, skill_type: SkillType) -> Vec<String> {
        let mut specialists = Vec::new();

        for (spec_id, skill_nodes) in &self.skill_trees {
            if skill_nodes.iter().any(|node| node.skill_type == skill_type) {
                specialists.push(spec_id.clone());
            }
        }

        specialists
    }

    /// Get top specialists by metric
    pub fn get_top_specialists(
        &self,
        metric: &str, // "xp", "rank", "success_rate", "awakenings"
        limit: usize,
    ) -> Vec<(String, f64)> {
        let mut rankings: Vec<_> = self.specialist_statuses.values().collect();

        match metric {
            "xp" => {
                rankings.sort_by(|a, b| b.total_xp.cmp(&a.total_xp));
                rankings
                    .into_iter()
                    .take(limit)
                    .map(|s| (s.specialist_id.clone(), s.total_xp as f64))
                    .collect()
            }
            "rank" => {
                rankings.sort_by(|a, b| (b.current_rank as u8).cmp(&(a.current_rank as u8)));
                rankings
                    .into_iter()
                    .take(limit)
                    .map(|s| (s.specialist_id.clone(), s.current_rank as u8 as f64))
                    .collect()
            }
            "success_rate" => {
                rankings.sort_by(|a, b| {
                    b.average_success_rate
                        .partial_cmp(&a.average_success_rate)
                        .unwrap()
                });
                rankings
                    .into_iter()
                    .take(limit)
                    .map(|s| (s.specialist_id.clone(), s.average_success_rate))
                    .collect()
            }
            "awakenings" => {
                rankings.sort_by(|a, b| b.awakened_skills.cmp(&a.awakened_skills));
                rankings
                    .into_iter()
                    .take(limit)
                    .map(|s| (s.specialist_id.clone(), s.awakened_skills as f64))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Get capability summary
    pub fn get_summary(&self) -> &CapabilitySummary {
        &self.capability_summary
    }

    /// Health check - ensure all systems operational
    pub fn health_check(&self) -> HealthStatus {
        let mut issues = Vec::new();

        if self.specialist_statuses.is_empty() {
            issues.push("No specialists registered".to_string());
        }

        let stale_threshold = Utc::now() - chrono::Duration::hours(1);
        let stale_count = self
            .specialist_statuses
            .values()
            .filter(|s| s.last_activity.is_none() || s.last_activity.unwrap() < stale_threshold)
            .count();

        if stale_count > self.specialist_statuses.len() / 2 {
            issues.push(format!("{} specialists inactive", stale_count));
        }

        HealthStatus {
            operational: issues.is_empty(),
            issues,
            last_update: self.last_update,
        }
    }
}

/// Health status report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub operational: bool,
    pub issues: Vec<String>,
    pub last_update: DateTime<Utc>,
}

/// Dashboard query builder for complex queries
pub struct DashboardQuery {
    filter_rank: Option<SoulRank>,
    filter_skill_type: Option<SkillType>,
    min_xp: Option<u32>,
    min_success_rate: Option<f64>,
    has_awakening: Option<bool>,
    has_mentees: Option<bool>,
}

impl DashboardQuery {
    pub fn new() -> Self {
        Self {
            filter_rank: None,
            filter_skill_type: None,
            min_xp: None,
            min_success_rate: None,
            has_awakening: None,
            has_mentees: None,
        }
    }

    pub fn rank(mut self, rank: SoulRank) -> Self {
        self.filter_rank = Some(rank);
        self
    }

    pub fn skill_type(mut self, skill_type: SkillType) -> Self {
        self.filter_skill_type = Some(skill_type);
        self
    }

    pub fn min_xp(mut self, xp: u32) -> Self {
        self.min_xp = Some(xp);
        self
    }

    pub fn min_success_rate(mut self, rate: f64) -> Self {
        self.min_success_rate = Some(rate);
        self
    }

    pub fn with_awakenings(mut self) -> Self {
        self.has_awakening = Some(true);
        self
    }

    pub fn with_mentees(mut self) -> Self {
        self.has_mentees = Some(true);
        self
    }

    pub fn execute(&self, dashboard: &CapabilityDashboard) -> Vec<String> {
        dashboard
            .get_all_statuses()
            .into_iter()
            .filter(|status| {
                if let Some(rank) = self.filter_rank {
                    if status.current_rank != rank {
                        return false;
                    }
                }

                if let Some(min_xp) = self.min_xp {
                    if status.total_xp < min_xp {
                        return false;
                    }
                }

                if let Some(min_rate) = self.min_success_rate {
                    if status.average_success_rate < min_rate {
                        return false;
                    }
                }

                if let Some(true) = self.has_awakening {
                    if status.awakened_skills == 0 {
                        return false;
                    }
                }

                if let Some(true) = self.has_mentees {
                    if status.mentees.is_empty() {
                        return false;
                    }
                }

                // Skill type check would require access to skill tree
                if self.filter_skill_type.is_some() {
                    // TODO: Implement when skill tree is available
                }

                true
            })
            .map(|s| s.specialist_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_status_creation() {
        let status = SpecialistStatus::new("spec_1".to_string(), "Odin".to_string());
        assert_eq!(status.specialist_id, "spec_1");
        assert_eq!(status.specialist_name, "Odin");
        assert_eq!(status.current_rank, SoulRank::Rank1NovellyDigested);
    }

    #[test]
    fn test_capability_summary_creation() {
        let summary = CapabilitySummary::new();
        assert_eq!(summary.total_specialists, 0);
        assert_eq!(summary.total_skills, 0);
    }

    #[test]
    fn test_dashboard_creation() {
        let dashboard = CapabilityDashboard::new();
        assert_eq!(dashboard.specialist_statuses.len(), 0);
        assert!(dashboard.get_summary().total_specialists == 0);
    }

    #[test]
    fn test_dashboard_register_specialist() {
        let mut dashboard = CapabilityDashboard::new();
        dashboard.register_specialist("spec_1".to_string(), "Odin".to_string());

        assert_eq!(dashboard.specialist_statuses.len(), 1);
        assert!(dashboard.get_specialist_status("spec_1").is_some());
    }

    #[test]
    fn test_crisis_capability_building() {
        let mut dashboard = CapabilityDashboard::new();
        dashboard.register_specialist("spec_1".to_string(), "Odin".to_string());
        dashboard.register_specialist("spec_2".to_string(), "Merlin".to_string());

        let team = dashboard.build_crisis_team(3.5, 2);
        assert_eq!(team.len(), 2);
    }

    #[test]
    fn test_dashboard_query() {
        let dashboard = CapabilityDashboard::new();
        let query = DashboardQuery::new()
            .rank(SoulRank::Rank4Master)
            .min_xp(10000);

        let results = query.execute(&dashboard);
        assert_eq!(results.len(), 0); // No specialists registered
    }

    #[test]
    fn test_health_check() {
        let dashboard = CapabilityDashboard::new();
        let health = dashboard.health_check();

        assert!(!health.operational);
        assert!(!health.issues.is_empty());
    }
}
