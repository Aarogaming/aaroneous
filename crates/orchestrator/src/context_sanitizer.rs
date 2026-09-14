//! Context sanitizer for orchestrator crate
//! Provides bounded prompt construction for isolated remediation and forensic auditing

use std::path::Path;

/// Sanitizer for constructing bounded prompts for LLM interactions
pub struct ContextSanitizer;

impl ContextSanitizer {
    /// Construct a strictly bounded, delta-only prompt for isolated remediation (<1,500 tokens).
    ///
    /// # Arguments
    /// * `target_path` - Path to the target file
    /// * `file_chunk` - The file content to analyze
    /// * `line_range` - Range of lines to focus on (inclusive)
    /// * `defect` - Description of the defect to fix
    /// * `compiler_feedback` - Optional compiler feedback
    pub fn sanitize_remediation_prompt(
        target_path: &Path,
        file_chunk: &str,
        line_range: (usize, usize),
        defect: &str,
        compiler_feedback: Option<&str>,
    ) -> Vec<crate::lmstudio_client::ChatMessage> {
        let messages = vec![

        // System message defining the task
        crate::lmstudio_client::ChatMessage {
            role: "system".to_string(),
            content: "You are a code remediation assistant. Focus only on the specified line range and defect. Provide minimal, targeted fixes.".to_string(),
        },

        // User message with context
        crate::lmstudio_client::ChatMessage {
            role: "user".to_string(),
            content: Self::build_remediation_context(
                target_path,
                file_chunk,
                line_range,
                defect,
                compiler_feedback,
            ),
        },

        ];
        messages
    }

    /// Construct a strictly bounded prompt for forensic auditing.
    ///
    /// # Arguments
    /// * `category` - Audit category (e.g., "security", "performance")
    /// * `candidate_code_sample` - Code to audit
    /// * `file_path` - Path to the file
    pub fn sanitize_audit_prompt(
        category: &str,
        candidate_code_sample: &str,
        file_path: &Path,
    ) -> Vec<crate::lmstudio_client::ChatMessage> {
        let mut messages = Vec::new();

        // System message defining the audit task
        messages.push(crate::lmstudio_client::ChatMessage {
            role: "system".to_string(),
            content: format!(
                "You are a forensic code auditor. Analyze the provided code for {} issues. Be thorough and specific.",
                category
            ),
        });

        // User message with audit context
        messages.push(crate::lmstudio_client::ChatMessage {
            role: "user".to_string(),
            content: Self::build_audit_context(category, candidate_code_sample, file_path),
        });

        messages
    }

    /// Build remediation context string
    fn build_remediation_context(
        target_path: &Path,
        file_chunk: &str,
        line_range: (usize, usize),
        defect: &str,
        compiler_feedback: Option<&str>,
    ) -> String {
        let (start_line, end_line) = line_range;

        let mut context = String::new();
        context.push_str(&format!("File: {}\n", target_path.display()));
        context.push_str(&format!("Lines: {}-{}\n", start_line, end_line));
        context.push_str(&format!("Defect: {}\n", defect));

        if let Some(feedback) = compiler_feedback {
            context.push_str(&format!("Compiler Feedback: {}\n", feedback));
        }

        context.push_str("\nCode Context:\n");
        context.push_str(&format!("{}...", file_chunk));

        context
    }

    /// Build audit context string
    fn build_audit_context(
        category: &str,
        candidate_code_sample: &str,
        file_path: &Path,
    ) -> String {
        let mut context = String::new();
        context.push_str(&format!("Category: {}\n", category));
        context.push_str(&format!("File: {}\n", file_path.display()));
        context.push_str("\nCode Sample:\n");
        context.push_str(candidate_code_sample);

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_sanitize_remediation_prompt_message_count() {
        let target_path = Path::new("test.rs");
        let file_chunk = "fn main() { println!(\"hello\"); }";
        let line_range = (1, 10);
        let defect = "Missing error handling";
        let compiler_feedback = Some("warning: unused variable");

        let messages = ContextSanitizer::sanitize_remediation_prompt(
            target_path,
            file_chunk,
            line_range,
            defect,
            compiler_feedback,
        );

        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_sanitize_remediation_prompt_roles() {
        let target_path = Path::new("test.rs");
        let file_chunk = "fn main() { println!(\"hello\"); }";
        let line_range = (1, 10);
        let defect = "Missing error handling";
        let compiler_feedback = Some("warning: unused variable");

        let messages = ContextSanitizer::sanitize_remediation_prompt(
            target_path,
            file_chunk,
            line_range,
            defect,
            compiler_feedback,
        );

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_sanitize_remediation_prompt_content_includes_defect() {
        let target_path = Path::new("test.rs");
        let file_chunk = "fn main() { println!(\"hello\"); }";
        let line_range = (1, 10);
        let defect = "Missing error handling";
        let compiler_feedback = Some("warning: unused variable");

        let messages = ContextSanitizer::sanitize_remediation_prompt(
            target_path,
            file_chunk,
            line_range,
            defect,
            compiler_feedback,
        );

        let user_content = &messages[1].content;
        assert!(user_content.contains("Defect: Missing error handling"));
        assert!(user_content.contains("File: test.rs"));
        assert!(user_content.contains("Lines: 1-10"));
    }

    #[test]
    fn test_sanitize_remediation_prompt_no_compiler_feedback() {
        let target_path = Path::new("test.rs");
        let file_chunk = "fn main() { println!(\"hello\"); }";
        let line_range = (1, 10);
        let defect = "Missing error handling";
        let compiler_feedback: Option<&str> = None;

        let messages = ContextSanitizer::sanitize_remediation_prompt(
            target_path,
            file_chunk,
            line_range,
            defect,
            compiler_feedback,
        );

        let user_content = &messages[1].content;
        assert!(!user_content.contains("Compiler Feedback"));
    }

    #[test]
    fn test_sanitize_audit_prompt_message_count() {
        let category = "security";
        let candidate_code_sample = "let x = unsafe { ... };";
        let file_path = Path::new("audit.rs");

        let messages =
            ContextSanitizer::sanitize_audit_prompt(category, candidate_code_sample, file_path);

        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_sanitize_audit_prompt_roles() {
        let category = "security";
        let candidate_code_sample = "let x = unsafe { ... };";
        let file_path = Path::new("audit.rs");

        let messages =
            ContextSanitizer::sanitize_audit_prompt(category, candidate_code_sample, file_path);

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_sanitize_audit_prompt_content_includes_category() {
        let category = "security";
        let candidate_code_sample = "let x = unsafe { ... };";
        let file_path = Path::new("audit.rs");

        let messages =
            ContextSanitizer::sanitize_audit_prompt(category, candidate_code_sample, file_path);

        let user_content = &messages[1].content;
        assert!(user_content.contains("Category: security"));
        assert!(user_content.contains("File: audit.rs"));
        assert!(user_content.contains("let x = unsafe { ... };"));
    }

    #[test]
    fn test_sanitize_audit_prompt_system_message_format() {
        let category = "performance";
        let candidate_code_sample = "loop { ... }";
        let file_path = Path::new("perf.rs");

        let messages =
            ContextSanitizer::sanitize_audit_prompt(category, candidate_code_sample, file_path);

        let system_content = &messages[0].content;
        assert!(system_content.contains("forensic code auditor"));
        assert!(system_content.contains("performance"));
    }
}
