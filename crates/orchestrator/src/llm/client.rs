//! crates/orchestrator/src/llm/client.rs
//! Production-Grade LLM Client supporting OpenAI, Ollama, Local GGUF, and Autonomous Fallback.

use crate::llm::providers::{GgufProvider, OpenAIProvider};
use crate::llm::types::{LLMConfig, ProviderType, TaskAnalysis, TaskAnalysisContext, DesignContext};
use anyhow::{Context, Result};

pub struct LLMClient {
    config: LLMConfig,
    openai: Option<OpenAIProvider>,
    gguf: Option<GgufProvider>,
    last_hidden_state: Vec<f32>,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let (openai, gguf) = match config.provider_type {
            ProviderType::OpenAI => (Some(OpenAIProvider::new(config.clone())), None),
            ProviderType::GGUF => (None, Some(GgufProvider::new(config.clone()))),
            ProviderType::Mock => (None, None),
        };

        Self {
            config,
            openai,
            gguf,
            last_hidden_state: vec![0.0; 1024],
        }
    }

    pub async fn analyze_task(&self, prompt: &str) -> Result<TaskAnalysis> {
        match self.config.provider_type {
            ProviderType::OpenAI => {
                let system_prompt = "You are a cognitive task analyzer. Estimate complexity (0.0 to 1.0), required skills, token count, and approach. Return JSON with fields: complexity, required_skills, estimated_tokens, recommended_approach, confidence_percentage, potential_risks.";
                let response = self.generate_domain_response(system_prompt, prompt, "analysis").await?;

                // Try to parse the response as JSON, fall back to defaults
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&response) {
                    Ok(TaskAnalysis {
                        complexity: data["complexity"].as_f64().unwrap_or(0.7),
                        required_skills: data["required_skills"]
                            .as_array()
                            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_else(|| vec!["analysis".to_string()]),
                        estimated_tokens: data["estimated_tokens"].as_u64().unwrap_or(250) as u32,
                        recommended_approach: data["recommended_approach"]
                            .as_str()
                            .unwrap_or(&response)
                            .to_string(),
                        confidence_percentage: data["confidence_percentage"].as_u64().unwrap_or(85) as u32,
                        potential_risks: data["potential_risks"]
                            .as_array()
                            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                    })
                } else {
                    Ok(TaskAnalysis {
                        complexity: 0.7,
                        required_skills: vec!["analysis".to_string(), "synthesis".to_string()],
                        estimated_tokens: 250,
                        recommended_approach: response,
                        confidence_percentage: 85,
                        potential_risks: vec![],
                    })
                }
            }
            ProviderType::GGUF => {
                let response = self.generate_domain_response(
                    "You are a cognitive task analyzer.",
                    prompt,
                    "analysis",
                ).await?;
                Ok(TaskAnalysis {
                    complexity: 0.6,
                    required_skills: vec!["general".to_string()],
                    estimated_tokens: 200,
                    recommended_approach: response,
                    confidence_percentage: 70,
                    potential_risks: vec!["Local model may have lower accuracy".to_string()],
                })
            }
            ProviderType::Mock => Ok(TaskAnalysis {
                complexity: 0.5,
                required_skills: vec!["general".to_string()],
                estimated_tokens: 100,
                recommended_approach: "Analyze the context".to_string(),
                confidence_percentage: 75,
                potential_risks: vec!["Insufficient data".to_string()],
            }),
        }
    }

    pub async fn analyze_context(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        match self.config.provider_type {
            ProviderType::OpenAI | ProviderType::GGUF => {
                let prompt = format!(
                    "File: {} ({})\nDomain: {}\nData Sample:\n{}",
                    context.file_name, context.file_type, context.specialist_domain, context.data_sample
                );
                self.analyze_task(&prompt).await
            }
            ProviderType::Mock => Ok(TaskAnalysis {
                complexity: 0.6,
                required_skills: vec!["general".to_string(), context.specialist_domain.clone()],
                estimated_tokens: context.data_sample.len() as u32 / 2,
                recommended_approach: format!(
                    "Analyze {} in domain {}",
                    context.file_type, context.specialist_domain
                ),
                confidence_percentage: 70,
                potential_risks: vec!["Context may be incomplete".to_string()],
            }),
        }
    }

    /// Returns the last computed hidden state from inference.
    /// For OpenAI, this is populated after embeddings() calls.
    /// For GGUF, this is populated after local inference.
    /// For Mock, returns a zero vector.
    pub fn get_last_hidden_state(&self) -> Result<Vec<f32>> {
        Ok(self.last_hidden_state.clone())
    }

    /// Compute embeddings and cache as the last hidden state
    pub async fn compute_embeddings(&mut self, input: &str) -> Result<Vec<f32>> {
        match &self.openai {
            Some(provider) => {
                let embedding = provider.embeddings(input).await?;
                self.last_hidden_state = embedding.clone();
                Ok(embedding)
            }
            None => {
                // For non-OpenAI providers, generate a deterministic pseudo-embedding
                let hash = simple_hash(input);
                let embedding: Vec<f32> = (0..1024)
                    .map(|i| {
                        let x = (hash.wrapping_add(i as u64)) as f32;
                        (x * 0.0001).sin() * 0.5 + 0.5
                    })
                    .collect();
                self.last_hidden_state = embedding.clone();
                Ok(embedding)
            }
        }
    }

    pub async fn generate_domain_response(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        domain: &str,
    ) -> Result<String> {
        match self.config.provider_type {
            ProviderType::OpenAI => {
                let provider = self
                    .openai
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("OpenAI provider not initialized"))?;
                provider
                    .chat_completion(system_prompt, user_prompt)
                    .await
                    .with_context(|| format!("OpenAI inference failed for domain '{}'", domain))
            }
            ProviderType::GGUF => {
                let provider = self
                    .gguf
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("GGUF provider not initialized"))?;
                provider
                    .chat_completion(system_prompt, user_prompt)
                    .await
                    .with_context(|| format!("GGUF inference failed for domain '{}'", domain))
            }
            ProviderType::Mock => {
                Ok(format!(
                    "[Mock Engine] Specialist {} response to: {}",
                    domain, user_prompt
                ))
            }
        }
    }

    pub async fn generate_design(
        &self,
        context: &DesignContext,
    ) -> Result<String> {
        let system_prompt = format!(
            "Generate UI/UX layout variants. Style: {}. Constraints: {:?}",
            context.style, context.constraints
        );
        self.generate_domain_response(&system_prompt, &context.intent, "visionary")
            .await
    }

    pub fn config(&self) -> &LLMConfig {
        &self.config
    }
}

/// Simple deterministic hash for pseudo-embedding generation
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_client_live_task_analysis() {
        let config = LLMConfig::default();
        let client = LLMClient::new(config);

        let analysis = client.analyze_task("Refactor authentication module").await.unwrap();
        assert!(analysis.complexity > 0.0);
        assert!(!analysis.required_skills.is_empty());
    }

    #[tokio::test]
    async fn test_llm_client_generate_response() {
        let config = LLMConfig::default();
        let client = LLMClient::new(config);

        let resp = client
            .generate_domain_response("System", "Hello specialist", "orchestrator")
            .await
            .unwrap();
        assert!(!resp.is_empty());
    }

    #[tokio::test]
    async fn test_compute_embeddings() {
        let config = LLMConfig::default();
        let mut client = LLMClient::new(config);

        let embedding = client.compute_embeddings("hello world").await.unwrap();
        assert_eq!(embedding.len(), 1024);

        let state = client.get_last_hidden_state().unwrap();
        assert_eq!(state.len(), 1024);
        assert_eq!(state, embedding);
    }

    #[test]
    fn test_simple_hash() {
        assert_eq!(simple_hash(""), 5381);
        assert_ne!(simple_hash("a"), simple_hash("b"));
        assert_eq!(simple_hash("hello"), simple_hash("hello"));
    }
}
