// Autonomous Planning Engine
// Generates detailed execution plans from task analysis and specialist capabilities

use crate::capability_matching_v2::SpecialistCapabilityMatch;
use crate::llm::{LLMClient, TaskAnalysis, ExecutionPlan, PlanStep};
use crate::task_analysis::TaskAnalysisResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Autonomous execution plan for task completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousPlan {
    pub plan_id: String,
    pub task_id: String,
    pub primary_specialist: String,
    pub steps: Vec<ExecutionStep>,
    pub estimated_duration_minutes: u32,
    pub success_probability: f32,
    pub contingencies: Vec<Contingency>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Detailed execution step with validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub sequence: u32,
    pub action: String,
    pub expected_outcome: String,
    pub estimated_time_minutes: u32,
    pub required_skills: Vec<String>,
    pub validation_criteria: Vec<String>,
    pub rollback_action: Option<String>,
}

/// Contingency plan for potential failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contingency {
    pub failure_scenario: String,
    pub likelihood: f32, // 0.0-1.0
    pub recovery_steps: Vec<String>,
    pub escalation_required: bool,
}

/// Planning engine
pub struct AutonomousPlanningEngine {
    llm_client: Arc<LLMClient>,
}

impl AutonomousPlanningEngine {
    /// Create new planning engine
    pub fn new(llm_client: Arc<LLMClient>) -> Self {
        Self { llm_client }
    }

    /// Generate autonomous plan from task analysis and capability match
    pub async fn generate_plan(
        &self,
        task_result: &TaskAnalysisResult,
        primary_match: &SpecialistCapabilityMatch,
        supporting_matches: &[SpecialistCapabilityMatch],
    ) -> anyhow::Result<AutonomousPlan> {
        let start = Instant::now();
        info!(
            "Generating autonomous plan for task: {} with specialist: {}",
            task_result.task_id, primary_match.specialist_name
        );

        // Call the LLM to generate a real execution plan.
        // Build the specialist context from the primary match.
        let specialist_context = crate::llm::types::SpecialistContext {
            name: primary_match.specialist_name.clone(),
            archetype: primary_match.specialist_name.clone(),
            rank: 1,
            xp: 0,
            skills: primary_match.matching_skills.iter()
                .map(|s| crate::llm::types::SkillInfo {
                    name: s.clone(),
                    level: 1,
                    is_awakened: false,
                })
                .collect(),
            recent_lessons: vec![],
            collaboration_history: vec![],
            current_goal: Some(task_result.analysis.recommended_approach.clone()),
        };

        let llm_plan = match self.llm_client
            .generate_plan(&task_result.analysis, &specialist_context)
            .await
        {
            Ok(plan) => plan,
            Err(e) => {
                // LLM call failed — fall back to the synthetic plan so the
                // coordinator can still make progress.
                warn!("LLM generate_plan failed: {}, using synthetic fallback", e);
                ExecutionPlan {
                    task_id: task_result.task_id.clone(),
                    specialist_name: primary_match.specialist_name.clone(),
                    steps: task_result.analysis
                        .suggested_collaborators
                        .iter()
                        .enumerate()
                        .map(|(i, _)| PlanStep {
                            sequence: (i + 1) as u32,
                            description: format!("Execute step {} (synthetic)", i + 1),
                            estimated_time_minutes: task_result.analysis.estimated_time_minutes /
                                task_result.analysis.suggested_collaborators.len().max(1) as u32,
                            required_skills: vec![],
                            checkpoints: vec![],
                        })
                        .collect(),
                    total_estimated_time: task_result.analysis.estimated_time_minutes,
                    success_probability: task_result.analysis.confidence_percentage as f32 / 100.0,
                    reasoning: task_result.analysis.reasoning.clone(),
                }
            }
        };

        // Convert LLM plan to detailed execution steps
        let steps = self.convert_to_execution_steps(&llm_plan, primary_match);

        // Generate contingency plans
        let contingencies =
            self.generate_contingencies(&task_result.analysis, &steps);

        // Calculate success probability
        let success_probability =
            self.calculate_success_probability(primary_match, supporting_matches);

        let plan = AutonomousPlan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            task_id: task_result.task_id.clone(),
            primary_specialist: primary_match.specialist_name.clone(),
            steps,
            estimated_duration_minutes: llm_plan.total_estimated_time,
            success_probability,
            contingencies,
            created_at: chrono::Utc::now(),
        };

        info!(
            "Plan generated in {}ms with {:.0}% success probability",
            start.elapsed().as_millis(),
            plan.success_probability * 100.0
        );

        Ok(plan)
    }

    /// Convert LLM execution plan to detailed execution steps
    fn convert_to_execution_steps(
        &self,
        llm_plan: &ExecutionPlan,
        specialist: &SpecialistCapabilityMatch,
    ) -> Vec<ExecutionStep> {
        llm_plan
            .steps
            .iter()
            .map(|step| ExecutionStep {
                sequence: step.sequence,
                action: step.description.clone(),
                expected_outcome: format!(
                    "Complete step {}: {}",
                    step.sequence, step.description
                ),
                estimated_time_minutes: step.estimated_time_minutes,
                required_skills: step.required_skills.clone(),
                validation_criteria: step.checkpoints.clone(),
                rollback_action: None,
            })
            .collect()
    }

    /// Generate contingency plans for potential failures
    fn generate_contingencies(
        &self,
        analysis: &TaskAnalysis,
        steps: &[ExecutionStep],
    ) -> Vec<Contingency> {
        let mut contingencies = Vec::new();

        // Add contingencies based on task risks
        for risk in &analysis.potential_risks {
            contingencies.push(Contingency {
                failure_scenario: format!("Risk: {}", risk),
                likelihood: 0.3,
                recovery_steps: vec![
                    format!("Re-assess task given: {}", risk),
                    "Consult with support specialist".to_string(),
                    "Execute alternative approach".to_string(),
                ],
                escalation_required: false,
            });
        }

        // Add step-level contingencies
        for step in steps.iter().skip(1) {
            contingencies.push(Contingency {
                failure_scenario: format!("Step {} validation fails", step.sequence),
                likelihood: 0.2,
                recovery_steps: vec![
                    format!("Review output from step {}", step.sequence - 1),
                    "Execute step again with adjusted parameters".to_string(),
                    format!("Skip to step {} if recoverable", step.sequence + 1),
                ],
                escalation_required: step.sequence > 3,
            });
        }

        contingencies
    }

    /// Calculate probability of successful task completion
    fn calculate_success_probability(
        &self,
        primary: &SpecialistCapabilityMatch,
        supporting: &[SpecialistCapabilityMatch],
    ) -> f32 {
        // Base on primary specialist's overall score
        let mut prob = primary.overall_score;

        // Boost for supporting specialists
        if !supporting.is_empty() {
            let support_avg =
                supporting.iter().map(|m| m.overall_score).sum::<f32>()
                    / supporting.len() as f32;
            prob = (prob + support_avg * 0.1).min(1.0);
        }

        // Reduce based on missing skills
        let missing_ratio = primary.missing_skills.len() as f32
            / (primary.matching_skills.len() + primary.missing_skills.len()) as f32;
        prob = (prob * (1.0 - missing_ratio * 0.15)).max(0.1);

        prob
    }
}

/// Plan execution tracker
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionTracker {
    pub plan_id: String,
    pub current_step: u32,
    pub steps_completed: u32,
    pub total_steps: u32,
    pub execution_start: chrono::DateTime<chrono::Utc>,
    pub actual_duration_minutes: f32,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Planning,
    InProgress,
    OnTrack,
    AtRisk,
    Failed,
    Completed,
    RolledBack,
}

impl ExecutionTracker {
    pub fn new(plan_id: String, total_steps: u32) -> Self {
        Self {
            plan_id,
            current_step: 0,
            steps_completed: 0,
            total_steps,
            execution_start: chrono::Utc::now(),
            actual_duration_minutes: 0.0,
            status: ExecutionStatus::Planning,
        }
    }

    pub fn start_execution(mut self) -> Self {
        self.status = ExecutionStatus::InProgress;
        self
    }

    pub fn mark_step_complete(mut self) -> Self {
        if self.current_step < self.total_steps {
            self.current_step += 1;
            self.steps_completed += 1;
            
            let elapsed = chrono::Utc::now()
                .signed_duration_since(self.execution_start);
            self.actual_duration_minutes = elapsed.num_seconds() as f32 / 60.0;

            self.status = ExecutionStatus::OnTrack;
        }
        self
    }

    pub fn mark_at_risk(mut self) -> Self {
        if self.status != ExecutionStatus::Failed {
            self.status = ExecutionStatus::AtRisk;
        }
        self
    }

    pub fn mark_failed(mut self) -> Self {
        self.status = ExecutionStatus::Failed;
        self
    }

    pub fn mark_completed(mut self) -> Self {
        let elapsed = chrono::Utc::now()
            .signed_duration_since(self.execution_start);
        self.actual_duration_minutes = elapsed.num_seconds() as f32 / 60.0;
        self.steps_completed = self.total_steps;
        self.current_step = self.total_steps;
        self.status = ExecutionStatus::Completed;
        self
    }

    pub fn progress_percentage(&self) -> f32 {
        if self.total_steps == 0 {
            100.0
        } else {
            (self.steps_completed as f32 / self.total_steps as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_tracker_progress() {
        let mut tracker = ExecutionTracker::new("plan-1".to_string(), 10);
        assert_eq!(tracker.progress_percentage(), 0.0);

        for _ in 0..5 {
            tracker = tracker.mark_step_complete();
        }
        assert_eq!(tracker.progress_percentage(), 50.0);

        tracker = tracker.mark_completed();
        assert_eq!(tracker.progress_percentage(), 100.0);
    }

    #[test]
    fn test_execution_status_transitions() {
        let tracker = ExecutionTracker::new("plan-1".to_string(), 5);
        assert_eq!(tracker.status, ExecutionStatus::Planning);

        let tracker = tracker.start_execution();
        assert_eq!(tracker.status, ExecutionStatus::InProgress);

        let tracker = tracker.mark_step_complete();
        assert_eq!(tracker.status, ExecutionStatus::OnTrack);

        let tracker = tracker.mark_at_risk();
        assert_eq!(tracker.status, ExecutionStatus::AtRisk);

        let tracker = tracker.mark_failed();
        assert_eq!(tracker.status, ExecutionStatus::Failed);
    }

    #[test]
    fn test_contingency_plan() {
        let contingency = Contingency {
            failure_scenario: "Network timeout".to_string(),
            likelihood: 0.25,
            recovery_steps: vec![
                "Retry operation".to_string(),
                "Use cached result".to_string(),
            ],
            escalation_required: false,
        };

        assert_eq!(contingency.likelihood, 0.25);
        assert_eq!(contingency.recovery_steps.len(), 2);
    }

    #[test]
    fn test_execution_step() {
        let step = ExecutionStep {
            sequence: 1,
            action: "Validate input".to_string(),
            expected_outcome: "Input passes validation".to_string(),
            estimated_time_minutes: 2,
            required_skills: vec!["Validation".to_string()],
            validation_criteria: vec!["No errors".to_string()],
            rollback_action: None,
        };

        assert_eq!(step.sequence, 1);
        assert_eq!(step.required_skills.len(), 1);
    }

    #[test]
    fn test_autonomous_plan_creation() {
        let plan = AutonomousPlan {
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
            primary_specialist: "Merlin".to_string(),
            steps: vec![],
            estimated_duration_minutes: 30,
            success_probability: 0.85,
            contingencies: vec![],
            created_at: chrono::Utc::now(),
        };

        assert_eq!(plan.plan_id, "plan-1");
        assert_eq!(plan.success_probability, 0.85);
    }
}
