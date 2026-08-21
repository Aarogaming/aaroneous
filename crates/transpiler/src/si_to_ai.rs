//! si_to_ai.rs
//! Translates Synthetic Intelligence (SI) machine-native binary state and tensors
//! into structured prompts, instructions, and schemas for conventional AI models.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Formatted prompt packet for conventional AI consumption (OpenAI, Claude, Ollama, GGUF)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPromptContext {
    pub system_instruction: String,
    pub user_prompt: String,
    pub target_schema: Option<String>,
    pub temperature: f32,
    pub max_tokens: usize,
}

/// SI to AI Transpiler Engine
pub struct SiToAiTranspiler;

impl SiToAiTranspiler {
    /// Translates an SI task intent and tensor context into an optimal LLM prompt
    pub fn serialize_task_to_prompt(
        task_id: &str,
        domain: &str,
        tensor_context: &[f32],
        goal_description: &str,
    ) -> Result<AiPromptContext> {
        let system_instruction = format!(
            "You are a specialized AI co-pilot working with the Aaroneous Synthetic Intelligence engine.\n\
             Domain: {}\n\
             Task ID: {}\n\
             Output format: Strict code block or structured JSON with zero conversational conversational preamble.",
            domain, task_id
        );

        // Summarize numeric tensor context
        let tensor_summary = if tensor_context.is_empty() {
            "None".to_string()
        } else {
            format!("Tensor vector (len {}): [{:.3}, {:.3}, ...]", tensor_context.len(), tensor_context[0], tensor_context.get(1).unwrap_or(&0.0))
        };

        let user_prompt = format!(
            "Execute the following task specification:\n\n\
             Goal: {}\n\
             Context Tensors: {}\n\n\
             Provide the complete drop-in implementation code.",
            goal_description, tensor_summary
        );

        Ok(AiPromptContext {
            system_instruction,
            user_prompt,
            target_schema: Some("code_or_json".to_string()),
            temperature: 0.2, // Low temperature for deterministic code synthesis
            max_tokens: 4096,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_si_to_ai_serialization() {
        let prompt = SiToAiTranspiler::serialize_task_to_prompt(
            "task_42",
            "SoftwareAdaptation",
            &[0.1, 0.9, 0.4],
            "Fix panic condition in memory mapped synapse",
        ).unwrap();

        assert!(prompt.system_instruction.contains("SoftwareAdaptation"));
        assert!(prompt.system_instruction.contains("task_42"));
        assert_eq!(prompt.temperature, 0.2);
    }
}
