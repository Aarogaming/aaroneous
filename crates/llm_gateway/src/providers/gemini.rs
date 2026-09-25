use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct GeminiProvider {
    api_key: String,
    model: String,
    temperature: f32,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String, temperature: f32) -> Self {
        Self {
            api_key,
            model,
            temperature,
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Part {
            text: String,
        }
        #[derive(Serialize)]
        struct Content {
            parts: Vec<Part>,
        }
        #[derive(Serialize)]
        struct GenerationConfig {
            temperature: f32,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request {
            contents: Vec<Content>,
            generation_config: GenerationConfig,
        }

        let req_body = Request {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: self.temperature,
            },
        };

        // The API key travels in a header, never in the URL, so it cannot leak
        // through proxy/server access logs or error messages that echo the URL.
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;

        let res = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .json(&req_body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API error: {} - {}", status, text));
        }

        #[derive(Deserialize)]
        struct ResponsePart {
            text: String,
        }
        #[derive(Deserialize)]
        struct ResponseContent {
            parts: Vec<ResponsePart>,
        }
        #[derive(Deserialize)]
        struct Candidate {
            content: ResponseContent,
        }
        #[derive(Deserialize)]
        struct Response {
            candidates: Vec<Candidate>,
        }

        let parsed: Response = res.json().await?;
        let text = parsed
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| anyhow!("No content returned from Gemini"))?;

        Ok(text)
    }
}
