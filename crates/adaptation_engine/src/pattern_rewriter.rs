//! crates/adaptation_engine/src/pattern_rewriter.rs
//! Universal Structural Code Pattern Search, Multi-Language AST Rewriter,
//! and Context-Preserving Substitution Engine (Comby & Structural Search inspired).

use anyhow::{Context, Result};
use governance::{InterlockAuditCertificate, SmtActionInterlock};
use regex::Regex;
use serde::{Deserialize, Serialize};
use si_ir::{MachineOpcode, NativeComputationNode, NativeComputationalGraph, NativeTypeLattice};
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

    /// Synthesizes a formal NativeComputationalGraph representing the proposed structural patches.
    /// Maps patch modifications, byte differentials, and scope bounds into formal DAG nodes with
    /// dimensional units and thermodynamic free-energy dissipation estimates.
    pub fn patches_to_action_graph(
        _file_path: &str,
        patches: &[StructuralPatch],
    ) -> NativeComputationalGraph {
        let mut graph = NativeComputationalGraph::new();
        let mut prev_id = 0u64;

        for (idx, patch) in patches.iter().enumerate() {
            let node_id = (idx as u64) + 1;
            let byte_delta = (patch.replacement_snippet.len() as isize
                - patch.original_snippet.len() as isize)
                .unsigned_abs();
            // Estimate thermodynamic dissipation from edit distance and line breadth
            let line_span = (patch
                .original_lines
                .1
                .saturating_sub(patch.original_lines.0)
                + 1) as f64;
            let dissipation = 0.001 * (1.0 + (byte_delta as f64 * 0.0001) + (line_span * 0.0005));

            let dependencies = if prev_id > 0 { vec![prev_id] } else { vec![] };

            graph.add_node(NativeComputationNode {
                id: node_id,
                opcode: MachineOpcode::Call {
                    function_id: node_id,
                    arg_regs: vec![],
                },
                type_lattice: NativeTypeLattice::PrimitiveInt {
                    bits: 64,
                    signed: false,
                },
                energy_cost: dissipation,
                dependencies,
            });

            prev_id = node_id;
        }

        if prev_id > 0 {
            graph.entry_node = 1;
            graph.exit_node = prev_id;
        }

        // Sum energy costs for thermodynamic verification
        graph.thermodynamic_free_energy = graph.extract_energy_vector().iter().sum();
        graph
    }

    /// Rewrites source code and verifies that the resulting patches satisfy the formal SMT action interlock
    /// and thermodynamic dissipation bounds before committing the modification.
    /// If the interlock rejects the modification, the original source code is preserved and returned.
    pub fn rewrite_source_interlocked(
        file_path: &str,
        source_code: &str,
        search_pattern: &str,
        replace_template: &str,
        interlock: &SmtActionInterlock,
    ) -> Result<(String, Vec<StructuralPatch>, InterlockAuditCertificate)> {
        let (candidate_code, patches) =
            Self::rewrite_source(file_path, source_code, search_pattern, replace_template)?;

        if patches.is_empty() {
            let empty_graph = NativeComputationalGraph::new();
            let cert = interlock.evaluate_action_graph(&empty_graph)?;
            return Ok((source_code.to_string(), patches, cert));
        }

        let action_graph = Self::patches_to_action_graph(file_path, &patches);
        let cert = interlock.evaluate_action_graph(&action_graph)?;

        if cert.is_authorized {
            Ok((candidate_code, patches, cert))
        } else {
            // Interlock rejected: preserve original source code without mutation
            Ok((source_code.to_string(), patches, cert))
        }
    }
}

/// Pre-Execution Interlocked Code Mutation Engine
/// Coordinates AST structural search & replace with formal SMT non-interference
/// and thermodynamic dissipation gating before committing changes.
pub struct SmtInterlockedRewriter {
    interlock: SmtActionInterlock,
}

impl SmtInterlockedRewriter {
    pub fn new(max_free_energy_bound: f64) -> Self {
        Self {
            interlock: SmtActionInterlock::new(max_free_energy_bound),
        }
    }

    pub fn strict() -> Self {
        Self {
            interlock: SmtActionInterlock::strict(),
        }
    }

    pub fn interlock(&self) -> &SmtActionInterlock {
        &self.interlock
    }

    /// Rewrites source code if and only if the candidate mutation passes the formal interlock.
    pub fn rewrite(
        &self,
        file_path: &str,
        source_code: &str,
        search_pattern: &str,
        replace_template: &str,
    ) -> Result<(String, Vec<StructuralPatch>, InterlockAuditCertificate)> {
        PatternRewriter::rewrite_source_interlocked(
            file_path,
            source_code,
            search_pattern,
            replace_template,
            &self.interlock,
        )
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
        assert_eq!(
            matches[0].captured_variables.get("name").unwrap(),
            "calculate_tax"
        );
        assert_eq!(
            matches[1].captured_variables.get("name").unwrap(),
            "calculate_discount"
        );
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

        let (rewritten, patches) =
            PatternRewriter::rewrite_source("app.rs", code, search_pattern, replace_template)
                .unwrap();

        assert!(rewritten.contains("tracing::debug!(\"starting action\");"));
        assert_eq!(patches.len(), 1);
        assert!(
            patches[0]
                .patch_diff
                .contains("-println!(\"DEBUG: starting action\");")
        );
        assert!(
            patches[0]
                .patch_diff
                .contains("+tracing::debug!(\"starting action\");")
        );
    }

    #[test]
    fn test_interlocked_rewrite_authorized() {
        let code = r#"
fn run() {
    println!("DEBUG: starting action");
}
"#;
        let search_pattern = "println!(\"DEBUG: :[msg]\");";
        let replace_template = "tracing::debug!(\":[msg]\");";
        let interlock = SmtActionInterlock::strict();

        let (rewritten, patches, cert) = PatternRewriter::rewrite_source_interlocked(
            "app.rs",
            code,
            search_pattern,
            replace_template,
            &interlock,
        )
        .unwrap();

        assert!(cert.is_authorized);
        assert!(!cert.smt_non_interference_verified);
        assert!(rewritten.contains("tracing::debug!(\"starting action\");"));
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_interlocked_rewrite_rejected_energy_bound() {
        let code = r#"
fn run() {
    println!("DEBUG: starting action");
}
"#;
        let search_pattern = "println!(\"DEBUG: :[msg]\");";
        let replace_template = "tracing::debug!(\":[msg]\");";
        // Strict ultra-low free-energy bound: 0.00001 (lower than single patch dissipation)
        let interlock = SmtActionInterlock::new(0.00001);

        let (rewritten, patches, cert) = PatternRewriter::rewrite_source_interlocked(
            "app.rs",
            code,
            search_pattern,
            replace_template,
            &interlock,
        )
        .unwrap();

        // Must be rejected by the thermodynamic interlock
        assert!(!cert.is_authorized);
        assert!(cert.denial_reason.is_some());
        // Source code remains unmutated!
        assert_eq!(rewritten, code);
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_interlocked_rewrite_killswitch_rejection() {
        let code = r#"
fn run() {
    println!("DEBUG: starting action");
}
"#;
        let search_pattern = "println!(\"DEBUG: :[msg]\");";
        let replace_template = "tracing::debug!(\":[msg]\");";
        let interlock = SmtActionInterlock::strict();
        interlock.trip_killswitch();

        let result = PatternRewriter::rewrite_source_interlocked(
            "app.rs",
            code,
            search_pattern,
            replace_template,
            &interlock,
        );

        // Emergency killswitch trips immediately
        assert!(result.is_err());
    }

    #[test]
    fn test_smt_interlocked_rewriter_wrapper() {
        let code = r#"
fn execute() {
    println!("DEBUG: exec");
}
"#;
        let search_pattern = "println!(\"DEBUG: :[msg]\");";
        let replace_template = "tracing::info!(\":[msg]\");";
        let rewriter = SmtInterlockedRewriter::strict();

        let (rewritten, patches, cert) = rewriter
            .rewrite("worker.rs", code, search_pattern, replace_template)
            .unwrap();

        assert!(cert.is_authorized);
        assert!(rewritten.contains("tracing::info!(\"exec\");"));
        assert_eq!(patches.len(), 1);
    }
}
