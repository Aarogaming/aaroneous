// LLM Integration Module
// Provides abstraction layer for multiple LLM providers
// Supports: OpenAI, Local (Ollama/vLLM), Anthropic, Mock

pub mod providers;
pub mod types;
pub mod cache;
pub mod rate_limiter;
pub mod model_registry;
pub mod model_loader;
pub mod model_environment;
pub mod auto_discover;
pub mod batch_request_system;

pub use providers::{LLMProvider, GGUFProvider, MockProvider};
pub use types::*;
pub use model_registry::{ModelRegistry, ModelInfo, ModelType};
pub use model_loader::{ModelLoader, TOP_RECOMMENDED_MODELS};
pub use model_environment::{ModelEnvironment, DetectedEnvironment, ModelEnvironmentDetector};
pub use batch_request_system::{LLMBatchRequestManager, BatchRequestConfig, AnalysisBatch, PendingAnalysisRequest, BatchStats};

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Main LLM client managing different providers
pub struct LLMClient {
    provider: Arc<dyn LLMProvider>,
    cache: cache::LLMCache,
    rate_limiter: rate_limiter::RateLimiter,
    config: LLMConfig,
}

#[derive(Debug, Clone)]
pub struct LLMConfig {
    pub provider_type: ProviderType,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub enable_caching: bool,
    pub cache_ttl_secs: u64,
    pub gguf_model_path: Option<PathBuf>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::GGUF,
            temperature: 0.7,
            max_tokens: 2048,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProviderType {
    GGUF,      // Local GGUF models (llama.cpp) - RECOMMENDED
    Mock,      // Mock provider for testing
}

impl LLMClient {
    /// Create new LLM client with GGUF provider
    pub async fn new(config: LLMConfig) -> Result<Self> {
        info!("Initializing LLM client with provider: {:?}", config.provider_type);

        let provider: Arc<dyn LLMProvider> = match config.provider_type {
            ProviderType::GGUF => {
                let model_path = if let Some(path) = config.gguf_model_path.clone() {
                    // Use explicitly configured path
                    path
                } else {
                    // Auto-discover best available model
                    info!("Auto-discovering available GGUF models...");
                    match auto_discover::get_recommended_model_for_llm().await {
                        Ok(Some(model)) => {
                            info!("Auto-discovered model: {} ({})", model.name, model.model_type);
                            model.path
                        }
                        Ok(None) => {
                            warn!("No GGUF models found during auto-discovery, using default Qwen path");
                            GGUFProvider::default_qwen_path()
                        }
                        Err(e) => {
                            warn!("Auto-discovery error: {}, using default Qwen path", e);
                            GGUFProvider::default_qwen_path()
                        }
                    }
                };

                Arc::new(GGUFProvider::new(model_path, 2048, 8)?)
            }
            ProviderType::Mock => Arc::new(MockProvider::default()),
        };

        let cache = cache::LLMCache::new(config.cache_ttl_secs);
        // 0 = unlimited for local GGUF inference (no API cost, no external throttle).
        // Set AARONEOUS_LLM_RATE_LIMIT env var to a positive integer to cap it.
        let rate_limit = std::env::var("AARONEOUS_LLM_RATE_LIMIT")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let rate_limiter = rate_limiter::RateLimiter::new(rate_limit);

        info!("LLM client initialized successfully");

        Ok(Self {
            provider,
            cache,
            rate_limiter,
            config,
        })
    }

    /// Analyze a task to determine best approach
    pub async fn analyze_task(
        &self,
        task_context: &TaskAnalysisContext,
    ) -> Result<TaskAnalysis> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("analyze_task:{}", task_context.task_id);

        // Check cache first
        if self.config.enable_caching {
            if let Some(cached) = self.cache.get::<TaskAnalysis>(&cache_key).await {
                debug!("Cache hit for task analysis: {}", task_context.task_id);
                return Ok(cached);
            }
        }

        debug!("Analyzing task: {}", task_context.task_id);

        let analysis = self.provider.analyze_task(task_context).await?;

        // Cache result
        if self.config.enable_caching {
            self.cache.set(&cache_key, analysis.clone()).await?;
        }

        Ok(analysis)
    }

    /// Find best collaborators for a specialist
    pub async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("collaborators:{}", specialist.name);

        if self.config.enable_caching {
            if let Some(cached) = self.cache.get::<Vec<CollaboratorSuggestion>>(&cache_key).await {
                debug!("Cache hit for collaborators: {}", specialist.name);
                return Ok(cached);
            }
        }

        debug!("Finding collaborators for: {}", specialist.name);

        let suggestions = self.provider.find_collaborators(specialist).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, suggestions.clone()).await?;
        }

        Ok(suggestions)
    }

    /// Generate execution plan for a task
    pub async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("plan:{}:{}", specialist.name, task.task_id);

        if self.config.enable_caching {
            if let Some(cached) = self.cache.get::<ExecutionPlan>(&cache_key).await {
                debug!("Cache hit for execution plan");
                return Ok(cached);
            }
        }

        debug!("Generating plan for specialist: {}", specialist.name);

        let plan = self.provider.generate_plan(task, specialist).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, plan.clone()).await?;
        }

        Ok(plan)
    }

    /// Analyze a failure and suggest recovery
    pub async fn analyze_failure(
        &self,
        failure: &FailureContext,
    ) -> Result<FailureAnalysis> {
        self.rate_limiter.check_limit().await?;

        debug!("Analyzing failure for task: {}", failure.task_id);

        let analysis = self.provider.analyze_failure(failure).await?;

        Ok(analysis)
    }

    /// Explain a skill to a specialist
    pub async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("skill:{}:{}", specialist.name, skill_name);

        if self.config.enable_caching {
            if let Some(cached) = self.cache.get::<SkillExplanation>(&cache_key).await {
                debug!("Cache hit for skill explanation");
                return Ok(cached);
            }
        }

        debug!("Explaining skill: {}", skill_name);

        let explanation = self.provider.explain_skill(skill_name, specialist).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, explanation.clone()).await?;
        }

        Ok(explanation)
    }

    /// Get current cost tracking
    pub fn get_cost_info(&self) -> CostInfo {
        self.rate_limiter.get_cost_info()
    }

    /// Check if within cost budget (always true for local GGUF)
    pub fn is_within_budget(&self) -> bool {
        true // Local GGUF has no per-call costs
    }

    /// Clear cache for testing
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    /// Generate UI/UX design variants for the Visionary specialist.
    ///
    /// Results are cached keyed on the intent string. Call `clear_cache()`
    /// to force fresh generation for the same intent.
    /// Generate a domain-specific response for a given intent using a system
    /// prompt appropriate for the specialist's domain.
    ///
    /// Unlike `generate_design()` (which hardcodes a UI/UX system prompt),
    /// this method builds a prompt from the provided `system_prompt` and
    /// `user_prompt`, then calls the provider's `generate_design()` with
    /// Return the LLM configuration for this client (temperature, max_tokens, etc.)
    pub fn config(&self) -> &LLMConfig {
        &self.config
    }

    /// the intent set to the full prompt.  The result is returned as a plain
    /// string (the first variant's description, or the batch output).
    ///
    /// Used by `GenericSpecialist` so each sovereign gets its own domain framing.
    pub async fn generate_domain_response(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        domain: &str,
    ) -> Result<String> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("domain:{}:{:.60}", domain, user_prompt);

        if self.config.enable_caching {
            if let Some(cached) = self.cache.get::<String>(&cache_key).await {
                return Ok(cached);
            }
        }

        // Use the proper chat() path — gives GGUF a real ChatML system+user
        // turn and gives the mock provider structured domain routing.
        let response = self.provider.chat(system_prompt, user_prompt, domain).await
            .unwrap_or_else(|e| format!("[{}] LLM error: {}", domain, e));

        // If GGUF inference is disabled (feature not compiled in), the response
        // starts with "GGUF inference disabled". In that case, fall back to
        // the MockProvider which returns properly structured domain JSON.
        let response = if response.starts_with("GGUF inference disabled") {
            let mock = providers::MockProvider;
            mock.chat(system_prompt, user_prompt, domain).await
                .unwrap_or(response)
        } else {
            response
        };

        if self.config.enable_caching {
            self.cache.set(&cache_key, response.clone()).await?;
        }

        Ok(response)
    }

    pub async fn generate_design(
        &self,
        context: &DesignContext,
    ) -> Result<DesignGeneration> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("design:{}", context.intent);

        if self.config.enable_caching {
            if let Some(cached) = self.cache.get::<DesignGeneration>(&cache_key).await {
                debug!("Cache hit for design generation: {}", context.intent);
                return Ok(cached);
            }
        }

        debug!("Generating design for intent: {}", context.intent);

        let generation = self.provider.generate_design(context).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, generation.clone()).await?;
        }

        Ok(generation)
    }
}

#[derive(Debug, Clone)]
pub struct CostInfo {
    pub tokens_used: u64,
    pub total_cost: f64,
    pub calls_made: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_client_creation() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        };

        let client = LLMClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_cost_tracking() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        };

        let client = LLMClient::new(config).await.unwrap();
        assert!(client.is_within_budget());
    }
}
