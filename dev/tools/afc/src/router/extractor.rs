// dev/tools/afc/src/router/extractor.rs
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

pub struct TypedExtractor;

impl TypedExtractor {
    /// Extract a typed struct from a raw string containing JSON, automatically stripping markdown code blocks
    pub fn extract_json<T: DeserializeOwned>(raw_text: &str) -> Result<T> {
        let trimmed = raw_text.trim();

        // 1. Check for ```json ... ``` markdown code fences
        let json_str = if let Some(start) = trimmed.find("```json") {
            let content_start = start + 7;
            if let Some(end) = trimmed[content_start..].find("```") {
                trimmed[content_start..content_start + end].trim()
            } else {
                trimmed[content_start..].trim()
            }
        } else if let Some(start) = trimmed.find("```") {
            let content_start = start + 3;
            if let Some(end) = trimmed[content_start..].find("```") {
                trimmed[content_start..content_start + end].trim()
            } else {
                trimmed[content_start..].trim()
            }
        } else if let (Some(first_brace), Some(last_brace)) =
            (trimmed.find('{'), trimmed.rfind('}'))
        {
            if first_brace <= last_brace {
                &trimmed[first_brace..=last_brace]
            } else {
                trimmed
            }
        } else if let (Some(first_bracket), Some(last_bracket)) =
            (trimmed.find('['), trimmed.rfind(']'))
        {
            if first_bracket <= last_bracket {
                &trimmed[first_bracket..=last_bracket]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        serde_json::from_str::<T>(json_str).context(format!(
            "Failed to parse JSON into target type. Extracted payload:\n{json_str}"
        ))
    }

    /// Extract tool call arguments by name
    pub fn extract_tool_arguments<T: DeserializeOwned>(
        tool_calls: &[crate::router::types::ToolCall],
        target_function_name: &str,
    ) -> Result<T> {
        let matching = tool_calls
            .iter()
            .find(|tc| tc.function.name == target_function_name)
            .ok_or_else(|| {
                anyhow::anyhow!("No tool call found matching '{target_function_name}'")
            })?;

        Self::extract_json::<T>(&matching.function.arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct SamplePayload {
        id: String,
        count: u32,
    }

    #[test]
    fn test_extract_json_markdown() {
        let text = "Here is the result:\n```json\n{\n  \"id\": \"sample_1\",\n  \"count\": 42\n}\n```\nHope that helps!";
        let extracted: SamplePayload = TypedExtractor::extract_json(text).expect("Should parse");
        assert_eq!(
            extracted,
            SamplePayload {
                id: "sample_1".into(),
                count: 42
            }
        );
    }

    #[test]
    fn test_extract_json_embedded() {
        let text = "Prefix text {\"id\": \"embedded\", \"count\": 7} suffix text";
        let extracted: SamplePayload = TypedExtractor::extract_json(text).expect("Should parse");
        assert_eq!(
            extracted,
            SamplePayload {
                id: "embedded".into(),
                count: 7
            }
        );
    }
}
