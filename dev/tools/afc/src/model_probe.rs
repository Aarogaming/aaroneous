// dev/tools/afc/src/model_probe.rs
use serde::Deserialize;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub enum ModelEndpointStatus {
    Connected {
        provider: String,
        configured_model: String,
        discovered_models: Vec<String>,
    },
    Disconnected {
        provider: String,
        target_endpoint: String,
        reason: String,
    },
    Unconfigured,
}

#[derive(Debug, Deserialize)]
struct OpenCodeConfig {
    #[serde(default)]
    model: Option<String>,
}

pub struct ModelProbe;

impl ModelProbe {
    /// Inspect opencode.json and probe the active AI provider endpoint.
    pub async fn check_endpoint(repo_path: &Path) -> ModelEndpointStatus {
        let config_path = repo_path.join("opencode.json");
        let configured_model = if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path).await {
                if let Ok(cfg) = serde_json::from_str::<OpenCodeConfig>(&content) {
                    cfg.model.unwrap_or_else(|| "default".to_string())
                } else {
                    "local-lmstudio/qwen".to_string()
                }
            } else {
                "local-lmstudio/qwen".to_string()
            }
        } else {
            return ModelEndpointStatus::Unconfigured;
        };

        // Determine provider from model string prefix
        if configured_model.starts_with("local-lmstudio") || configured_model.contains("lmstudio") {
            Self::probe_lm_studio(&configured_model).await
        } else if configured_model.starts_with("ollama") || configured_model.contains("ollama") {
            Self::probe_ollama(&configured_model).await
        } else if configured_model.contains("gemini") {
            let has_key = std::env::var("GEMINI_API_KEY").is_ok();
            if has_key {
                ModelEndpointStatus::Connected {
                    provider: "Google Gemini".to_string(),
                    configured_model,
                    discovered_models: vec![
                        "gemini-1.5-pro".to_string(),
                        "gemini-2.0-flash".to_string(),
                    ],
                }
            } else {
                ModelEndpointStatus::Disconnected {
                    provider: "Google Gemini".to_string(),
                    target_endpoint: "https://generativelanguage.googleapis.com".to_string(),
                    reason: "GEMINI_API_KEY environment variable not detected".to_string(),
                }
            }
        } else if configured_model.contains("copilot") || configured_model.contains("github") {
            ModelEndpointStatus::Connected {
                provider: "GitHub Copilot".to_string(),
                configured_model,
                discovered_models: vec!["copilot-chat".to_string()],
            }
        } else if configured_model.contains("huggingface") {
            ModelEndpointStatus::Connected {
                provider: "HuggingFace".to_string(),
                configured_model,
                discovered_models: vec!["hf-inference".to_string()],
            }
        } else {
            // Default fallback: probe standard local LM Studio port 1234
            Self::probe_lm_studio(&configured_model).await
        }
    }

    /// Probe local LM Studio OpenAI-compatible endpoint on 127.0.0.1:1234
    async fn probe_lm_studio(model_name: &str) -> ModelEndpointStatus {
        let host = "127.0.0.1";
        let port = 1234;

        let connect_fut = TcpStream::connect((host, port));
        let Ok(Ok(mut stream)) = timeout(Duration::from_millis(600), connect_fut).await else {
            return ModelEndpointStatus::Disconnected {
                provider: "LM Studio".to_string(),
                target_endpoint: format!("http://{host}:{port}/v1"),
                reason: "Port 1234 closed (LM Studio local server not running)".to_string(),
            };
        };

        let request = format!("GET /v1/models HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).await.is_err() {
            return ModelEndpointStatus::Disconnected {
                provider: "LM Studio".to_string(),
                target_endpoint: format!("http://{host}:{port}/v1"),
                reason: "Failed writing HTTP request to LM Studio".to_string(),
            };
        }

        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let response = String::from_utf8_lossy(&buf[..n]);

        let mut discovered = Vec::new();
        if let Some(json_start) = response.find('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response[json_start..]) {
                if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                    for item in data {
                        if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                            discovered.push(id.to_string());
                        }
                    }
                }
            }
        }

        ModelEndpointStatus::Connected {
            provider: "LM Studio".to_string(),
            configured_model: model_name.to_string(),
            discovered_models: discovered,
        }
    }

    /// Probe local Ollama endpoint on 127.0.0.1:11434
    async fn probe_ollama(model_name: &str) -> ModelEndpointStatus {
        let host = "127.0.0.1";
        let port = 11434;

        let connect_fut = TcpStream::connect((host, port));
        let Ok(Ok(mut stream)) = timeout(Duration::from_millis(600), connect_fut).await else {
            return ModelEndpointStatus::Disconnected {
                provider: "Ollama".to_string(),
                target_endpoint: format!("http://{host}:{port}"),
                reason: "Port 11434 closed (Ollama daemon not running)".to_string(),
            };
        };

        let request = format!("GET /api/tags HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).await.is_err() {
            return ModelEndpointStatus::Disconnected {
                provider: "Ollama".to_string(),
                target_endpoint: format!("http://{host}:{port}"),
                reason: "Failed writing HTTP request to Ollama".to_string(),
            };
        }

        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let response = String::from_utf8_lossy(&buf[..n]);

        let mut discovered = Vec::new();
        if let Some(json_start) = response.find('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response[json_start..]) {
                if let Some(models) = val.get("models").and_then(|d| d.as_array()) {
                    for item in models {
                        if let Some(name) = item.get("name").and_then(|i| i.as_str()) {
                            discovered.push(name.to_string());
                        }
                    }
                }
            }
        }

        ModelEndpointStatus::Connected {
            provider: "Ollama".to_string(),
            configured_model: model_name.to_string(),
            discovered_models: discovered,
        }
    }
}
