/// GenericSpecialist — a runtime-spawnable specialist driven by any GGUF model.
///
/// Where the five core specialists (Visionary, Omnipresent, Symbiotic, Phygital,
/// Archivist) are compiled-in with hard-coded domain logic, `GenericSpecialist`
/// is a blank-slate agent that gets its intelligence entirely from an attached
/// `LLMClient` (backed by any GGUF model — Qwen, abliterated variants, custom
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
use crate::llm::{LLMClient, LLMConfig, ProviderType};

// ────────────────────────────────────────────────────────────────────────────
// Learning data
// ────────────────────────────────────────────────────────────────────────────

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
        if self.total_executions > 0 {
            self.confidence_score = self.success_count as f32 / self.total_executions as f32;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs();
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

// ────────────────────────────────────────────────────────────────────────────
// GenericSpecialist
// ────────────────────────────────────────────────────────────────────────────

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
    /// SQLite persistence key — must be unique across the federation
    pub persistence_key: String,
    /// Underlying GGUF inference client.  `None` → structured fallback output.
    pub llm: Option<Arc<LLMClient>>,
    /// Path to the GGUF model file backing this specialist (for metadata)
    pub model_path: Option<std::path::PathBuf>,
    /// Learning state with interior mutability for `&self` execute()
    pub learning: Arc<Mutex<GenericLearningData>>,
}

pub const PERSISTENCE_KEY_PREFIX: &str = "Generic:";

impl GenericSpecialist {
    /// Create a new generic specialist with the given name and domain.
    ///
    /// No LLM is attached yet — call `.with_mock_llm()` or `.with_gguf_path()`
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
        }
    }

    /// Attach a `MockProvider` LLM (no GGUF required — useful for dev/tests).
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

// ────────────────────────────────────────────────────────────────────────────
// Specialist trait impl
// ────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Specialist for GenericSpecialist {
    /// Returns `SpecialistId::Visionary` as the closest match for routing
    /// purposes.  The real identity is in `self.name` / `self.domain`.
    ///
    /// A future `SpecialistId::Custom(String)` variant will replace this.
    fn id(&self) -> SpecialistId {
        SpecialistId::Visionary
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
            specialist: SpecialistId::Visionary,
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

        // Try LLM-backed execution first
        let output = if let Some(llm) = &self.llm {
            use crate::llm::DesignContext;
            let ctx = DesignContext {
                intent: intent.clone(),
                style_hints: vec![self.domain.clone()],
                constraints: vec![],
                variants_requested: 2,
                approved_examples: vec![],
                rejected_examples: vec![],
            };
            match llm.generate_design(&ctx).await {
                Ok(result) if !result.variants.is_empty() => {
                    format!(
                        "[{}] {} variant(s) generated for '{}': {}",
                        self.name,
                        result.variants.len(),
                        intent,
                        result.variants.iter()
                            .map(|v| v.description.as_str())
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                }
                Ok(_) => format!("[{}] No variants generated for '{}'", self.name, intent),
                Err(e) => format!("[{}] LLM error for '{}': {}", self.name, intent, e),
            }
        } else {
            format!(
                "[{}] Domain '{}' — processed intent '{}' (no LLM attached)",
                self.name, self.domain, intent
            )
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let success = !output.contains("error");

        {
            let mut l = self.learning.lock();
            l.record_result(success);
        }

        Ok(ExecutionResult {
            specialist: SpecialistId::Visionary,
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

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

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
            specialist: SpecialistId::Visionary,
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
            specialist: SpecialistId::Visionary,
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
            specialist: SpecialistId::Visionary,
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
