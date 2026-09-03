//! ai_to_si.rs
//! Ingests token streams, markdown responses, and JSON outputs from AI models,
//! strips conversational fluff, and parses clean machine-native payloads.

use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static CODE_FENCE_RE: OnceLock<Regex> = OnceLock::new();

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
        let code_fence_re = CODE_FENCE_RE.get_or_init(|| {
            Regex::new(r"```([a-zA-Z0-9_-]*)\r?\n([\s\S]*?)(?:```|$)").unwrap_or_else(|_| {
                Regex::new("").expect("fallback regex must be valid")
            })
        });

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

        // If no code fence is present, detect language dynamically from syntax heuristics
        let trimmed = raw_ai_response.trim();
        let detected_lang = if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("use ") || trimmed.starts_with("struct ") || trimmed.starts_with("impl ") {
            Some("rust")
        } else if trimmed.starts_with("def ") || trimmed.starts_with("import ") || trimmed.starts_with("from ") || trimmed.starts_with("class ") {
            Some("python")
        } else if trimmed.starts_with("function ") || trimmed.starts_with("export ") || trimmed.starts_with("const ") || trimmed.starts_with("interface ") {
            Some("typescript")
        } else if trimmed.starts_with("#include ") || trimmed.starts_with("int main") || trimmed.starts_with("void ") {
            Some("cpp")
        } else {
            None
        };

        if let Some(lang) = detected_lang {
            return Ok(ExtractedCodePayload {
                language: lang.to_string(),
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

    #[test]
    fn test_extract_code_unclosed_block() {
        let ai_reply = "```rust\npub fn stream_calc() -> u32 {\n    100\n}";
        let extracted = AiToSiTranspiler::extract_code(ai_reply).unwrap();
        assert_eq!(extracted.language, "rust");
        assert!(extracted.source_code.contains("pub fn stream_calc()"));
    }
}
