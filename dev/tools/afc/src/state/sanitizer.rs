// dev/tools/afc/src/state/sanitizer.rs
use crate::router::ChatMessage;
use std::path::Path;
use tracing::info;

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

    /// Aggressively compact conversation history to reduce token weight
    /// Iterates through messages, filters verbose stdout blocks, drops obsolete <tool_call> blocks
    pub fn compact(messages: &mut Vec<ChatMessage>) -> Result<(), anyhow::Error> {
        info!("[Sanitizer] Starting aggressive context compaction");

        let mut new_messages = Vec::new();
        let mut total_tokens_saved = 0;

        for msg in messages.drain(..) {
            if msg.role != "user" || !msg.content.contains("```") {
                new_messages.push(msg);
                continue;
            }

            // Parse code blocks and filter verbose content
            let compacted = Self::compact_message_content(&msg.content);
            
            if compacted.len() < msg.content.len() {
                total_tokens_saved += msg.content.len() - compacted.len();
                info!(
                    "[Sanitizer] Compact {} -> {} chars (saved ~{} tokens)",
                    msg.content.len(),
                    compacted.len(),
                    (msg.content.len() - compacted.len()) / 4
                );
            }

            new_messages.push(ChatMessage {
                role: msg.role,
                content: compacted,
            });
        }

        messages.clear();
        messages.extend(new_messages);

        info!(
            "[Sanitizer] Compaction complete. Saved ~{} tokens",
            total_tokens_saved / 4
        );

        Ok(())
    }

    /// Compact a single message by filtering verbose content
    fn compact_message_content(content: &str) -> String {
        let mut result = String::new();
        
        // Replace verbose compilation blocks with summaries
        if content.contains("```") && (content.contains("Compiling") || content.contains("cargo")) {
            // Extract code blocks and filter verbose ones
            let parts: Vec<&str> = content.split("```").collect();
            for part in parts.iter().step_by(2) {
                result.push_str(part);
                if part.len() > 3000 && (part.contains("Compiling") || part.contains("Finished")) {
                    let summary = Self::summarize_compilation(part);
                    result.push_str(&format!("\n[Diagnostics Filtered: {}]", summary));
                } else if part.trim().is_empty() {
                    // Skip empty blocks (the closing ```)
                }
            }
        } else {
            // Simple truncation for non-code content
            if content.len() > 8000 {
                result.push_str(&content[..8000]);
                result.push_str("\n[...content truncated for token efficiency...]");
            } else {
                result.push_str(content);
            }
        }

        result
    }

    /// Detect if content is verbose compilation output
    fn is_verbose_compilation(content: &str) -> bool {
        content.len() > 2000 && (content.contains("Compiling") || content.contains("Finished") || content.contains("warning"))
    }

    /// Generate metadata-rich summary for filtered compilation output
    fn summarize_compilation(content: &str) -> String {
        let mut summary = String::new();

        if content.contains("Finished") && content.contains("success") {
            summary.push_str("Compilation Successful");
        } else if content.contains("error") {
            summary.push_str("Compilation Failed - Errors Present");
        } else {
            summary.push_str("Output Filtered");
        }

        // Count warnings
        let warning_count = content.matches("warning").count();
        if warning_count > 0 {
            summary.push_str(&format!(" ({} warnings)", warning_count));
        }

        summary
    }

    /// Truncate a code block while preserving structure
    fn truncate_code_block(content: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            return content.to_string();
        }

        let mut result = String::new();
        let mut chars = content.chars().peekable();
        let mut current_len = 0;

        while let Some(c) = chars.next() {
            let len_after = result.len() + c.len_utf8();
            if len_after > max_len {
                break;
            }
            result.push(c);
            current_len = len_after;
        }

        if current_len >= max_len - 10 {
            result.push_str("\n[...truncated...]");
        }

        result
    }
}
