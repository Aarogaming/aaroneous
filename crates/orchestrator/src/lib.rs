pub mod agents;
pub mod archetypes;
pub mod assimilation;
/// crates/orchestrator
/// Multi-Agent Federation, Hive Runtime, MDP Task Routing, and Control Plane for Aaroneous.
pub mod combat_agent;
pub mod onboarding;
pub use onboarding::{
    AssimilationError, AssimilationTask, AuditResult, ComponentOnboardingTask, OnboardingError,
    OnboardingPhase, OnboardingRecord, Staged, StagedSandbox, handle_assimilation_event,
    handle_onboarding_event, process_client_request, process_onboarding_request,
};
pub mod aura_ui;
pub mod aura_ui_manifest;
pub mod cartridge_manager;
pub mod compaction_engine;
pub mod control;
pub mod crucible_provider;
pub mod dynamic_ui;
pub mod fs_watcher;
pub mod hive_runtime;
pub mod intent_engine;
pub mod linguistic_intercom;
pub mod linguistic_transducer;
pub mod llm;
pub mod lmstudio_client;
pub mod mdps_router;
pub mod memory_pipeline;
pub mod otel_export;
pub mod plugin_compiler;
pub mod rag_router;
pub mod tagger;
pub mod tier_allocator;
pub mod web_crawler;
pub use tier_allocator as pantheon_orchestrator;
pub mod context_sanitizer;
pub mod diagnostics_filter;
pub mod priority_scheduler;
pub mod supervision;
pub mod swarm_balancer;
pub mod workflow_engine;
pub mod workspace;

pub use supervision::{
    RestartPolicy, SupervisedTask, Supervisor, SupervisorBudget, SupervisorConfig, TaskStatus,
};

pub use context_sanitizer::ContextSanitizer;
pub use diagnostics_filter::{DiagnosticEntry, DiagnosticsFilter};
pub use fs_watcher::FsWatcher;
pub use memory_pipeline::EpisodicInsertionPipeline;
pub use otel_export::{OTelExporter, OtelExporter};
pub use plugin_compiler::PluginCompiler;
pub use priority_scheduler::{PriorityScheduler, PriorityTier, TaskMetadata};
pub use rag_router::{DynamicRagPipeline, RagRouter};
pub use tagger::{EdgeComputeTagger, Tagger};
pub use web_crawler::WebCrawler;

pub use cartridge_manager::{
    CartridgePackManager, CartridgePackManifest, HardwareAutoTuner, HostSystemProfile,
};
pub use crucible_provider::{CrucibleTeacherEndpoint, TeacherBackendConfig, UniversalHttpTeacher};
pub use linguistic_intercom::{
    CompanionPersona, ExecutionDomain, LinguisticIntercom, TransducedIntent,
};
pub use lmstudio_client::{
    ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LmStudioClient,
};

pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;
pub extern crate governance as biology;
pub use governance;

pub use compaction_engine::{
    CompactionEngine, CompactionSummary, HibernationManifest, SpecialistHibernationEngine,
    SpecialistHibernationState,
};
pub use tier_allocator::{PantheonOrchestrator, TierRuntimeAllocator, pin_current_thread_to_core};

pub use dynamic_ui::{
    DynamicUiNode, DynamicUiSynthesizer, DynamicWindowManifest, NonOverlapSolver, RectAabb,
    WindowArrangementStrategy,
};

// Re-export agent types
pub use agents::{
    Agent, AgentType, BaseAgent, BaselineReferenceAgent, CognitiveBias, Domain, RelicAgent,
    SpecialistAgent, UserAgent, create_reference_agent, create_relic, create_specialist,
};
pub use archetypes::{Archetype, ForceVector, NativeThinker};

// Re-export control plane
pub use control::{ControlMessage, ControlPlane, SpecialistState, parse_control_message};

// Re-export hive runtime
pub use hive_runtime::{HiveRuntime, HiveRuntimeConfig, RuntimeStatistics, RuntimeStatus};

// Re-export intelligence & router
pub use intent_engine::{DispatchResult, IntentEngine, ParsedIntent};
pub use llm::{LLMClient, LLMConfig, ProviderType, TaskAnalysis, TaskAnalysisContext};
pub use mdps_router::{
    RoutableTask, RoutingDecision, Specialist, SpecialistRoute, TaskRoutingEngine, TaskType,
};
pub use swarm_balancer::{SwarmBalancer, SwarmHealth, SwarmWorker};
pub use workflow_engine::{StepStatus, WorkflowGraph, WorkflowStep};
pub use workspace::WorkspacePaths;

/// Aligned type alias for the MDP Task Router
pub type MdpTaskRouter = TaskRoutingEngine;

/// Aligned type alias for the Swarm Load Balancer
pub type SwarmLoadBalancer = SwarmBalancer;

use nervous_system::SharedMemorySynapse;

/// Typed domain error for the Unified Intelligence Engine
#[derive(Debug, thiserror::Error)]
pub enum IntelligenceEngineError {
    #[error("Shared memory synapse initialization failed: {0}")]
    SynapseInit(String),
    #[error("Task analysis failed: {0}")]
    TaskAnalysis(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Unified Intelligence Engine for task routing and cognitive planning
pub struct IntelligenceEngine {
    pub synapse: SharedMemorySynapse,
    pub client: LLMClient,
    pub router: TaskRoutingEngine,
}

impl IntelligenceEngine {
    /// Creates an IntelligenceEngine via explicit constructor injection of the SharedMemorySynapse handle.
    pub fn with_synapse(
        synapse: SharedMemorySynapse,
        config: LLMConfig,
        specialists: Vec<Specialist>,
    ) -> Self {
        Self {
            synapse,
            client: LLMClient::new(config),
            router: TaskRoutingEngine::new(specialists),
        }
    }

    pub fn new(
        config: LLMConfig,
        specialists: Vec<Specialist>,
    ) -> Result<Self, IntelligenceEngineError> {
        let synapse = match SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024) {
            Ok(s) => s,
            Err(_) => {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static COUNTER: AtomicUsize = AtomicUsize::new(0);
                let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
                let fallback = format!("SAB_STORE_{}_{}", std::process::id(), counter);
                SharedMemorySynapse::new_sync(&fallback, 1024 * 1024)
                    .map_err(|e| IntelligenceEngineError::SynapseInit(e.to_string()))?
            }
        };
        Ok(Self {
            synapse,
            client: LLMClient::new(config),
            router: TaskRoutingEngine::new(specialists),
        })
    }

    pub async fn new_async(
        config: LLMConfig,
        specialists: Vec<Specialist>,
    ) -> Result<Self, IntelligenceEngineError> {
        let synapse = match SharedMemorySynapse::new("SAB_STORE", 1024 * 1024).await {
            Ok(s) => s,
            Err(_) => {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static COUNTER: AtomicUsize = AtomicUsize::new(0);
                let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
                let fallback = format!("SAB_STORE_{}_{}", std::process::id(), counter);
                SharedMemorySynapse::new(&fallback, 1024 * 1024)
                    .await
                    .map_err(|e| IntelligenceEngineError::SynapseInit(e.to_string()))?
            }
        };
        Ok(Self {
            synapse,
            client: LLMClient::new(config),
            router: TaskRoutingEngine::new(specialists),
        })
    }

    pub async fn analyze_task(
        &self,
        prompt: &str,
    ) -> Result<TaskAnalysis, IntelligenceEngineError> {
        self.client
            .analyze_task(prompt)
            .await
            .map_err(|e| IntelligenceEngineError::TaskAnalysis(e.to_string()))
    }

    pub fn route_task(&mut self, task: &RoutableTask) -> RoutingDecision {
        self.router.find_optimal_specialist(task)
    }

    pub fn record_outcome(&mut self, agent_id: &str, success: bool, completion_time: f64) {
        self.router
            .update_specialist_performance(agent_id, success, completion_time);
    }
}
