// Gemini Provider
// Integration with Google's Gemini API (generateContent / embedContent).
//
// Design notes (see docs/CRATIFY_SPEC.md, `control` profile):
// - The endpoint base URL, model name, and API key all arrive via the
//   constructor (`GeminiProvider::new`) rather than being hardcoded or read
//   from the environment. Callers (see `crate::LLMClient::new`) resolve the
//   effective base URL from `LLMConfig` before constructing the provider.
// - The `reqwest::Client` is injected too, rather than the provider building
//   its own per-call client, so the caller controls timeouts/pooling/TLS
//   configuration in one place.
// - The API key is sent exclusively via the `x-goog-api-key` header. It is
//   never interpolated into the request URL, so it cannot leak into access
//   logs, proxies, or error messages that echo back a URL.
// - Response parsing concatenates *all* text parts of the *first* candidate
//   (Gemini can return a candidate whose content is split across multiple
//   `parts`), instead of only the first part.
// - All fallible response handling goes through `GeminiError`, a typed error
//   enum, instead of `.unwrap()`/`.expect()` on network/model output.

use crate::providers::local::extract_json;
use crate::types::*;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, warn};

/// Default Gemini API base URL. Only used by callers that choose not to
/// supply their own (e.g. `LLMClient::new` falls back to this when
/// `LLMConfig::gemini_base_url` is `None`); `GeminiProvider::new` itself
/// never substitutes this value silently.
pub const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default Gemini model when the caller's config leaves the model name empty.
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-1.5-flash";

/// Typed errors for the Gemini provider. Kept distinct from generic
/// `anyhow::Error` so call sites (and tests) can pattern-match on the
/// specific failure mode without string-sniffing.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GeminiError {
    #[error("Gemini API returned HTTP {status}: {body}")]
    Http { status: u16, body: String },

    #[error("Gemini response contained no candidates")]
    NoCandidates,

    #[error("Gemini request was blocked: {reason}")]
    Blocked { reason: String },

    #[error("Gemini candidate contained no text content")]
    EmptyContent,

    #[error("failed to parse Gemini response: {0}")]
    Parse(String),
}

pub struct GeminiProvider {
    api_key: String,
    base_url: String,
    model: String,
    temperature: f32,
    max_output_tokens: u32,
    client: reqwest::Client,
}

// ---------------------------------------------------------------------
// Wire format (request)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct GeminiGenerationConfig {
    pub temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GeminiGenerationConfig,
}

/// Build a `generateContent` request body. Pure function (no I/O) so it can
/// be unit-tested directly.
pub(crate) fn build_request(
    system_prompt: Option<&str>,
    user_message: &str,
    temperature: f32,
    max_output_tokens: u32,
) -> GeminiRequest {
    GeminiRequest {
        contents: vec![GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart {
                text: user_message.to_string(),
            }],
        }],
        system_instruction: system_prompt.map(|s| GeminiContent {
            role: None,
            parts: vec![GeminiPart {
                text: s.to_string(),
            }],
        }),
        generation_config: GeminiGenerationConfig {
            temperature,
            max_output_tokens,
        },
    }
}

// ---------------------------------------------------------------------
// Wire format (response)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct GeminiResponsePart {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct GeminiResponseContent {
    pub parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GeminiCandidate {
    pub content: Option<GeminiResponseContent>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GeminiPromptFeedback {
    #[serde(rename = "blockReason")]
    pub block_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: Option<GeminiPromptFeedback>,
}

/// Parse a `generateContent` response body into a single text string by
/// concatenating *all* text parts of the *first* candidate. Pure function
/// (no I/O), so it can be exercised directly with fixture JSON in tests.
pub(crate) fn parse_text_response(body: &str) -> Result<String, GeminiError> {
    let parsed: GeminiResponse =
        serde_json::from_str(body).map_err(|e| GeminiError::Parse(e.to_string()))?;

    let candidates = parsed.candidates.unwrap_or_default();
    if candidates.is_empty() {
        if let Some(reason) = parsed.prompt_feedback.and_then(|f| f.block_reason) {
            return Err(GeminiError::Blocked { reason });
        }
        return Err(GeminiError::NoCandidates);
    }

    let parts = candidates
        .into_iter()
        .next()
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .unwrap_or_default();

    let mut text = String::new();
    for part in parts {
        if let Some(t) = part.text {
            text.push_str(&t);
        }
    }

    if text.is_empty() {
        return Err(GeminiError::EmptyContent);
    }

    Ok(text)
}

// ---------------------------------------------------------------------
// Embeddings
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct GeminiEmbedRequest {
    pub content: GeminiContent,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct GeminiEmbeddingValues {
    pub values: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct GeminiEmbedResponse {
    pub embedding: Option<GeminiEmbeddingValues>,
}

pub(crate) fn build_embed_request(text: &str) -> GeminiEmbedRequest {
    GeminiEmbedRequest {
        content: GeminiContent {
            role: None,
            parts: vec![GeminiPart {
                text: text.to_string(),
            }],
        },
    }
}

pub(crate) fn parse_embed_response(body: &str) -> Result<Vec<f32>, GeminiError> {
    let parsed: GeminiEmbedResponse =
        serde_json::from_str(body).map_err(|e| GeminiError::Parse(e.to_string()))?;

    let values = parsed.embedding.and_then(|e| e.values).unwrap_or_default();

    if values.is_empty() {
        return Err(GeminiError::EmptyContent);
    }

    Ok(values)
}

impl GeminiProvider {
    /// Construct a new Gemini provider.
    ///
    /// `api_key`, `base_url`, and `model` all come from the caller's
    /// configuration (see `crate::LLMConfig` / `crate::LLMClient::new`) —
    /// this constructor never falls back to an environment variable or a
    /// baked-in endpoint. `client` is injected so the caller controls
    /// connection pooling and timeouts.
    pub async fn new(
        api_key: String,
        base_url: String,
        model: String,
        client: reqwest::Client,
    ) -> Result<Self> {
        if api_key.trim().is_empty() {
            return Err(anyhow!("Gemini API key must not be empty"));
        }
        if base_url.trim().is_empty() {
            return Err(anyhow!("Gemini base URL must not be empty"));
        }

        debug!("Initialized Gemini provider (model={})", model);

        Ok(Self {
            api_key,
            base_url,
            model: if model.trim().is_empty() {
                DEFAULT_GEMINI_MODEL.to_string()
            } else {
                model
            },
            temperature: 0.7,
            max_output_tokens: 2048,
            client,
        })
    }

    fn generate_content_url(&self) -> String {
        format!(
            "{}/models/{}:generateContent",
            self.base_url.trim_end_matches('/'),
            self.model
        )
    }

    fn embed_content_url(&self) -> String {
        format!(
            "{}/models/{}:embedContent",
            self.base_url.trim_end_matches('/'),
            self.model
        )
    }

    async fn call_api(&self, system_prompt: Option<&str>, user_message: &str) -> Result<String> {
        let request = build_request(
            system_prompt,
            user_message,
            self.temperature,
            self.max_output_tokens,
        );

        let send_fut = self
            .client
            .post(self.generate_content_url())
            // The API key travels only in this header, never in the URL —
            // this is the one thing the earlier draft got right, and it's
            // load-bearing: query strings end up in proxy/server logs.
            .header("x-goog-api-key", &self.api_key)
            .json(&request)
            .send();

        let response = match tokio::time::timeout(Duration::from_secs(60), send_fut).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(anyhow::Error::from(e)),
            Err(_) => return Err(anyhow!("Gemini API request timed out after 60s")),
        };

        let status = response.status();
        let body_text = response.text().await?;

        if !status.is_success() {
            warn!("Gemini API error {}: {}", status, body_text);
            return Err(GeminiError::Http {
                status: status.as_u16(),
                body: body_text,
            }
            .into());
        }

        parse_text_response(&body_text).map_err(anyhow::Error::from)
    }

    fn build_task_analysis_prompt(&self, context: &TaskAnalysisContext) -> String {
        format!(
            r#"You are a specialist AI analyzer helping to break down complex tasks.

A specialist with skills [{skills}] working in {domain} received this task:

FILE: {file}
SIZE: {size} bytes
TYPE: {file_type}

DATA SAMPLE:
{sample}

Analyze this task. Return ONLY a JSON object with these fields:
- analysis_type: string (what kind of analysis is this)
- complexity: string ("Simple", "Moderate", or "Complex")
- recommended_approach: string (how to approach this)
- estimated_time_minutes: number
- confidence_percentage: number (0-100)
- suggested_collaborators: array of strings
- potential_risks: array of strings
- reasoning: string (explain your analysis)

Return ONLY JSON, no other text.
"#,
            skills = context.specialist_skills.join(", "),
            domain = context.specialist_domain,
            file = context.file_name,
            size = context.file_size,
            file_type = context.file_type,
            sample = context.data_sample
        )
    }
}

#[async_trait]
impl super::LLMProvider for GeminiProvider {
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        let prompt = self.build_task_analysis_prompt(context);
        let response = self.call_api(None, &prompt).await?;

        debug!("Gemini task analysis response received");

        let json_str = extract_json(&response)?;
        let analysis: TaskAnalysis = serde_json::from_str(&json_str)?;

        Ok(analysis)
    }

    async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>> {
        let prompt = format!(
            r#"You are a team coordinator for autonomous AI specialists.

Specialist: {name} ({archetype}), Rank {rank}
Skills: {skills}

Recommend 2-3 team members they should collaborate with.

Return ONLY a JSON array with objects containing:
- specialist_name: string
- reason: string
- relevance_score: number (0.0-1.0)
- complementary_skills: array of strings

Return ONLY JSON array, no other text.
"#,
            name = specialist.name,
            archetype = specialist.archetype,
            rank = specialist.rank,
            skills = specialist
                .skills
                .iter()
                .map(|s| format!("{} L{}", s.name, s.level))
                .collect::<Vec<_>>()
                .join(", "),
        );

        let response = self.call_api(None, &prompt).await?;

        debug!("Gemini collaborator finding response received");

        let json_str = extract_json(&response)?;
        let suggestions: Vec<CollaboratorSuggestion> = serde_json::from_str(&json_str)?;

        Ok(suggestions)
    }

    async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan> {
        let prompt = format!(
            r#"Create a step-by-step execution plan:

Specialist: {name} ({archetype})
Task: {task_type}
Approach: {approach}
Estimated: {time} minutes

Return ONLY JSON with:
- task_id: "{task_id}"
- specialist_name: "{name}"
- steps: array of objects with (sequence, description, estimated_time_minutes, required_skills, checkpoints)
- total_estimated_time: number
- success_probability: number (0.0-1.0)
- reasoning: string

Return ONLY JSON, no other text.
"#,
            name = specialist.name,
            archetype = specialist.archetype,
            task_type = task.analysis_type,
            approach = task.recommended_approach,
            time = task.estimated_time_minutes,
            task_id = task.task_id
        );

        let response = self.call_api(None, &prompt).await?;

        debug!("Gemini plan generation response received");

        let json_str = extract_json(&response)?;
        let plan: ExecutionPlan = serde_json::from_str(&json_str)?;

        Ok(plan)
    }

    async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis> {
        let prompt = format!(
            r#"Analyze this failure:

Task: {task}
Error: {error}
Approach: {approach}
Skills: {skills}

Return ONLY JSON with:
- root_cause: string
- explanation: string
- prevention_strategy: string
- recovery_approach: string
- new_strategy: string
- confidence_percentage: number (0-100)

Return ONLY JSON, no other text.
"#,
            task = failure.task_id,
            error = failure.error_message,
            approach = failure.attempted_approach,
            skills = failure.available_skills.join(", ")
        );

        let response = self.call_api(None, &prompt).await?;

        debug!("Gemini failure analysis response received");

        let json_str = extract_json(&response)?;
        let analysis: FailureAnalysis = serde_json::from_str(&json_str)?;

        Ok(analysis)
    }

    async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation> {
        let prompt = format!(
            r#"Explain this skill to {name}:

Skill: {skill}
Domain: {domain}

Return ONLY JSON with:
- skill_name: "{skill}"
- description: string
- use_cases: array of strings
- example: string
- how_to_improve: string
- synergies_with: array of skill names

Return ONLY JSON, no other text.
"#,
            name = specialist.name,
            skill = skill_name,
            domain = specialist.archetype
        );

        let response = self.call_api(None, &prompt).await?;

        debug!("Gemini skill explanation response received");

        let json_str = extract_json(&response)?;
        let explanation: SkillExplanation = serde_json::from_str(&json_str)?;

        Ok(explanation)
    }

    async fn generate_design(&self, context: &DesignContext) -> Result<DesignGeneration> {
        let prompt = format!(
            r##"Generate UI/UX design variants.
Intent: {intent}
Constraints: {constraints}
Variants requested: {count}

Respond ONLY with valid JSON matching:
{{
  "intent": "{intent}",
  "variants": [
    {{
      "title": "String",
      "description": "String",
      "colors": ["#hex1", "#hex2"],
      "typography": "String",
      "layout": "String",
      "confidence": 0.9,
      "reasoning": "String"
    }}
  ],
  "tokens_used": 0,
  "batch_confidence": 0.9
}}
"##,
            intent = context.intent,
            constraints = context.constraints.join(", "),
            count = context.variants_requested
        );

        let response = self.call_api(None, &prompt).await?;
        debug!("Gemini design generation response received");

        let json_str = extract_json(&response)?;
        let generation: DesignGeneration = serde_json::from_str(&json_str)?;

        Ok(generation)
    }

    async fn chat(&self, system_prompt: &str, user_message: &str, domain: &str) -> Result<String> {
        debug!("Gemini chat: domain={}", domain);
        self.call_api(Some(system_prompt), user_message).await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = build_embed_request(text);

        let send_fut = self
            .client
            .post(self.embed_content_url())
            .header("x-goog-api-key", &self.api_key)
            .json(&request)
            .send();

        let response = match tokio::time::timeout(Duration::from_secs(60), send_fut).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(anyhow::Error::from(e)),
            Err(_) => return Err(anyhow!("Gemini embedding request timed out after 60s")),
        };

        let status = response.status();
        let body_text = response.text().await?;

        if !status.is_success() {
            warn!("Gemini embedding API error {}: {}", status, body_text);
            return Err(GeminiError::Http {
                status: status.as_u16(),
                body: body_text,
            }
            .into());
        }

        parse_embed_response(&body_text).map_err(anyhow::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    // -----------------------------------------------------------------
    // Pure request-serialization tests
    // -----------------------------------------------------------------

    #[test]
    fn test_build_request_shape_with_system_prompt() {
        let req = build_request(Some("You are helpful."), "Hello there", 0.5, 1024);

        assert_eq!(req.contents.len(), 1);
        assert_eq!(req.contents[0].role.as_deref(), Some("user"));
        assert_eq!(req.contents[0].parts.len(), 1);
        assert_eq!(req.contents[0].parts[0].text, "Hello there");

        let system = req.system_instruction.expect("system instruction present");
        assert_eq!(system.parts[0].text, "You are helpful.");

        assert_eq!(req.generation_config.temperature, 0.5);
        assert_eq!(req.generation_config.max_output_tokens, 1024);

        let json = serde_json::to_value(&req).unwrap();
        // camelCase field names as the Gemini API expects.
        assert!(json.get("generationConfig").is_some());
        assert!(json.get("systemInstruction").is_some());
        // The request body never contains a credential field of any kind.
        assert!(json.get("api_key").is_none());
        assert!(json.get("apiKey").is_none());
    }

    #[test]
    fn test_build_request_without_system_prompt_omits_field() {
        let req = build_request(None, "Just a user message", 0.7, 2048);
        assert!(req.system_instruction.is_none());

        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("systemInstruction").is_none());
    }

    // -----------------------------------------------------------------
    // Pure response-parsing tests
    // -----------------------------------------------------------------

    #[test]
    fn test_parse_multi_part_response_concatenates_all_parts() {
        let body = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {"text": "Hello, "},
                            {"text": "multi-part "},
                            {"text": "world!"}
                        ]
                    }
                }
            ]
        }"#;

        let text = parse_text_response(body).expect("should parse");
        assert_eq!(text, "Hello, multi-part world!");
    }

    #[test]
    fn test_parse_uses_only_first_candidate() {
        let body = r#"{
            "candidates": [
                {"content": {"parts": [{"text": "first candidate"}]}},
                {"content": {"parts": [{"text": "second candidate"}]}}
            ]
        }"#;

        let text = parse_text_response(body).expect("should parse");
        assert_eq!(text, "first candidate");
    }

    #[test]
    fn test_parse_response_no_candidates_errors() {
        let body = r#"{"candidates": []}"#;
        let err = parse_text_response(body).unwrap_err();
        assert_eq!(err, GeminiError::NoCandidates);
    }

    #[test]
    fn test_parse_response_missing_candidates_field_errors() {
        let body = r#"{}"#;
        let err = parse_text_response(body).unwrap_err();
        assert_eq!(err, GeminiError::NoCandidates);
    }

    #[test]
    fn test_parse_response_blocked_errors() {
        let body = r#"{
            "candidates": [],
            "promptFeedback": {"blockReason": "SAFETY"}
        }"#;

        let err = parse_text_response(body).unwrap_err();
        assert_eq!(
            err,
            GeminiError::Blocked {
                reason: "SAFETY".to_string()
            }
        );
    }

    #[test]
    fn test_parse_response_empty_parts_errors() {
        let body = r#"{"candidates": [{"content": {"parts": []}}]}"#;
        let err = parse_text_response(body).unwrap_err();
        assert_eq!(err, GeminiError::EmptyContent);
    }

    #[test]
    fn test_parse_response_missing_content_errors() {
        let body = r#"{"candidates": [{}]}"#;
        let err = parse_text_response(body).unwrap_err();
        assert_eq!(err, GeminiError::EmptyContent);
    }

    #[test]
    fn test_parse_response_malformed_json_errors() {
        let body = "not json at all";
        let err = parse_text_response(body).unwrap_err();
        assert!(matches!(err, GeminiError::Parse(_)));
    }

    #[test]
    fn test_parse_embed_response_extracts_values() {
        let body = r#"{"embedding": {"values": [0.1, 0.2, 0.3]}}"#;
        let values = parse_embed_response(body).expect("should parse");
        assert_eq!(values, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_parse_embed_response_empty_errors() {
        let body = r#"{"embedding": {"values": []}}"#;
        let err = parse_embed_response(body).unwrap_err();
        assert_eq!(err, GeminiError::EmptyContent);
    }

    #[test]
    fn test_build_embed_request_shape() {
        let req = build_embed_request("some text");
        assert_eq!(req.content.parts.len(), 1);
        assert_eq!(req.content.parts[0].text, "some text");
        assert!(req.content.role.is_none());
    }

    // -----------------------------------------------------------------
    // Local mock-server tests (no external network, no real keys).
    //
    // These spin up a plain `std::net::TcpListener` on 127.0.0.1:0 and hand
    // back a canned HTTP response, so `GeminiProvider::call_api` can be
    // exercised end-to-end (header injection, URL construction, HTTP-error
    // mapping) without a network dependency or a mocking crate.
    // -----------------------------------------------------------------

    /// Spawn a single-request mock HTTP server on localhost. Returns the
    /// base URL to hit and a handle that will contain the raw request text
    /// once the single expected request has been served.
    fn spawn_single_request_mock(
        raw_http_response: String,
    ) -> (String, Arc<Mutex<Option<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock listener");
        let addr = listener.local_addr().expect("local addr");
        let captured = Arc::new(Mutex::new(None));
        let captured_clone = captured.clone();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 16 * 1024];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                *captured_clone.lock().expect("lock captured request") = Some(request);
                let _ = stream.write_all(raw_http_response.as_bytes());
                let _ = stream.flush();
            }
        });

        (format!("http://{addr}"), captured)
    }

    fn http_ok_response(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }

    fn http_error_response(status_line: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status_line,
            body.len(),
            body
        )
    }

    #[tokio::test]
    async fn test_chat_sends_key_in_header_never_in_url_and_parses_multi_part_body() {
        let body =
            r#"{"candidates":[{"content":{"parts":[{"text":"Hello, "},{"text":"world!"}]}}]}"#;
        let (base_url, captured) = spawn_single_request_mock(http_ok_response(body));

        let provider = GeminiProvider::new(
            "test-secret-key".to_string(),
            base_url,
            "gemini-1.5-flash".to_string(),
            reqwest::Client::new(),
        )
        .await
        .expect("provider construction should succeed");

        let text = provider
            .chat("system prompt", "user message", "general")
            .await
            .expect("chat should succeed against mock server");

        assert_eq!(text, "Hello, world!");

        let request = captured
            .lock()
            .expect("lock captured request")
            .clone()
            .expect("a request should have been captured");

        let request_line = request.lines().next().unwrap_or_default();
        assert!(
            !request_line.contains("test-secret-key"),
            "API key must never appear in the request line/URL: {request_line}"
        );
        assert!(
            request
                .lines()
                .any(|l| l.eq_ignore_ascii_case("x-goog-api-key: test-secret-key")),
            "API key must be sent via the x-goog-api-key header: {request}"
        );
        assert!(request_line.contains(":generateContent"));
    }

    #[tokio::test]
    async fn test_chat_maps_http_error_status_to_typed_error() {
        let body = r#"{"error":{"message":"bad request","status":"INVALID_ARGUMENT"}}"#;
        let (base_url, _captured) =
            spawn_single_request_mock(http_error_response("400 Bad Request", body));

        let provider = GeminiProvider::new(
            "test-secret-key".to_string(),
            base_url,
            "gemini-1.5-flash".to_string(),
            reqwest::Client::new(),
        )
        .await
        .expect("provider construction should succeed");

        let err = provider
            .chat("system", "user", "general")
            .await
            .expect_err("HTTP 400 should surface as an error");

        assert!(err.to_string().contains("400"));
        assert!(err.to_string().contains("INVALID_ARGUMENT"));
    }

    #[tokio::test]
    async fn test_new_rejects_empty_api_key() {
        let err = GeminiProvider::new(
            String::new(),
            DEFAULT_GEMINI_BASE_URL.to_string(),
            DEFAULT_GEMINI_MODEL.to_string(),
            reqwest::Client::new(),
        )
        .await
        .expect_err("empty api key must be rejected");
        assert!(err.to_string().contains("API key"));
    }

    #[tokio::test]
    async fn test_new_rejects_empty_base_url() {
        let err = GeminiProvider::new(
            "some-key".to_string(),
            String::new(),
            DEFAULT_GEMINI_MODEL.to_string(),
            reqwest::Client::new(),
        )
        .await
        .expect_err("empty base url must be rejected");
        assert!(err.to_string().contains("base URL"));
    }

    #[tokio::test]
    async fn test_new_defaults_empty_model_to_default_model() {
        let provider = GeminiProvider::new(
            "some-key".to_string(),
            DEFAULT_GEMINI_BASE_URL.to_string(),
            String::new(),
            reqwest::Client::new(),
        )
        .await
        .expect("construction should succeed with default model fallback");
        assert_eq!(provider.model, DEFAULT_GEMINI_MODEL);
    }
}
