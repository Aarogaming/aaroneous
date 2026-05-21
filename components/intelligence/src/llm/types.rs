use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAI,
    GGUF,
    Mock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider_type: ProviderType,
    pub model_name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub gguf_model_path: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub enable_caching: bool,
    pub cache_ttl_secs: u64,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::Mock,
            model_name: "mock".to_string(),
            api_key: None,
            base_url: None,
            gguf_model_path: None,
            temperature: 0.7,
            max_tokens: 4096,
            timeout_secs: 30,
            enable_caching: false,
            cache_ttl_secs: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub complexity: f64,
    pub required_skills: Vec<String>,
    pub estimated_tokens: u32,
}

/// Context for design generation requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesignContext {
    pub style: String,
    pub constraints: Vec<String>,
    pub references: Vec<String>,
    pub style_hints: Vec<String>,
    pub variants_requested: usize,
    pub approved_examples: Vec<String>,
    pub rejected_examples: Vec<String>,
    pub intent: String,
}
