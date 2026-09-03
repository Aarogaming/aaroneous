// crates/orchestrator/src/crucible_provider.rs
//! Universal Pluggable Teacher Endpoint Bus for The Crucible.
//!
//! Provides a polymorphic provider interface supporting:
//! 1. LMStudio (localhost:1234)
//! 2. Ollama (localhost:11434)
//! 3. Hugging Face Inference API
//! 4. OpenRouter / Cloud OpenAI-compatible endpoints
//! 5. Auto-Discovery (probes local ports to auto-bind available engines)

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::lmstudio_client::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage};

/// Configuration for any model hosting backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeacherBackendConfig {
    LmStudio {
        endpoint: String,
        model: String,
    },
    Ollama {
        endpoint: String,
        model: String,
    },
    HuggingFace {
        endpoint: String,
        api_key: String,
        model: String,
    },
    OpenRouter {
        api_key: String,
        model: String,
    },
    GenericOpenAiCompatible {
        endpoint: String,
        api_key: Option<String>,
        model: String,
    },
}

impl Default for TeacherBackendConfig {
    fn default() -> Self {
        Self::LmStudio {
            endpoint: "http://localhost:1234/v1".to_string(),
            model: "local-model".to_string(),
        }
    }
}

/// The Universal Teacher Trait
#[async_trait]
pub trait CrucibleTeacherEndpoint: Send + Sync {
    /// Provider identification name (e.g. "LMStudio-Qwen", "Ollama-DeepSeek")
    fn provider_name(&self) -> &str;

    /// Generates a problem or adversarial challenge for the apprentice core
    async fn generate_challenge(&self, scenario_description: &str) -> Result<String>;

    /// Health check to confirm endpoint is reachable
    async fn health_check(&self) -> bool;
}

/// Universal HTTP Implementation of `CrucibleTeacherEndpoint`
pub struct UniversalHttpTeacher {
    config: TeacherBackendConfig,
    client: Client,
}

impl UniversalHttpTeacher {
    pub fn new(config: TeacherBackendConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { config, client }
    }

    /// Auto-detects whether LMStudio (port 1234) or Ollama (port 11434) is currently running
    pub async fn auto_detect_local() -> Option<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap_or_else(|_| Client::new());

        // Probe LMStudio (1234)
        if let Ok(res) = client.get("http://localhost:1234/v1/models").send().await {
            if res.status().is_success() {
                return Some(Self::new(TeacherBackendConfig::LmStudio {
                    endpoint: "http://localhost:1234/v1".to_string(),
                    model: "local-model".to_string(),
                }));
            }
        }

        // Probe Ollama (11434)
        if let Ok(res) = client.get("http://localhost:11434/api/tags").send().await {
            if res.status().is_success() {
                return Some(Self::new(TeacherBackendConfig::Ollama {
                    endpoint: "http://localhost:11434/v1".to_string(),
                    model: "deepseek-r1".to_string(),
                }));
            }
        }

        None
    }
}

#[async_trait]
impl CrucibleTeacherEndpoint for UniversalHttpTeacher {
    fn provider_name(&self) -> &str {
        match &self.config {
            TeacherBackendConfig::LmStudio { .. } => "LMStudio-Local",
            TeacherBackendConfig::Ollama { .. } => "Ollama-Local",
            TeacherBackendConfig::HuggingFace { .. } => "HuggingFace-Inference",
            TeacherBackendConfig::OpenRouter { .. } => "OpenRouter-Cloud",
            TeacherBackendConfig::GenericOpenAiCompatible { .. } => "Generic-OpenAI-API",
        }
    }

    async fn generate_challenge(&self, scenario_description: &str) -> Result<String> {
        let (endpoint, model, api_key) = match &self.config {
            TeacherBackendConfig::LmStudio { endpoint, model } => (endpoint.as_str(), model.as_str(), None),
            TeacherBackendConfig::Ollama { endpoint, model } => (endpoint.as_str(), model.as_str(), None),
            TeacherBackendConfig::HuggingFace { endpoint, api_key, model } => {
                (endpoint.as_str(), model.as_str(), Some(api_key.as_str()))
            }
            TeacherBackendConfig::OpenRouter { api_key, model } => {
                ("https://openrouter.ai/api/v1", model.as_str(), Some(api_key.as_str()))
            }
            TeacherBackendConfig::GenericOpenAiCompatible { endpoint, api_key, model } => {
                (endpoint.as_str(), model.as_str(), api_key.as_deref())
            }
        };

        let system_prompt = "You are the Crucible Master. Generate a precise, mathematical or physical coding scenario for testing an apprentice autonomous agent. Emphasize boundary conditions, friction, or edge cases.";
        let user_prompt = format!("Generate an adversarial test challenge for the following scenario: {scenario_description}");

        let req = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: Some(512),
            stream: false,
        };

        let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
        let mut request_builder = self.client.post(&url).json(&req);

        if let Some(key) = api_key {
            request_builder = request_builder.bearer_auth(key);
        }

        let response = request_builder.send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Teacher provider [{}] returned HTTP {status}: {body}", self.provider_name());
        }

        let resp_body: ChatCompletionResponse = response.json().await?;
        if let Some(choice) = resp_body.choices.into_iter().next() {
            Ok(choice.message.content)
        } else {
            bail!("Teacher provider response contained zero choices");
        }
    }

    async fn health_check(&self) -> bool {
        let (endpoint, api_key) = match &self.config {
            TeacherBackendConfig::LmStudio { endpoint, .. } => (endpoint.as_str(), None),
            TeacherBackendConfig::Ollama { endpoint, .. } => (endpoint.as_str(), None),
            TeacherBackendConfig::HuggingFace { endpoint, api_key, .. } => (endpoint.as_str(), Some(api_key.as_str())),
            TeacherBackendConfig::OpenRouter { api_key, .. } => {
                ("https://openrouter.ai/api/v1", Some(api_key.as_str()))
            }
            TeacherBackendConfig::GenericOpenAiCompatible { endpoint, api_key, .. } => {
                (endpoint.as_str(), api_key.as_deref())
            }
        };

        let url = format!("{}/models", endpoint.trim_end_matches('/'));
        let mut request_builder = self.client.get(&url);
        if let Some(key) = api_key {
            request_builder = request_builder.bearer_auth(key);
        }

        match request_builder.send().await {
            Ok(res) => res.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_teacher_provider_names() {
        let lm = UniversalHttpTeacher::new(TeacherBackendConfig::LmStudio {
            endpoint: "http://localhost:1234/v1".to_string(),
            model: "qwen".to_string(),
        });
        assert_eq!(lm.provider_name(), "LMStudio-Local");

        let ollama = UniversalHttpTeacher::new(TeacherBackendConfig::Ollama {
            endpoint: "http://localhost:11434/v1".to_string(),
            model: "deepseek".to_string(),
        });
        assert_eq!(ollama.provider_name(), "Ollama-Local");

        let hf = UniversalHttpTeacher::new(TeacherBackendConfig::HuggingFace {
            endpoint: "https://api-inference.huggingface.co/v1".to_string(),
            api_key: "hf_dummy".to_string(),
            model: "llama3".to_string(),
        });
        assert_eq!(hf.provider_name(), "HuggingFace-Inference");
    }
}
