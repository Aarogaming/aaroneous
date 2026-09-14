use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub temperature: f32,
}
impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            temperature: 0.7,
        }
    }
}
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Local,
}
#[derive(Debug, Clone)]
pub struct IntelligenceEngine;
impl Default for IntelligenceEngine {
    fn default() -> Self {
        Self
    }
}
impl IntelligenceEngine {
    pub async fn analyze(&self, _input: &str) -> String {
        "analysis".to_string()
    }
}
