// GGUF Provider
// In-process local GGUF model inference via `local_inference`, a thin
// wrapper around the `llama-gguf` crate — a separate, pure-Rust inference
// implementation, not a binding to (or fork of) llama.cpp.
// Uses Qwen models (or other open source GGUF)

use crate::types::*;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, info, warn};
pub struct GGUFProvider {
    model_path: PathBuf,
    _context_size: u32,
    _threads: u32,
    /// Cached engine instance — loaded once on first call, reused for all subsequent calls.
    ///
    /// Without this cache, every `generate_text()` call would:
    /// - Open the GGUF file (kernel call)
    /// - Map it into virtual address space (mmap)
    /// - Parse the tensor info table
    /// - Allocate KV cache
    ///   Total: 500ms–3s per call for a 4GB model.
    ///
    /// With the cache: first call pays the load cost, subsequent calls are
    /// instant — the Engine is already in memory and the KV cache is hot.
    ///
    /// The Mutex is needed because `Engine::generate()` likely takes &mut self
    /// (inference modifies the KV cache state).
    engine_cache: std::sync::Arc<tokio::sync::Mutex<Option<local_inference::LocalEngine>>>,
}

impl GGUFProvider {
    /// Create GGUF provider with local model
    pub fn new(model_path: PathBuf, context_size: u32, threads: u32) -> Result<Self> {
        // Fail fast, at construction, if this binary was built without an
        // explicit local-inference backend feature — don't wait until the
        // first `generate_text()` call deep inside a spawn_blocking task to
        // discover it. See `local_inference::ensure_backend_selected` for
        // why this must never silently succeed.
        local_inference::ensure_backend_selected()?;

        if !model_path.exists() {
            return Err(anyhow!("Model file not found at: {}", model_path.display()));
        }

        info!(
            "Initialized GGUF provider with model: {} ({}KB)",
            model_path.display(),
            std::fs::metadata(&model_path)?.len() / 1024
        );

        Ok(Self {
            model_path,
            _context_size: context_size,
            _threads: threads,
            engine_cache: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    /// Generate text from a prompt using the loaded GGUF model.
    ///
    /// # Backend selection
    ///
    /// This always calls the real `local_inference::LocalEngine` — there is
    /// no mock or degraded fallback path. `local_inference` itself never
    /// guesses a backend: `llm_gateway` must enable exactly one of its
    /// `gguf-cpu`, `gguf-cuda`, `gguf-vulkan`, or `gguf-metal` Cargo
    /// features (each forwarding to the matching `local_inference`
    /// feature). [`GGUFProvider::new`] checks this eagerly at construction
    /// via `local_inference::ensure_backend_selected`, and `LocalEngine::load`
    /// (invoked here, on first call, via the cached-engine path below) checks
    /// it again — both return a clearly-named error rather than silently
    /// falling back to CPU or a mock if no backend feature is enabled.
    ///
    /// Inference itself runs on `tokio::task::spawn_blocking` since it's
    /// CPU-bound and synchronous.
    async fn generate_text(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        use local_inference::{InferenceConfig, LocalEngine};

        let prompt_owned = prompt.to_string();
        let engine_cache = self.engine_cache.clone();
        let model_path_str = self.model_path.to_string_lossy().to_string();

        let result = tokio::task::spawn_blocking(move || -> Result<String> {
            let mut guard = engine_cache.blocking_lock();
            if guard.is_none() {
                info!("GGUF: loading engine from {} (first call)", model_path_str);
                let config = InferenceConfig {
                    model_path: model_path_str.into(),
                    max_tokens,
                    temperature: 0.7,
                    top_p: 0.95,
                };
                *guard = Some(
                    LocalEngine::load(&config)
                        .map_err(|e| anyhow::anyhow!("Engine::load failed: {:?}", e))?,
                );
            }
            let engine = guard.as_mut().unwrap();
            engine
                .generate(&prompt_owned, max_tokens)
                .map_err(|e| anyhow::anyhow!("generation failed: {:?}", e))
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking panicked: {}", e))??;

        Ok(result)
    }

    fn build_task_analysis_prompt(&self, context: &TaskAnalysisContext) -> String {
        format!(
            r#"Analyze this task and provide a JSON response.

File: {file}
Size: {size} bytes
Type: {file_type}
Skills: {skills}

Data sample:
{sample}

Return JSON only:
{{
  "analysis_type": "string",
  "complexity": "Simple|Moderate|Complex",
  "recommended_approach": "string",
  "estimated_time_minutes": number,
  "confidence_percentage": number,
  "suggested_collaborators": ["string"],
  "potential_risks": ["string"],
  "reasoning": "string"
}}
"#,
            file = context.file_name,
            size = context.file_size,
            file_type = context.file_type,
            skills = context.specialist_skills.join(", "),
            sample = context.data_sample
        )
    }

    fn build_collaborators_prompt(&self, specialist: &SpecialistContext) -> String {
        format!(
            r#"Recommend collaborators for {name}:

Archetype: {archetype}
Skills: {skills}
Goal: {goal}

Return JSON array only:
[
  {{
    "specialist_name": "string",
    "reason": "string",
    "relevance_score": 0.0,
    "complementary_skills": ["string"]
  }}
]
"#,
            name = specialist.name,
            archetype = specialist.archetype,
            skills = specialist
                .skills
                .iter()
                .map(|s| format!("{} L{}", s.name, s.level))
                .collect::<Vec<_>>()
                .join(", "),
            goal = specialist
                .current_goal
                .as_ref()
                .unwrap_or(&"none".to_string())
        )
    }
}

#[async_trait]
impl super::LLMProvider for GGUFProvider {
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        let prompt = self.build_task_analysis_prompt(context);
        let response = self.generate_text(&prompt, 500).await?;

        debug!("GGUF task analysis complete");

        // Parse JSON from response
        let json_str = extract_json_from_response(&response)?;
        let analysis: TaskAnalysis = serde_json::from_str(&json_str)?;

        Ok(analysis)
    }

    async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>> {
        let prompt = self.build_collaborators_prompt(specialist);
        let response = self.generate_text(&prompt, 300).await?;

        debug!("GGUF collaborator finding complete");

        let json_str = extract_json_from_response(&response)?;
        let suggestions: Vec<CollaboratorSuggestion> = serde_json::from_str(&json_str)?;

        Ok(suggestions)
    }

    async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan> {
        let prompt = format!(
            r#"Create execution plan for {name} on task {task_type}:

Approach: {approach}
Time estimate: {time} min

Return JSON only:
{{
  "task_id": "{task_id}",
  "specialist_name": "{name}",
  "steps": [
    {{
      "sequence": 1,
      "description": "string",
      "estimated_time_minutes": number,
      "required_skills": ["string"],
      "checkpoints": ["string"]
    }}
  ],
  "total_estimated_time": number,
  "success_probability": 0.8,
  "reasoning": "string"
}}
"#,
            name = specialist.name,
            task_type = task.analysis_type,
            approach = task.recommended_approach,
            time = task.estimated_time_minutes,
            task_id = task.task_id
        );

        let response = self.generate_text(&prompt, 400).await?;

        debug!("GGUF plan generation complete");

        let json_str = extract_json_from_response(&response)?;
        let plan: ExecutionPlan = serde_json::from_str(&json_str)?;

        Ok(plan)
    }

    async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis> {
        let prompt = format!(
            r#"Analyze failure and suggest recovery:

Task: {task}
Error: {error}
Approach: {approach}

Return JSON only:
{{
  "root_cause": "string",
  "explanation": "string",
  "prevention_strategy": "string",
  "recovery_approach": "string",
  "new_strategy": "string",
  "confidence_percentage": 80
}}
"#,
            task = failure.task_id,
            error = failure.error_message,
            approach = failure.attempted_approach
        );

        let response = self.generate_text(&prompt, 300).await?;

        debug!("GGUF failure analysis complete");

        let json_str = extract_json_from_response(&response)?;
        let analysis: FailureAnalysis = serde_json::from_str(&json_str)?;

        Ok(analysis)
    }

    async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation> {
        let prompt = format!(
            r#"Explain skill {skill} for {specialist}:

Specialist domain: {domain}

Return JSON only:
{{
  "skill_name": "{skill}",
  "description": "string",
  "use_cases": ["string"],
  "example": "string",
  "how_to_improve": "string",
  "synergies_with": ["string"]
}}
"#,
            skill = skill_name,
            specialist = specialist.name,
            domain = specialist.archetype
        );

        let response = self.generate_text(&prompt, 250).await?;

        debug!("GGUF skill explanation complete");

        let json_str = extract_json_from_response(&response)?;
        let explanation: SkillExplanation = serde_json::from_str(&json_str)?;

        Ok(explanation)
    }

    async fn chat(&self, system_prompt: &str, user_message: &str, _domain: &str) -> Result<String> {
        // Build a Qwen2/ChatML-style prompt:
        //   <|im_start|>system\n{system}\n<|im_end|>\n
        //   <|im_start|>user\n{user}\n<|im_end|>\n
        //   <|im_start|>assistant\n
        let prompt = format!(
            "<|im_start|>system\n{system}\n<|im_end|>\n\
             <|im_start|>user\n{user}\n<|im_end|>\n\
             <|im_start|>assistant\n",
            system = system_prompt,
            user = user_message,
        );
        self.generate_text(&prompt, 512).await
    }

    async fn generate_design(&self, context: &DesignContext) -> Result<DesignGeneration> {
        let style = context.style_hints.join(", ");
        let constraints = context.constraints.join(", ");
        let count = context.variants_requested.clamp(1, 3);

        let prompt = format!(
            r#"You are Visionary, a UI/UX design specialist. Generate {count} distinct design variants.

Intent: {intent}
Style hints: {style}
Constraints: {constraints}
Avoid: {rejected}

Return a JSON array of {count} variant(s), each with:
{{
  "title": "string",
  "description": "string",
  "colors": ["hex_or_token"],
  "typography": "font-stack",
  "layout": "string",
  "confidence": 0.0-1.0,
  "reasoning": "string"
}}

JSON array only:"#,
            count = count,
            intent = context.intent,
            style = if style.is_empty() {
                "modern, clean".to_string()
            } else {
                style
            },
            constraints = if constraints.is_empty() {
                "none".to_string()
            } else {
                constraints
            },
            rejected = if context.rejected_examples.is_empty() {
                "nothing known".to_string()
            } else {
                context.rejected_examples[..3.min(context.rejected_examples.len())].join("; ")
            }
        );

        let response = self.generate_text(&prompt, 600).await?;
        debug!("GGUF design generation complete");

        // Parse the JSON array of variants
        let json_str = extract_json_from_response(&response)?;
        let variants: Vec<DesignVariant> = serde_json::from_str(&json_str).unwrap_or_else(|_| {
            // If parsing fails, return a single fallback variant
            warn!("Failed to parse GGUF design response; using fallback variant");
            vec![DesignVariant {
                title: format!("Design for {}", context.intent),
                description: format!(
                    "Generated design: {}",
                    response.chars().take(200).collect::<String>()
                ),
                colors: vec!["#6366F1".to_string(), "#F8FAFC".to_string()],
                typography: "Inter, sans-serif".to_string(),
                layout: "single-column".to_string(),
                confidence: 0.5,
                reasoning: "Fallback from unparseable GGUF response".to_string(),
            }]
        });

        let batch_confidence = if variants.is_empty() {
            0.0
        } else {
            variants.iter().map(|v| v.confidence).sum::<f32>() / variants.len() as f32
        };

        Ok(DesignGeneration {
            intent: context.intent.clone(),
            variants,
            tokens_used: 0,
            batch_confidence,
        })
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "Embedding not natively supported by local_inference yet"
        ))
    }
}

/// Extract JSON from text that may contain extra content
fn extract_json_from_response(text: &str) -> Result<String> {
    // Try to find JSON object
    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
    {
        return Ok(text[start..=end].to_string());
    }

    // Try to find JSON array
    if let Some(start) = text.find('[')
        && let Some(end) = text.rfind(']')
    {
        return Ok(text[start..=end].to_string());
    }

    // No JSON found
    Err(anyhow!("No JSON found in response: {}", text))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With no `gguf-cpu`/`gguf-cuda`/`gguf-vulkan`/`gguf-metal` feature
    /// enabled on `llm_gateway` (its `default = []`, so an ordinary
    /// `cargo test -p llm_gateway` exercises exactly this), constructing a
    /// `GGUFProvider` must fail immediately with a clearly-named
    /// backend-selection error — never silently succeed, and never wait
    /// until the first `generate_text()` call to discover the problem.
    #[cfg(not(any(
        feature = "gguf-cpu",
        feature = "gguf-cuda",
        feature = "gguf-vulkan",
        feature = "gguf-metal"
    )))]
    #[test]
    fn new_without_backend_feature_fails_with_named_error() {
        // No filesystem access ever happens: `ensure_backend_selected` is
        // checked before the model-path existence check, so this
        // deliberately nonexistent path is never touched.
        match GGUFProvider::new(PathBuf::from("nonexistent-model.gguf"), 4096, 4) {
            Ok(_) => panic!("GGUFProvider::new must fail when no backend feature is enabled"),
            Err(err) => assert!(
                err.to_string()
                    .contains("no inference backend feature is enabled"),
                "expected the named backend-selection error, got: {err}"
            ),
        }
    }

    #[test]
    fn test_json_extraction() {
        let text = "Here's the JSON: {\"key\": \"value\"} and some more text";
        let json = extract_json_from_response(text).unwrap();
        assert!(json.contains("key"));
    }
}
