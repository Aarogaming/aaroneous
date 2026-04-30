// Autonomous Task Coordinator
// Orchestrates the full autonomous task pipeline:
// Task → Analysis → Matching → Planning → Execution → Learning

use crate::autonomous_planning::{AutonomousPlanningEngine, AutonomousPlan, ExecutionTracker};
use crate::capability_matching_v2::{CapabilityMatchingEngine, SpecialistCapabilityMatch};
use crate::task_analysis::{Task, TaskAnalysisEngine};
use crate::agents::SpecialistAgent;
use crate::llm::LLMClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Autonomous task coordinator
pub struct AutonomousCoordinator {
    task_analysis_engine: TaskAnalysisEngine,
    matching_engine: CapabilityMatchingEngine,
    planning_engine: AutonomousPlanningEngine,
    llm_client: Arc<LLMClient>,
    active_tasks: HashMap<String, TaskExecutionState>,
}

/// Execution state for a task
#[derive(Debug, Clone, Serialize)]
pub struct TaskExecutionState {
    pub task: Task,
    #[serde(skip)]
    pub analysis: Option<crate::task_analysis::TaskAnalysisResult>,
    pub matches: Option<Vec<SpecialistCapabilityMatch>>,
    pub plan: Option<AutonomousPlan>,
    #[serde(skip)]
    pub tracker: Option<ExecutionTracker>,
    pub status: TaskCoordinationStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskCoordinationStatus {
    Submitted,
    Analyzing,
    AnalysisComplete,
    Matching,
    MatchingComplete,
    Planning,
    PlanningComplete,
    Executing,
    Completed,
    Failed,
}

impl AutonomousCoordinator {
    /// Create new autonomous coordinator
    pub fn new(
        llm_client: Arc<LLMClient>,
        task_analysis_engine: TaskAnalysisEngine,
    ) -> Self {
        Self {
            task_analysis_engine,
            matching_engine: CapabilityMatchingEngine,
            planning_engine: AutonomousPlanningEngine::new(llm_client.clone()),
            llm_client,
            active_tasks: HashMap::new(),
        }
    }

    /// Submit a task to the autonomous pipeline
    pub async fn submit_task(&mut self, task: Task) -> Result<String> {
        let task_id = task.id.clone();
        info!("Submitting autonomous task: {}", task_id);

        self.active_tasks.insert(
            task_id.clone(),
            TaskExecutionState {
                task,
                analysis: None,
                matches: None,
                plan: None,
                tracker: None,
                status: TaskCoordinationStatus::Submitted,
                error_message: None,
            },
        );

        Ok(task_id)
    }

    /// Process a single task through the autonomous pipeline
    pub async fn process_task(
        &mut self,
        task_id: &str,
        available_specialists: &[SpecialistAgent],
    ) -> Result<()> {
        if let Some(state) = self.active_tasks.get_mut(task_id) {
            // Step 1: Analyze task
            state.status = TaskCoordinationStatus::Analyzing;
            debug!("Analyzing task: {}", task_id);

            match self
                .task_analysis_engine
                .analyze_task(&state.task, available_specialists)
                .await
            {
                Ok(analysis) => {
                    state.analysis = Some(analysis.clone());
                    state.status = TaskCoordinationStatus::AnalysisComplete;
                    info!("Task analysis complete: {}", task_id);

                    // Step 2: Match specialists
                    state.status = TaskCoordinationStatus::Matching;
                    debug!("Matching specialists to task: {}", task_id);

                    let matches = CapabilityMatchingEngine::find_matches(
                        &state.task,
                        &analysis.analysis,
                        available_specialists,
                        &[], // No memory yet for pure matching
                    );

                    state.matches = Some(matches.clone());
                    state.status = TaskCoordinationStatus::MatchingComplete;
                    info!("Specialist matching complete: {} matches found", matches.len());

                    // Step 3: Generate plan
                    if !matches.is_empty() {
                        state.status = TaskCoordinationStatus::Planning;
                        debug!("Generating execution plan for task: {}", task_id);

                        let primary_match = &matches[0];
                        let supporting: Vec<SpecialistCapabilityMatch> = matches.iter().skip(1).cloned().collect();

                        match self
                            .planning_engine
                            .generate_plan(&analysis, primary_match, &supporting)
                            .await
                        {
                            Ok(plan) => {
                                state.plan = Some(plan.clone());
                                state.status = TaskCoordinationStatus::PlanningComplete;
                                info!("Execution plan generated: {}", plan.plan_id);

                                // Step 4: Initialize tracking
                                let tracker = ExecutionTracker::new(
                                    plan.plan_id.clone(),
                                    plan.steps.len() as u32,
                                )
                                .start_execution();

                                state.tracker = Some(tracker);
                                state.status = TaskCoordinationStatus::Executing;
                                info!("Task execution started: {}", task_id);
                            }
                            Err(e) => {
                                state.status = TaskCoordinationStatus::Failed;
                                state.error_message = Some(format!("Planning failed: {}", e));
                                warn!("Planning failed for task {}: {}", task_id, e);
                            }
                        }
                    } else {
                        state.status = TaskCoordinationStatus::Failed;
                        state.error_message =
                            Some("No suitable specialists found".to_string());
                        warn!("No specialists matched task: {}", task_id);
                    }
                }
                Err(e) => {
                    state.status = TaskCoordinationStatus::Failed;
                    state.error_message = Some(format!("Analysis failed: {}", e));
                    warn!("Task analysis failed for {}: {}", task_id, e);
                }
            }
        }

        Ok(())
    }

    /// Mark execution step as complete
    pub fn mark_step_complete(&mut self, task_id: &str) {
        if let Some(state) = self.active_tasks.get_mut(task_id) {
            if let Some(mut tracker) = state.tracker.take() {
                tracker = tracker.mark_step_complete();
                state.tracker = Some(tracker);
            }
        }
    }

    /// Mark task as completed
    pub fn mark_task_completed(&mut self, task_id: &str) {
        if let Some(state) = self.active_tasks.get_mut(task_id) {
            if let Some(mut tracker) = state.tracker.take() {
                tracker = tracker.mark_completed();
                state.tracker = Some(tracker);
                state.status = TaskCoordinationStatus::Completed;
                info!("Task completed: {}", task_id);
            }
        }
    }

    /// Mark task as failed
    pub fn mark_task_failed(&mut self, task_id: &str, error: String) {
        if let Some(state) = self.active_tasks.get_mut(task_id) {
            if let Some(mut tracker) = state.tracker.take() {
                tracker = tracker.mark_failed();
                state.tracker = Some(tracker);
            }
            state.status = TaskCoordinationStatus::Failed;
            state.error_message = Some(error);
            warn!("Task marked as failed: {}", task_id);
        }
    }

    /// Get task execution state
    pub fn get_task_state(&self, task_id: &str) -> Option<&TaskExecutionState> {
        self.active_tasks.get(task_id)
    }

    /// Get all active task states
    pub fn get_all_active_tasks(&self) -> Vec<(&String, &TaskExecutionState)> {
        self.active_tasks
            .iter()
            .filter(|(_, state)| {
                state.status != TaskCoordinationStatus::Completed
                    && state.status != TaskCoordinationStatus::Failed
            })
            .collect()
    }

    /// Get task summary for display
    pub fn get_task_summary(&self, task_id: &str) -> Option<TaskSummary> {
        self.active_tasks.get(task_id).map(|state| {
            let progress = state
                .tracker
                .as_ref()
                .map(|t| t.progress_percentage())
                .unwrap_or(0.0);

            let primary_specialist = state
                .matches
                .as_ref()
                .and_then(|m| m.first())
                .map(|m| m.specialist_name.clone());

            TaskSummary {
                task_id: state.task.id.clone(),
                task_name: state.task.name.clone(),
                status: state.status,
                progress_percentage: progress,
                primary_specialist,
                estimated_completion_minutes: state
                    .plan
                    .as_ref()
                    .map(|p| p.estimated_duration_minutes),
                error_message: state.error_message.clone(),
            }
        })
    }

    /// Clean up completed tasks (keeps last 100)
    pub fn cleanup_old_tasks(&mut self) {
        if self.active_tasks.len() > 100 {
            let mut completed: Vec<_> = self
                .active_tasks
                .iter()
                .filter(|(_, state)| {
                    state.status == TaskCoordinationStatus::Completed
                        || state.status == TaskCoordinationStatus::Failed
                })
                .map(|(id, _)| id.clone())
                .collect();

            completed.sort();
            while self.active_tasks.len() > 100 && !completed.is_empty() {
                if let Some(id) = completed.pop() {
                    self.active_tasks.remove(&id);
                }
            }
        }
    }
}

/// Task summary for monitoring/display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub task_id: String,
    pub task_name: String,
    pub status: TaskCoordinationStatus,
    pub progress_percentage: f32,
    pub primary_specialist: Option<String>,
    pub estimated_completion_minutes: Option<u32>,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_execution_state_creation() {
        let task = Task {
            id: "task-1".to_string(),
            name: "Test Task".to_string(),
            description: "Test".to_string(),
            data_sample: None,
            priority: crate::task_analysis::TaskPriority::Normal,
            deadline_secs: None,
            required_skills: vec![],
            tags: vec![],
        };

        let state = TaskExecutionState {
            task: task.clone(),
            analysis: None,
            matches: None,
            plan: None,
            tracker: None,
            status: TaskCoordinationStatus::Submitted,
            error_message: None,
        };

        assert_eq!(state.task.id, "task-1");
        assert_eq!(state.status, TaskCoordinationStatus::Submitted);
    }

    #[test]
    fn test_task_coordination_status_progression() {
        let mut state = TaskCoordinationStatus::Submitted;
        assert_eq!(state, TaskCoordinationStatus::Submitted);

        state = TaskCoordinationStatus::Analyzing;
        assert_eq!(state, TaskCoordinationStatus::Analyzing);

        state = TaskCoordinationStatus::Completed;
        assert_eq!(state, TaskCoordinationStatus::Completed);
    }

    #[test]
    fn test_task_summary_creation() {
        let summary = TaskSummary {
            task_id: "task-1".to_string(),
            task_name: "Analyze Data".to_string(),
            status: TaskCoordinationStatus::Executing,
            progress_percentage: 50.0,
            primary_specialist: Some("Merlin".to_string()),
            estimated_completion_minutes: Some(30),
            error_message: None,
        };

        assert_eq!(summary.progress_percentage, 50.0);
        assert_eq!(summary.primary_specialist, Some("Merlin".to_string()));
    }
}
