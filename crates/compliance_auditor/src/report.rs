//! The final compliance report: structural (`ast_auditor`) findings plus the
//! verified multi-angle diff findings, in one artifact a human or CI step
//! can read.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::verify::{Finding, Verdict};

/// Upper bound on structural finding lines kept in one report, so a large
/// change cannot flood the PR comment.
const MAX_STRUCTURAL_LINES: usize = 20;

#[derive(Debug, Serialize)]
pub struct ComplianceReport {
    pub base_sha: String,
    pub head_sha: String,
    pub files_reviewed: Vec<String>,
    /// Structural invariant violations from `cratify_core::audit_targets`
    /// over the changed files, kept separate from the LLM-derived findings
    /// since they come from a deterministic checker, not a verified opinion.
    pub structural_errors: Vec<String>,
    pub findings: Vec<Finding>,
}

impl ComplianceReport {
    /// Run `ast_auditor`'s deterministic invariant checks over the changed
    /// files that still exist in the working tree (a file the diff deletes
    /// has nothing left to audit) and that the auditor actually examines
    /// (Rust source and `Cargo.toml`; see `cratify_core::is_auditable_file`).
    ///
    /// A diff with no such file (docs, lockfile, workflow YAML, other TOML)
    /// returns no structural errors: `ast_auditor audit` on a scope with no
    /// source reports "No source files were audited" and fails, which says
    /// nothing about the change. Each real finding is returned as its own
    /// line with the auditor's own wording and a repo-relative path, capped
    /// at [`MAX_STRUCTURAL_LINES`]. Uses the non-printing
    /// `cratify_core::audit_targets`, so nothing reaches stdout (which
    /// `--json` mode reserves for the report).
    pub fn run_structural_audit(repo: &Path, changed_files: &[String]) -> Vec<String> {
        let auditable: Vec<PathBuf> = changed_files
            .iter()
            .map(|f| repo.join(f))
            .filter(|p| p.is_file() && cratify_core::is_auditable_file(p))
            .collect();
        if auditable.is_empty() {
            return Vec::new();
        }

        let report = cratify_core::audit_targets(auditable);
        let prefix = repo.display().to_string();
        let mut lines: Vec<String> = report
            .violation_messages()
            .into_iter()
            .map(|line| {
                line.replace(&format!("{prefix}/"), "")
                    .replace(&format!("{prefix}\\"), "")
            })
            .collect();
        if lines.len() > MAX_STRUCTURAL_LINES {
            let extra = lines.len() - MAX_STRUCTURAL_LINES;
            lines.truncate(MAX_STRUCTURAL_LINES);
            lines.push(format!(
                "... and {extra} more; re-run `cargo run -p ast_auditor -- audit <paths>` for the full list."
            ));
        }
        lines
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, text).expect("write fixture");
    }

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|f| (*f).to_string()).collect()
    }

    const AMBIENT: &str = "pub fn f() { let _ = std::env::current_dir(); }\n";

    #[test]
    fn docs_only_diff_reports_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "AGENTS.md", "# notes\n");
        write(dir.path(), "Cargo.lock", "version = 4\n");
        write(dir.path(), "deny.toml", "[advisories]\n");
        write(dir.path(), ".github/workflows/ci.yml", "name: ci\n");
        write(dir.path(), "xtask/baseline.txt", "a 1\n");
        let errors = ComplianceReport::run_structural_audit(
            dir.path(),
            &files(&[
                "AGENTS.md",
                "Cargo.lock",
                "deny.toml",
                ".github/workflows/ci.yml",
                "xtask/baseline.txt",
            ]),
        );
        assert!(errors.is_empty(), "got: {errors:?}");
    }

    #[test]
    fn deleted_file_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let errors = ComplianceReport::run_structural_audit(dir.path(), &files(&["src/gone.rs"]));
        assert!(errors.is_empty(), "got: {errors:?}");
    }

    #[test]
    fn clean_rust_reports_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "src/lib.rs", "pub fn f() -> u32 { 1 }\n");
        let errors = ComplianceReport::run_structural_audit(dir.path(), &files(&["src/lib.rs"]));
        assert!(errors.is_empty(), "got: {errors:?}");
    }

    #[test]
    fn real_violation_is_reported_with_auditor_text_and_relative_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "src/lib.rs", AMBIENT);
        let errors = ComplianceReport::run_structural_audit(dir.path(), &files(&["src/lib.rs"]));
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(errors[0].starts_with("src/lib.rs:"), "got: {}", errors[0]);
        assert!(errors[0].contains("current_dir"), "got: {}", errors[0]);
        assert!(
            !errors[0].contains(&dir.path().display().to_string()),
            "path should be repo-relative: {}",
            errors[0]
        );
    }

    #[test]
    fn non_rust_files_in_a_mixed_diff_are_not_audited() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "README.md", "# hi\n");
        write(dir.path(), "src/lib.rs", AMBIENT);
        let errors = ComplianceReport::run_structural_audit(
            dir.path(),
            &files(&["README.md", "src/lib.rs"]),
        );
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(errors[0].contains("current_dir"));
    }

    #[test]
    fn unparsable_rust_reports_the_real_parse_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "src/lib.rs", "fn (\n");
        let errors = ComplianceReport::run_structural_audit(dir.path(), &files(&["src/lib.rs"]));
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(
            errors[0].starts_with("[AUDIT ERROR] "),
            "got: {}",
            errors[0]
        );
        assert!(errors[0].contains("lib.rs"), "got: {}", errors[0]);
    }

    #[test]
    fn long_findings_list_is_capped_with_a_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut source = String::new();
        for i in 0..25 {
            source.push_str(&format!(
                "pub fn f{i}() {{ let _ = std::env::current_dir(); }}\n"
            ));
        }
        write(dir.path(), "src/lib.rs", &source);
        let errors = ComplianceReport::run_structural_audit(dir.path(), &files(&["src/lib.rs"]));
        assert_eq!(errors.len(), MAX_STRUCTURAL_LINES + 1, "got: {errors:?}");
        assert!(
            errors[MAX_STRUCTURAL_LINES].contains("and 5 more"),
            "got: {}",
            errors[MAX_STRUCTURAL_LINES]
        );
    }
}
