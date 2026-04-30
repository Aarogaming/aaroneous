// Specialist Memory System
// Tracks lessons, strategies, decisions, and reasoning outcomes
// Enables specialists to learn from experience and improve decision-making

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Memory entry types for specialist learning
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    Lesson,      // Something learned from experience
    Strategy,    // A proven approach or technique
    Decision,    // A decision made and its outcome
    Reflection,  // Meta-cognitive analysis
    Goal,        // An objective the specialist is pursuing
    Failure,     // A mistake and what was learned
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Lesson => write!(f, "Lesson"),
            MemoryType::Strategy => write!(f, "Strategy"),
            MemoryType::Decision => write!(f, "Decision"),
            MemoryType::Reflection => write!(f, "Reflection"),
            MemoryType::Goal => write!(f, "Goal"),
            MemoryType::Failure => write!(f, "Failure"),
        }
    }
}

/// Confidence level for memory entries
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Confidence {
    Low = 1,
    Medium = 2,
    High = 3,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::Low => write!(f, "Low"),
            Confidence::Medium => write!(f, "Medium"),
            Confidence::High => write!(f, "High"),
        }
    }
}

/// A single memory entry in a specialist's experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub specialist_id: String,
    pub memory_type: MemoryType,
    pub title: String,
    pub description: String,
    pub context: String, // Task, situation, or trigger that caused this memory
    pub confidence: Confidence,
    pub relevance_score: f32, // 0.0-1.0: How relevant this memory still is
    pub usage_count: u32, // How many times this memory was referenced
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>, // For searching and categorizing
    pub related_memories: Vec<String>, // IDs of related memory entries
    pub source: MemorySource, // Where did this memory come from?
}

impl MemoryEntry {
    pub fn new(
        specialist_id: String,
        memory_type: MemoryType,
        title: String,
        description: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            specialist_id,
            memory_type,
            title,
            description,
            context: String::new(),
            confidence: Confidence::Medium,
            relevance_score: 1.0,
            usage_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: Vec::new(),
            related_memories: Vec::new(),
            source: MemorySource::Experience,
        }
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = context;
        self
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_source(mut self, source: MemorySource) -> Self {
        self.source = source;
        self
    }
}

/// Where did this memory originate?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemorySource {
    Experience,           // From doing tasks
    LLMReasoning,         // Generated from LLM analysis
    PeerLearning,         // From another specialist
    Configuration,        // From initial setup
    ErrorRecovery,        // From handling a failure
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySource::Experience => write!(f, "Experience"),
            MemorySource::LLMReasoning => write!(f, "LLM Reasoning"),
            MemorySource::PeerLearning => write!(f, "Peer Learning"),
            MemorySource::Configuration => write!(f, "Configuration"),
            MemorySource::ErrorRecovery => write!(f, "Error Recovery"),
        }
    }
}

/// A decision made by a specialist and its outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub specialist_id: String,
    pub decision: String,
    pub reasoning: String, // Why this decision was made
    pub alternatives_considered: Vec<String>,
    pub outcome: Option<DecisionOutcome>,
    pub confidence_before: Confidence,
    pub confidence_after: Option<Confidence>, // Updated after seeing outcome
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub success: bool,
    pub description: String,
    pub lessons_learned: Vec<String>,
    pub recorded_at: DateTime<Utc>,
}

impl DecisionRecord {
    pub fn new(
        specialist_id: String,
        decision: String,
        reasoning: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            specialist_id,
            decision,
            reasoning,
            alternatives_considered: Vec::new(),
            outcome: None,
            confidence_before: Confidence::Medium,
            confidence_after: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_outcome(mut self, success: bool, description: String) -> Self {
        self.outcome = Some(DecisionOutcome {
            success,
            description,
            lessons_learned: Vec::new(),
            recorded_at: Utc::now(),
        });
        self.updated_at = Utc::now();
        self
    }
}

/// Strategy for approaching a type of task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: String,
    pub specialist_id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<StrategyStep>,
    pub effectiveness_score: f32, // 0.0-1.0: How well has this worked?
    pub success_count: u32,
    pub failure_count: u32,
    pub applicable_to: Vec<String>, // Task types or domains
    pub prerequisites: Vec<String>, // What skills/knowledge needed
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStep {
    pub sequence: u32,
    pub action: String,
    pub expected_outcome: String,
    pub fallback: Option<String>,
}

impl Strategy {
    pub fn new(specialist_id: String, name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            specialist_id,
            name,
            description,
            steps: Vec::new(),
            effectiveness_score: 0.5,
            success_count: 0,
            failure_count: 0,
            applicable_to: Vec::new(),
            prerequisites: Vec::new(),
            created_at: now,
            last_used: now,
        }
    }

    pub fn add_step(mut self, action: String, expected_outcome: String) -> Self {
        let sequence = self.steps.len() as u32 + 1;
        self.steps.push(StrategyStep {
            sequence,
            action,
            expected_outcome,
            fallback: None,
        });
        self
    }

    pub fn record_success(mut self) -> Self {
        self.success_count += 1;
        self.recalculate_effectiveness();
        self
    }

    pub fn record_failure(mut self) -> Self {
        self.failure_count += 1;
        self.recalculate_effectiveness();
        self
    }

    fn recalculate_effectiveness(&mut self) {
        let total = (self.success_count + self.failure_count) as f32;
        if total > 0.0 {
            self.effectiveness_score = self.success_count as f32 / total;
        }
    }
}

/// A goal a specialist is pursuing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub specialist_id: String,
    pub objective: String,
    pub reason: String, // Why is this goal important?
    pub status: GoalStatus,
    pub priority: u32, // 1-10: higher is more important
    pub created_at: DateTime<Utc>,
    pub target_completion: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_percentage: u32, // 0-100
    pub blockers: Vec<String>, // Things preventing progress
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalStatus {
    Planning,
    Active,
    Blocked,
    Paused,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub description: String,
    pub target_date: Option<DateTime<Utc>>,
    pub completed: bool,
}

impl Goal {
    pub fn new(specialist_id: String, objective: String, reason: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            specialist_id,
            objective,
            reason,
            status: GoalStatus::Planning,
            priority: 5,
            created_at: Utc::now(),
            target_completion: None,
            completed_at: None,
            progress_percentage: 0,
            blockers: Vec::new(),
            milestones: Vec::new(),
        }
    }

    pub fn activate(mut self) -> Self {
        self.status = GoalStatus::Active;
        self
    }

    pub fn complete(mut self) -> Self {
        self.status = GoalStatus::Completed;
        self.progress_percentage = 100;
        self.completed_at = Some(Utc::now());
        self
    }

    pub fn add_blocker(mut self, blocker: String) -> Self {
        self.blockers.push(blocker);
        if self.status == GoalStatus::Active {
            self.status = GoalStatus::Blocked;
        }
        self
    }
}

/// Complete memory system for a specialist
pub struct SpecialistMemory {
    pub specialist_id: String,
    pub memories: HashMap<String, MemoryEntry>,
    pub decisions: HashMap<String, DecisionRecord>,
    pub strategies: HashMap<String, Strategy>,
    pub goals: HashMap<String, Goal>,
}

impl SpecialistMemory {
    pub fn new(specialist_id: String) -> Self {
        Self {
            specialist_id,
            memories: HashMap::new(),
            decisions: HashMap::new(),
            strategies: HashMap::new(),
            goals: HashMap::new(),
        }
    }

    /// Add a new memory entry
    pub fn record_memory(&mut self, entry: MemoryEntry) -> String {
        let id = entry.id.clone();
        self.memories.insert(id.clone(), entry);
        id
    }

    /// Add a decision record
    pub fn record_decision(&mut self, decision: DecisionRecord) -> String {
        let id = decision.id.clone();
        self.decisions.insert(id.clone(), decision);
        id
    }

    /// Add or update a strategy
    pub fn save_strategy(&mut self, strategy: Strategy) -> String {
        let id = strategy.id.clone();
        self.strategies.insert(id.clone(), strategy);
        id
    }

    /// Add or update a goal
    pub fn save_goal(&mut self, goal: Goal) -> String {
        let id = goal.id.clone();
        self.goals.insert(id.clone(), goal);
        id
    }

    /// Retrieve memory entries by type
    pub fn get_memories_by_type(&self, memory_type: MemoryType) -> Vec<&MemoryEntry> {
        self.memories
            .values()
            .filter(|m| m.memory_type == memory_type)
            .collect()
    }

    /// Retrieve memory entries by tag
    pub fn search_memories(&self, tag: &str) -> Vec<&MemoryEntry> {
        self.memories
            .values()
            .filter(|m| m.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get all active goals
    pub fn get_active_goals(&self) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|g| g.status == GoalStatus::Active)
            .collect()
    }

    /// Get strategies by task type
    pub fn get_strategies_for_task(&self, task_type: &str) -> Vec<&Strategy> {
        self.strategies
            .values()
            .filter(|s| s.applicable_to.contains(&task_type.to_string()))
            .collect()
    }

    /// Get most effective strategy for a task
    pub fn get_best_strategy(&self, task_type: &str) -> Option<&Strategy> {
        self.get_strategies_for_task(task_type)
            .into_iter()
            .max_by(|a, b| a.effectiveness_score.partial_cmp(&b.effectiveness_score).unwrap())
    }

    /// Get recent decisions with their outcomes
    pub fn get_recent_decisions(&self, limit: usize) -> Vec<&DecisionRecord> {
        let mut decisions: Vec<_> = self.decisions.values().collect();
        decisions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        decisions.into_iter().take(limit).collect()
    }

    /// Calculate memory utilization (for efficiency metrics)
    pub fn get_memory_stats(&self) -> MemoryStats {
        let total_memories = self.memories.len();
        let lessons = self.get_memories_by_type(MemoryType::Lesson).len();
        let strategies = self.strategies.len();
        let active_goals = self.get_active_goals().len();
        let decisions_with_outcomes = self.decisions.values().filter(|d| d.outcome.is_some()).count();

        MemoryStats {
            total_memories,
            lessons,
            strategies,
            active_goals,
            decisions_with_outcomes,
            memory_health: self.calculate_health(),
        }
    }

    fn calculate_health(&self) -> f32 {
        // Health based on memory diversity and usage
        let memory_diversity = (self.memories.len() as f32 / 100.0).min(1.0);
        let strategy_effectiveness = if self.strategies.is_empty() {
            0.5
        } else {
            self.strategies.values().map(|s| s.effectiveness_score).sum::<f32>()
                / self.strategies.len() as f32
        };
        let goal_progress = if self.goals.is_empty() {
            0.5
        } else {
            self.goals.values().map(|g| g.progress_percentage as f32 / 100.0).sum::<f32>()
                / self.goals.len() as f32
        };

        (memory_diversity * 0.3 + strategy_effectiveness * 0.4 + goal_progress * 0.3).min(1.0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub lessons: usize,
    pub strategies: usize,
    pub active_goals: usize,
    pub decisions_with_outcomes: usize,
    pub memory_health: f32, // 0.0-1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_memory_entry() {
        let entry = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Pattern Recognition".to_string(),
            "Learned how to identify recurring patterns in data".to_string(),
        );

        assert_eq!(entry.specialist_id, "spec-1");
        assert_eq!(entry.memory_type, MemoryType::Lesson);
        assert_eq!(entry.confidence, Confidence::Medium);
    }

    #[test]
    fn test_strategy_effectiveness() {
        let mut strategy = Strategy::new(
            "spec-1".to_string(),
            "Quick Analysis".to_string(),
            "Fast data processing".to_string(),
        );

        for _ in 0..8 {
            strategy = strategy.record_success();
        }
        for _ in 0..2 {
            strategy = strategy.record_failure();
        }

        assert_eq!(strategy.effectiveness_score, 0.8);
    }

    #[test]
    fn test_goal_lifecycle() {
        let goal = Goal::new(
            "spec-1".to_string(),
            "Master Task Decomposition".to_string(),
            "Improve task analysis quality".to_string(),
        )
        .activate();

        assert_eq!(goal.status, GoalStatus::Active);
        assert_eq!(goal.progress_percentage, 0);

        let goal = goal.complete();
        assert_eq!(goal.status, GoalStatus::Completed);
        assert_eq!(goal.progress_percentage, 100);
    }

    #[test]
    fn test_specialist_memory_operations() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        // Add a lesson
        let entry = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Lesson 1".to_string(),
            "Description".to_string(),
        )
        .with_tags(vec!["data-processing".to_string()]);

        memory.record_memory(entry);

        // Search by tag
        let results = memory.search_memories("data-processing");
        assert_eq!(results.len(), 1);

        // Check stats
        let stats = memory.get_memory_stats();
        assert_eq!(stats.total_memories, 1);
        assert_eq!(stats.lessons, 1);
    }

    #[test]
    fn test_decision_recording() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let decision = DecisionRecord::new(
            "spec-1".to_string(),
            "Use clustering algorithm".to_string(),
            "Data appears to have natural groupings".to_string(),
        );

        memory.record_decision(decision);
        assert_eq!(memory.decisions.len(), 1);

        let recent = memory.get_recent_decisions(10);
        assert_eq!(recent.len(), 1);
    }
}
