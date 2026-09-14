// src/decision_engine.rs

/// Action to take for a task
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Action {
    ExecuteImmediately, // High confidence, low risk
    QueueForLater,      // Moderate confidence or moderate risk
    DelegateToWASM,     // Compute-heavy task suitable for WASM
    RequestHumanInput,  // Low confidence, high uncertainty
    Reject,             // Cannot process
}

/// Task evaluation result from the decision engine
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEvaluation {
    pub routing: RoutingDecision,
    pub memory_recommendation: String,
    pub reasoning: String,
    pub recommended_action: Action,
}

impl Default for TaskEvaluation {
    fn default() -> Self {
        Self {
            routing: RoutingDecision::default(),
            memory_recommendation: String::new(),
            reasoning: String::new(),
            recommended_action: Action::QueueForLater,
        }
    }
}

/// Routing decision from specialist evaluation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingDecision {
    pub specialist_id: String,
    pub memory_searched: bool,
    pub confidence_score: f32,
}

impl Default for RoutingDecision {
    fn default() -> Self {
        Self {
            specialist_id: String::new(),
            memory_searched: false,
            confidence_score: 0.5,
        }
    }
}
