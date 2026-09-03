// crates/orchestrator/src/lmstudio_client.rs
//! High-Speed Localhost LMStudio & OpenAI-Compatible REST Client.
//!
//! Connects directly to local models running on your PC GPU via LMStudio's
//! local server (default: `http://localhost:1234/v1`).
//! Streams teacher challenge prompts and extracts structured responses
//! for distillation into `.si` cartridges.

use anyhow::{bail, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const DEFAULT_LMSTUDIO_ENDPOINT: &str = "http://localhost:1234/v1";

/// A chat message in the standard OpenAI format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Request payload sent to LMStudio /v1/chat/completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub stream: bool,
}

/// A choice emitted in the chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: usize,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Response payload returned by LMStudio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

/// High-Speed Client for Local LMStudio Instances
pub struct LmStudioClient {
    client: Client,
    endpoint_url: String,
    default_model: String,
}

impl Default for LmStudioClient {
    fn default() -> Self {
        Self::new(DEFAULT_LMSTUDIO_ENDPOINT, "local-model")
    }
}

impl LmStudioClient {
    pub fn new(endpoint_url: impl Into<String>, default_model: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint_url: endpoint_url.into(),
            default_model: default_model.into(),
        }
    }

    /// Queries the local LMStudio model and returns the generated content string
    pub async fn query(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let req = ChatCompletionRequest {
            model: self.default_model.clone(),
            messages,
            temperature: 0.2, // Low temperature for deterministic reasoning & logic
            max_tokens: Some(1024),
            stream: false,
        };

        let url = format!("{}/chat/completions", self.endpoint_url.trim_end_matches('/'));
        let response = self.client.post(&url).json(&req).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("LMStudio returned HTTP error {status}: {body}");
        }

        let resp_body: ChatCompletionResponse = response.json().await?;
        if let Some(choice) = resp_body.choices.into_iter().next() {
            Ok(choice.message.content)
        } else {
            bail!("LMStudio response contained zero choices");
        }
    }

    /// Health check to verify whether LMStudio local server is running
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/models", self.endpoint_url.trim_end_matches('/'));
        match self.client.get(&url).send().await {
            Ok(res) => res.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatCompletionRequest {
            model: "qwen2.5-coder".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            temperature: 0.1,
            max_tokens: Some(256),
            stream: false,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("qwen2.5-coder"));
        assert!(json.contains("ping"));
    }
}
