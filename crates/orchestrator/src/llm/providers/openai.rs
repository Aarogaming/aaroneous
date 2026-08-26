//! OpenAI-compatible API provider implementation.

use anyhow::{Context, Result};
use serde_json::json;

use super::super::types::LLMConfig;

/// OpenAI-compatible provider (works with OpenAI, Ollama, LM Studio, etc.)
pub struct OpenAIProvider {
    config: LLMConfig,
    http_client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: LLMConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
        }
    }

    pub async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let base_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let body = json!({
            "model": self.config.model_name,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        let mut req = self.http_client.post(&url).json(&body);
        if let Some(key) = &self.config.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .context("Failed to send request to LLM endpoint")?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        if status.is_success() {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&body_text) {
                if let Some(content) = data["choices"][0]["message"]["content"].as_str() {
                    return Ok(content.to_string());
                }
            }
            // JSON parsed but content not found — return raw body as fallback
            return Ok(body_text);
        }

        Err(anyhow::anyhow!(
            "LLM request failed with status {}: {}",
            status,
            body_text
        ))
    }

    pub async fn embeddings(&self, input: &str) -> Result<Vec<f32>> {
        let base_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let url = format!("{}/embeddings", base_url.trim_end_matches('/'));

        let body = json!({
            "model": "text-embedding-ada-002",
            "input": input
        });

        let mut req = self.http_client.post(&url).json(&body);
        if let Some(key) = &self.config.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .context("Failed to send embedding request")?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        if status.is_success() {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&body_text) {
                if let Some(embedding) = data["data"][0]["embedding"].as_array() {
                    let vec: Vec<f32> = embedding
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect();
                    return Ok(vec);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Embedding request failed with status: {}",
            status
        ))
    }
}
