//! crates/orchestrator
//! Multi-Agent Federation, Hive Runtime, MDP Task Routing, and Control Plane for Aaroneous.

pub mod agents;
pub mod aura_ui;
pub mod aura_ui_manifest;
pub mod control;
pub mod dynamic_ui;
pub mod grim_reaper;
pub mod hive_runtime;
pub mod linguistic_transducer;
pub mod llm;
pub mod mdps_router;
pub mod pantheon_orchestrator;
pub mod workflow_engine;
pub mod workspace;

pub use grim_reaper::{
    CompactionSummary, GrimReaperEngine, HibernationManifest, SpecialistHibernationState,
};
pub use pantheon_orchestrator::{pin_current_thread_to_core, PantheonOrchestrator};

pub use dynamic_ui::{
    DynamicUiNode, DynamicUiSynthesizer, DynamicWindowManifest, NonOverlapSolver, RectAabb,
    WindowArrangementStrategy,
};

// Re-export agent types
pub use agents::{
    create_relic, create_specialist, Agent, AgentType, BaseAgent, CognitiveBias, Domain,
    RelicAgent, SpecialistAgent, UserAgent,
};

// Re-export control plane
pub use control::{parse_control_message, ControlMessage, ControlPlane, SpecialistState};

// Re-export hive runtime
pub use hive_runtime::{HiveRuntime, HiveRuntimeConfig, RuntimeStatistics, RuntimeStatus};

// Re-export intelligence & router
pub use llm::{LLMClient, LLMConfig, ProviderType, TaskAnalysis, TaskAnalysisContext};
pub use mdps_router::{RoutableTask, RoutingDecision, Specialist, TaskRoutingEngine, TaskType};
pub use workflow_engine::{StepStatus, WorkflowGraph, WorkflowStep};
pub use workspace::WorkspacePaths;

use nervous_system::SharedMemorySynapse;

/// Unified Intelligence Engine for task routing and cognitive planning
pub struct IntelligenceEngine {
    pub synapse: SharedMemorySynapse,
    pub client: LLMClient,
    pub router: TaskRoutingEngine,
}

impl IntelligenceEngine {
    pub fn new(config: LLMConfig, specialists: Vec<Specialist>) -> Self {
        Self {
            synapse: SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024).unwrap(),
            client: LLMClient::new(config),
            router: TaskRoutingEngine::new(specialists),
        }
    }

    pub async fn new_async(config: LLMConfig, specialists: Vec<Specialist>) -> Self {
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

    pub fn route_task(&mut self, task: &RoutableTask) -> RoutingDecision {
        self.router.find_optimal_specialist(task)
    }

    pub fn record_outcome(&mut self, specialist_id: &str, success: bool, completion_time: f64) {
        self.router
            .update_specialist_performance(specialist_id, success, completion_time);
    }
}
