pub mod aura_ui_manifest;
pub mod linguistic_transducer;
pub mod llm;
pub mod mdps_router;
pub use llm::{LLMClient, LLMConfig, ProviderType, TaskAnalysis, TaskAnalysisContext};
pub use mdps_router::{RoutableTask, RoutingDecision, Specialist, TaskRoutingEngine, TaskType};

use nervous_system::SharedMemorySynapse;

/// The Intelligence Component.
/// Handles LLM orchestration, task analysis, cognitive planning, and MDP-based routing.
pub struct IntelligenceEngine {
    pub synapse: SharedMemorySynapse,
    pub client: LLMClient,
    pub router: TaskRoutingEngine,
}

impl IntelligenceEngine {
    pub fn new(config: LLMConfig, specialists: Vec<Specialist>) -> Self {
        println!("[DEBUG] IntelligenceEngine::new() called");
        Self {
            synapse: SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024).unwrap(),
            client: LLMClient::new(config),
            router: TaskRoutingEngine::new(specialists),
        }
    }

    pub async fn new_async(config: LLMConfig, specialists: Vec<Specialist>) -> Self {
        println!("[DEBUG] IntelligenceEngine::new_async() called");
        Self {
            synapse: SharedMemorySynapse::new("SAB_STORE", 1024 * 1024)
                .await
                .unwrap(),
            client: LLMClient::new(config),
            router: TaskRoutingEngine::new(specialists),
        }
    }

    pub async fn analyze_task(&self, prompt: &str) -> anyhow::Result<TaskAnalysis> {
        self.client.analyze_task(prompt).await
    }

    /// Route a task to the optimal specialist using MDP
    pub fn route_task(&mut self, task: &RoutableTask) -> RoutingDecision {
        self.router.find_optimal_specialist(task)
    }

    /// Update specialist performance after task completion
    pub fn record_outcome(&mut self, specialist_id: &str, success: bool, completion_time: f64) {
        self.router
            .update_specialist_performance(specialist_id, success, completion_time);
    }
}
