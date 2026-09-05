// crates/flight_controller/src/git.rs
use anyhow::{bail, Context, Result};
use std::path::Path;
use tokio::process::Command;
use tracing::{info, warn};

pub struct GitEngine;

impl GitEngine {
    /// Retrieve the current active git branch name.
    pub async fn current_branch(repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(["branch", "--show-current"])
            .output()
            .await
            .context("Failed to execute git branch command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git branch --show-current failed: {stderr}");
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(branch)
    }

    /// Create and switch to a new git branch.
    pub async fn checkout_new_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
        info!("Branch safety: checking out new branch '{branch_name}'");
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(["checkout", "-b", branch_name])
            .output()
            .await
            .context("Failed to execute git checkout -b")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to switch branch: {stderr}");
        }

        Ok(())
    }

    /// Check if the working tree has unstaged or staged changes.
    pub async fn is_dirty(repo_path: &Path) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(["status", "--porcelain"])
            .output()
            .await
            .context("Failed to query git status")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git status query failed: {stderr}");
        }

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(!status.is_empty())
    }

    /// Roll back all unstaged working tree changes to preserve repository integrity.
    pub async fn rollback_working_tree(repo_path: &Path) -> Result<()> {
        warn!("Executing git rollback to restore clean working tree");
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(["checkout", "--", "."])
            .output()
            .await
            .context("Failed to execute git checkout -- .")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git rollback failed: {stderr}");
        }

        Ok(())
    }

    /// Stage all changes and create an atomic commit.
    pub async fn atomic_commit(repo_path: &Path, message: &str) -> Result<()> {
        let add_out = Command::new("git")
            .current_dir(repo_path)
            .args(["add", "-A"])
            .output()
            .await
            .context("Failed to stage git changes")?;

        if !add_out.status.success() {
            let stderr = String::from_utf8_lossy(&add_out.stderr);
            bail!("git add -A failed: {stderr}");
        }

        let commit_out = Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", message])
            .output()
            .await
            .context("Failed to commit git changes")?;

        if !commit_out.status.success() {
            let stderr = String::from_utf8_lossy(&commit_out.stderr);
            bail!("git commit failed: {stderr}");
        }

        info!("Committed atomic progress cleanly: '{message}'");
        Ok(())
    }
}
