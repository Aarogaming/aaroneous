// dev/tools/afc/src/gatekeeper.rs
use crate::recipe::{DiagnosticsFilter, PipelineReport, RecipePipeline};
use anyhow::{bail, Context, Result};
use std::path::Path;
use tokio::process::Command;
use tracing::{error, info, warn};

pub struct Gatekeeper;

impl Gatekeeper {
    /// Execute the full verification gate pipeline via RecipePipeline
    pub async fn run_verification_pipeline(
        repo_path: &Path,
        enforce_clippy: bool,
        enforce_test: bool,
        enforce_fmt: bool,
    ) -> Result<PipelineReport> {
        let pipeline = RecipePipeline::verification_gates(
            repo_path,
            enforce_clippy,
            enforce_test,
            enforce_fmt,
        );
        pipeline.run().await
    }

    pub async fn check_workspace(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Running cargo check --workspace on target repo");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(repo_path);
        cmd.args(["check", "--workspace"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd.output().await.context("Failed to run cargo check")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let filtered = DiagnosticsFilter::summarize_for_prompt(&stderr, 10);
            bail!("Cargo check failed:\n{filtered}");
        }

        Ok(())
    }

    pub async fn test_workspace(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Running cargo test --workspace on target repo");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(repo_path);
        cmd.args(["test", "--workspace"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd.output().await.context("Failed to run cargo test")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let filtered = DiagnosticsFilter::summarize_for_prompt(&stderr, 10);
            bail!("Cargo test failed:\n{filtered}");
        }

        Ok(())
    }

    pub async fn format_workspace(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Running cargo fmt --all on target repo");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(repo_path);
        cmd.args(["fmt", "--all"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd.output().await.context("Failed to run cargo fmt")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Cargo fmt completed with warnings:\n{stderr}");
        }

        Ok(())
    }

    pub async fn inspect_clippy(repo_path: &Path) -> Result<bool> {
        info!("Gatekeeper: Running cargo clippy --workspace --no-deps on target repo");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(repo_path);
        cmd.args(["clippy", "--workspace", "--no-deps"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd.output().await.context("Failed to run cargo clippy")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let filtered = DiagnosticsFilter::summarize_for_prompt(&stderr, 5);
            warn!("Clippy flagged warnings or issues:\n{filtered}");
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub async fn audit_security(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Checking supply chain security via cargo audit");
        let mut check_cmd = Command::new("cargo");
        check_cmd.current_dir(repo_path);
        check_cmd.args(["audit", "--version"]);
        #[cfg(windows)]
        check_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let check_res = check_cmd.output().await;

        if !check_res.as_ref().is_ok_and(|o| o.status.success()) {
            info!("cargo-audit not installed. Attempting installation...");
            let mut install_cmd = Command::new("cargo");
            install_cmd.args(["install", "cargo-audit"]);
            #[cfg(windows)]
            install_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

            let install_out = install_cmd.output().await;
            if let Err(e) = install_out {
                warn!("Could not install cargo-audit: {e}. Skipping security gate.");
                return Ok(());
            }
        }

        let mut audit_cmd = Command::new("cargo");
        audit_cmd.current_dir(repo_path);
        audit_cmd.arg("audit");
        #[cfg(windows)]
        audit_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = audit_cmd
            .output()
            .await
            .context("Failed to execute cargo audit")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Supply chain security vulnerabilities flagged:\n{stderr}");
        } else {
            info!("Supply chain security audit passed cleanly.");
        }

        Ok(())
    }

    pub async fn build_release(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Compiling release binaries via cargo build --release -p a_run");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(repo_path);
        cmd.args(["build", "--release", "-p", "a_run"]);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = cmd
            .output()
            .await
            .context("Failed to build release binaries")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let filtered = DiagnosticsFilter::summarize_for_prompt(&stderr, 10);
            error!("Release build failed:\n{filtered}");
            bail!("Cargo release build failed:\n{filtered}");
        }

        Ok(())
    }

    /// Execute comprehensive systems health pipeline via RecipePipeline
    pub async fn run_systems_health_pipeline(repo_path: &Path) -> Result<PipelineReport> {
        let pipeline = RecipePipeline::systems_health_pipeline(repo_path);
        pipeline.run().await
    }
}
