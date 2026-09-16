//! Git diff gathering for a compliance review target.
//!
//! The reviewer always diffs two fixed refs (`base..head`) rather than the
//! working tree: a working-tree diff can change out from under a long-running
//! multi-pass review if another process (a human, CI, or another agent) is
//! committing concurrently. Pinning to refs keeps every finder/verifier call
//! looking at the exact same bytes.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// One file's change within the reviewed diff.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    /// Approximate number of changed lines (insertions + deletions), used to
    /// prioritize which files get reviewed first when the diff is large.
    pub churn: usize,
    pub patch: String,
}

/// The full reviewed diff: a resolved commit range plus the per-file patches,
/// already sorted by churn (largest first).
#[derive(Debug, Clone)]
pub struct DiffTarget {
    pub base_sha: String,
    pub head_sha: String,
    pub files: Vec<FileDiff>,
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to spawn git {:?}", args))?;
    if !output.status.success() {
        bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn resolve_sha(repo: &Path, rev: &str) -> Result<String> {
    Ok(run_git(repo, &["rev-parse", rev])?.trim().to_string())
}

/// Gather the diff between `base` and `head` (any git revision — a ref, a
/// SHA, `HEAD`, or a branch name). Both are resolved to SHAs up front so the
/// returned `DiffTarget` is immutable even if the refs themselves move later.
pub fn gather(repo: &Path, base: &str, head: &str, max_files: usize) -> Result<DiffTarget> {
    let base_sha = resolve_sha(repo, base)?;
    let head_sha = resolve_sha(repo, head)?;
    let range = format!("{base_sha}..{head_sha}");

    let numstat = run_git(repo, &["diff", "--numstat", &range])?;
    let mut ranked: Vec<(String, usize)> = Vec::new();
    for line in numstat.lines() {
        let mut parts = line.splitn(3, '\t');
        let ins = parts.next().unwrap_or("0");
        let del = parts.next().unwrap_or("0");
        let path = match parts.next() {
            Some(p) => p.to_string(),
            None => continue,
        };
        // Binary files report "-" for both counts; treat as zero churn so
        // they sort last rather than erroring the parse.
        let ins: usize = ins.parse().unwrap_or(0);
        let del: usize = del.parse().unwrap_or(0);
        ranked.push((path, ins + del));
    }
    ranked.sort_by_key(|a| std::cmp::Reverse(a.1));
    ranked.truncate(max_files);

    let mut files = Vec::with_capacity(ranked.len());
    for (path, churn) in ranked {
        let patch = run_git(repo, &["diff", &range, "--", &path])?;
        if patch.trim().is_empty() {
            continue;
        }
        files.push(FileDiff {
            path,
            churn,
            patch,
        });
    }

    Ok(DiffTarget {
        base_sha,
        head_sha,
        files,
    })
}

impl DiffTarget {
    /// Concatenate up to `budget_chars` worth of the highest-churn file
    /// patches into one block, suitable for a single LLM call. Files are
    /// already churn-sorted by `gather`, so truncating here keeps the most
    /// consequential hunks even when the diff is too large to send whole.
    pub fn concatenated_patch(&self, budget_chars: usize) -> String {
        let mut out = String::new();
        for file in &self.files {
            if out.len() >= budget_chars {
                out.push_str("\n[... remaining files omitted, diff exceeds review budget ...]\n");
                break;
            }
            out.push_str(&format!("\n=== {} ({} changed lines) ===\n", file.path, file.churn));
            let remaining = budget_chars.saturating_sub(out.len());
            if file.patch.len() > remaining {
                out.push_str(&file.patch[..remaining.min(file.patch.len())]);
                out.push_str("\n[... file truncated ...]\n");
            } else {
                out.push_str(&file.patch);
            }
        }
        out
    }

    pub fn file_list(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }
}
