//! ai_to_si.rs
//! Ingests token streams, markdown responses, and JSON outputs from AI models,
//! strips conversational fluff, and parses clean machine-native payloads.

use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Parsed Machine-Native payload extracted from AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedCodePayload {
    pub language: String,
    pub source_code: String,
    pub is_valid: bool,
}

/// AI to SI Transpiler Engine
pub struct AiToSiTranspiler;

impl AiToSiTranspiler {
    /// Extracts clean executable code from raw markdown or LLM response text
    pub fn extract_code(raw_ai_response: &str) -> Result<ExtractedCodePayload> {
        let code_fence_re = Regex::new(r"(?s)```([a-zA-Z0-9_-]*)\r?\n(.*?)```")?;

        if let Some(captures) = code_fence_re.captures(raw_ai_response) {
            let lang = captures.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_else(|| "rust".to_string());
            let code = captures.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

            let clean_lang = if lang.is_empty() { "rust".to_string() } else { lang };
            return Ok(ExtractedCodePayload {
                language: clean_lang,
                source_code: code,
                is_valid: true,
            });
        }

        // If no code fence is present, check if raw response is direct code
        let trimmed = raw_ai_response.trim();
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("use ") || trimmed.starts_with("struct ") {
            return Ok(ExtractedCodePayload {
                language: "rust".to_string(),
                source_code: trimmed.to_string(),
                is_valid: true,
            });
        }

        bail!("No valid code block found in AI response");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_from_markdown() {
        let ai_reply = "Sure! Here is the implementation:\n\n```rust\npub fn calculate() -> u32 {\n    42\n}\n```\nHope this helps!";
        let extracted = AiToSiTranspiler::extract_code(ai_reply).unwrap();
        assert_eq!(extracted.language, "rust");
        assert!(extracted.source_code.contains("pub fn calculate()"));
        assert!(!extracted.source_code.contains("Sure!"));
    }
}
