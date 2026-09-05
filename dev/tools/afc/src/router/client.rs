// dev/tools/afc/src/router/client.rs
use crate::router::types::*;
use crate::state::ContextSanitizer;
use anyhow::{bail, Context, Result};
use reqwest::Client as ReqwestClient;
use std::time::Duration;
use tracing::{info, warn};
use tokio::time::timeout;

const CONTEXT_BUFFER_TOKENS: usize = 1024; // ~4KB generation buffer
const MAX_SAFE_TOKENS: usize = 3200; // Conservative limit for LM Studio

#[derive(Debug, Clone)]
pub struct TypedRouterClient {
    pub host: String,
    pub port: u16,
    pub api_token: Option<String>,
    pub timeout_duration: Duration,
    pub use_http: bool,
}

impl TypedRouterClient {
    pub fn new(host: impl Into<String>, port: u16, api_token: Option<String>) -> Self {
        Self {
            host: host.into(),
            port,
            api_token,
            timeout_duration: Duration::from_secs(120),
            use_http: true,
        }
    }

    pub fn local_lm_studio(api_token: Option<String>) -> Self {
        Self::new("127.0.0.1", 1234, api_token)
    }

    pub fn local_ollama() -> Self {
        Self::new("127.0.0.1", 11434, None)
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    pub fn with_http(mut self, enabled: bool) -> Self {
        self.use_http = enabled;
        self
    }

    /// Estimate token count from message content (rough approximation: 1 token ≈ 4 chars)
    pub fn estimate_token_count(&self, messages: &[ChatMessage]) -> usize {
        messages.iter().map(|m| m.content.len() / 4).sum::<usize>().max(1)
    }

    /// Check if payload approaches safe threshold (reserving 4k buffer)
    pub fn should_compact(&self, token_estimate: usize) -> bool {
        const CONTEXT_BUFFER_TOKENS: usize = 1024; // ~4KB generation buffer
        const MAX_SAFE_TOKENS: usize = 3200; // Conservative limit for LM Studio
        token_estimate >= (MAX_SAFE_TOKENS - CONTEXT_BUFFER_TOKENS)
    }

    /// Preemptively compact context if payload approaches threshold
    pub async fn pre_check_and_compact(
        &self,
        messages: &mut Vec<ChatMessage>,
    ) -> Result<()> {
        let token_estimate = self.estimate_token_count(messages);
        
        if !self.should_compact(token_estimate) {
            return Ok(());
        }

        info!(
            "[Client] Token estimate: {} (threshold: {}), triggering pre-emptive compaction",
            token_estimate,
            MAX_SAFE_TOKENS - CONTEXT_BUFFER_TOKENS
        );

        ContextSanitizer::compact(messages).context("Failed to compact context")?;

        let after_token_estimate = self.estimate_token_count(messages);
        info!(
            "[Client] After compaction: {} tokens (saved {})",
            after_token_estimate,
            token_estimate - after_token_estimate
        );

        Ok(())
    }

    /// Send a strongly-typed chat completion request with resilience loop
    pub async fn complete(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let reqwest_client = ReqwestClient::builder()
            .timeout(self.timeout_duration)
            .build()
            .context("Failed to create reqwest client")?;

        let url = format!("http://{}:{}/v1/chat/completions", self.host, self.port);
        
        let serialized =
            serde_json::to_string(req).context("Failed to serialize ChatCompletionRequest")?;

        let mut attempt = 0;
        const MAX_RETRIES: usize = 3;
        const BACKOFF_MS: u64 = 1500;

        loop {
            attempt += 1;
            info!("[Client] Attempt {attempt}/{MAX_RETRIES} to {}", url);

            let request = reqwest_client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .body(serialized.clone());

            let response = match timeout(self.timeout_duration, request.send()).await {
                Ok(Ok(res)) => res,
                Ok(Err(e)) => {
                    bail!("Request failed: {}", e);
                }
                Err(_) => {
                    bail!("Connection timed out");
                }
            };

            let status = response.status();

            if status.is_success() {
                let body = response.text().await.context("Failed to read response body")?;
                
                if let Some(json_start) = body.find('{') {
                    let res: ChatCompletionResponse = serde_json::from_str(&body[json_start..])
                        .context("Failed to parse ChatCompletionResponse JSON")?;
                    return Ok(res);
                } else {
                    bail!("Invalid response format: no JSON object found");
                }
            }

            match status.as_u16() {
                400 => {
                    warn!("[Client] HTTP 400 Bad Request - likely context length exceeded");
                    if attempt > MAX_RETRIES {
                        bail!("Max retries ({MAX_RETRIES}) exceeded after HTTP 400 errors");
                    }

                    let mut compacted_req = req.clone();
                    self.pre_check_and_compact(&mut compacted_req.messages)
                        .await.context("Failed to compact context after HTTP 400")?;

                    info!("[Client] Retrying with compacted payload...");
                    tokio::time::sleep(Duration::from_millis(BACKOFF_MS)).await;
                }
                408 | 409 | 413 | 429 => {
                    warn!(
                        "[Client] HTTP {} - server constraint",
                        status.as_u16()
                    );
                    if attempt > MAX_RETRIES {
                        bail!("Max retries ({MAX_RETRIES}) exceeded after server constraint errors");
                    }

                    let mut compacted_req = req.clone();
                    self.pre_check_and_compact(&mut compacted_req.messages)
                        .await.context("Failed to compact context")?;

                    info!("[Client] Retrying with compacted payload...");
                    tokio::time::sleep(Duration::from_millis(BACKOFF_MS)).await;
                }
                401 | 403 => {
                    bail!("Authentication error (401/403)");
                }
                500 | 502 | 503 | 504 => {
                    warn!("[Client] Server error HTTP {} - retrying", status.as_u16());
                    if attempt > MAX_RETRIES {
                        bail!("Max retries ({MAX_RETRIES}) exceeded after server errors");
                    }

                    tokio::time::sleep(Duration::from_millis(BACKOFF_MS)).await;
                }
                _ => {
                    bail!("Unexpected HTTP status: {}", status);
                }
            }
        }
    }
}
