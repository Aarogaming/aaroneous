// OpenAI Provider
// Integration with OpenAI API (GPT-4, GPT-3.5, etc.)

use crate::llm::types::*;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct OpenAIProvider {
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Usage {
    _prompt_tokens: u32,
    _completion_tokens: u32,
    total_tokens: u32,
}

impl OpenAIProvider {
    pub async fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY environment variable not set"))?;

        info!("Initialized OpenAI provider");

        Ok(Self {
            api_key,
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
        })
    }

    async fn call_api(&self, prompt: &str) -> Result<(String, u32, f64)> {
        let client = reqwest::Client::new();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        debug!("Calling OpenAI API with model: {}", self.model);

        // Wrap the upstream call with a 60-second per-request
        // timeout. The OpenAI API occasionally hangs on long
        // prompts; without this, a stuck connection ties up
        // the federation's HTTP client for the OS-default
        // keep-alive duration. `tokio::time::timeout` is the
        // raw primitive; the `with_timeout` helper in
        // `resilience` is the same shape but lives alongside
        // the other async resilience primitives.
        let send_fut = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send();
        let response =
            match tokio::time::timeout(std::time::Duration::from_secs(60), send_fut).await {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => return Err(anyhow::Error::from(e)),
                Err(_) => {
                    return Err(anyhow!("OpenAI API request timed out after 60s"));
                }
            };

        if !response.status().is_success() {
            let error_text = response.text().await?;
            warn!("OpenAI API error: {}", error_text);
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let data: OpenAIResponse = response.json().await?;

        let content = data
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from OpenAI"))?
            .message
            .content
            .clone();

        let tokens = data.usage.total_tokens;

        // Rough cost estimation: GPT-4 is ~$0.03/1K input tokens
        let cost = (tokens as f64) * 0.00003;

        Ok((content, tokens, cost))
    }

    fn build_task_analysis_prompt(&self, context: &TaskAnalysisContext) -> String {
        format!(
            r#"You are a specialist AI analyzer helping to break down complex tasks.

A specialist named with skills [{skills}] working in {domain} received this task:

FILE: {file}
SIZE: {size} bytes
TYPE: {file_type}

DATA SAMPLE:
{sample}

Analyze this task and provide a JSON response with:
1. analysis_type: String (what kind of analysis is this)
2. complexity: "Simple" | "Moderate" | "Complex"
3. recommended_approach: String (how to approach this)
4. estimated_time_minutes: number
5. confidence_percentage: number (0-100)
6. suggested_collaborators: string[] (team members to ask for help)
7. potential_risks: string[] (what could go wrong)
8. reasoning: String (explain your analysis)

Response must be valid JSON only.
"#,
            skills = context.specialist_skills.join(", "),
            domain = context.specialist_domain,
            file = context.file_name,
            size = context.file_size,
            file_type = context.file_type,
            sample = context.data_sample
        )
    }

    fn build_collaborators_prompt(&self, specialist: &SpecialistContext) -> String {
        format!(
            r#"You are a team coordinator for autonomous AI specialists.

Specialist: {name}
Role: {archetype}
Rank: {rank}
Skills: {skills}
Current Goal: {goal}

Recommend 2-3 team members they should collaborate with based on:
1. Complementary skills
2. Recent collaboration patterns
3. Current team needs

Respond with JSON array:
[
  {{
    "specialist_name": "String",
    "reason": "String",
    "relevance_score": 0.0-1.0,
    "complementary_skills": ["skill1", "skill2"]
  }}
]

Response must be valid JSON array only.
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
            goal = specialist
                .current_goal
                .as_ref()
                .unwrap_or(&"None".to_string())
        )
    }
}

#[async_trait]
impl super::LLMProvider for OpenAIProvider {
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        let prompt = self.build_task_analysis_prompt(context);
        let (response, tokens, _cost) = self.call_api(&prompt).await?;

        debug!("OpenAI task analysis response: {} tokens", tokens);

        // Parse JSON response
        let analysis: TaskAnalysis = serde_json::from_str(&response)?;

        Ok(analysis)
    }

    async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>> {
        let prompt = self.build_collaborators_prompt(specialist);
        let (response, tokens, _cost) = self.call_api(&prompt).await?;

        debug!("OpenAI collaborator finding: {} tokens", tokens);

        let suggestions: Vec<CollaboratorSuggestion> = serde_json::from_str(&response)?;

        Ok(suggestions)
    }

    async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan> {
        let prompt = format!(
            r#"Create an execution plan for:
Specialist: {name} ({archetype})
Task: {task_type}
Approach: {approach}
Estimated Time: {time} minutes

Respond with JSON:
{{
  "task_id": "{task_id}",
  "specialist_name": "{name}",
  "steps": [
    {{
      "sequence": 1,
      "description": "String",
      "estimated_time_minutes": number,
      "required_skills": ["skill"],
      "checkpoints": ["checkpoint"]
    }}
  ],
  "total_estimated_time": number,
  "success_probability": 0.0-1.0,
  "reasoning": "String"
}}

Response must be valid JSON only.
"#,
            name = specialist.name,
            archetype = specialist.archetype,
            task_type = task.analysis_type,
            approach = task.recommended_approach,
            time = task.estimated_time_minutes,
            task_id = task.task_id
        );

        let (response, tokens, _cost) = self.call_api(&prompt).await?;

        debug!("OpenAI plan generation: {} tokens", tokens);

        let plan: ExecutionPlan = serde_json::from_str(&response)?;

        Ok(plan)
    }

    async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis> {
        let prompt = format!(
            r#"Analyze this failure and suggest recovery:

Specialist: {specialist}
Task: {task}
Error: {error}
Attempted Approach: {approach}
Available Skills: {skills}

Provide JSON response:
{{
  "root_cause": "String",
  "explanation": "String",
  "prevention_strategy": "String",
  "recovery_approach": "String",
  "new_strategy": "String",
  "confidence_percentage": 0-100
}}

Response must be valid JSON only.
"#,
            specialist = failure.specialist_name,
            task = failure.task_id,
            error = failure.error_message,
            approach = failure.attempted_approach,
            skills = failure.available_skills.join(", ")
        );

        let (response, tokens, _cost) = self.call_api(&prompt).await?;

        debug!("OpenAI failure analysis: {} tokens", tokens);

        let analysis: FailureAnalysis = serde_json::from_str(&response)?;

        Ok(analysis)
    }

    async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation> {
        let prompt = format!(
            r#"Explain this skill to {specialist_name}:

Skill: {skill}
Specialist's Domain: {domain}
Current Skills: {skills}

Respond with JSON:
{{
  "skill_name": "{skill}",
  "description": "String",
  "use_cases": ["case1", "case2"],
  "example": "String",
  "how_to_improve": "String",
  "synergies_with": ["skill1", "skill2"]
}}

Response must be valid JSON only.
"#,
            specialist_name = specialist.name,
            skill = skill_name,
            domain = specialist.archetype,
            skills = specialist
                .skills
                .iter()
                .map(|s| &s.name)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );

        let (response, tokens, _cost) = self.call_api(&prompt).await?;

        debug!("OpenAI skill explanation: {} tokens", tokens);

        let explanation: SkillExplanation = serde_json::from_str(&response)?;

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
  "source": "openai_api",
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

        let (response, tokens, _cost) = self.call_api(&prompt).await?;
        debug!("OpenAI design generation: {} tokens", tokens);

        let json_str = if let Some(s) = response.find('{') {
            if let Some(e) = response.rfind('}') {
                &response[s..=e]
            } else {
                &response
            }
        } else {
            &response
        };

        let generation: DesignGeneration = serde_json::from_str(json_str)?;
        Ok(generation)
    }

    async fn chat(&self, system_prompt: &str, user_message: &str, _domain: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()?;

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let data: OpenAIResponse = response.json().await?;
        let content = data
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from OpenAI"))?
            .message
            .content
            .clone();

        Ok(content)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .build()?;

        #[derive(Serialize)]
        struct EmbedRequest {
            input: String,
            model: String,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            data: Vec<EmbedData>,
        }

        #[derive(Deserialize)]
        struct EmbedData {
            embedding: Vec<f32>,
        }

        let req = EmbedRequest {
            input: text.to_string(),
            model: "text-embedding-3-small".to_string(),
        };

        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let data: EmbedResponse = response.json().await?;
        Ok(data
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires OPENAI_API_KEY
    async fn test_openai_provider_creation() {
        let result = OpenAIProvider::new().await;
        assert!(result.is_ok());
    }
}
