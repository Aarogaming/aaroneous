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
pub use model_environment::{DetectedEnvironment, ModelEnvironment, ModelEnvironmentDetector};
pub use model_loader::{ModelLoader, TOP_RECOMMENDED_MODELS};
pub use model_registry::{ModelInfo, ModelRegistry, ModelType};
pub use providers::{GGUFProvider, LLMProvider, MockProvider};
pub use types::*;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Main LLM client managing different providers
pub struct LLMClient {
    provider: Arc<dyn LLMProvider>,
    cache: cache::LLMCache,
    ast_prefix_mgr: cache::AstPrefixCacheManager,
    rate_limiter: rate_limiter::RateLimiter,
    config: LLMConfig,
}

#[derive(Debug, Clone)]
pub struct LLMConfig {
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
    /// Rate limit: max calls per hour (0 = unlimited for local GGUF)
    pub rate_limit: Option<u32>,
    /// Local LLM endpoint URL (for ProviderType::Local)
    pub local_endpoint: Option<String>,
    /// Local LLM model name (for ProviderType::Local)
    pub local_model: Option<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::GGUF,
            model_name: "qwen2.5-1.5b-instruct".to_string(),
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    GGUF,   // Local GGUF models (llama.cpp) - RECOMMENDED
    Mock,   // Mock provider for testing
    OpenAI, // Cloud OpenAI provider
    Local,  // Local API provider (Ollama, vLLM)
}

impl LLMClient {
    /// Create new LLM client with GGUF provider
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
}
