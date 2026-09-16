//! Aaroneous compliance auditor.
//!
//! Runs the same multi-angle diff review process a careful human reviewer
//! would run by hand — several independent finder passes, each biased
//! toward recall, followed by an adversarial verification pass that only
//! lets a finding through when it's actually real — as native, scriptable
//! Rust rather than an ad hoc agent transcript. Intended for both local use
//! (`cargo run -p compliance_auditor -- review --base origin/main --head HEAD`)
//! and CI gating.
//!
//! Pipeline:
//! 1. [`diff`] pins the review to two resolved commit SHAs so nothing can
//!    shift under a long-running review (a live working tree can).
//! 2. [`angles`] runs the independent finder passes concurrently over the
//!    pooled diff text.
//! 3. [`conventions`] separately checks the diff against any governing
//!    CLAUDE.md/CLAUDE.local.md — skipped entirely when none exists.
//! 4. Candidates are deduplicated, then [`verify`] runs one independent
//!    verifier call per surviving candidate and drops anything Refuted.
//! 5. [`report`] folds in `ast_auditor`'s deterministic structural checks
//!    over the changed files and produces the final artifact.

pub mod angles;
pub mod conventions;
pub mod diff;
pub mod report;
pub mod verify;

use std::path::Path;

use anyhow::Result;
use llm_gateway::LLMClient;

pub use angles::{Angle, Candidate};
pub use diff::DiffTarget;
pub use report::ComplianceReport;
pub use verify::{Finding, Verdict};

/// Tunable knobs for a review run, kept separate from the CLI's `clap`
/// struct so the pipeline is callable directly from other Rust code (a CI
/// step, a test, a future workflow orchestrator) without going through argv.
#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub base: String,
    pub head: String,
    /// Cap on how many highest-churn files get pulled into the reviewed
    /// diff; keeps a very large change from blowing the LLM context budget.
    pub max_files: usize,
    /// Cap on how many characters of concatenated diff get sent per angle
    /// call.
    pub diff_char_budget: usize,
    /// Cap on how many findings the final report keeps after ranking.
    pub findings_cap: usize,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
            max_files: 40,
            diff_char_budget: 60_000,
            findings_cap: 10,
        }
    }
}

/// Run the full pipeline against `repo` and return the finished report.
pub async fn run_review(client: &LLMClient, repo: &Path, config: &ReviewConfig) -> Result<ComplianceReport> {
    let target = diff::gather(repo, &config.base, &config.head, config.max_files)?;
    let files: Vec<String> = target.file_list();
    let diff_text = target.concatenated_patch(config.diff_char_budget);

    let (mut candidates, conventions_candidates) = tokio::join!(
        angles::run_all_angles(client, &diff_text),
        async {
            conventions::run(client, repo, &files, &diff_text)
                .await
                .unwrap_or_default()
        }
    );
    candidates.extend(conventions_candidates);
    let candidates = angles::dedup(candidates);

    let findings = verify::verify_all(client, &candidates).await;
    let findings = verify::rank_and_cap(findings, config.findings_cap);

    let structural_errors = ComplianceReport::run_structural_audit(repo, &files);

    Ok(ComplianceReport {
        base_sha: target.base_sha,
        head_sha: target.head_sha,
        files_reviewed: files,
        structural_errors,
        findings,
    })
}
