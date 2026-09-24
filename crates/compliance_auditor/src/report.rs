//! The final compliance report: structural (`ast_auditor`) findings plus the
//! verified multi-angle diff findings, in one artifact a human or CI step
//! can read.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::verify::{Finding, Verdict};

#[derive(Debug, Serialize)]
pub struct ComplianceReport {
    pub base_sha: String,
    pub head_sha: String,
    pub files_reviewed: Vec<String>,
    /// Structural invariant violations from `cratify_core::run_workspace_audit`
    /// over the changed files, kept separate from the LLM-derived findings
    /// since they come from a deterministic checker, not a verified opinion.
    pub structural_errors: Vec<String>,
    pub findings: Vec<Finding>,
}

impl ComplianceReport {
    /// Run `ast_auditor`'s deterministic invariant checks over the changed
    /// files that still exist in the working tree (a file the diff deletes
    /// has nothing left to audit). Best-effort: a checker error is recorded
    /// as a structural error rather than failing the whole review.
    pub fn run_structural_audit(repo: &Path, changed_files: &[String]) -> Vec<String> {
        let existing: Vec<PathBuf> = changed_files
            .iter()
            .map(|f| repo.join(f))
            .filter(|p| p.is_file())
            .collect();
        if existing.is_empty() {
            return Vec::new();
        }

        match cratify_core::run_workspace_audit(existing) {
            Ok(()) => Vec::new(),
            Err(_) => vec![
                "ast_auditor reported one or more architectural invariant violations; \
                 re-run `cargo run -p ast_auditor -- audit <paths>` for details."
                    .to_string(),
            ],
        }
    }

    /// A finding counts against CI gating only once it's Confirmed and came
    /// from a correctness-class angle (or a quoted conventions violation) —
    /// Plausible findings and cleanup-class findings are surfaced but don't
    /// fail the build on their own.
    pub fn has_blocking_findings(&self) -> bool {
        !self.structural_errors.is_empty()
            || self
                .findings
                .iter()
                .any(|f| f.verdict == Verdict::Confirmed && f.candidate.angle.is_correctness())
    }

    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn print_human(&self) {
        println!(
            "Aaroneous compliance review: {}..{}",
            &self.base_sha[..12.min(self.base_sha.len())],
            &self.head_sha[..12.min(self.head_sha.len())]
        );
        println!("Files reviewed: {}", self.files_reviewed.len());

        if !self.structural_errors.is_empty() {
            println!("\n[structural]");
            for e in &self.structural_errors {
                println!("  - {e}");
            }
        }

        if self.findings.is_empty() {
            println!("\nNo findings survived verification.");
            return;
        }

        println!("\n[findings] {} survived verification", self.findings.len());
        for f in &self.findings {
            let line = f
                .candidate
                .line
                .map(|l| l.to_string())
                .unwrap_or_else(|| "?".to_string());
            println!(
                "\n{:?} [{:?}] {}:{}\n  {}\n  failure: {}\n  verifier: {}",
                f.candidate.angle,
                f.verdict,
                f.candidate.file,
                line,
                f.candidate.summary,
                f.candidate.failure_scenario,
                f.justification,
            );
        }
    }
}
