// dev/tools/afc/src/router/client.rs
use crate::router::types::*;
use anyhow::{bail, Context, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct TypedRouterClient {
    pub host: String,
    pub port: u16,
    pub api_token: Option<String>,
    pub timeout_duration: Duration,
}

impl TypedRouterClient {
    pub fn new(host: impl Into<String>, port: u16, api_token: Option<String>) -> Self {
        Self {
            host: host.into(),
            port,
            api_token,
            timeout_duration: Duration::from_secs(120),
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

    /// Send a strongly-typed chat completion request
    pub async fn complete(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let serialized =
            serde_json::to_string(req).context("Failed to serialize ChatCompletionRequest")?;

        let connect_fut = TcpStream::connect((self.host.as_str(), self.port));
        let mut stream = timeout(Duration::from_millis(1500), connect_fut)
            .await
            .context(format!(
                "Connection timed out connecting to {}:{}",
                self.host, self.port
            ))?
            .context(format!(
                "Failed to connect to local LLM on {}:{}",
                self.host, self.port
            ))?;

        let mut http_req = format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccept: application/json\r\n",
            self.host, self.port, serialized.len()
        );
        if let Some(ref token) = self.api_token {
            http_req.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        http_req.push_str("Connection: close\r\n\r\n");
        http_req.push_str(&serialized);

        stream
            .write_all(http_req.as_bytes())
            .await
            .context("Failed to write HTTP payload to LLM stream")?;

        let mut buf = Vec::new();
        let _ = timeout(self.timeout_duration, stream.read_to_end(&mut buf))
            .await
            .context(format!(
                "Model response timed out after {:?}",
                self.timeout_duration
            ))?;

        let response_str = String::from_utf8_lossy(&buf);

        if response_str.contains("401 Unauthorized") || response_str.contains("invalid_api_key") {
            bail!("Authentication failed with local LLM endpoint (API token missing or invalid).");
        }

        let body = if let Some(idx) = response_str.find("\r\n\r\n") {
            &response_str[idx + 4..]
        } else {
            &response_str
        };

        if let Some(json_start) = body.find('{') {
            let res: ChatCompletionResponse = serde_json::from_str(&body[json_start..])
                .context("Failed to parse ChatCompletionResponse JSON from model")?;
            Ok(res)
        } else {
            bail!("Invalid response from model endpoint: {response_str}");
        }
    }
}
