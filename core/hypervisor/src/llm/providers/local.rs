// Local LLM Provider
// Integration with local models via Ollama, vLLM, etc.

use crate::llm::types::*;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

pub struct LocalLLMProvider {
    endpoint: String,
    model: String,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct LocalRequest {
    model: String,
    prompt: String,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct LocalResponse {
    response: String,
}

impl LocalLLMProvider {
    pub async fn new() -> Result<Self> {
        let endpoint = std::env::var("LOCAL_LLM_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        let model =
            std::env::var("LOCAL_LLM_MODEL").unwrap_or_else(|_| "mistral:latest".to_string());

        info!("Initialized Local LLM provider at: {}", endpoint);

        Ok(Self {
            endpoint,
            model,
            temperature: 0.7,
        })
    }

    async fn call_api(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let request = LocalRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            temperature: self.temperature,
            stream: false,
        };

        debug!("Calling local LLM at: {}", self.endpoint);

        let response = client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Local LLM error: {}", response.status()));
        }

        let data: LocalResponse = response.json().await?;

        Ok(data.response)
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
impl super::LLMProvider for LocalLLMProvider {
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        let prompt = self.build_task_analysis_prompt(context);
        let response = self.call_api(&prompt).await?;

        debug!("Local LLM task analysis response received");

        // Extract JSON from response (in case there's extra text)
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

        let response = self.call_api(&prompt).await?;

        debug!("Local LLM collaborator finding response received");

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

        let response = self.call_api(&prompt).await?;

        debug!("Local LLM plan generation response received");

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

        let response = self.call_api(&prompt).await?;

        debug!("Local LLM failure analysis response received");

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

        let response = self.call_api(&prompt).await?;

        debug!("Local LLM skill explanation response received");

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
  "source": "local_llm",
  "variants": [
    {{
      "id": "variant-id",
      "description": "String",
      "colors": ["#hex1", "#hex2"],
      "typography": "String",
      "layout": "String",
      "confidence": 0.9
    }}
  ]
}}
"##,
            intent = context.intent,
            constraints = context.constraints.join(", "),
            count = context.variants_requested
        );

        let response = self.call_api(&prompt).await?;
        debug!("Local LLM design generation response received");

        let json_str = extract_json(&response)?;
        let generation: DesignGeneration = serde_json::from_str(&json_str)?;

        Ok(generation)
    }

    async fn chat(&self, system_prompt: &str, user_message: &str, _domain: &str) -> Result<String> {
        let combined = format!("System: {}\n\nUser: {}", system_prompt, user_message);
        self.call_api(&combined).await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let client = reqwest::Client::new();

        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            prompt: String,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            embedding: Vec<f32>,
        }

        let req = EmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = client
            .post(format!("{}/api/embeddings", self.endpoint))
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Local LLM embedding error: {}", response.status()));
        }

        let data: EmbedResponse = response.json().await?;
        Ok(data.embedding)
    }
}

/// Extract JSON from text that might contain extra content
pub fn extract_json(text: &str) -> Result<String> {
    // Try to find JSON object or array
    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
    {
        return Ok(text[start..=end].to_string());
    }

    if let Some(start) = text.find('[')
        && let Some(end) = text.rfind(']')
    {
        return Ok(text[start..=end].to_string());
    }

    // If no JSON found, assume the whole thing is JSON
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires local LLM running
    async fn test_local_provider_creation() {
        let result = LocalLLMProvider::new().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_extraction() {
        let text = "Here's some analysis: {\"key\": \"value\"} and more text";
        let json = extract_json(text).unwrap();
        assert!(json.contains("key"));
    }
}
