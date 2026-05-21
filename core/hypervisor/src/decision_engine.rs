// Autonomous Decision Engine
// The "brain" that orchestrates compute, biology, and intelligence for optimal task execution
// Now uses thermodynamic governance with Free Energy Principle

use rand::{SeedableRng, Rng};
use serde::{Serialize, Deserialize};
use biology::{SystemBiology, ThermodynamicGovernor, ThermodynamicGovernorConfig, ThermodynamicForecast, SystemHealthReport};
use intelligence::{IntelligenceEngine, RoutableTask, TaskType, RoutingDecision};
use compute::{ComputeEngine, entropy, thermodynamics::SystemPhase};

/// Represents a task in the decision pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTask {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub raw_input: String,
    pub priority: f64,           // 0.0-1.0
    pub deadline_seconds: Option<f64>,
}

/// Complete evaluation of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvaluation {
    pub task_id: String,
    pub complexity: f64,
    pub confidence: f64,          // Bayesian confidence in the evaluation
    pub entropy: f64,             // Shannon entropy of the task description
    pub routing: RoutingDecision,
    pub metabolic_risk: f64,      // Predicted metabolic impact
    pub recommended_action: Action,
    pub reasoning: String,
}

/// Action to take for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    ExecuteImmediately,           // High confidence, low risk
    QueueForLater,                // Moderate confidence or moderate risk
    DelegateToWASM,               // Compute-heavy task suitable for WASM
    RequestHumanInput,            // Low confidence, high uncertainty
    Reject,                       // Cannot process
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
            rng: rand::rngs::StdRng::from_entropy(),
            prior_success_count: 10.0,  // Laplace smoothing
            prior_failure_count: 2.0,
            execution_history: Vec::new(),
            max_history: 100,
        }
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
        
        // Step 5: Predict metabolic risk using thermodynamic governor
        let current_load = 1.0 - (self.biology.tokens / 100.0) as f64;
        self.governor.record_load(current_load);
        let forecast = self.governor.predict_metabolic_risk();
        let metabolic_risk = forecast.risk_score;
        
        // Step 6: Decide action
        let action = self.decide_action(confidence, metabolic_risk, complexity, &routing);
        
        // Step 7: Generate reasoning
        let reasoning = self.generate_reasoning(confidence, metabolic_risk, complexity, &action, &routing);
        
        Ok(TaskEvaluation {
            task_id: task.id.clone(),
            complexity,
            confidence,
            entropy: task_entropy,
            routing,
            metabolic_risk,
            recommended_action: action,
            reasoning,
        })
    }

    /// Execute a task based on evaluation
    pub async fn execute_task(&mut self, task: &DecisionTask, evaluation: &TaskEvaluation) -> ExecutionOutcome {
        match &evaluation.recommended_action {
            Action::ExecuteImmediately => {
                // Check metabolic availability
                if !self.biology.can_execute_specialist(&evaluation.routing.specialist_id) {
                    return ExecutionOutcome::Blocked("Insufficient metabolic tokens".to_string());
                }
                
                // Consume token
                self.biology.consume_specialist_token(&evaluation.routing.specialist_id);
                
                // Simulate execution (would be replaced with actual execution)
                let start = std::time::Instant::now();
                let success = self.simulate_execution(task, evaluation).await;
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
            
            Action::Reject => {
                ExecutionOutcome::Rejected("Task rejected".to_string())
            }
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
        
        let success_count = outcomes.iter().filter(|o| matches!(o, ExecutionOutcome::Completed { .. })).count();
        let failed_count = outcomes.iter().filter(|o| matches!(o, ExecutionOutcome::Failed(_))).count();
        
        IngestionReport {
            total_tasks,
            success_count,
            failed_count,
            queued_count: outcomes.iter().filter(|o| matches!(o, ExecutionOutcome::Queued(_))).count(),
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
    fn decide_action(&self, confidence: f64, metabolic_risk: f64, complexity: f64, _routing: &RoutingDecision) -> Action {
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
    fn generate_reasoning(&self, confidence: f64, metabolic_risk: f64, complexity: f64, action: &Action, routing: &RoutingDecision) -> String {
        format!(
            "Confidence: {:.2}, Metabolic Risk: {:.2}, Complexity: {:.2} → {:?} via {}",
            confidence, metabolic_risk, complexity, action, routing.specialist_name
        )
    }

    /// Simulate task execution (placeholder for actual execution)
    async fn simulate_execution(&self, task: &DecisionTask, evaluation: &TaskEvaluation) -> bool {
        // In a real implementation, this would execute the actual task
        // For now, simulate based on confidence
        let success_prob = evaluation.confidence;
        let mut rng = self.rng.clone();
        let roll: f64 = rng.gen_range(0.0..1.0);
        roll < success_prob
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

    /// Get system status summary
    pub fn get_status(&self) -> SystemStatus {
        SystemStatus {
            metabolic_health: self.biology.get_health_report(),
            bayesian_confidence: self.prior_success_count / (self.prior_success_count + self.prior_failure_count),
            execution_count: self.execution_history.len(),
            recent_success_rate: if self.execution_history.is_empty() {
                0.5
            } else {
                let recent = &self.execution_history[self.execution_history.len().saturating_sub(10)..];
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
    use intelligence::{Specialist, LLMConfig, ProviderType};

    fn create_test_intelligence() -> IntelligenceEngine {
        let specialists = vec![
            Specialist {
                id: "spec_1".to_string(),
                name: "Test Specialist".to_string(),
                skills: vec!["general".to_string()],
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 5.0,
            },
        ];
        
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
        
        IntelligenceEngine::new(config, specialists)
    }

    #[tokio::test]
    async fn test_evaluate_task() {
        let intelligence = create_test_intelligence();
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
        let intelligence = create_test_intelligence();
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
}
