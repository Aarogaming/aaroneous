//! crates/chimera/src/pattern_rewriter.rs
//! Universal Structural Code Pattern Search, Multi-Language AST Rewriter,
//! and Context-Preserving Substitution Engine (Comby & Structural Search inspired).

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A match result from a structural pattern query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub matched_text: String,
    pub captured_variables: HashMap<String, String>,
}

/// A structured replacement proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralPatch {
    pub file_path: String,
    pub original_lines: (usize, usize),
    pub original_snippet: String,
    pub replacement_snippet: String,
    pub patch_diff: String,
    pub confidence_score: f64,
}

/// Universal Pattern Rewriter & Multi-Language AST Mutation Engine
pub struct PatternRewriter;

impl PatternRewriter {
    /// Matches structural patterns in source code with Comby-style hole variables `:[name]`
    pub fn find_matches(
        file_path: &str,
        source_code: &str,
        pattern: &str,
    ) -> Result<Vec<PatternMatch>> {
        let (regex_pattern, var_names) = Self::compile_structural_pattern(pattern)?;
        let re = Regex::new(&regex_pattern)
            .with_context(|| format!("Failed to compile structural regex: {}", regex_pattern))?;

        let mut matches = Vec::new();

        for cap in re.captures_iter(source_code) {
            let full_match = cap.get(0).unwrap();
            let matched_text = full_match.as_str().to_string();
            let start_byte = full_match.start();
            let end_byte = full_match.end();

            let start_line = source_code[..start_byte].matches('\n').count() + 1;
            let end_line = source_code[..end_byte].matches('\n').count() + 1;

            let mut captured_variables = HashMap::new();
            for var_name in &var_names {
                if let Some(val) = cap.name(var_name) {
                    captured_variables.insert(var_name.clone(), val.as_str().to_string());
                }
            }

            matches.push(PatternMatch {
                file_path: file_path.to_string(),
                start_line,
                end_line,
                matched_text,
                captured_variables,
            });
        }

        Ok(matches)
    }

    /// Rewrites source code by substituting structural pattern matches with a replacement template
    pub fn rewrite_source(
        file_path: &str,
        source_code: &str,
        search_pattern: &str,
        replace_template: &str,
    ) -> Result<(String, Vec<StructuralPatch>)> {
        let matches = Self::find_matches(file_path, source_code, search_pattern)?;
        if matches.is_empty() {
            return Ok((source_code.to_string(), Vec::new()));
        }

        let mut patches = Vec::new();
        let mut rewritten = source_code.to_string();

        for m in matches.iter().rev() {
            let mut replacement = replace_template.to_string();
            for (var_name, var_value) in &m.captured_variables {
                let placeholder = format!(":[{}]", var_name);
                replacement = replacement.replace(&placeholder, var_value);
            }

            let text_diff = similar::TextDiff::from_lines(&m.matched_text, &replacement);
            let diff = text_diff
                .unified_diff()
                .header(&format!("a/{}", file_path), &format!("b/{}", file_path))
                .to_string();

            patches.push(StructuralPatch {
                file_path: file_path.to_string(),
                original_lines: (m.start_line, m.end_line),
                original_snippet: m.matched_text.clone(),
                replacement_snippet: replacement.clone(),
                patch_diff: diff,
                confidence_score: 0.95,
            });

            // Perform in-place replacement
            if let Some(pos) = rewritten.rfind(&m.matched_text) {
                rewritten.replace_range(pos..pos + m.matched_text.len(), &replacement);
            }
        }

        patches.reverse();
        Ok((rewritten, patches))
    }

    /// Converts a Comby-like pattern `:[var]` into a robust regex with named capture groups
    fn compile_structural_pattern(pattern: &str) -> Result<(String, Vec<String>)> {
        let hole_regex = Regex::new(r":\[([a-zA-Z0-9_]+)\]")?;
        let mut var_names = Vec::new();

        let mut compiled = regex::escape(pattern);

        for cap in hole_regex.captures_iter(pattern) {
            let var_name = cap[1].to_string();
            let escaped_hole = regex::escape(&format!(":[{}]", var_name));
            let replacement = format!("(?P<{}>[\\s\\S]*?)", var_name);
            compiled = compiled.replace(&escaped_hole, &replacement);
            var_names.push(var_name);
        }

        Ok((compiled, var_names))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structural_pattern_matching() {
        let code = r#"
fn calculate_tax(amount: f64) -> f64 {
    panic!("tax service unavailable");
}

fn calculate_discount(amount: f64) -> f64 {
    panic!("discount service unavailable");
}
"#;

        let pattern = "fn :[name](:[params]) -> :[ret] {\n    panic!(:[msg]);\n}";
        let matches = PatternRewriter::find_matches("finance.rs", code, pattern).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].captured_variables.get("name").unwrap(), "calculate_tax");
        assert_eq!(matches[1].captured_variables.get("name").unwrap(), "calculate_discount");
    }

    #[test]
    fn test_structural_rewriting() {
        let code = r#"
fn perform_action() {
    println!("DEBUG: starting action");
}
"#;

        let search_pattern = "println!(\"DEBUG: :[msg]\");";
        let replace_template = "tracing::debug!(\":[msg]\");";

        let (rewritten, patches) = PatternRewriter::rewrite_source("app.rs", code, search_pattern, replace_template).unwrap();

        assert!(rewritten.contains("tracing::debug!(\"starting action\");"));
        assert_eq!(patches.len(), 1);
        assert!(patches[0].patch_diff.contains("-println!(\"DEBUG: starting action\");"));
        assert!(patches[0].patch_diff.contains("+tracing::debug!(\"starting action\");"));
    }
}
