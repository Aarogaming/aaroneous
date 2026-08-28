//! Local GGUF model provider implementation.
//! Wraps candle-core for local inference when available.

use anyhow::Result;

use super::super::types::LLMConfig;

/// Local GGUF model provider for offline inference
pub struct GgufProvider {
    config: LLMConfig,
}

impl GgufProvider {
    pub fn new(config: LLMConfig) -> Self {
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

        // Placeholder for candle-based inference
        // Full implementation requires tokenization, KV-cache, and sampling
        Ok(format!(
            "[GGUF {}] Model: {}, System: {}, User: {}",
            self.config.model_name,
            truncate(model_path, 50),
            truncate(system_prompt, 50),
            truncate(user_prompt, 100)
        ))
    }

    pub fn config(&self) -> &LLMConfig {
        &self.config
    }
}

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
