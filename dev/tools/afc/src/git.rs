// dev/tools/afc/src/git.rs
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct GitEngine;

impl GitEngine {
    /// Resolve the actual `.git` directory for a repository.
    /// Handles regular repositories (`.git/` dir) and worktrees/submodules (`.git` file with `gitdir:`).
    pub fn resolve_git_dir(repo_path: &Path) -> Result<PathBuf> {
        let git_target = repo_path.join(".git");
        if git_target.is_dir() {
            return Ok(git_target);
        }
        if git_target.is_file() {
            let content = fs::read_to_string(&git_target)
                .with_context(|| format!("Failed to read git link file at {:?}", git_target))?;
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("gitdir:") {
                    let gitdir_path = PathBuf::from(rest.trim());
                    return if gitdir_path.is_absolute() {
                        Ok(gitdir_path)
                    } else {
                        Ok(repo_path.join(gitdir_path))
                    };
                }
            }
        }
        bail!(
            "Directory {:?} is not a valid git repository (.git missing)",
            repo_path
        )
    }

    /// Pure-Rust in-process inspection of current HEAD branch name without spawning `git.exe`.
    /// Zero latency, works in airgapped environments, and does not require process creation.
    pub fn fast_current_branch(repo_path: &Path) -> Result<String> {
        let git_dir = Self::resolve_git_dir(repo_path)?;
        let head_path = git_dir.join("HEAD");
        let content = fs::read_to_string(&head_path)
            .with_context(|| format!("Failed to read HEAD file at {:?}", head_path))?;
        let trimmed = content.trim();

        if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
            Ok(branch_ref.trim().to_string())
        } else if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            let short = if trimmed.len() >= 7 {
                &trimmed[..7]
            } else {
                trimmed
            };
            Ok(format!("HEAD (detached at {short})"))
        } else {
            bail!("Unrecognized HEAD content: '{trimmed}'")
        }
    }

    /// Pure-Rust in-process lookup of the current commit SHA-1.
    pub fn fast_head_commit(repo_path: &Path) -> Result<String> {
        let git_dir = Self::resolve_git_dir(repo_path)?;
        let head_path = git_dir.join("HEAD");
        let head_content = fs::read_to_string(&head_path)
            .with_context(|| format!("Failed to read HEAD file at {:?}", head_path))?;
        let trimmed = head_content.trim();

        if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(trimmed.to_string());
        }

        if let Some(ref_path_rel) = trimmed.strip_prefix("ref: ") {
            let loose_ref = git_dir.join(ref_path_rel.trim());
            if loose_ref.is_file() {
                let sha = fs::read_to_string(&loose_ref)
                    .with_context(|| format!("Failed to read ref at {:?}", loose_ref))?;
                let sha_trimmed = sha.trim();
                if sha_trimmed.len() == 40 {
                    return Ok(sha_trimmed.to_string());
                }
            }

            // Check packed-refs if loose ref does not exist
            let packed_refs = git_dir.join("packed-refs");
            if packed_refs.is_file() {
                let packed = fs::read_to_string(&packed_refs)
                    .with_context(|| format!("Failed to read packed-refs at {:?}", packed_refs))?;
                for line in packed.lines() {
                    let l = line.trim();
                    if l.starts_with('#') || l.starts_with('^') {
                        continue;
                    }
                    let parts: Vec<&str> = l.split_whitespace().collect();
                    if parts.len() == 2 && parts[1] == ref_path_rel.trim() {
                        return Ok(parts[0].to_string());
                    }
                }
            }
        }

        bail!("Could not resolve commit SHA for HEAD at {:?}", repo_path)
    }

    /// Get current branch, trying fast in-process inspection first, falling back to windowless git process.
    pub async fn current_branch(repo_path: &Path) -> Result<String> {
        if let Ok(branch) = Self::fast_current_branch(repo_path) {
            debug!("Resolved branch '{branch}' via in-process Git inspection");
            return Ok(branch);
        }

        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.args(["branch", "--show-current"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd
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

    /// Get current commit SHA, trying fast in-process inspection first.
    pub async fn head_commit_sha(repo_path: &Path) -> Result<String> {
        if let Ok(sha) = Self::fast_head_commit(repo_path) {
            debug!("Resolved commit SHA '{sha}' via in-process Git inspection");
            return Ok(sha);
        }

        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.args(["rev-parse", "HEAD"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd
            .output()
            .await
            .context("Failed to execute git rev-parse HEAD")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git rev-parse HEAD failed: {stderr}");
        }

        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(sha)
    }

    pub async fn checkout_new_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
        info!("Branch safety: checking out new branch '{branch_name}'");
        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.args(["checkout", "-b", branch_name]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd
            .output()
            .await
            .context("Failed to execute git checkout -b")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to switch branch: {stderr}");
        }

        Ok(())
    }

    pub async fn is_dirty(repo_path: &Path) -> Result<bool> {
        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.args(["status", "--porcelain"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd.output().await.context("Failed to query git status")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git status query failed: {stderr}");
        }

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(!status.is_empty())
    }

    pub async fn rollback_working_tree(repo_path: &Path) -> Result<()> {
        warn!("Executing git rollback to restore clean working tree");
        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.args(["checkout", "--", "."]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd
            .output()
            .await
            .context("Failed to execute git checkout -- .")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git rollback failed: {stderr}");
        }

        Ok(())
    }

    pub async fn atomic_commit(repo_path: &Path, message: &str) -> Result<()> {
        let mut add_cmd = Command::new("git");
        add_cmd.current_dir(repo_path);
        add_cmd.args(["add", "-A"]);
        #[cfg(windows)]
        add_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let add_out = add_cmd
            .output()
            .await
            .context("Failed to stage git changes")?;

        if !add_out.status.success() {
            let stderr = String::from_utf8_lossy(&add_out.stderr);
            bail!("git add -A failed: {stderr}");
        }

        let mut commit_cmd = Command::new("git");
        commit_cmd.current_dir(repo_path);
        commit_cmd.args(["commit", "-m", message]);
        #[cfg(windows)]
        commit_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let commit_out = commit_cmd
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_git_inspection() {
        let repo_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap_or_else(|| Path::new("."));

        if let Ok(branch) = GitEngine::fast_current_branch(repo_path) {
            assert!(!branch.is_empty());
        }

        if let Ok(sha) = GitEngine::fast_head_commit(repo_path) {
            assert_eq!(sha.len(), 40);
        }
    }
}
