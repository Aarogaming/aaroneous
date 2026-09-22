use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

/// Cumulative cost tracking for LLM usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostInfo {
    pub tokens_used: u64,
    pub total_cost: f64,
    pub calls_made: u64,
}
