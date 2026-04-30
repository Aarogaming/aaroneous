// Goal-Driven Autonomy System
// Enables specialists to set, pursue, and achieve their own goals autonomously

use crate::specialist_memory::Goal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, debug};

/// Autonomous goal for a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousGoal {
    pub id: String,
    pub specialist_id: String,
    pub title: String,
    pub description: String,
    pub category: GoalCategory,
    pub priority: GoalPriority,
    pub target_metrics: HashMap<String, f32>, // e.g., "xp" -> 1000.0
    pub progress: f32, // 0.0-1.0
    pub status: AutonomousGoalStatus,
    pub milestones: Vec<Milestone>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub target_completion: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalCategory {
    SkillDevelopment,
    XPThreshold,
    Collaboration,
    Specialization,
    MentorshipGiving,
    MentorshipReceiving,
    TaskCompletion,
    Innovation,
}

impl std::fmt::Display for GoalCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalCategory::SkillDevelopment => write!(f, "SkillDevelopment"),
            GoalCategory::XPThreshold => write!(f, "XPThreshold"),
            GoalCategory::Collaboration => write!(f, "Collaboration"),
            GoalCategory::Specialization => write!(f, "Specialization"),
            GoalCategory::MentorshipGiving => write!(f, "MentorshipGiving"),
            GoalCategory::MentorshipReceiving => write!(f, "MentorshipReceiving"),
            GoalCategory::TaskCompletion => write!(f, "TaskCompletion"),
            GoalCategory::Innovation => write!(f, "Innovation"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoalPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for GoalPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalPriority::Low => write!(f, "Low"),
            GoalPriority::Medium => write!(f, "Medium"),
            GoalPriority::High => write!(f, "High"),
            GoalPriority::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutonomousGoalStatus {
    Planning,
    Active,
    InProgress,
    OnTrack,
    AtRisk,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for AutonomousGoalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutonomousGoalStatus::Planning => write!(f, "Planning"),
            AutonomousGoalStatus::Active => write!(f, "Active"),
            AutonomousGoalStatus::InProgress => write!(f, "InProgress"),
            AutonomousGoalStatus::OnTrack => write!(f, "OnTrack"),
            AutonomousGoalStatus::AtRisk => write!(f, "AtRisk"),
            AutonomousGoalStatus::Paused => write!(f, "Paused"),
            AutonomousGoalStatus::Completed => write!(f, "Completed"),
            AutonomousGoalStatus::Failed => write!(f, "Failed"),
            AutonomousGoalStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Milestone toward goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target_value: f32,
    pub current_value: f32,
    pub completed: bool,
    pub completion_date: Option<chrono::DateTime<chrono::Utc>>,
}

impl Milestone {
    /// Calculate progress as percentage
    pub fn progress_percentage(&self) -> f32 {
        if self.target_value > 0.0 {
            ((self.current_value / self.target_value) * 100.0).min(100.0)
        } else {
            0.0
        }
    }
}

/// Goal-driven autonomy engine
pub struct GoalDrivenAutonomyEngine {
    goals: HashMap<String, AutonomousGoal>,
    goal_history: HashMap<String, Vec<AutonomousGoal>>, // by specialist_id
    active_goals: Vec<String>, // ordered by priority
}

impl GoalDrivenAutonomyEngine {
    /// Create new goal engine
    pub fn new() -> Self {
        Self {
            goals: HashMap::new(),
            goal_history: HashMap::new(),
            active_goals: vec![],
        }
    }

    /// Create autonomous goal
    pub fn create_goal(
        &mut self,
        specialist_id: String,
        title: String,
        description: String,
        category: GoalCategory,
        priority: GoalPriority,
        target_metrics: HashMap<String, f32>,
        milestones: Vec<Milestone>,
        target_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> String {
        let goal_id = format!("goal-{}", uuid::Uuid::new_v4());

        let goal = AutonomousGoal {
            id: goal_id.clone(),
            specialist_id: specialist_id.clone(),
            title,
            description,
            category,
            priority,
            target_metrics,
            progress: 0.0,
            status: AutonomousGoalStatus::Planning,
            milestones,
            created_at: chrono::Utc::now(),
            target_completion: target_date,
        };

        info!("Goal created: {} for specialist {}", goal_id, specialist_id);

        self.goals.insert(goal_id.clone(), goal);
        self.reorder_goals();

        goal_id
    }

    /// Activate goal
    pub fn activate_goal(&mut self, goal_id: &str) -> bool {
        if let Some(goal) = self.goals.get_mut(goal_id) {
            goal.status = AutonomousGoalStatus::Active;
            self.active_goals.push(goal_id.to_string());
            self.reorder_goals();
            info!("Goal activated: {}", goal_id);
            true
        } else {
            false
        }
    }

    /// Update goal progress
    pub fn update_goal_progress(&mut self, goal_id: &str, new_progress: f32) -> bool {
        if let Some(goal) = self.goals.get_mut(goal_id) {
            goal.progress = new_progress.clamp(0.0, 1.0);

            // Update status based on progress
            if goal.progress >= 1.0 {
                goal.status = AutonomousGoalStatus::Completed;
            } else if goal.progress >= 0.8 {
                goal.status = AutonomousGoalStatus::OnTrack;
            } else if goal.progress >= 0.2 {
                goal.status = AutonomousGoalStatus::InProgress;
            } else if goal.progress < 0.2 && goal.status == AutonomousGoalStatus::Active {
                goal.status = AutonomousGoalStatus::AtRisk;
            }

            debug!("Goal {} progress updated to {:.1}%", goal_id, goal.progress * 100.0);
            true
        } else {
            false
        }
    }

    /// Update milestone
    pub fn update_milestone(
        &mut self,
        goal_id: &str,
        milestone_id: &str,
        new_value: f32,
    ) -> bool {
        if let Some(goal) = self.goals.get_mut(goal_id) {
            for milestone in &mut goal.milestones {
                if milestone.id == milestone_id {
                    milestone.current_value = new_value.clamp(0.0, milestone.target_value);
                    if milestone.current_value >= milestone.target_value {
                        milestone.completed = true;
                        milestone.completion_date = Some(chrono::Utc::now());
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Complete goal
    pub fn complete_goal(&mut self, goal_id: &str) -> bool {
        if let Some(goal) = self.goals.get_mut(goal_id) {
            goal.status = AutonomousGoalStatus::Completed;
            goal.progress = 1.0;
            
            // Move to history
            let goal_clone = goal.clone();
            self.goal_history
                .entry(goal.specialist_id.clone())
                .or_insert_with(Vec::new)
                .push(goal_clone);

            // Remove from active
            self.active_goals.retain(|id| id != goal_id);

            info!("Goal completed: {}", goal_id);
            true
        } else {
            false
        }
    }

    /// Fail goal
    pub fn fail_goal(&mut self, goal_id: &str, reason: &str) -> bool {
        if let Some(goal) = self.goals.get_mut(goal_id) {
            goal.status = AutonomousGoalStatus::Failed;
            info!("Goal failed: {} ({})", goal_id, reason);

            // Move to history
            let goal_clone = goal.clone();
            self.goal_history
                .entry(goal.specialist_id.clone())
                .or_insert_with(Vec::new)
                .push(goal_clone);

            // Remove from active
            self.active_goals.retain(|id| id != goal_id);
            true
        } else {
            false
        }
    }

    /// Get goal
    pub fn get_goal(&self, goal_id: &str) -> Option<&AutonomousGoal> {
        self.goals.get(goal_id)
    }

    /// Get goals for specialist
    pub fn get_specialist_goals(&self, specialist_id: &str) -> Vec<&AutonomousGoal> {
        self.goals
            .values()
            .filter(|g| g.specialist_id == specialist_id)
            .collect()
    }

    /// Get active goals for specialist
    pub fn get_active_specialist_goals(&self, specialist_id: &str) -> Vec<&AutonomousGoal> {
        self.goals
            .values()
            .filter(|g| {
                g.specialist_id == specialist_id
                    && (g.status == AutonomousGoalStatus::Active
                        || g.status == AutonomousGoalStatus::InProgress
                        || g.status == AutonomousGoalStatus::OnTrack
                        || g.status == AutonomousGoalStatus::AtRisk)
            })
            .collect()
    }

    /// Get next goal to pursue
    pub fn get_next_goal(&self, specialist_id: &str) -> Option<&AutonomousGoal> {
        let mut goals: Vec<_> = self
            .goals
            .values()
            .filter(|g| {
                g.specialist_id == specialist_id
                    && (g.status == AutonomousGoalStatus::Active || g.status == AutonomousGoalStatus::InProgress)
            })
            .collect();

        // Sort by priority
        goals.sort_by(|a, b| b.priority.cmp(&a.priority));
        goals.first().copied()
    }

    /// Calculate specialist autonomy index (0.0-1.0)
    pub fn autonomy_index(&self, specialist_id: &str) -> f32 {
        let active_count = self
            .active_goals
            .iter()
            .filter(|id| {
                self.goals
                    .get(*id)
                    .map(|g| g.specialist_id == specialist_id)
                    .unwrap_or(false)
            })
            .count();

        let completed_count = self
            .goal_history
            .get(specialist_id)
            .map(|goals| goals.iter().filter(|g| g.status == AutonomousGoalStatus::Completed).count())
            .unwrap_or(0);

        let total = active_count + completed_count;
        if total == 0 {
            0.0
        } else {
            (completed_count as f32 / total as f32).min(1.0)
        }
    }

    /// Reorder goals by priority
    fn reorder_goals(&mut self) {
        self.active_goals.sort_by(|a, b| {
            let goal_a = self.goals.get(a).map(|g| g.priority).unwrap_or(GoalPriority::Low);
            let goal_b = self.goals.get(b).map(|g| g.priority).unwrap_or(GoalPriority::Low);
            goal_b.cmp(&goal_a) // Higher priority first
        });
    }
}

impl Default for GoalDrivenAutonomyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_creation() {
        let mut engine = GoalDrivenAutonomyEngine::new();
        let goal_id = engine.create_goal(
            "spec-1".to_string(),
            "Learn Rust".to_string(),
            "Master Rust programming".to_string(),
            GoalCategory::SkillDevelopment,
            GoalPriority::High,
            vec![("rust_xp".to_string(), 1000.0)].into_iter().collect(),
            vec![],
            None,
        );

        assert!(!goal_id.is_empty());
        assert!(engine.get_goal(&goal_id).is_some());
    }

    #[test]
    fn test_goal_activation() {
        let mut engine = GoalDrivenAutonomyEngine::new();
        let goal_id = engine.create_goal(
            "spec-1".to_string(),
            "Test Goal".to_string(),
            "Description".to_string(),
            GoalCategory::SkillDevelopment,
            GoalPriority::Medium,
            HashMap::new(),
            vec![],
            None,
        );

        assert!(engine.activate_goal(&goal_id));
        let goal = engine.get_goal(&goal_id).unwrap();
        assert_eq!(goal.status, AutonomousGoalStatus::Active);
    }

    #[test]
    fn test_goal_progress() {
        let mut engine = GoalDrivenAutonomyEngine::new();
        let goal_id = engine.create_goal(
            "spec-1".to_string(),
            "Progress Test".to_string(),
            "Test".to_string(),
            GoalCategory::XPThreshold,
            GoalPriority::Low,
            HashMap::new(),
            vec![],
            None,
        );

        engine.activate_goal(&goal_id);
        engine.update_goal_progress(&goal_id, 0.5);

        let goal = engine.get_goal(&goal_id).unwrap();
        assert_eq!(goal.progress, 0.5);
        assert_eq!(goal.status, AutonomousGoalStatus::InProgress);
    }

    #[test]
    fn test_milestone_progress() {
        let milestone = Milestone {
            id: "m1".to_string(),
            name: "First Step".to_string(),
            description: "Do thing".to_string(),
            target_value: 100.0,
            current_value: 50.0,
            completed: false,
            completion_date: None,
        };

        assert_eq!(milestone.progress_percentage(), 50.0);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(GoalPriority::Critical > GoalPriority::High);
        assert!(GoalPriority::High > GoalPriority::Medium);
        assert!(GoalPriority::Medium > GoalPriority::Low);
    }

    #[test]
    fn test_autonomy_index() {
        let mut engine = GoalDrivenAutonomyEngine::new();

        let goal_id = engine.create_goal(
            "spec-1".to_string(),
            "Goal 1".to_string(),
            "Test".to_string(),
            GoalCategory::SkillDevelopment,
            GoalPriority::High,
            HashMap::new(),
            vec![],
            None,
        );

        engine.activate_goal(&goal_id);
        engine.complete_goal(&goal_id);

        let index = engine.autonomy_index("spec-1");
        assert!(index > 0.0);
    }

    #[test]
    fn test_goal_completion() {
        let mut engine = GoalDrivenAutonomyEngine::new();
        let goal_id = engine.create_goal(
            "spec-1".to_string(),
            "Complete Me".to_string(),
            "Test".to_string(),
            GoalCategory::TaskCompletion,
            GoalPriority::Critical,
            HashMap::new(),
            vec![],
            None,
        );

        engine.activate_goal(&goal_id);
        assert!(engine.complete_goal(&goal_id));

        let goal = engine.get_goal(&goal_id).unwrap();
        assert_eq!(goal.status, AutonomousGoalStatus::Completed);
        assert_eq!(goal.progress, 1.0);
    }
}
