//! crates/adaptation_engine/src/self_rebuild.rs
//! Autonomous Ouroboros Self-Rebuild & Binary Reload Engine
//! Enables Aaroneous to compile, verify, and reload its own crates and binaries on demand.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Report produced after self-compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildReport {
    pub target_crate: String,
    pub is_success: bool,
    pub duration_ms: u64,
    pub stdout_summary: String,
    pub binary_path: Option<PathBuf>,
}

/// Autonomous Self-Rebuilder Pipeline
pub struct SelfRebuildEngine {
    workspace_root: PathBuf,
}

impl Default for SelfRebuildEngine {
    fn default() -> Self {
        Self {
            workspace_root: aaroneous_paths::WorkspacePaths::discover()
                .root()
                .to_path_buf(),
        }
    }
}

impl SelfRebuildEngine {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Runs `cargo check` across a specific crate
    pub fn check_crate(&self, crate_name: &str) -> Result<RebuildReport> {
        let start = Instant::now();
        let output = Command::new("cargo")
            .current_dir(&self.workspace_root)
            .args(["check", "-p", crate_name])
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let is_success = output.status.success();
        let stdout_summary = String::from_utf8_lossy(if is_success { &output.stdout } else { &output.stderr }).to_string();

        Ok(RebuildReport {
            target_crate: crate_name.to_string(),
            is_success,
            duration_ms,
            stdout_summary,
            binary_path: None,
        })
    }

    /// Compiles a target binary (e.g. `a_hud` or `a_run`)
    pub fn build_binary(&self, pkg: &str, bin_name: &str, release: bool) -> Result<RebuildReport> {
        let start = Instant::now();
        let mut args = vec!["build", "-p", pkg, "--bin", bin_name];
        if release {
            args.push("--release");
        }

        let output = Command::new("cargo")
            .current_dir(&self.workspace_root)
            .args(&args)
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let is_success = output.status.success();
        let stdout_summary = String::from_utf8_lossy(if is_success { &output.stdout } else { &output.stderr }).to_string();

        let profile = if release { "release" } else { "debug" };
        let mut bin_path = self.workspace_root.join("target").join(profile).join(bin_name);
        #[cfg(target_os = "windows")]
        {
            bin_path.set_extension("exe");
        }

        let binary_path = if is_success && bin_path.exists() {
            Some(bin_path)
        } else {
            None
        };

        if is_success {
            Ok(RebuildReport {
                target_crate: format!("{}:{}", pkg, bin_name),
                is_success: true,
                duration_ms,
                stdout_summary,
                binary_path,
            })
        } else {
            Err(anyhow!("Compilation failed: {}", stdout_summary))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_rebuild_engine_initialization() {
        let engine = SelfRebuildEngine::default();
        assert!(engine.workspace_root.exists());
    }
}
