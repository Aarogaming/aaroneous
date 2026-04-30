// Specialist Memory Reflection Engine
// Generates lessons and insights from LLM reasoning outcomes
// Enables specialists to learn and improve from each task execution

use crate::llm::{LLMClient, TaskAnalysis, FailureAnalysis};
use crate::specialist_memory::{
    MemoryEntry, MemoryType, Confidence, MemorySource, DecisionRecord, Strategy, Goal,
    GoalStatus, StrategyStep, SpecialistMemory,
};
use crate::agents::SpecialistAgent;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Reflection event that triggered learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReflectionTrigger {
    TaskCompleted,          // Task finished successfully
    TaskFailed,             // Task execution failed
    StrategyUsed,           // Strategy was applied
    CollaborationOccurred,  // Worked with another specialist
    MilestoneReached,       // Goal milestone completed
    LessonIdentified,       // Manual lesson recording
}

/// Result of reflection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    pub trigger: ReflectionTrigger,
    pub memories_created: Vec<String>,      // IDs of new memory entries
    pub strategies_updated: Vec<String>,    // IDs of updated strategies
    pub goals_updated: Vec<String>,         // IDs of updated goals
    pub confidence_change: f32,             // How much confidence changed
    pub learning_effectiveness: f32,        // 0.0-1.0: Quality of learning
}

/// Memory Reflection Engine
pub struct MemoryReflectionEngine {
    specialist: SpecialistAgent,
    llm_client: LLMClient,
}

impl MemoryReflectionEngine {
    /// Create new reflection engine
    pub fn new(specialist: SpecialistAgent, llm_client: LLMClient) -> Self {
        Self {
            specialist,
            llm_client,
        }
    }

    /// Reflect on task completion and extract lessons
    pub async fn reflect_on_task_completion(
        &self,
        memory: &mut SpecialistMemory,
        task_id: &str,
        task_analysis: &TaskAnalysis,
        execution_time_ms: u64,
    ) -> Result<ReflectionResult> {
        info!(
            "Specialist {} reflecting on task completion: {}",
            self.specialist.name, task_id
        );

        let mut memories_created = Vec::new();
        let mut strategies_updated = Vec::new();

        // Extract key insights from task analysis
        let lesson_entry = self.create_lesson_from_analysis(
            memory.specialist_id.clone(),
            task_id,
            task_analysis,
            execution_time_ms,
        );

        let lesson_id = lesson_entry.id.clone();
        memory.record_memory(lesson_entry);
        memories_created.push(lesson_id);

        // Try to extract or refine strategy
        if let Some(strategy_id) = self.extract_strategy_from_task(memory, task_analysis) {
            memory.save_strategy(
                memory.strategies.get(&strategy_id).unwrap().clone().record_success(),
            );
            strategies_updated.push(strategy_id);
        }

        Ok(ReflectionResult {
            trigger: ReflectionTrigger::TaskCompleted,
            memories_created,
            strategies_updated,
            goals_updated: Vec::new(),
            confidence_change: 0.1, // Small increase in confidence
            learning_effectiveness: 0.8,
        })
    }

    /// Reflect on task failure and generate recovery strategy
    pub async fn reflect_on_task_failure(
        &self,
        memory: &mut SpecialistMemory,
        task_id: &str,
        failure_analysis: &FailureAnalysis,
    ) -> Result<ReflectionResult> {
        info!(
            "Specialist {} reflecting on task failure: {}",
            self.specialist.name, task_id
        );

        let mut memories_created = Vec::new();
        let mut strategies_updated = Vec::new();

        // Create failure lesson
        let failure_memory = MemoryEntry::new(
            memory.specialist_id.clone(),
            MemoryType::Failure,
            format!("Failed Task: {}", task_id),
            format!("Task failed with: {}\nRecovery: {}", failure_analysis.root_cause, failure_analysis.recovery_approach),
        )
        .with_context(format!("Task ID: {}", task_id))
        .with_tags(vec!["failure".to_string(), task_id.to_string()])
        .with_source(MemorySource::LLMReasoning)
        .with_confidence(Confidence::High);

        let failure_id = failure_memory.id.clone();
        memory.record_memory(failure_memory);
        memories_created.push(failure_id);

        // Create recovery strategy from the suggested new strategy
        if !failure_analysis.new_strategy.is_empty() {
            let recovery_strategy = Strategy::new(
                memory.specialist_id.clone(),
                format!("Recovery from {}", task_id),
                format!("Approach to handle: {}", failure_analysis.root_cause),
            )
            .add_step(
                failure_analysis.new_strategy.clone(),
                "Execute new approach for improved results".to_string()
            )
            .add_step(
                failure_analysis.prevention_strategy.clone(),
                "Prevent this failure in future tasks".to_string()
            );

            let strategy_id = recovery_strategy.id.clone();
            memory.save_strategy(recovery_strategy);
            strategies_updated.push(strategy_id);
        }

        Ok(ReflectionResult {
            trigger: ReflectionTrigger::TaskFailed,
            memories_created,
            strategies_updated,
            goals_updated: Vec::new(),
            confidence_change: -0.05, // Small decrease in confidence
            learning_effectiveness: 0.9, // Failures teach well
        })
    }

    /// Create memory entry from task analysis
    fn create_lesson_from_analysis(
        &self,
        specialist_id: String,
        task_id: &str,
        analysis: &TaskAnalysis,
        execution_time_ms: u64,
    ) -> MemoryEntry {
        let title = format!("Lesson from task {}", task_id);
        let description = format!(
            "Task completed in {}ms. Recommended approach: {}\nReasoning: {}",
            execution_time_ms, analysis.recommended_approach, analysis.reasoning
        );

        MemoryEntry::new(specialist_id, MemoryType::Lesson, title, description)
            .with_context(format!("Task: {}", task_id))
            .with_source(MemorySource::LLMReasoning)
            .with_confidence(Confidence::High)
            .with_tags(vec![
                "task-analysis".to_string(),
                task_id.to_string(),
                "execution".to_string(),
            ])
    }

    /// Try to extract or update a strategy from task analysis
    fn extract_strategy_from_task(
        &self,
        memory: &SpecialistMemory,
        _analysis: &TaskAnalysis,
    ) -> Option<String> {
        // Look for existing strategies that apply to this task type
        memory
            .strategies
            .keys()
            .next()
            .map(|id| id.clone())
    }

    /// Analyze specialist decision patterns and recommend improvements
    pub async fn analyze_decision_patterns(&self, memory: &SpecialistMemory) -> DecisionPattern {
        let recent_decisions = memory.get_recent_decisions(20);

        let total = recent_decisions.len() as f32;
        let successful = recent_decisions
            .iter()
            .filter(|d| {
                d.outcome
                    .as_ref()
                    .map(|o| o.success)
                    .unwrap_or(false)
            })
            .count() as f32;

        let success_rate = if total > 0.0 {
            successful / total
        } else {
            0.5
        };

        // Calculate average confidence from decisions
        let confidence_sum = recent_decisions
            .iter()
            .map(|d| match d.confidence_before {
                Confidence::Low => 1.0,
                Confidence::Medium => 2.0,
                Confidence::High => 3.0,
            })
            .sum::<f32>();

        let average_confidence = if !recent_decisions.is_empty() {
            (confidence_sum / recent_decisions.len() as f32) / 3.0
        } else {
            0.5
        };

        DecisionPattern {
            total_decisions: recent_decisions.len(),
            success_rate,
            average_confidence,
            recommendations: self.generate_recommendations(success_rate),
        }
    }

    fn generate_recommendations(&self, success_rate: f32) -> Vec<String> {
        let mut recs = Vec::new();

        if success_rate < 0.6 {
            recs.push("Consider consulting with other specialists on decision strategy".to_string());
        }

        if success_rate > 0.85 {
            recs.push("Your decision-making is strong - mentor other specialists".to_string());
        }

        if success_rate >= 0.7 && success_rate <= 0.85 {
            recs.push("Good success rate - focus on edge cases for improvement".to_string());
        }

        recs
    }
}

/// Results of pattern analysis
#[derive(Debug, Clone, Serialize)]
pub struct DecisionPattern {
    pub total_decisions: usize,
    pub success_rate: f32,
    pub average_confidence: f32,
    pub recommendations: Vec<String>,
}

/// Extension trait for Strategy to support building from actions
trait StrategyBuilding {
    fn with_strategy_steps_from_actions(self, actions: &[String]) -> Self;
}

impl StrategyBuilding for Strategy {
    fn with_strategy_steps_from_actions(mut self, actions: &[String]) -> Self {
        for (idx, action) in actions.iter().enumerate() {
            self = self.add_step(
                action.clone(),
                format!("Complete: {}", action),
            );
        }
        self
    }
}

/// Learning event recorded when specialist gains experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    pub event_type: String,
    pub specialist_id: String,
    pub description: String,
    pub impact_score: f32, // 0.0-1.0: How much did this improve the specialist?
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl LearningEvent {
    pub fn new(
        specialist_id: String,
        event_type: String,
        description: String,
        impact_score: f32,
    ) -> Self {
        Self {
            event_type,
            specialist_id,
            description,
            impact_score: impact_score.min(1.0).max(0.0),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_pattern_analysis() {
        let pattern = DecisionPattern {
            total_decisions: 20,
            success_rate: 0.75,
            average_confidence: 0.7,
            recommendations: vec!["Improve edge case handling".to_string()],
        };

        assert_eq!(pattern.total_decisions, 20);
        assert!(pattern.success_rate > 0.7 && pattern.success_rate < 0.8);
    }

    #[test]
    fn test_reflection_result() {
        let result = ReflectionResult {
            trigger: ReflectionTrigger::TaskCompleted,
            memories_created: vec!["mem-1".to_string()],
            strategies_updated: vec!["strat-1".to_string()],
            goals_updated: vec![],
            confidence_change: 0.1,
            learning_effectiveness: 0.8,
        };

        assert!(!result.memories_created.is_empty());
        assert!(result.confidence_change > 0.0);
    }

    #[test]
    fn test_learning_event() {
        let event = LearningEvent::new(
            "spec-1".to_string(),
            "task_completion".to_string(),
            "Completed complex task successfully".to_string(),
            0.85,
        );

        assert_eq!(event.specialist_id, "spec-1");
        assert_eq!(event.impact_score, 0.85);
    }
}
