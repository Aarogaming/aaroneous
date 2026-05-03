// GGUF Provider
// Direct integration with llama.cpp for local GGUF model inference
// Uses Qwen models (or other open source GGUF)

use crate::llm::types::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct GGUFProvider {
    model_path: PathBuf,
    context_size: u32,
    threads: u32,
    /// Cached engine instance — loaded once on first call, reused for all subsequent calls.
    ///
    /// Without this cache, every `generate_text()` call would:
    /// - Open the GGUF file (kernel call)
    /// - Map it into virtual address space (mmap)
    /// - Parse the tensor info table
    /// - Allocate KV cache
    /// Total: 500ms–3s per call for a 4GB model.
    ///
    /// With the cache: first call pays the load cost, subsequent calls are
    /// instant — the Engine is already in memory and the KV cache is hot.
    ///
    /// The Mutex is needed because `Engine::generate()` likely takes &mut self
    /// (inference modifies the KV cache state).
    #[cfg(feature = "llama-gguf")]
    engine_cache: std::sync::Arc<tokio::sync::Mutex<Option<llama_gguf::engine::Engine>>>,
}

impl GGUFProvider {
    /// Create GGUF provider with local model
    pub fn new(model_path: PathBuf, context_size: u32, threads: u32) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow!(
                "Model file not found at: {}",
                model_path.display()
            ));
        }

        info!(
            "Initialized GGUF provider with model: {} ({}KB)",
            model_path.display(),
            std::fs::metadata(&model_path)?.len() / 1024
        );

        Ok(Self {
            model_path,
            context_size,
            threads,
            #[cfg(feature = "llama-gguf")]
            engine_cache: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    /// Get the best available Qwen model path, probing known locations.
    ///
    /// Search order (first existing file wins):
    /// 1. Aaroneous models directory (`D:\Aaroneous\models\`) — Qwen2.5 abliterated variants
    /// 2. Aaroneous models directory — legacy Qwen names
    /// 3. Relative `./models/` paths (development/CI)
    /// 4. Parent-relative `../models/` path
    ///
    /// Returns the first existing path, or the Aaroneous default path as a
    /// fallback even if it doesn't exist (so `LLMConfig::gguf_model_path`
    /// is always populated with a sane value).
    pub fn default_qwen_path() -> PathBuf {
        let locations: Vec<PathBuf> = vec![
            // Crystallized sovereign models (preferred — domain-specialized)
            PathBuf::from("D:\\Aaroneous\\models\\ariel-qwen2.5-7b.gguf"),
            PathBuf::from("D:\\Aaroneous\\models\\wen-qwen2.5-7b.gguf"),  // smallest (847MB)
            // Foundation model fallback
            PathBuf::from("D:\\Aaroneous\\models\\foundation_v1.gguf"),
            // Legacy/abliterated variants
            PathBuf::from("D:\\Aaroneous\\models\\qwen2.5-1.5b-instruct-abliterated.gguf"),
            PathBuf::from("D:\\Aaroneous\\models\\qwen2.5-1.5b.gguf"),
            // Relative paths for CI/development
            PathBuf::from("./models/qwen2.5-1.5b.gguf"),
            PathBuf::from("./models/qwen-1.8b.gguf"),
        ];

        for loc in &locations {
            if loc.exists() {
                return loc.clone();
            }
        }

        // Default: Aaroneous preferred path (may not exist yet)
        PathBuf::from("D:\\Aaroneous\\models\\qwen2.5-1.5b-instruct-abliterated.gguf")
    }

    /// Generate text from a prompt using the loaded GGUF model.
    ///
    /// # Feature gating
    ///
    /// - **With `llama-gguf` feature**: uses the pure-Rust `llama-gguf` crate
    ///   for real model inference. No C library required. The inference runs
    ///   on `tokio::task::spawn_blocking` since it's CPU-bound and synchronous.
    ///
    /// - **Without `llama-gguf` feature** (default): returns a structured mock
    ///   response so the rest of the system continues to work without a model.
    async fn generate_text(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        #[cfg(feature = "llama-gguf")]
        {
            use llama_gguf::engine::{Engine, EngineConfig};

            let prompt_owned = prompt.to_string();
            let max_tokens_usize = max_tokens as usize;

            // Use the cached engine — load once on first call, reuse for all subsequent calls.
            // This turns 500ms–3s load cost per call into a one-time startup cost.
            let engine_cache = self.engine_cache.clone();
            let model_path_str = self.model_path.to_string_lossy().to_string();

            let result = tokio::task::spawn_blocking(move || -> Result<String> {
                // Lock the engine cache. On first call: load the engine.
                // On subsequent calls: use the already-loaded engine.
                let mut guard = engine_cache.blocking_lock();
                if guard.is_none() {
                    info!("GGUF: loading engine from {} (first call — one-time cost)",
                          model_path_str);
                    let config = EngineConfig {
                        model_path: model_path_str,
                        temperature: 0.7,
                        top_p: 0.95,
                        ..Default::default()
                    };
                    *guard = Some(
                        Engine::load(config)
                            .map_err(|e| anyhow!("Engine::load failed: {:?}", e))?
                    );
                    info!("GGUF: engine loaded and cached — subsequent calls will be instant");
                }

                let engine = guard.as_mut()
                    .ok_or_else(|| anyhow!("engine cache invariant violated"))?;

                engine
                    .generate(&prompt_owned, max_tokens_usize)
                    .map_err(|e| anyhow!("generation failed: {:?}", e))
            })
            .await
            .map_err(|e| anyhow!("spawn_blocking panicked: {}", e))??;

            debug!("GGUF inference complete: {} chars generated", result.len());
            return Ok(result);
        }

        // Fallback when llama-gguf feature is not enabled
        #[cfg(not(feature = "llama-gguf"))]
        {
            debug!(
                "GGUF mock (no llama-gguf feature): '{}...' ({} max_tokens)",
                &prompt[..50.min(prompt.len())],
                max_tokens
            );

            // Return a structured response that downstream parsers can still
            // extract JSON from when available, or identify as a mock.
            Ok(format!(
                "GGUF inference disabled (compile with --features llama-gguf \
                 to enable real model inference). \
                 Prompt summary: '{}'. Max tokens: {}.",
                &prompt[..80.min(prompt.len())],
                max_tokens
            ))
        }
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
            goal = specialist.current_goal.as_ref().unwrap_or(&"none".to_string())
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

    async fn chat(
        &self,
        system_prompt: &str,
        user_message: &str,
        _domain: &str,
    ) -> Result<String> {
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
        let count = context.variants_requested.min(3).max(1);

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
            style = if style.is_empty() { "modern, clean".to_string() } else { style },
            constraints = if constraints.is_empty() { "none".to_string() } else { constraints },
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
        let variants: Vec<DesignVariant> = serde_json::from_str(&json_str)
            .unwrap_or_else(|_| {
                // If parsing fails, return a single fallback variant
                warn!("Failed to parse GGUF design response; using fallback variant");
                vec![DesignVariant {
                    title: format!("Design for {}", context.intent),
                    description: format!("Generated design: {}", response.chars().take(200).collect::<String>()),
                    colors: vec!["#6366F1".to_string(), "#F8FAFC".to_string()],
                    typography: "Inter, sans-serif".to_string(),
                    layout: "single-column".to_string(),
                    confidence: 0.5,
                    reasoning: "Fallback from unparseable GGUF response".to_string(),
                }]
            });

        let batch_confidence = if variants.is_empty() { 0.0 } else {
            variants.iter().map(|v| v.confidence).sum::<f32>() / variants.len() as f32
        };

        Ok(DesignGeneration {
            intent: context.intent.clone(),
            variants,
            tokens_used: 0,
            batch_confidence,
        })
    }
}

/// Extract JSON from text that may contain extra content
fn extract_json_from_response(text: &str) -> Result<String> {
    // Try to find JSON object
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return Ok(text[start..=end].to_string());
        }
    }

    // Try to find JSON array
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return Ok(text[start..=end].to_string());
        }
    }

    // No JSON found
    Err(anyhow!("No JSON found in response: {}", text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_extraction() {
        let text = "Here's the JSON: {\"key\": \"value\"} and some more text";
        let json = extract_json_from_response(text).unwrap();
        assert!(json.contains("key"));
    }

    #[test]
    fn test_default_model_path() {
        let path = GGUFProvider::default_qwen_path();
        // Returns first existing GGUF; on dev machine the sovereign models are present.
        // On CI with no models, falls back to the legacy path.
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        assert!(
            name.ends_with(".gguf"),
            "expected a .gguf path, got: {}",
            path.display()
        );
    }
}
