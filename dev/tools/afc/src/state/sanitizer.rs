// dev/tools/afc/src/state/sanitizer.rs
use crate::router::ChatMessage;
use std::path::Path;

pub struct ContextSanitizer;

impl ContextSanitizer {
    /// Construct a strictly bounded, delta-only prompt for isolated remediation.
    /// Keeps token count under 1,500 tokens to preserve KV cache efficiency and generation speed.
    pub fn sanitize_remediation_prompt(
        target_path: &Path,
        file_chunk: &str,
        line_range: (usize, usize),
        defect: &str,
        compiler_feedback: Option<&str>,
    ) -> Vec<ChatMessage> {
        let file_display = target_path.to_string_lossy();
        let mut user_prompt = format!(
            "File: {file_display}\nLines {}-{}:\n```rust\n{}\n```\nDefect: {}\n",
            line_range.0,
            line_range.1,
            file_chunk.trim(),
            defect
        );

        if let Some(err) = compiler_feedback {
            user_prompt.push_str(&format!("\nCompiler Feedback:\n{}\n", err.trim()));
        }

        user_prompt.push_str("\nInstruction: Provide only the replacement Rust code for lines ");
        user_prompt.push_str(&format!("{}-{}. ", line_range.0, line_range.1));
        user_prompt.push_str("Ensure Result propagation, zero .unwrap()/.expect(), and no unsafe.");

        vec![
            ChatMessage::system(
                "You are an expert Rust systems engineer. Output strictly replacement Rust code without markdown conversation."
            ),
            ChatMessage::user(user_prompt),
        ]
    }

    /// Construct a strictly bounded prompt for forensic auditing
    pub fn sanitize_audit_prompt(
        category: &str,
        candidate_code_sample: &str,
        file_path: &Path,
    ) -> Vec<ChatMessage> {
        let file_display = file_path.to_string_lossy();
        let user_prompt = format!(
            "Audit Category: {category}\nFile: {file_display}\nCode:\n```rust\n{}\n```\nTask: Identify unhandled unwrap/panics, memory leaks, or concurrency bugs. Report concisely.",
            candidate_code_sample.trim()
        );

        vec![
            ChatMessage::system(
                "You are an expert Rust static analysis auditor. Report findings in concise JSON format with file, line, tier, and fix recommendation."
            ),
            ChatMessage::user(user_prompt),
        ]
    }
}
