// LLM Integration Module
// Provides abstraction layer for multiple LLM providers
// Supports: OpenAI, Local (Ollama/vLLM), Anthropic, Mock

pub mod auto_discover;
pub mod cache;
pub mod mcp_gateway;
pub mod model_environment;
pub mod model_loader;
pub mod model_registry;
pub mod providers;
pub mod rate_limiter;
pub mod types;

pub use cache::{AstPrefixCacheManager, LLMCache};
pub use mcp_gateway::McpGateway;
pub use model_environment::{
    DetectedEnvironment, ModelEnvironment, ModelEnvironmentDetector, StrictModelEnvironmentConfig,
    StrictModelEnvironmentError,
};
pub use model_loader::{ModelLoader, TOP_RECOMMENDED_MODELS};
pub use model_registry::{ModelInfo, ModelRegistry, ModelType};
pub use providers::{GGUFProvider, GeminiProvider, LLMProvider, MockProvider};
use thiserror::Error;
pub use types::*;

/// Explicit missing-input and validation errors for strict controller LLM initialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrictLLMError {
    #[error("Missing required model name")]
    MissingModel,
    #[error("Missing required GGUF model path")]
    MissingModelPath,
    #[error("GGUF model file not found at path: {0}")]
    ModelPathNotFound(PathBuf),
    #[error("Specified GGUF model path is a directory, expected a file: {0}")]
    ModelPathIsDirectory(PathBuf),
    #[error("Missing required GGUF context size (must be explicitly provided)")]
    MissingGgufContextSize,
    #[error("Missing required GGUF thread count (must be explicitly provided)")]
    MissingGgufThreads,
    #[error("Missing required local LLM endpoint URL")]
    MissingEndpoint,
    #[error("Invalid endpoint URL: {0}")]
    InvalidEndpoint(String),
    #[error("Missing required API key for provider {0:?}")]
    MissingApiKey(ProviderType),
    #[error("Missing required base URL for provider {0:?}")]
    MissingBaseUrl(ProviderType),
    #[error("Provider initialization failed: {0}")]
    ProviderInitialization(String),
}

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

/// Main LLM client managing different providers
pub struct LLMClient {
    provider: Arc<dyn LLMProvider>,
    cache: cache::LLMCache,
    ast_prefix_mgr: cache::AstPrefixCacheManager,
    rate_limiter: rate_limiter::RateLimiter,
    config: LLMConfig,
}

impl std::fmt::Debug for LLMClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMClient")
            .field("config", &self.config)
            .finish()
    }
}

#[derive(Clone)]
pub struct LLMConfig {
    pub provider_type: ProviderType,
    pub model_name: String,
    pub explicit_model_name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub enable_caching: bool,
    pub cache_ttl_secs: u64,
    pub gguf_model_path: Option<PathBuf>,
    /// Rate limit: max calls per hour (0 = unlimited for local GGUF)
    pub rate_limit: Option<u32>,
    /// Local LLM endpoint URL (for ProviderType::Local)
    pub local_endpoint: Option<String>,
    /// Local LLM model name (for ProviderType::Local)
    pub local_model: Option<String>,
    /// Gemini API base URL override (for ProviderType::Gemini). Falls back to
    /// `providers::gemini::DEFAULT_GEMINI_BASE_URL` when `None`.
    pub gemini_base_url: Option<String>,
    /// Explicit GGUF context size (defaults to 2048 if omitted)
    pub gguf_context_size: Option<u32>,
    /// Explicit GGUF execution threads (defaults to 8 if omitted)
    pub gguf_threads: Option<u32>,
}

// Manual impl (not derived) so `api_key` is redacted from any panic or log
// output that formats this struct, rather than printed verbatim.
impl std::fmt::Debug for LLMConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMConfig")
            .field("provider_type", &self.provider_type)
            .field("model_name", &self.model_name)
            .field("explicit_model_name", &self.explicit_model_name)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("base_url", &self.base_url)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("timeout_secs", &self.timeout_secs)
            .field("enable_caching", &self.enable_caching)
            .field("cache_ttl_secs", &self.cache_ttl_secs)
            .field("gguf_model_path", &self.gguf_model_path)
            .field("rate_limit", &self.rate_limit)
            .field("local_endpoint", &self.local_endpoint)
            .field("local_model", &self.local_model)
            .field("gemini_base_url", &self.gemini_base_url)
            .field("gguf_context_size", &self.gguf_context_size)
            .field("gguf_threads", &self.gguf_threads)
            .finish()
    }
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::GGUF,
            model_name: "qwen2.5-1.5b-instruct".to_string(),
            explicit_model_name: None,
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: 2048,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
            rate_limit: None,
            local_endpoint: None,
            local_model: None,
            gemini_base_url: None,
            gguf_context_size: None,
            gguf_threads: None,
        }
    }
}

/// Fully explicit, zero-fallback LLM configuration for external controllers.
///
/// Unlike `LLMConfig`, `StrictLLMConfig` intentionally implements NO `Default`.
/// Every field required for the selected provider must be explicitly supplied.
#[derive(Clone)]
pub struct StrictLLMConfig {
    pub provider_type: ProviderType,
    pub model_name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub enable_caching: bool,
    pub cache_ttl_secs: u64,
    pub gguf_model_path: Option<PathBuf>,
    pub rate_limit: Option<u32>,
    pub local_endpoint: Option<String>,
    pub gemini_base_url: Option<String>,
    pub gguf_context_size: Option<u32>,
    pub gguf_threads: Option<u32>,
}

impl std::fmt::Debug for StrictLLMConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrictLLMConfig")
            .field("provider_type", &self.provider_type)
            .field("model_name", &self.model_name)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("base_url", &self.base_url)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("timeout_secs", &self.timeout_secs)
            .field("enable_caching", &self.enable_caching)
            .field("cache_ttl_secs", &self.cache_ttl_secs)
            .field("gguf_model_path", &self.gguf_model_path)
            .field("rate_limit", &self.rate_limit)
            .field("local_endpoint", &self.local_endpoint)
            .field("gemini_base_url", &self.gemini_base_url)
            .field("gguf_context_size", &self.gguf_context_size)
            .field("gguf_threads", &self.gguf_threads)
            .finish()
    }
}

impl StrictLLMConfig {
    pub fn validate(&self) -> Result<(), StrictLLMError> {
        if self.model_name.trim().is_empty() {
            return Err(StrictLLMError::MissingModel);
        }
        match self.provider_type {
            ProviderType::Local => {
                let ep = self.local_endpoint.as_deref().unwrap_or("").trim();
                if ep.is_empty() {
                    return Err(StrictLLMError::MissingEndpoint);
                }
                if !ep.starts_with("http://") && !ep.starts_with("https://") {
                    return Err(StrictLLMError::InvalidEndpoint(ep.to_string()));
                }
            }
            ProviderType::GGUF => {
                let path = self
                    .gguf_model_path
                    .as_ref()
                    .ok_or(StrictLLMError::MissingModelPath)?;
                if path.as_os_str().is_empty() {
                    return Err(StrictLLMError::MissingModelPath);
                }
                if !path.exists() {
                    return Err(StrictLLMError::ModelPathNotFound(path.clone()));
                }
                if path.is_dir() {
                    return Err(StrictLLMError::ModelPathIsDirectory(path.clone()));
                }
                if self.gguf_context_size.is_none() {
                    return Err(StrictLLMError::MissingGgufContextSize);
                }
                if self.gguf_threads.is_none() {
                    return Err(StrictLLMError::MissingGgufThreads);
                }
            }
            ProviderType::OpenAI => {
                let key = self.api_key.as_deref().unwrap_or("").trim();
                if key.is_empty() {
                    return Err(StrictLLMError::MissingApiKey(ProviderType::OpenAI));
                }
            }
            ProviderType::Gemini => {
                let key = self.api_key.as_deref().unwrap_or("").trim();
                if key.is_empty() {
                    return Err(StrictLLMError::MissingApiKey(ProviderType::Gemini));
                }
                let base_url = self
                    .gemini_base_url
                    .as_deref()
                    .or(self.base_url.as_deref())
                    .unwrap_or("")
                    .trim();
                if base_url.is_empty() {
                    return Err(StrictLLMError::MissingBaseUrl(ProviderType::Gemini));
                }
            }
            ProviderType::Mock => {}
        }
        Ok(())
    }
}

impl LLMConfig {
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        let m = model.into();
        self.explicit_model_name = Some(m.clone());
        self.model_name = m;
        self
    }

    pub fn with_gguf_params(mut self, context_size: u32, threads: u32) -> Self {
        self.gguf_context_size = Some(context_size);
        self.gguf_threads = Some(threads);
        self
    }

    /// Converts this configuration to a strictly validated `StrictLLMConfig`.
    ///
    /// Requires an explicit model name (default placeholder strings are strictly rejected).
    pub fn to_strict(&self) -> Result<StrictLLMConfig, StrictLLMError> {
        let model = self
            .explicit_model_name
            .as_deref()
            .or_else(|| {
                if self.model_name != "qwen2.5-1.5b-instruct" && !self.model_name.trim().is_empty()
                {
                    Some(self.model_name.as_str())
                } else {
                    None
                }
            })
            .or(self.local_model.as_deref())
            .ok_or(StrictLLMError::MissingModel)?;

        if model.trim().is_empty() {
            return Err(StrictLLMError::MissingModel);
        }

        let strict = StrictLLMConfig {
            provider_type: self.provider_type,
            model_name: model.to_string(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            timeout_secs: self.timeout_secs,
            enable_caching: self.enable_caching,
            cache_ttl_secs: self.cache_ttl_secs,
            gguf_model_path: self.gguf_model_path.clone(),
            rate_limit: self.rate_limit,
            local_endpoint: self.local_endpoint.clone(),
            gemini_base_url: self.gemini_base_url.clone(),
            gguf_context_size: self.gguf_context_size,
            gguf_threads: self.gguf_threads,
        };

        strict.validate()?;
        Ok(strict)
    }

    /// Validates this configuration against the strict external-controller contract.
    ///
    /// Every input required for the selected provider MUST be explicitly provided.
    /// Host-local fallbacks (e.g. `http://localhost:11434`, `mistral:latest`, `models/qwen2.5-1.5b.gguf`,
    /// or `DEFAULT_GEMINI_BASE_URL`) will NOT be substituted.
    pub fn validate_strict(&self) -> Result<(), StrictLLMError> {
        self.to_strict().map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    GGUF,   // Local GGUF models (llama.cpp) - RECOMMENDED
    Mock,   // Mock provider for testing
    OpenAI, // Cloud OpenAI provider
    Local,  // Local API provider (Ollama, vLLM)
    Gemini, // Cloud Google Gemini provider
}

impl LLMClient {
    /// Create new LLM client with strict explicit inputs for external controllers.
    ///
    /// Validates all parameters explicitly with ZERO ambient or host-local fallbacks.
    /// Returns typed `StrictLLMError` on missing or invalid inputs.
    pub async fn new_strict(config: LLMConfig) -> Result<Self, StrictLLMError> {
        let strict = config.to_strict()?;
        Self::new_from_strict_config(strict).await
    }

    /// Create new LLM client from a `StrictLLMConfig`.
    pub async fn new_from_strict_config(config: StrictLLMConfig) -> Result<Self, StrictLLMError> {
        config.validate()?;

        let provider: Arc<dyn LLMProvider> = match config.provider_type {
            ProviderType::OpenAI => {
                let api_key = config
                    .api_key
                    .clone()
                    .ok_or(StrictLLMError::MissingApiKey(ProviderType::OpenAI))?;
                let model = config.model_name.clone();
                Arc::new(
                    providers::OpenAIProvider::new_explicit(
                        api_key,
                        model,
                        config.base_url.clone(),
                    )
                    .await
                    .map_err(|e| StrictLLMError::ProviderInitialization(e.to_string()))?,
                )
            }
            ProviderType::Local => {
                let endpoint = config
                    .local_endpoint
                    .clone()
                    .ok_or(StrictLLMError::MissingEndpoint)?;
                let model = config.model_name.clone();
                Arc::new(
                    providers::LocalLLMProvider::new(endpoint, model)
                        .await
                        .map_err(|e| StrictLLMError::ProviderInitialization(e.to_string()))?,
                )
            }
            ProviderType::Gemini => {
                let api_key = config
                    .api_key
                    .clone()
                    .ok_or(StrictLLMError::MissingApiKey(ProviderType::Gemini))?;
                let base_url = config
                    .gemini_base_url
                    .clone()
                    .or_else(|| config.base_url.clone())
                    .ok_or(StrictLLMError::MissingBaseUrl(ProviderType::Gemini))?;
                let model = config.model_name.clone();
                let client = reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .map_err(|e| StrictLLMError::ProviderInitialization(e.to_string()))?;
                Arc::new(
                    providers::GeminiProvider::new(api_key, base_url, model, client)
                        .await
                        .map_err(|e| StrictLLMError::ProviderInitialization(e.to_string()))?,
                )
            }
            ProviderType::GGUF => {
                let model_path = config
                    .gguf_model_path
                    .clone()
                    .ok_or(StrictLLMError::MissingModelPath)?;
                let ctx_size = config
                    .gguf_context_size
                    .ok_or(StrictLLMError::MissingGgufContextSize)?;
                let threads = config
                    .gguf_threads
                    .ok_or(StrictLLMError::MissingGgufThreads)?;
                Arc::new(
                    GGUFProvider::new(model_path, ctx_size, threads)
                        .map_err(|e| StrictLLMError::ProviderInitialization(e.to_string()))?,
                )
            }
            ProviderType::Mock => Arc::new(MockProvider),
        };

        let cache = cache::LLMCache::new(config.cache_ttl_secs);
        let ast_prefix_mgr = cache::AstPrefixCacheManager::new();
        let rate_limit = config.rate_limit.unwrap_or(0);
        let rate_limiter = rate_limiter::RateLimiter::new(rate_limit);

        let converted_config = LLMConfig {
            provider_type: config.provider_type,
            model_name: config.model_name.clone(),
            explicit_model_name: Some(config.model_name),
            api_key: config.api_key,
            base_url: config.base_url,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            timeout_secs: config.timeout_secs,
            enable_caching: config.enable_caching,
            cache_ttl_secs: config.cache_ttl_secs,
            gguf_model_path: config.gguf_model_path,
            rate_limit: config.rate_limit,
            local_endpoint: config.local_endpoint,
            local_model: None,
            gemini_base_url: config.gemini_base_url,
            gguf_context_size: config.gguf_context_size,
            gguf_threads: config.gguf_threads,
        };

        Ok(Self {
            provider,
            cache,
            ast_prefix_mgr,
            rate_limiter,
            config: converted_config,
        })
    }

    /// Legacy/bootstrap compatibility adapter for developer and interactive workflows.
    ///
    /// Applies host-local default fallbacks for unconfigured fields.
    pub async fn bootstrap_adapter(config: LLMConfig) -> Result<Self> {
        Self::new(config).await
    }

    /// Create new LLM client with fallback defaults (legacy constructor).
    ///
    /// External controllers MUST use [`Self::new_strict`] instead.
    pub async fn new(config: LLMConfig) -> Result<Self> {
        info!(
            "Initializing LLM client with provider: {:?}",
            config.provider_type
        );

        let provider: Arc<dyn LLMProvider> = match config.provider_type {
            ProviderType::OpenAI => {
                let api_key = config.api_key.clone().ok_or_else(|| {
                    anyhow::anyhow!("OPENAI_API_KEY must be provided via LLMConfig")
                })?;
                Arc::new(providers::OpenAIProvider::new(api_key).await?)
            }
            ProviderType::Local => {
                let endpoint = config
                    .local_endpoint
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let model = config
                    .local_model
                    .clone()
                    .unwrap_or_else(|| "mistral:latest".to_string());
                Arc::new(providers::LocalLLMProvider::new(endpoint, model).await?)
            }
            ProviderType::Gemini => {
                let api_key = config.api_key.clone().ok_or_else(|| {
                    anyhow::anyhow!("Gemini API key must be provided via LLMConfig")
                })?;
                let base_url = config
                    .gemini_base_url
                    .clone()
                    .or_else(|| config.base_url.clone())
                    .unwrap_or_else(|| providers::DEFAULT_GEMINI_BASE_URL.to_string());
                let model = config.model_name.clone();
                let client = reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .timeout(std::time::Duration::from_secs(120))
                    .build()?;
                Arc::new(providers::GeminiProvider::new(api_key, base_url, model, client).await?)
            }
            ProviderType::GGUF => {
                let model_path = config
                    .gguf_model_path
                    .clone()
                    .unwrap_or_else(|| std::path::PathBuf::from("models/qwen2.5-1.5b.gguf"));

                std::sync::Arc::new(GGUFProvider::new(model_path, 2048, 8)?)
            }
            ProviderType::Mock => Arc::new(MockProvider),
        };

        let cache = cache::LLMCache::new(config.cache_ttl_secs);
        let ast_prefix_mgr = cache::AstPrefixCacheManager::new();
        let rate_limit = config.rate_limit.unwrap_or(0);
        let rate_limiter = rate_limiter::RateLimiter::new(rate_limit);

        info!("LLM client initialized successfully");

        Ok(Self {
            provider,
            cache,
            ast_prefix_mgr,
            rate_limiter,
            config,
        })
    }

    /// Access the underlying AST prefix cache manager
    pub fn ast_prefix_manager(&self) -> &cache::AstPrefixCacheManager {
        &self.ast_prefix_mgr
    }

    /// Pin codebase context using demand-driven AST caching for maximal KV cache reuse
    pub fn pin_code_context(
        &self,
        path: &str,
        content: &str,
    ) -> Result<(
        transpiler::prefix_cache_integration::PromptPrefixKey,
        String,
    )> {
        self.ast_prefix_mgr
            .format_pinned_code_context(path, content)
    }

    /// Analyze a task to determine best approach
    pub async fn analyze_task(&self, task_context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("analyze_task:{}", task_context.task_id);

        // Check cache first
        if self.config.enable_caching
            && let Some(cached) = self.cache.get::<TaskAnalysis>(&cache_key).await
        {
            debug!("Cache hit for task analysis: {}", task_context.task_id);
            return Ok(cached);
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

        if self.config.enable_caching
            && let Some(cached) = self
                .cache
                .get::<Vec<CollaboratorSuggestion>>(&cache_key)
                .await
        {
            debug!("Cache hit for collaborators: {}", specialist.name);
            return Ok(cached);
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

        if self.config.enable_caching
            && let Some(cached) = self.cache.get::<ExecutionPlan>(&cache_key).await
        {
            debug!("Cache hit for execution plan");
            return Ok(cached);
        }

        debug!("Generating plan for specialist: {}", specialist.name);

        let plan = self.provider.generate_plan(task, specialist).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, plan.clone()).await?;
        }

        Ok(plan)
    }

    /// Analyze a failure and suggest recovery
    pub async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis> {
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

        if self.config.enable_caching
            && let Some(cached) = self.cache.get::<SkillExplanation>(&cache_key).await
        {
            debug!("Cache hit for skill explanation");
            return Ok(cached);
        }

        debug!("Explaining skill: {}", skill_name);

        let explanation = self.provider.explain_skill(skill_name, specialist).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, explanation.clone()).await?;
        }

        Ok(explanation)
    }

    /// Analyze task context for intent analysis.
    pub async fn analyze_context(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        self.analyze_task(context).await
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
    /// Return the LLM configuration for this client (temperature, max_tokens, etc.)
    pub fn config(&self) -> &LLMConfig {
        &self.config
    }

    /// Generate a domain-specific response for a given intent using a system
    /// prompt appropriate for the specialist's domain.
    ///
    /// Results are cached keyed on the domain, system prompt, and user prompt. Call `clear_cache()`
    /// to force fresh generation.
    pub async fn generate_domain_response(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        domain: &str,
    ) -> Result<String> {
        self.rate_limiter.check_limit().await?;

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&(domain, system_prompt, user_prompt), &mut hasher);
        let cache_key = format!(
            "domain:{}:{:016x}",
            domain,
            std::hash::Hasher::finish(&hasher)
        );

        if self.config.enable_caching
            && let Some(cached) = self.cache.get::<String>(&cache_key).await
        {
            return Ok(cached);
        }

        // Use the proper chat() path — gives GGUF a real ChatML system+user
        // turn and gives the mock provider structured domain routing.
        let response = self
            .provider
            .chat(system_prompt, user_prompt, domain)
            .await
            .unwrap_or_else(|e| format!("[{}] LLM error: {}", domain, e));

        // If GGUF inference is disabled (feature not compiled in), the response
        // starts with "GGUF inference disabled". In that case, fall back to
        // the MockProvider which returns properly structured domain JSON.
        let response = if response.starts_with("GGUF inference disabled") {
            let mock = providers::MockProvider;
            mock.chat(system_prompt, user_prompt, domain)
                .await
                .unwrap_or(response)
        } else {
            response
        };

        if self.config.enable_caching {
            self.cache.set(&cache_key, response.clone()).await?;
        }

        Ok(response)
    }

    /// Send a chat completion request to the provider with domain framing, rate limiting, and response caching.
    pub async fn chat(
        &self,
        system_prompt: &str,
        user_message: &str,
        domain: &str,
    ) -> Result<String> {
        self.generate_domain_response(system_prompt, user_message, domain)
            .await
    }

    /// Perform a chat completion with deterministic AST prefix context pinning.
    ///
    /// Computes or retrieves the Salsa-style AST node signature key for `file_path`,
    /// formats the pinned code context header, injects it into the prompt prefix to ensure
    /// stable KV cache slots on local runners (e.g., LM Studio, llama.cpp, GGUF), and dispatches
    /// the request via the configured provider.
    pub async fn chat_with_ast_prefix(
        &self,
        system_prompt: &str,
        user_message: &str,
        file_path: &str,
        file_content: &str,
        domain: &str,
    ) -> Result<(
        transpiler::prefix_cache_integration::PromptPrefixKey,
        String,
    )> {
        let (prefix_key, pinned_ast_context) = self.pin_code_context(file_path, file_content)?;
        let combined_system = if system_prompt.is_empty() {
            pinned_ast_context
        } else {
            format!("{}\n\n{}", system_prompt, pinned_ast_context)
        };

        let response = self.chat(&combined_system, user_message, domain).await?;
        Ok((prefix_key, response))
    }

    pub async fn generate_design(&self, context: &DesignContext) -> Result<DesignGeneration> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("design:{}", context.intent);

        if self.config.enable_caching
            && let Some(cached) = self.cache.get::<DesignGeneration>(&cache_key).await
        {
            debug!("Cache hit for design generation: {}", context.intent);
            return Ok(cached);
        }

        debug!("Generating design for intent: {}", context.intent);

        let generation = self.provider.generate_design(context).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, generation.clone()).await?;
        }

        Ok(generation)
    }

    /// Generate an embedding vector for the given text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.rate_limiter.check_limit().await?;

        let cache_key = format!("embed:{}", text);
        if self.config.enable_caching
            && let Some(cached) = self.cache.get::<Vec<f32>>(&cache_key).await
        {
            return Ok(cached);
        }

        let vector = self.provider.embed(text).await?;

        if self.config.enable_caching {
            self.cache.set(&cache_key, vector.clone()).await?;
        }

        Ok(vector)
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
            model_name: "mock".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
            rate_limit: None,
            local_endpoint: None,
            local_model: None,
            gemini_base_url: None,
            explicit_model_name: None,
            gguf_context_size: None,
            gguf_threads: None,
        };

        let client = LLMClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_cost_tracking() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            model_name: "mock".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
            rate_limit: None,
            local_endpoint: None,
            local_model: None,
            gemini_base_url: None,
            explicit_model_name: None,
            gguf_context_size: None,
            gguf_threads: None,
        };

        let client = LLMClient::new(config).await.unwrap();
        assert!(client.is_within_budget());
    }

    #[tokio::test]
    async fn test_chat_and_ast_prefix_pinning() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            model_name: "mock".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
            rate_limit: None,
            local_endpoint: None,
            local_model: None,
            gemini_base_url: None,
            explicit_model_name: None,
            gguf_context_size: None,
            gguf_threads: None,
        };

        let client = LLMClient::new(config)
            .await
            .expect("client creation failed");

        // Basic chat test
        let chat_res = client
            .chat("You are an assistant.", "Hello world", "general")
            .await
            .expect("chat failed");
        assert!(!chat_res.is_empty());

        // AST prefix pinning test
        let sys_prompt = "You are a code auditor.";
        let user_query = "Inspect this function.";
        let file_path = "crates/demo/src/main.rs";
        let content_v1 = "read sensor; compute delta; write motor;";

        let (key1, resp1) = client
            .chat_with_ast_prefix(sys_prompt, user_query, file_path, content_v1, "pipeline")
            .await
            .expect("chat_with_ast_prefix failed");

        assert!(!resp1.is_empty());
        assert_ne!(key1.hash, 0);

        // Identical file content query produces exact same prefix key and matches response cache
        let (key2, resp2) = client
            .chat_with_ast_prefix(sys_prompt, user_query, file_path, content_v1, "pipeline")
            .await
            .expect("repeat chat_with_ast_prefix failed");

        assert_eq!(key1, key2);
        assert_eq!(resp1, resp2);

        // Edit source file: adds an operation into the AST DAG
        let content_v2 = "read sensor; compute delta; filter noise; write motor;";
        let (key3, _resp3) = client
            .chat_with_ast_prefix(sys_prompt, user_query, file_path, content_v2, "pipeline")
            .await
            .expect("edit chat_with_ast_prefix failed");

        // Prefix key must change because the AST structure changed
        assert_ne!(key1.hash, key3.hash);
    }

    #[tokio::test]
    async fn test_strict_missing_model() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            model_name: "".to_string(),
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingModel);
    }

    #[tokio::test]
    async fn test_strict_defaulted_model_name_rejected_without_explicit_specification() {
        let config = LLMConfig {
            provider_type: ProviderType::Local,
            local_endpoint: Some("http://10.0.0.5:8000".to_string()),
            ..Default::default()
        };
        // Default::default() has a fallback model string, but strict validation MUST reject
        // unless explicitly supplied
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictLLMError::MissingModel);

        // Explicit with_model must succeed
        let explicit = config.with_model("qwen2.5-1.5b-instruct");
        assert!(explicit.validate_strict().is_ok());
    }

    #[tokio::test]
    async fn test_strict_missing_endpoint_local() {
        let config = LLMConfig {
            provider_type: ProviderType::Local,
            model_name: "custom-model".to_string(),
            local_endpoint: None,
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingEndpoint);
    }

    #[tokio::test]
    async fn test_strict_invalid_endpoint_local() {
        let config = LLMConfig {
            provider_type: ProviderType::Local,
            model_name: "custom-model".to_string(),
            local_endpoint: Some("not-a-valid-url".to_string()),
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(
            err,
            StrictLLMError::InvalidEndpoint("not-a-valid-url".to_string())
        );
    }

    #[tokio::test]
    async fn test_strict_missing_model_local() {
        let config = LLMConfig {
            provider_type: ProviderType::Local,
            model_name: "   ".to_string(),
            local_endpoint: Some("http://localhost:11434".to_string()),
            local_model: None,
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingModel);
    }

    #[tokio::test]
    async fn test_strict_missing_model_path_gguf() {
        let config = LLMConfig {
            provider_type: ProviderType::GGUF,
            model_name: "qwen".to_string(),
            gguf_model_path: None,
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingModelPath);
    }

    #[tokio::test]
    async fn test_strict_model_path_not_found_gguf() {
        let temp = tempfile::tempdir().unwrap();
        let missing_path = temp.path().join("non_existent_model_file_98765.gguf");
        let config = LLMConfig {
            provider_type: ProviderType::GGUF,
            model_name: "qwen".to_string(),
            gguf_model_path: Some(missing_path.clone()),
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::ModelPathNotFound(missing_path));
    }

    #[tokio::test]
    async fn test_strict_model_path_is_dir_gguf() {
        let temp = tempfile::tempdir().unwrap();
        let dir_path = temp.path().to_path_buf();
        let config = LLMConfig {
            provider_type: ProviderType::GGUF,
            model_name: "qwen".to_string(),
            gguf_model_path: Some(dir_path.clone()),
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::ModelPathIsDirectory(dir_path));
    }

    #[tokio::test]
    async fn test_strict_missing_gguf_context_size() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("model.gguf");
        std::fs::write(&file_path, b"dummy").unwrap();

        let config = LLMConfig {
            provider_type: ProviderType::GGUF,
            model_name: "qwen".to_string(),
            gguf_model_path: Some(file_path),
            gguf_context_size: None,
            gguf_threads: Some(4),
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingGgufContextSize);
    }

    #[tokio::test]
    async fn test_strict_missing_gguf_threads() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("model.gguf");
        std::fs::write(&file_path, b"dummy").unwrap();

        let config = LLMConfig {
            provider_type: ProviderType::GGUF,
            model_name: "qwen".to_string(),
            gguf_model_path: Some(file_path),
            gguf_context_size: Some(2048),
            gguf_threads: None,
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingGgufThreads);
    }

    #[tokio::test]
    async fn test_strict_missing_api_key_openai() {
        let config = LLMConfig {
            provider_type: ProviderType::OpenAI,
            model_name: "gpt-4".to_string(),
            api_key: None,
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingApiKey(ProviderType::OpenAI));
    }

    #[tokio::test]
    async fn test_strict_missing_base_url_gemini() {
        let config = LLMConfig {
            provider_type: ProviderType::Gemini,
            model_name: "gemini-1.5-flash".to_string(),
            api_key: Some("dummy-key".to_string()),
            gemini_base_url: None,
            base_url: None,
            ..Default::default()
        };
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingBaseUrl(ProviderType::Gemini));
    }

    #[tokio::test]
    async fn test_strict_supplied_values_mock() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            model_name: "test-mock-model".to_string(),
            ..Default::default()
        };
        let client = LLMClient::new_strict(config).await;
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config().model_name, "test-mock-model");
    }

    #[tokio::test]
    async fn test_strict_supplied_values_local_config_validation() {
        let config = LLMConfig {
            provider_type: ProviderType::Local,
            model_name: "llama3:latest".to_string(),
            local_endpoint: Some("http://127.0.0.1:8000".to_string()),
            ..Default::default()
        };
        assert!(config.validate_strict().is_ok());
    }

    #[tokio::test]
    async fn test_strict_absence_of_ambient_fallback() {
        // Default LLMConfig has provider_type GGUF and gguf_model_path None.
        // On strict route with explicit model, this must fail rather than defaulting to models/qwen2.5-1.5b.gguf.
        let config = LLMConfig::default().with_model("qwen2.5-1.5b-instruct");
        let err = LLMClient::new_strict(config).await.unwrap_err();
        assert_eq!(err, StrictLLMError::MissingModelPath);
    }

    #[tokio::test]
    async fn test_bootstrap_adapter_retains_legacy_behavior() {
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            ..Default::default()
        };
        let client = LLMClient::bootstrap_adapter(config).await;
        assert!(client.is_ok());
    }
}
