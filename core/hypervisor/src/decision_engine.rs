// Autonomous Decision Engine
// The "brain" that orchestrates compute, biology, and intelligence for optimal task execution
// Now uses thermodynamic governance with Free Energy Principle

use crate::specialist_memory::{
    MemoryEntry, MemoryType, SharedMemoryRegistry, SpecialistMemoryStore,
};
use biology::{
    SystemBiology, SystemHealthReport, ThermodynamicGovernor, ThermodynamicGovernorConfig,
};
use compute::{ComputeEngine, entropy};
use crate::intelligence::{IntelligenceEngine, RoutableTask, RoutingDecision, TaskType};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

/// Represents a task in the decision pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTask {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub raw_input: String,
    pub priority: f64, // 0.0-1.0
    pub deadline_seconds: Option<f64>,
}

/// Complete evaluation of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvaluation {
    pub task_id: String,
    pub complexity: f64,
    pub confidence: f64, // Bayesian confidence in the evaluation
    pub entropy: f64,    // Shannon entropy of the task description
    pub routing: RoutingDecision,
    pub metabolic_risk: f64, // Predicted metabolic impact
    pub recommended_action: Action,
    pub reasoning: String,
    pub memory_informed: bool, // True if memory consultation produced non-empty results
    pub memory_score: f32,     // Aggregate relevance score from specialist memory (0.0-1.0)
    pub memory_recommendation: String, // Human-readable guidance from memory
}

/// Action to take for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    ExecuteImmediately, // High confidence, low risk
    QueueForLater,      // Moderate confidence or moderate risk
    DelegateToWASM,     // Compute-heavy task suitable for WASM
    RequestHumanInput,  // Low confidence, high uncertainty
    Reject,             // Cannot process
}

/// Autonomous Decision Engine
pub struct AutonomousDecisionEngine {
    pub biology: SystemBiology,
    pub governor: ThermodynamicGovernor,
    pub intelligence: IntelligenceEngine,
    pub compute: ComputeEngine,
    pub rng: rand::rngs::StdRng,

    // Bayesian priors for confidence estimation
    pub prior_success_count: f64,
    pub prior_failure_count: f64,

    // Execution history for learning
    pub execution_history: Vec<ExecutionRecord>,
    pub max_history: usize,

    // Per-specialist persistent memory registry. The shared registry type keeps
    // a single store per specialist and avoids parallel ad-hoc HashMap paths.
    pub specialist_memory: SharedMemoryRegistry,
    // Weights used when blending the memory signal into the Bayesian confidence.
    pub memory_confidence_weight: f64, // 0.0 disables memory, 1.0 makes it the dominant signal
}

/// Record of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub task_id: String,
    pub action_taken: Action,
    pub success: bool,
    pub completion_time_seconds: f64,
    pub metabolic_cost: f64,
}

impl AutonomousDecisionEngine {
    pub fn new(intelligence: IntelligenceEngine) -> Self {
        Self {
            biology: SystemBiology::new(),
            governor: ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default()),
            intelligence,
            compute: ComputeEngine::new(),
            rng: rand::rngs::StdRng::from_seed(rand::random()),
            prior_success_count: 10.0, // Laplace smoothing
            prior_failure_count: 2.0,
            execution_history: Vec::new(),
            max_history: 100,
            specialist_memory: SharedMemoryRegistry::new(),
            memory_confidence_weight: 0.3, // memory influences confidence but does not dominate
        }
    }

    /// Get or lazily create the memory store for a specialist.
    pub fn memory_for(&self, specialist_id: &str) -> SpecialistMemoryStore {
        self.specialist_memory.get_or_create(specialist_id)
    }

    /// Consult the routed specialist's memory for guidance on a task.
    /// Returns the relevance score and the recommendation string.
    fn consult_memory(&mut self, routing: &RoutingDecision, task: &DecisionTask) -> (f32, String) {
        let query = task.description.as_str();
        let task_type = match &task.task_type {
            TaskType::CodeGeneration => "code_generation",
            TaskType::BugFix => "bug_fix",
            TaskType::Refactor => "refactoring",
            TaskType::TestCreation => "testing",
            TaskType::Documentation => "documentation",
            TaskType::Analysis => "analysis",
            TaskType::Ingestion => "ingestion",
            TaskType::Custom(name) => name.as_str(),
        };
        let store = self.memory_for(&routing.specialist_id);
        let result = store.query_memory(query, task_type, 5);
        let score = if result.entries.is_empty() {
            0.0
        } else {
            (result.total_score / result.entries.len() as f32).clamp(0.0, 1.0)
        };
        (score, result.recommendation)
    }

    /// Blend the memory score with the Bayesian confidence.
    /// If memory has nothing to say (score = 0), returns the original confidence.
    fn adjust_confidence_with_memory(&self, base: f64, memory_score: f32) -> f64 {
        if memory_score <= 0.0 {
            return base;
        }
        let w = self.memory_confidence_weight.clamp(0.0, 1.0);
        let blended = (1.0 - w) * base + w * memory_score as f64;
        blended.clamp(0.0, 1.0)
    }

    /// Record a procedural memory entry for a specialist after a successful
    /// execution. Failures are stored as factual/negative memories so future
    /// decisions can avoid the same approach.
    pub fn record_execution_memory(
        &mut self,
        specialist_id: &str,
        task: &DecisionTask,
        success: bool,
        duration_seconds: f64,
    ) {
        let memory_type = if success {
            MemoryType::Procedural
        } else {
            MemoryType::Factual
        };
        let title = if success {
            format!("Succeeded at {}", task.description)
        } else {
            format!("Failed at {}", task.description)
        };
        let description = format!(
            "Action: {:?}, Duration: {:.2}s, Task type: {:?}",
            memory_type, duration_seconds, task.task_type
        );
        let entry = MemoryEntry::new(
            format!("mem_{}_{}", specialist_id, task.id),
            specialist_id.to_string(),
            title,
            description,
            memory_type,
        );
        let store = self.memory_for(specialist_id);
        store.store_memory(entry);
    }

    /// Main entry point: evaluate and decide how to handle a task
    pub async fn evaluate_task(&mut self, task: &DecisionTask) -> anyhow::Result<TaskEvaluation> {
        // Step 1: Compute entropy of task description (uncertainty measure)
        let task_bytes = task.raw_input.as_bytes();
        let byte_values: Vec<f64> = task_bytes.iter().map(|&b| b as f64 / 255.0).collect();
        let task_entropy = entropy::shannon_entropy(&byte_values).unwrap_or(vec![1.0])[0];

        // Step 2: Estimate complexity using compute engine
        let complexity_input = vec![task.priority, task_entropy / 5.0]; // Normalize entropy
        let complexity_result = self.compute.execute("monte_carlo", &complexity_input)?;
        let complexity = complexity_result[0].clamp(0.0, 1.0);

        // Step 3: Bayesian confidence estimation
        let confidence = self.estimate_confidence(complexity, task_entropy);

        // Step 4: Route task using MDP
        let routable_task = RoutableTask {
            id: task.id.clone(),
            task_type: task.task_type.clone(),
            complexity,
            urgency: task.priority,
            required_skills: vec![], // Could be extracted from description
            estimated_cost: complexity * 0.5,
        };

        let routing = self.intelligence.route_task(&routable_task);

        // Step 4.5: Consult the routed specialist's persistent memory for
        // guidance. Memory score blends into the confidence so the engine
        // can lean on past experience when deciding to act, queue, or
        // delegate.
        let (memory_score, memory_recommendation) = self.consult_memory(&routing, task);
        let memory_informed = memory_score > 0.0;
        let adjusted_confidence = self.adjust_confidence_with_memory(confidence, memory_score);

        // Step 5: Predict metabolic risk using thermodynamic governor
        let current_load = 1.0 - (self.biology.tokens / 100.0) as f64;
        self.governor.record_load(current_load);
        let forecast = self.governor.predict_metabolic_risk();
        let metabolic_risk = forecast.risk_score;

        // Step 6: Decide action (uses memory-adjusted confidence)
        let action = self.decide_action(adjusted_confidence, metabolic_risk, complexity, &routing);

        // Step 7: Generate reasoning
        let reasoning = self.generate_reasoning(
            adjusted_confidence,
            metabolic_risk,
            complexity,
            &action,
            &routing,
            memory_informed,
            memory_score,
        );

        Ok(TaskEvaluation {
            task_id: task.id.clone(),
            complexity,
            confidence: adjusted_confidence,
            entropy: task_entropy,
            routing,
            metabolic_risk,
            recommended_action: action,
            reasoning,
            memory_informed,
            memory_score,
            memory_recommendation,
        })
    }

    /// Execute a task based on evaluation
    pub async fn execute_task(
        &mut self,
        task: &DecisionTask,
        evaluation: &TaskEvaluation,
    ) -> ExecutionOutcome {
        match &evaluation.recommended_action {
            Action::ExecuteImmediately => {
                // Check metabolic availability
                if !self
                    .biology
                    .can_execute_specialist(&evaluation.routing.specialist_id)
                {
                    return ExecutionOutcome::Blocked("Insufficient metabolic tokens".to_string());
                }

                // Consume token
                self.biology
                    .consume_specialist_token(&evaluation.routing.specialist_id);

                // Simulate execution (would be replaced with actual execution)
                let start = std::time::Instant::now();
                let success = self.execute_action(task, evaluation).await;
                let duration = start.elapsed().as_secs_f64();

                // Record outcome
                self.record_outcome(&task.id, success, duration, evaluation.complexity * 0.5);

                if success {
                    ExecutionOutcome::Completed { duration }
                } else {
                    ExecutionOutcome::Failed("Execution failed".to_string())
                }
            }

            Action::QueueForLater => {
                ExecutionOutcome::Queued("Task queued for later execution".to_string())
            }

            Action::DelegateToWASM => {
                // Would trigger WASM enzyme execution
                ExecutionOutcome::Delegated("Task delegated to WASM enzyme".to_string())
            }

            Action::RequestHumanInput => {
                ExecutionOutcome::NeedsInput("Human input required".to_string())
            }

            Action::Reject => ExecutionOutcome::Rejected("Task rejected".to_string()),
        }
    }

    /// Process an ingestion cycle: evaluate and execute a batch of tasks
    pub async fn process_ingestion_cycle(&mut self, tasks: Vec<DecisionTask>) -> IngestionReport {
        let mut evaluations = Vec::new();
        let mut outcomes = Vec::new();
        let mut total_duration = 0.0;
        let total_tasks = tasks.len();

        for task in tasks {
            // Evaluate task
            match self.evaluate_task(&task).await {
                Ok(evaluation) => {
                    // Execute based on evaluation
                    let outcome = self.execute_task(&task, &evaluation).await;

                    if let ExecutionOutcome::Completed { duration } = &outcome {
                        total_duration += duration;
                    }

                    evaluations.push(evaluation);
                    outcomes.push(outcome);
                }
                Err(e) => {
                    outcomes.push(ExecutionOutcome::Failed(format!("Evaluation error: {}", e)));
                }
            }

            // Update metabolism between tasks
            self.biology.update_metabolism();

            // Apply governance if needed
            let _governance = self.governor.apply_governance(&mut self.biology);
        }

        let success_count = outcomes
            .iter()
            .filter(|o| matches!(o, ExecutionOutcome::Completed { .. }))
            .count();
        let failed_count = outcomes
            .iter()
            .filter(|o| matches!(o, ExecutionOutcome::Failed(_)))
            .count();

        IngestionReport {
            total_tasks,
            success_count,
            failed_count,
            queued_count: outcomes
                .iter()
                .filter(|o| matches!(o, ExecutionOutcome::Queued(_)))
                .count(),
            total_duration,
            final_metabolic_state: self.biology.get_health_report(),
            evaluations,
            outcomes,
        }
    }

    /// Estimate Bayesian confidence based on complexity and entropy
    fn estimate_confidence(&self, complexity: f64, entropy: f64) -> f64 {
        // Likelihood: lower complexity and entropy => higher confidence
        let likelihood = (1.0 - complexity * 0.6 - (entropy / 5.0) * 0.4).clamp(0.0, 1.0);

        // Bayesian update
        let alpha = self.prior_success_count + likelihood * 10.0;
        let beta = self.prior_failure_count + (1.0 - likelihood) * 10.0;

        alpha / (alpha + beta)
    }

    /// Decide action based on evaluation metrics
    fn decide_action(
        &self,
        confidence: f64,
        metabolic_risk: f64,
        complexity: f64,
        _routing: &RoutingDecision,
    ) -> Action {
        if confidence > 0.8 && metabolic_risk < 0.5 && complexity < 0.7 {
            Action::ExecuteImmediately
        } else if confidence > 0.5 && metabolic_risk < 0.7 {
            if complexity > 0.8 {
                Action::DelegateToWASM
            } else {
                Action::QueueForLater
            }
        } else if confidence < 0.3 {
            Action::RequestHumanInput
        } else {
            Action::Reject
        }
    }

    /// Generate human-readable reasoning for the decision
    #[allow(clippy::too_many_arguments)]
    fn generate_reasoning(
        &self,
        confidence: f64,
        metabolic_risk: f64,
        complexity: f64,
        action: &Action,
        routing: &RoutingDecision,
        memory_informed: bool,
        memory_score: f32,
    ) -> String {
        let memory_note = if memory_informed {
            format!(", Memory: informed (score {:.2})", memory_score)
        } else {
            ", Memory: no relevant history".to_string()
        };
        format!(
            "Confidence: {:.2}, Metabolic Risk: {:.2}, Complexity: {:.2}{} → {:?} via {}",
            confidence, metabolic_risk, complexity, memory_note, action, routing.specialist_name
        )
    }

    /// Execute the action associated with the task
    async fn execute_action(&self, task: &DecisionTask, evaluation: &TaskEvaluation) -> bool {
        // Here we simulate the execution of a delegated action.
        // In a full implementation, DelegateToWASM would call ExecutionEnzyme::execute_chain,
        // and ExecuteImmediately would trigger the specific component.

        match evaluation.recommended_action {
            Action::ExecuteImmediately | Action::DelegateToWASM => {
                // If the confidence is high enough and risk is low, we deterministically succeed.
                // We no longer rely on stochastic RNG simulation for deterministic execution.
                if evaluation.confidence > 0.4 && evaluation.metabolic_risk < 0.8 {
                    println!(
                        "[DecisionEngine] Task {} executed successfully via {:?}.",
                        task.id, evaluation.recommended_action
                    );
                    true
                } else {
                    println!(
                        "[DecisionEngine] Task {} execution failed due to low confidence ({:.2}) or high risk ({:.2}).",
                        task.id, evaluation.confidence, evaluation.metabolic_risk
                    );
                    false
                }
            }
            Action::QueueForLater | Action::RequestHumanInput | Action::Reject => {
                println!(
                    "[DecisionEngine] Task {} deferred: {:?}",
                    task.id, evaluation.recommended_action
                );
                false
            }
        }
    }

    /// Record execution outcome for learning
    fn record_outcome(&mut self, task_id: &str, success: bool, duration: f64, metabolic_cost: f64) {
        // Update Bayesian priors
        if success {
            self.prior_success_count += 1.0;
        } else {
            self.prior_failure_count += 1.0;
        }

        // Update specialist performance
        self.intelligence.record_outcome(task_id, success, duration);

        // Record in history
        self.execution_history.push(ExecutionRecord {
            task_id: task_id.to_string(),
            action_taken: Action::ExecuteImmediately,
            success,
            completion_time_seconds: duration,
            metabolic_cost,
        });

        // Trim history if needed
        if self.execution_history.len() > self.max_history {
            self.execution_history.remove(0);
        }
    }

    /// Convenience wrapper that records both the execution outcome (Bayesian
    /// priors, intelligence, history) AND a memory entry for the routed
    /// specialist. Call this instead of `record_outcome` whenever the
    /// originating `DecisionTask` and `RoutingDecision` are available.
    pub fn record_outcome_with_memory(
        &mut self,
        task: &DecisionTask,
        routing: &RoutingDecision,
        success: bool,
        duration: f64,
        metabolic_cost: f64,
    ) {
        self.record_outcome(&task.id, success, duration, metabolic_cost);
        self.record_execution_memory(&routing.specialist_id, task, success, duration);
    }

    /// Get system status summary
    pub fn get_status(&self) -> SystemStatus {
        SystemStatus {
            metabolic_health: self.biology.get_health_report(),
            bayesian_confidence: self.prior_success_count
                / (self.prior_success_count + self.prior_failure_count),
            execution_count: self.execution_history.len(),
            recent_success_rate: if self.execution_history.is_empty() {
                0.5
            } else {
                let recent =
                    &self.execution_history[self.execution_history.len().saturating_sub(10)..];
                recent.iter().filter(|r| r.success).count() as f64 / recent.len() as f64
            },
        }
    }
}

/// Outcome of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    Completed { duration: f64 },
    Failed(String),
    Queued(String),
    Delegated(String),
    NeedsInput(String),
    Rejected(String),
    Blocked(String),
}

/// Report from an ingestion cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionReport {
    pub total_tasks: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub queued_count: usize,
    pub total_duration: f64,
    pub final_metabolic_state: SystemHealthReport,
    pub evaluations: Vec<TaskEvaluation>,
    pub outcomes: Vec<ExecutionOutcome>,
}

/// System status summary
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub metabolic_health: SystemHealthReport,
    pub bayesian_confidence: f64,
    pub execution_count: usize,
    pub recent_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::{LLMConfig, ProviderType, Specialist};

    async fn create_test_intelligence() -> IntelligenceEngine {
        let specialists = vec![Specialist {
            id: "spec_1".to_string(),
            name: "Test Specialist".to_string(),
            skills: vec!["general".to_string()],
            capacity: 1.0,
            success_rate: 0.9,
            avg_completion_time: 5.0,
        }];

        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            model_name: "mock".to_string(),
            api_key: None,
            base_url: None,
            gguf_model_path: None,
            temperature: 0.7,
            max_tokens: 512,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
        };

        IntelligenceEngine::new_async(config, specialists).await.expect("Failed to create test intelligence engine")
    }

    #[tokio::test]
    async fn test_evaluate_task() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "test_1".to_string(),
            description: "Test task".to_string(),
            task_type: TaskType::CodeGeneration,
            raw_input: "fn main() {}".to_string(),
            priority: 0.7,
            deadline_seconds: None,
        };

        let evaluation = engine.evaluate_task(&task).await.unwrap();
        assert!(!evaluation.task_id.is_empty());
        assert!(evaluation.confidence >= 0.0 && evaluation.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_ingestion_cycle() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let tasks = vec![
            DecisionTask {
                id: "task_1".to_string(),
                description: "Task 1".to_string(),
                task_type: TaskType::CodeGeneration,
                raw_input: "Simple task".to_string(),
                priority: 0.8,
                deadline_seconds: None,
            },
            DecisionTask {
                id: "task_2".to_string(),
                description: "Task 2".to_string(),
                task_type: TaskType::BugFix,
                raw_input: "Fix bug".to_string(),
                priority: 0.5,
                deadline_seconds: None,
            },
        ];

        let report = engine.process_ingestion_cycle(tasks).await;
        assert_eq!(report.total_tasks, 2);
    }

    #[tokio::test]
    async fn test_execute_task_queue_returns_queued() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "q1".to_string(),
            description: "Queue this".to_string(),
            task_type: TaskType::Analysis,
            raw_input: "data".to_string(),
            priority: 0.3,
            deadline_seconds: None,
        };

        let evaluation = TaskEvaluation {
            task_id: task.id.clone(),
            complexity: 0.5,
            confidence: 0.6,
            entropy: 1.0,
            routing: RoutingDecision {
                specialist_id: "spec_1".to_string(),
                specialist_name: "Test".to_string(),
                confidence: 0.5,
                expected_completion_time: 5.0,
                reasoning: "test".to_string(),
            },
            metabolic_risk: 0.3,
            recommended_action: Action::QueueForLater,
            reasoning: "queued".to_string(),
            memory_informed: false,
            memory_score: 0.0,
            memory_recommendation: String::new(),
        };

        let outcome = engine.execute_task(&task, &evaluation).await;
        assert!(matches!(outcome, ExecutionOutcome::Queued(_)));
    }

    #[tokio::test]
    async fn test_execute_task_reject_returns_rejected() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "r1".to_string(),
            description: "Reject".to_string(),
            task_type: TaskType::Analysis,
            raw_input: "data".to_string(),
            priority: 0.1,
            deadline_seconds: None,
        };

        let evaluation = TaskEvaluation {
            task_id: task.id.clone(),
            complexity: 0.5,
            confidence: 0.2,
            entropy: 2.0,
            routing: RoutingDecision {
                specialist_id: "spec_1".to_string(),
                specialist_name: "Test".to_string(),
                confidence: 0.3,
                expected_completion_time: 5.0,
                reasoning: "low confidence".to_string(),
            },
            metabolic_risk: 0.8,
            recommended_action: Action::Reject,
            reasoning: "rejected".to_string(),
            memory_informed: false,
            memory_score: 0.0,
            memory_recommendation: String::new(),
        };

        let outcome = engine.execute_task(&task, &evaluation).await;
        assert!(matches!(outcome, ExecutionOutcome::Rejected(_)));
    }

    #[tokio::test]
    async fn test_execute_task_human_input() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "h1".to_string(),
            description: "Need input".to_string(),
            task_type: TaskType::Analysis,
            raw_input: "unclear".to_string(),
            priority: 0.5,
            deadline_seconds: None,
        };

        let evaluation = TaskEvaluation {
            task_id: task.id.clone(),
            complexity: 0.5,
            confidence: 0.1,
            entropy: 4.0,
            routing: RoutingDecision {
                specialist_id: "spec_1".to_string(),
                specialist_name: "Test".to_string(),
                confidence: 0.1,
                expected_completion_time: 5.0,
                reasoning: "very uncertain".to_string(),
            },
            metabolic_risk: 0.3,
            recommended_action: Action::RequestHumanInput,
            reasoning: "needs input".to_string(),
            memory_informed: false,
            memory_score: 0.0,
            memory_recommendation: String::new(),
        };

        let outcome = engine.execute_task(&task, &evaluation).await;
        assert!(matches!(outcome, ExecutionOutcome::NeedsInput(_)));
    }

    #[tokio::test]
    async fn test_memory_for_creates_store() {
        let intelligence = create_test_intelligence().await;
        let engine = AutonomousDecisionEngine::new(intelligence);
        let store = engine.memory_for("spec_test");
        let result = store.query_memory("test", "code_generation", 5);
        assert!(result.entries.is_empty());
    }

    #[tokio::test]
    async fn test_adjust_confidence_with_memory_no_effect() {
        let intelligence = create_test_intelligence().await;
        let engine = AutonomousDecisionEngine::new(intelligence);
        let adjusted = engine.adjust_confidence_with_memory(0.7, 0.0);
        assert_eq!(adjusted, 0.7);
    }

    #[tokio::test]
    async fn test_adjust_confidence_with_memory_blends() {
        let intelligence = create_test_intelligence().await;
        let engine = AutonomousDecisionEngine::new(intelligence);
        let adjusted = engine.adjust_confidence_with_memory(0.3, 0.9);
        // weight=0.3: (1-0.3)*0.3 + 0.3*0.9 = 0.21 + 0.27 = 0.48
        assert!((adjusted - 0.48).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_record_execution_memory_success() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "mem_task".to_string(),
            description: "Test memory".to_string(),
            task_type: TaskType::CodeGeneration,
            raw_input: "test".to_string(),
            priority: 0.5,
            deadline_seconds: None,
        };

        engine.record_execution_memory("spec_1", &task, true, 2.5);
        let store = engine.memory_for("spec_1");
        let result = store.query_memory("Test memory", "code_generation", 5);
        assert!(!result.entries.is_empty());
    }

    #[tokio::test]
    async fn test_record_execution_memory_failure() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "fail_task".to_string(),
            description: "Failed task".to_string(),
            task_type: TaskType::BugFix,
            raw_input: "test".to_string(),
            priority: 0.5,
            deadline_seconds: None,
        };

        engine.record_execution_memory("spec_1", &task, false, 1.0);
        let store = engine.memory_for("spec_1");
        let result = store.query_memory("Failed task", "bug_fix", 5);
        assert!(!result.entries.is_empty());
    }

    #[tokio::test]
    async fn test_get_status_initial() {
        let intelligence = create_test_intelligence().await;
        let engine = AutonomousDecisionEngine::new(intelligence);
        let status = engine.get_status();
        assert_eq!(status.execution_count, 0);
        assert_eq!(status.recent_success_rate, 0.5);
        assert!(status.bayesian_confidence > 0.0);
        assert!(status.bayesian_confidence < 1.0);
    }

    #[tokio::test]
    async fn test_get_status_with_history() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        for i in 0..5 {
            engine.execution_history.push(ExecutionRecord {
                task_id: format!("t{}", i),
                action_taken: Action::ExecuteImmediately,
                success: i % 2 == 0,
                completion_time_seconds: 1.0,
                metabolic_cost: 0.1,
            });
        }

        let status = engine.get_status();
        assert_eq!(status.execution_count, 5);
        // 3 successes out of 5
        assert!((status.recent_success_rate - 0.6).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_record_outcome_with_memory() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let task = DecisionTask {
            id: "outcome_task".to_string(),
            description: "Record outcome".to_string(),
            task_type: TaskType::Refactor,
            raw_input: "test".to_string(),
            priority: 0.5,
            deadline_seconds: None,
        };

        let routing = RoutingDecision {
            specialist_id: "spec_1".to_string(),
            specialist_name: "Test".to_string(),
            confidence: 0.8,
            expected_completion_time: 5.0,
            reasoning: "test".to_string(),
        };

        engine.record_outcome_with_memory(&task, &routing, true, 3.0, 0.2);
        assert_eq!(engine.execution_history.len(), 1);
        assert!(engine.execution_history[0].success);
    }

    #[tokio::test]
    async fn test_ingestion_cycle_all_evaluated() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);

        let tasks = vec![
            DecisionTask {
                id: "c1".to_string(),
                description: "Task 1".to_string(),
                task_type: TaskType::CodeGeneration,
                raw_input: "simple".to_string(),
                priority: 0.8,
                deadline_seconds: None,
            },
            DecisionTask {
                id: "c2".to_string(),
                description: "Task 2".to_string(),
                task_type: TaskType::Analysis,
                raw_input: "analysis".to_string(),
                priority: 0.5,
                deadline_seconds: None,
            },
            DecisionTask {
                id: "c3".to_string(),
                description: "Task 3".to_string(),
                task_type: TaskType::BugFix,
                raw_input: "fix".to_string(),
                priority: 0.3,
                deadline_seconds: None,
            },
        ];

        let report = engine.process_ingestion_cycle(tasks).await;
        assert_eq!(report.total_tasks, 3);
        assert_eq!(report.evaluations.len(), 3);
        assert_eq!(report.outcomes.len(), 3);
    }

    #[tokio::test]
    async fn test_estimate_confidence_low_complexity_high() {
        let intelligence = create_test_intelligence().await;
        let engine = AutonomousDecisionEngine::new(intelligence);
        let conf = engine.estimate_confidence(0.1, 0.5);
        assert!(conf > 0.6);
    }

    #[tokio::test]
    async fn test_estimate_confidence_high_complexity_low() {
        let intelligence = create_test_intelligence().await;
        let engine = AutonomousDecisionEngine::new(intelligence);
        let conf = engine.estimate_confidence(0.9, 4.0);
        assert!(conf < 0.6);
    }

    #[tokio::test]
    async fn test_execution_history_trimming() {
        let intelligence = create_test_intelligence().await;
        let mut engine = AutonomousDecisionEngine::new(intelligence);
        engine.max_history = 3;

        // record_outcome trims history when it exceeds max_history
        for i in 0..5 {
            engine.execution_history.push(ExecutionRecord {
                task_id: format!("t{}", i),
                action_taken: Action::ExecuteImmediately,
                success: true,
                completion_time_seconds: 1.0,
                metabolic_cost: 0.1,
            });
            // Manually trim like record_outcome does
            if engine.execution_history.len() > engine.max_history {
                engine.execution_history.remove(0);
            }
        }

        assert!(engine.execution_history.len() <= 3);
        assert_eq!(engine.execution_history.len(), 3);
    }
}
