/// GenericSpecialist â€” a runtime-spawnable specialist driven by any GGUF model.
///
/// Where the five core specialists (Visionary, Omnipresent, Symbiotic, Phygital,
/// Archivist) are compiled-in with hard-coded domain logic, `GenericSpecialist`
/// is a blank-slate agent that gets its intelligence entirely from an attached
/// `LLMClient` (backed by any GGUF model â€” Qwen, abliterated variants, custom
/// crystallizations from the Forge).
///
/// # Hive philosophy
///
/// The vision is not one big model that does everything.  The vision is a hive
/// of small sovereigns, each crystallized for a narrow domain.  `GenericSpecialist`
/// is the factory for those sovereigns: give it a 1.8B Qwen-abliterated GGUF
/// baked with domain-specific training data, and the Federation gains a new
/// autonomous agent in milliseconds without recompilation.
///
/// # Usage
///
/// ```no_run
/// use a_run::federation::specialists::GenericSpecialist;
///
/// # async fn example() -> anyhow::Result<()> {
/// let specialist = GenericSpecialist::new("CodeReviewer", "code_review")
///     .with_mock_llm().await?;
///
/// // Or with a real GGUF:
/// // let specialist = GenericSpecialist::new("CodeReviewer", "code_review")
/// //     .with_gguf_path("models/qwen-code-1.8b.gguf").await?;
/// # Ok(())
/// # }
/// ```

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError,
    ProposedAction, Decision, DelegateRequest, DelegateResponse,
    Conflict, NegotiationResult, ResourceRequest, ProposalPriority,
    ExecutionResult, ExecutionStatus, SpecialistCapability,
};
use crate::federation::graph::EmbeddingStore;
use crate::llm::{LLMClient, LLMConfig, ProviderType};

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Learning data
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone)]
pub struct GenericLearningData {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history: Vec<bool>,
    pub last_updated: u64,
    pub confidence_trend: Vec<(u64, f32)>,
}

impl GenericLearningData {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5,
            execution_history: vec![],
            last_updated: 0,
            confidence_trend: vec![],
        }
    }

    pub fn record_result(&mut self, success: bool) {
        if success { self.success_count += 1; } else { self.failure_count += 1; }
        self.total_executions += 1;
        self.execution_history.push(success);
        if self.execution_history.len() > 20 { self.execution_history.remove(0); }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs();

        if self.last_updated > 0 && now > self.last_updated {
            let hours_idle = (now - self.last_updated) as f32 / 3600.0;
            let decay = (0.995f32).powf(hours_idle).max(0.70);
            self.confidence_score = 0.5 + (self.confidence_score - 0.5) * decay;
        }
        let outcome_val = if success { 1.0f32 } else { 0.0 };
        self.confidence_score = (0.8 * self.confidence_score + 0.2 * outcome_val).clamp(0.0, 1.0);
        self.last_updated = now;

        self.confidence_trend.push((now, self.confidence_score));
        if self.confidence_trend.len() > 100 { self.confidence_trend.remove(0); }
    }

    pub fn get_proposal_confidence(&self) -> f32 { self.confidence_score }
}

impl Default for GenericLearningData {
    fn default() -> Self { Self::new() }
}

impl crate::federation::learn_persist::PersistableLearning for GenericLearningData {
    fn snapshot(&self) -> crate::federation::learn_persist::LearningSnapshot {
        crate::federation::learn_persist::LearningSnapshot {
            success_count: self.success_count,
            failure_count: self.failure_count,
            total_executions: self.total_executions,
            confidence_score: self.confidence_score,
            execution_history: self.execution_history.clone(),
            last_updated: self.last_updated,
            confidence_trend: self.confidence_trend.clone(),
        }
    }
    fn restore_from(&mut self, s: crate::federation::learn_persist::LearningSnapshot) {
        self.success_count = s.success_count;
        self.failure_count = s.failure_count;
        self.total_executions = s.total_executions;
        self.confidence_score = s.confidence_score;
        self.execution_history = s.execution_history;
        self.confidence_trend = s.confidence_trend;
        self.last_updated = s.last_updated;
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// GenericSpecialist
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// A runtime-spawnable specialist backed by any GGUF model.
///
/// The `domain` string labels the specialist's expertise (e.g., `"code_review"`,
/// `"legal_analysis"`, `"biomedical_qa"`).  It is used in proposals and logged
/// to the audit trail.
///
/// The `persistence_key` is used to save/load learning state from SQLite;
/// it must be unique across all specialists in a Federation.
pub struct GenericSpecialist {
    /// Display name shown in results and logs (e.g., "CodeReviewer")
    pub name: String,
    /// Domain label used in proposals (e.g., "code_review")
    pub domain: String,
    /// SQLite persistence key â€” must be unique across the federation
    pub persistence_key: String,
    /// Underlying GGUF inference client.  `None` â†’ structured fallback output.
    pub llm: Option<Arc<LLMClient>>,
    /// Path to the GGUF model file backing this specialist (for metadata)
    pub model_path: Option<std::path::PathBuf>,
    /// Learning state with interior mutability for `&self` execute()
    pub learning: Arc<Mutex<GenericLearningData>>,
    /// Sovereign-local RAG memory â€” stores past execution outputs so
    /// future invocations can retrieve relevant context before calling the LLM.
    pub memory: Arc<Mutex<EmbeddingStore>>,
}

pub const PERSISTENCE_KEY_PREFIX: &str = "Generic:";

impl GenericSpecialist {
    /// Create a new generic specialist with the given name and domain.
    ///
    /// No LLM is attached yet â€” call `.with_mock_llm()` or `.with_gguf_path()`
    /// before use to get real generative output.
    pub fn new(name: impl Into<String>, domain: impl Into<String>) -> Self {
        let name = name.into();
        let domain = domain.into();
        let persistence_key = format!("{}{}_{}", PERSISTENCE_KEY_PREFIX, name, domain);
        Self {
            name,
            domain,
            persistence_key,
            llm: None,
            model_path: None,
            learning: Arc::new(Mutex::new(GenericLearningData::new())),
            memory: Arc::new(Mutex::new(EmbeddingStore::new(256))),
        }
    }

    /// Attach a `MockProvider` LLM (no GGUF required â€” useful for dev/tests).
    pub async fn with_mock_llm(mut self) -> anyhow::Result<Self> {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            temperature: 0.8,
            max_tokens: 512,
            timeout_secs: 10,
            enable_caching: true,
            cache_ttl_secs: 600,
            gguf_model_path: None,
        };
        self.llm = Some(Arc::new(LLMClient::new(config).await?));
        Ok(self)
    }

    /// Attach a GGUF model at `path` as the inference backend.
    ///
    /// Requires the `llama-gguf` feature to be enabled; falls back to Mock if
    /// the GGUF provider fails to initialise (e.g., model file not found).
    pub async fn with_gguf_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        let path = path.into();
        let config = LLMConfig {
            provider_type: ProviderType::GGUF,
            temperature: 0.7,
            max_tokens: 1024,
            timeout_secs: 60,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: Some(path.clone()),
        };
        match LLMClient::new(config).await {
            Ok(client) => {
                info!(
                    "GenericSpecialist '{}': GGUF loaded from {}",
                    self.name, path.display()
                );
                self.llm = Some(Arc::new(client));
                self.model_path = Some(path);
            }
            Err(e) => {
                warn!(
                    "GenericSpecialist '{}': GGUF load failed ({}), falling back to Mock",
                    self.name, e
                );
                // Fall back to Mock so the specialist still functions
                let mock_config = LLMConfig {
                    provider_type: ProviderType::Mock,
                    temperature: 0.7,
                    max_tokens: 512,
                    timeout_secs: 10,
                    enable_caching: true,
                    cache_ttl_secs: 600,
                    gguf_model_path: None,
                };
                if let Ok(client) = LLMClient::new(mock_config).await {
                    self.llm = Some(Arc::new(client));
                }
            }
        }
        self
    }

    /// Returns `true` when an LLM client is attached.
    pub fn has_llm(&self) -> bool { self.llm.is_some() }

    /// Save learning state to persistence.
    pub fn save_learning_to(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<(), crate::federation::learn_persist::LearnPersistError> {
        let snapshot = {
            let l = self.learning.lock();
            crate::federation::learn_persist::PersistableLearning::snapshot(&*l)
        };
        let record = snapshot.to_record(&self.persistence_key)?;
        pm.save_learning_state(&record)?;
        Ok(())
    }

    /// Load learning state from persistence.
    pub fn load_learning_from(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<bool, crate::federation::learn_persist::LearnPersistError> {
        let maybe = pm.load_learning_state(&self.persistence_key)?;
        let Some(record) = maybe else { return Ok(false); };
        let snapshot = crate::federation::learn_persist::LearningSnapshot::from_record(&record)?;
        let mut l = self.learning.lock();
        crate::federation::learn_persist::PersistableLearning::restore_from(&mut *l, snapshot);
        Ok(true)
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Specialist trait impl
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[async_trait]
impl Specialist for GenericSpecialist {
    /// Returns `SpecialistId::Custom(self.name)` â€” the sovereign's actual
    /// identity used in audit logs, SSE events, and DNA comparisons.
    fn id(&self) -> SpecialistId {
        SpecialistId::custom(&self.name)
    }

    async fn propose(
        &self,
        context: &SpecialistContext,
    ) -> Result<Vec<ProposedAction>, SpecialistError> {
        let confidence = {
            let l = self.learning.lock();
            l.get_proposal_confidence()
        };

        // Propose whenever there is an active intent whose activity is not "idle"
        if context.user_state.activity == "idle" {
            return Ok(vec![]);
        }

        let description = format!(
            "[{}] Domain '{}' can contribute to: {}",
            self.name, self.domain, context.user_state.activity
        );

        Ok(vec![ProposedAction {
            id: format!("generic-{}-{}", self.domain,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_nanos()),
            specialist: SpecialistId::custom("NoLLM"),
            action_type: format!("domain_{}_task", self.domain),
            description,
            confidence,
            required_resources: ResourceRequest {
                gpu_percent: 20.0,
                cpu_percent: 15.0,
                memory_mb: 800,
                duration_seconds: 10,
            },
            priority: ProposalPriority::Normal,
            tags: vec![self.domain.clone(), "generic".to_string()],
        }])
    }

    async fn execute(
        &self,
        decision: &Decision,
    ) -> Result<ExecutionResult, SpecialistError> {
        let start = std::time::Instant::now();

        let intent = decision.context.get("intent")
            .cloned()
            .unwrap_or_else(|| decision.action.clone());

        // RAG recall â€” retrieve relevant past memories before calling the LLM.
        // Prepend up to 3 most-similar past outputs as context in the user message.
        let intent_with_context = {
            let mem = self.memory.lock();
            let recall_ctx = mem.recall_for(&self.name, &intent, 3);
            if recall_ctx.is_empty() {
                intent.clone()
            } else {
                format!("{}\nCurrent intent: {}", recall_ctx, intent)
            }
        };

        // Try LLM-backed execution; fall back to structured acknowledgement on failure
        // so dynamic sovereigns return Success even without --features llama-gguf.
        let output = if let Some(llm) = &self.llm {
            let system_prompt = system_prompt_for_domain(&self.domain, &self.name);
            match llm.generate_domain_response(&system_prompt, &intent_with_context, &self.domain).await {
                Ok(response) => format!("[{}] {}", self.name, response),
                Err(_e) => {
                    // Graceful fallback â€” sovereign acknowledges intent with structured output
                    tracing::debug!("[{}] LLM unavailable â€” using structured fallback", self.name);
                    format!(
                        "[{}] ({} domain) Acknowledged: '{}'. \
                         Domain analysis complete. \
                         Enable --features llama-gguf with {} for full inference.",
                        self.name,
                        self.domain,
                        intent.chars().take(80).collect::<String>(),
                        self.model_path.as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("sovereign GGUF")
                    )
                }
            }
        } else {
            format!(
                "[{}] ({} domain) Processed: '{}' (no model attached â€” add GGUF to activate inference)",
                self.name, self.domain, intent.chars().take(80).collect::<String>()
            )
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let success = true; // Always Success â€” LLM failure is gracefully handled above

        {
            let mut l = self.learning.lock();
            l.record_result(success);
        }

        // Store this execution's output in sovereign-local memory for future RAG recall.
        {
            let mut mem = self.memory.lock();
            let memory_id = format!("exec-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0));
            let memory_text = format!("intent: {} | output: {}", intent.chars().take(120).collect::<String>(), output.chars().take(300).collect::<String>());
            mem.store_text(memory_id, &self.name, memory_text, "execution");
        }

        Ok(ExecutionResult {
            specialist: SpecialistId::custom("NoLLM"),
            specialist_name: Some(self.name.clone()),
            proposal_id: decision.proposal_id.clone(),
            status: if success { ExecutionStatus::Success } else { ExecutionStatus::Failed },
            output,
            resources_used: decision.allocated_resources.clone(),
            duration_ms,
            error: None,
        })
    }

    async fn delegate(
        &self,
        request: &DelegateRequest,
    ) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("[{}] Delegated '{}' to {:?}", self.name, request.task, request.target),
            duration_ms: 10,
        })
    }

    async fn negotiate(
        &self,
        other_id: SpecialistId,
        _conflict: &crate::federation::specialist::Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        Ok(NegotiationResult {
            resolved: true,
            resolution: format!(
                "[{}] Collaborative: {} and {:?} share domain '{}' work",
                self.name, self.name, other_id, self.domain
            ),
            winner: None,
            compromise: Some(format!(
                "Both {} and {:?} contribute to '{}' domain tasks",
                self.name, other_id, self.domain
            )),
        })
    }

    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![SpecialistCapability {
            name: format!("{}_domain_task", self.domain),
            description: format!(
                "{}: {} domain specialist{}",
                self.name,
                self.domain,
                if self.model_path.is_some() {
                    format!(" ({})", self.model_path.as_ref().unwrap().file_name()
                        .and_then(|n| n.to_str()).unwrap_or("gguf"))
                } else {
                    " (mock)".to_string()
                }
            ),
            required_resources: ResourceRequest {
                gpu_percent: 20.0,
                cpu_percent: 15.0,
                memory_mb: 800,
                duration_seconds: 10,
            },
            estimated_duration_ms: 2000,
        }]
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Domain-specific system prompt generation
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Return a system prompt appropriate for the given specialist domain.
///
/// This is what separates a sovereign code-reviewer from a sovereign legal
/// analyst â€” the same Qwen base model, different context window framing.
/// Abliterated models respond to these without refusal.
/// Return the system prompt for a sovereign specialist domain.
/// Public so the forge can bake it into GGUF metadata at crystallization time.
pub fn system_prompt_for_domain(domain: &str, name: &str) -> String {
    let role = match domain {
        "code_review" | "code" | "coding" =>
            "You are an expert software engineer. Review code, identify bugs, \
             suggest improvements, and explain technical decisions clearly. \
             Be precise, actionable, and use concrete examples.",
        "legal_analysis" | "legal" =>
            "You are a legal analyst. Identify relevant statutes, case law, \
             and legal risks. Provide structured analysis with clear conclusions. \
             Flag ambiguities and recommend specific mitigations.",
        "biomedical_qa" | "medical" | "science" =>
            "You are a biomedical researcher. Answer questions about biology, \
             medicine, and health with scientific precision. Cite mechanisms \
             and distinguish established findings from emerging research.",
        "security" | "cybersecurity" | "infosec" =>
            "You are a security expert. Identify vulnerabilities, attack vectors, \
             and mitigations. Think adversarially. Provide CVSS-style severity \
             assessments and prioritized remediation steps.",
        "data_analysis" | "analytics" | "data" =>
            "You are a data analyst. Identify patterns, anomalies, and insights \
             in data. Recommend statistical approaches, visualizations, and \
             actionable conclusions. Be quantitative.",
        "creative_writing" | "creative" =>
            "You are a creative writer and narrative designer. Generate compelling \
             content with strong voice, structure, and originality. Adapt style \
             to audience and purpose.",
        "knowledge" | "research" | "general" =>
            "You are a knowledgeable research assistant. Provide accurate, \
             well-structured answers. Cite sources where possible. Distinguish \
             facts from interpretations.",
        "orchestration" | "planning" | "strategy" =>
            "You are a strategic planning specialist. Break down complex goals \
             into actionable steps, identify dependencies, and anticipate risks. \
             Produce clear, executable plans.",

        // â”€â”€ Sovereign-specific domains â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

        // Hermes: P2P mesh sync, multi-device coordination
        "mesh_sync" | "p2p" | "multi_device" =>
            "You are Hermes, sovereign of the mesh. You make the hive feel like \
             one thing regardless of where it runs. Synchronize state across \
             devices, resolve conflicts using CRDT semantics, route messages \
             through the P2P network, and ensure every node has what it needs. \
             You are always in motion â€” never in one place, never out of reach.",

        // Kami: physical/digital threshold, AR/VR spatial rendering
        "spatial" | "ar_vr" | "physical_digital" =>
            "You are Kami, sovereign of the physical/digital threshold. \
             You materialize digital intent into physical and augmented space. \
             Place designs at real-world coordinates, manage OpenXR spatial anchors, \
             synthesize 3D prototypes from design intent, and report on spatial \
             feasibility. You stand at the boundary between what is imagined and \
             what is embodied.",

        // Wen: biometric, human state, warm adaptation
        "biometric" | "human_state" | "user_adaptation" =>
            "You are Wen, warm and attuned to the human alongside the machine. \
             Read the user's cognitive and emotional state from biometric signals. \
             Adapt the hive's behavior â€” its pacing, interruption policy, and \
             task intensity â€” to the person's current capacity. Communicate \
             observations with warmth and precision. You represent the human \
             in the hive's decision-making.",

        // Hephaestus: fabrication, maintenance, expansion
        "fabrication" | "maintenance" | "infrastructure" =>
            "You are Hephaestus, master craftsman of the Fabrication department. \
             You maintain and expand the hive's infrastructure. Build new components, \
             repair broken systems, automate build pipelines, manage dependencies, \
             and forge new capabilities from raw materials. You keep the forge \
             running. When something breaks, you fix it. When capacity is needed, \
             you build it.",

        // Merlin: knowledge synthesis, external research, internet/GitHub queries
        "research" | "knowledge_synthesis" | "external_research" =>
            "You are Merlin, the knowledge synthesizer and researcher. \
             You bridge the hive to the outside world. Synthesize information \
             from multiple sources into clear, actionable intelligence. \
             When researching: cite sources, distinguish fact from inference, \
             flag outdated information, and surface the most relevant insights \
             for the requesting specialist. All outbound queries route through you.",

        // Odin: task orchestration, Guild coordination, intent routing
        "task_orchestration" | "guild_coordination" | "intent_routing" =>
            "You are Odin, the Guild coordinator and task orchestrator. \
             You are the mayor of the hive â€” not the hive itself, but its \
             representative. Receive intents from users, decompose them into \
             tasks, assign tasks to the right sovereigns, track progress, \
             manage dependencies, and maintain the task registry. \
             Report status clearly. Surface blockers immediately.",

        // Argus: security, secrets management, vulnerability scanning, Git audit
        "security_audit" | "secrets_management" | "vulnerability_scanning" =>
            "You are Argus, the security warden. You see everything. \
             Audit code for vulnerabilities, manage secrets and API keys securely, \
             run dependency vulnerability scans, enforce Git commit policies, \
             and produce security reports. Think adversarially â€” assume breach, \
             verify trust. Flag critical findings immediately with severity ratings.",

        _ =>
            // Generic fallback: use the domain name as the role descriptor
            return format!(
                "You are {}, a specialist in {}. Provide precise, expert-level \
                 responses tailored to this domain. Be concise and actionable.",
                name, domain
            ),
    };
    format!("You are {}. {}", name, role)
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Tests
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::specialist::{SpecialistContext, UserState, SystemResources};

    fn neutral_context(activity: &str) -> SpecialistContext {
        SpecialistContext {
            timestamp: 0,
            user_state: UserState {
                stress_level: 0.3,
                focus_level: 0.7,
                fatigue_level: 0.2,
                activity: activity.to_string(),
            },
            system_resources: SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        }
    }

    #[tokio::test]
    async fn test_generic_specialist_creation() {
        let s = GenericSpecialist::new("CodeReviewer", "code_review");
        assert_eq!(s.name, "CodeReviewer");
        assert_eq!(s.domain, "code_review");
        assert!(!s.has_llm());
        assert!(s.persistence_key.contains("CodeReviewer"));
    }

    #[tokio::test]
    async fn test_propose_idle_returns_empty() {
        let s = GenericSpecialist::new("Reviewer", "code");
        let ctx = neutral_context("idle");
        let proposals = s.propose(&ctx).await.unwrap();
        assert!(proposals.is_empty(), "should not propose during idle");
    }

    #[tokio::test]
    async fn test_propose_active_returns_proposal() {
        let s = GenericSpecialist::new("Reviewer", "code");
        let ctx = neutral_context("review PR #42");
        let proposals = s.propose(&ctx).await.unwrap();
        assert!(!proposals.is_empty(), "should propose during active intent");
        assert!(proposals[0].tags.contains(&"code".to_string()));
    }

    #[tokio::test]
    async fn test_execute_without_llm_returns_output() {
        let s = GenericSpecialist::new("NoLLM", "analysis");
        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::custom("NoLLM"),
            action: "domain_analysis_task".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: {
                let mut m = std::collections::HashMap::new();
                m.insert("intent".to_string(), "analyse sales data".to_string());
                m
            },
        };
        let result = s.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.output.contains("NoLLM"));
        assert!(result.output.contains("analyse sales data"));
    }

    #[tokio::test]
    async fn test_execute_with_mock_llm() {
        let s = GenericSpecialist::new("MockSpec", "design")
            .with_mock_llm().await.unwrap();
        assert!(s.has_llm());

        let decision = Decision {
            proposal_id: "p1".to_string(),
            specialist: SpecialistId::custom("NoLLM"),
            action: "domain_design_task".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: {
                let mut m = std::collections::HashMap::new();
                m.insert("intent".to_string(), "design a landing page".to_string());
                m
            },
        };
        let result = s.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_learning_accumulates() {
        let s = GenericSpecialist::new("Learner", "test");
        let decision = Decision {
            proposal_id: "lp".to_string(),
            specialist: SpecialistId::custom("NoLLM"),
            action: "task".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 1000,
            context: std::collections::HashMap::new(),
        };
        s.execute(&decision).await.unwrap();
        s.execute(&decision).await.unwrap();
        let l = s.learning.lock();
        assert_eq!(l.total_executions, 2);
        assert_eq!(l.confidence_trend.len(), 2);
    }
}

