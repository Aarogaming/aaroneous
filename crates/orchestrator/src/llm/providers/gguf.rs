//! Local GGUF model provider implementation.
//!
//! With `llama-gguf` feature: runs real quantized inference via the llama-gguf engine.
//! Without the feature: returns structured mock responses for development/testing.

use anyhow::Result;
#[cfg(not(feature = "llama-gguf"))]
use tracing::warn;

use super::super::types::LLMConfig;

/// Local GGUF model provider for offline inference
pub struct GgufProvider {
    config: LLMConfig,
}

impl GgufProvider {
    pub fn new(config: LLMConfig) -> Self {
        #[cfg(not(feature = "llama-gguf"))]
        warn!(
            "GGUF provider initialized without 'llama-gguf' feature. Returning mock responses. \
             Compile with --features llama-gguf for real local inference."
        );
        Self { config }
    }

    pub async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let model_path = self
            .config
            .gguf_model_path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No GGUF model path configured"))?;

        if !std::path::Path::new(model_path).exists() {
            return Err(anyhow::anyhow!(
                "GGUF model file not found: {}",
                model_path
            ));
        }

        #[cfg(feature = "llama-gguf")]
        {
            return self.run_inference(model_path, system_prompt, user_prompt).await;
        }

        #[cfg(not(feature = "llama-gguf"))]
        {
            Ok(format!(
                "[GGUF {}] Model: {}, System: {}, User: {}",
                self.config.model_name,
                truncate(model_path, 50),
                truncate(system_prompt, 50),
                truncate(user_prompt, 100)
            ))
        }
    }

    #[cfg(feature = "llama-gguf")]
    async fn run_inference(
        &self,
        model_path: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let model_path = model_path.to_string();
        let prompt = format!("<|im_start|>system\n{}\n<|im_start|>user\n{}\n<|im_start|>assistant\n", system_prompt, user_prompt);
        let max_tokens = self.config.max_tokens;
        let temperature = self.config.temperature;

        tokio::task::spawn_blocking(move || {
            let engine = llama_gguf::engine::Engine::load(llama_gguf::engine::EngineConfig {
                model_path: model_path,
                max_tokens: max_tokens as usize,
                temperature,
                ..Default::default()
            })
            .map_err(|e| anyhow::anyhow!("Failed to load GGUF model: {}", e))?;

            let result = engine
                .generate(&prompt, max_tokens as usize)
                .map_err(|e| anyhow::anyhow!("GGUF inference failed: {}", e))?;

            Ok(result)
        })
        .await?
    }

    pub fn config(&self) -> &LLMConfig {
        &self.config
    }
}

#[allow(dead_code)]
fn truncate(s: &str, max_len: usize) -> &str {
    match s.char_indices().nth(max_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gguf_provider_creation() {
        let config = LLMConfig::default();
        let provider = GgufProvider::new(config);
        assert!(provider.config().gguf_model_path.is_none());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("🦀🔥⚡🌟🚀", 2), "🦀🔥");
    }
}
