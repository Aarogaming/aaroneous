// crates/flight_controller/src/gatekeeper.rs
use anyhow::{bail, Context, Result};
use std::path::Path;
use tokio::process::Command;
use tracing::{error, info, warn};

pub struct Gatekeeper;

impl Gatekeeper {
    /// Verify workspace compilation (`cargo check --workspace`).
    pub async fn check_workspace(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Running cargo check --workspace");
        let output = Command::new("cargo")
            .current_dir(repo_path)
            .args(["check", "--workspace"])
            .output()
            .await
            .context("Failed to run cargo check")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo check failed:\n{stderr}");
        }

        Ok(())
    }

    /// Execute workspace unit and integration test suite (`cargo test --workspace`).
    pub async fn test_workspace(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Running cargo test --workspace");
        let output = Command::new("cargo")
            .current_dir(repo_path)
            .args(["test", "--workspace"])
            .output()
            .await
            .context("Failed to run cargo test")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo test failed:\n{stderr}");
        }

        Ok(())
    }

    /// Enforce workspace code formatting (`cargo fmt --all`).
    pub async fn format_workspace(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Running cargo fmt --all");
        let output = Command::new("cargo")
            .current_dir(repo_path)
            .args(["fmt", "--all"])
            .output()
            .await
            .context("Failed to run cargo fmt")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Cargo fmt completed with warnings:\n{stderr}");
        }

        Ok(())
    }

    /// Inspect code quality using Clippy (`cargo clippy --workspace --no-deps`).
    pub async fn inspect_clippy(repo_path: &Path) -> Result<bool> {
        info!("Gatekeeper: Running cargo clippy --workspace --no-deps");
        let output = Command::new("cargo")
            .current_dir(repo_path)
            .args(["clippy", "--workspace", "--no-deps"])
            .output()
            .await
            .context("Failed to run cargo clippy")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Clippy flagged warnings or issues:\n{stderr}");
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Check dependencies for security vulnerabilities (`cargo audit`).
    pub async fn audit_security(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Checking supply chain security via cargo audit");
        let check_cmd = Command::new("cargo")
            .current_dir(repo_path)
            .args(["audit", "--version"])
            .output()
            .await;

        if !check_cmd.as_ref().is_ok_and(|o| o.status.success()) {
            info!("cargo-audit not installed. Attempting installation...");
            let install_out = Command::new("cargo")
                .args(["install", "cargo-audit"])
                .output()
                .await;

            if let Err(e) = install_out {
                warn!("Could not install cargo-audit: {e}. Skipping security gate.");
                return Ok(());
            }
        }

        let output = Command::new("cargo")
            .current_dir(repo_path)
            .arg("audit")
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

    /// Build release binaries for deployment packaging.
    pub async fn build_release(repo_path: &Path) -> Result<()> {
        info!("Gatekeeper: Compiling release binaries via cargo build --release -p a_run");
        let output = Command::new("cargo")
            .current_dir(repo_path)
            .args(["build", "--release", "-p", "a_run"])
            .output()
            .await
            .context("Failed to build release binaries")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Release build failed:\n{stderr}");
            bail!("Cargo release build failed:\n{stderr}");
        }

        Ok(())
    }
}
