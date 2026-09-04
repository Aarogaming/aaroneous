//! crates/orchestrator/src/llm/providers/backend_trait.rs
//! Universal Model Backend Trait for unified local and remote LLM execution.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Structured tool description for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

/// Token usage statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Normalized completion response from any model backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub finish_reason: String,
    pub usage: Option<TokenUsage>,
}

/// Universal asynchronous model provider trait
#[async_trait]
pub trait ModelBackend: Send + Sync {
    /// Human-readable backend identifier (e.g. "Candle-GGUF", "OpenAI-Compat")
    fn backend_name(&self) -> &str;

    /// Generates completion text given system and user prompts
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str) -> Result<ModelResponse>;

    /// Estimate token count for a text input
    fn estimate_tokens(&self, text: &str) -> usize {
        // Universal heuristic fallback: ~4 characters per token
        text.len().div_ceil(4)
    }
}

#[async_trait]
impl ModelBackend for super::openai::OpenAIProvider {
    fn backend_name(&self) -> &str {
        "OpenAI-Compatible"
    }

    async fn generate_response(&self, system_prompt: &str, user_prompt: &str) -> Result<ModelResponse> {
        let text = self.chat_completion(system_prompt, user_prompt).await?;
        let completion_tokens = text.len().div_ceil(4);
        let prompt_tokens = (system_prompt.len() + user_prompt.len()).div_ceil(4);
        Ok(ModelResponse {
            content: text,
            finish_reason: "stop".to_string(),
            usage: Some(TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }),
        })
    }
}

#[async_trait]
impl ModelBackend for super::gguf::GgufProvider {
    fn backend_name(&self) -> &str {
        "Local-GGUF"
    }

    async fn generate_response(&self, system_prompt: &str, user_prompt: &str) -> Result<ModelResponse> {
        let text = self.chat_completion(system_prompt, user_prompt).await?;
        let completion_tokens = text.len().div_ceil(4);
        let prompt_tokens = (system_prompt.len() + user_prompt.len()).div_ceil(4);
        Ok(ModelResponse {
            content: text,
            finish_reason: "stop".to_string(),
            usage: Some(TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }),
        })
    }
}
