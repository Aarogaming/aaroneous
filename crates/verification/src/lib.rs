// src/lib.rs
use anyhow::{Result, Context};
use std::path::Path;
use std::process::Command;

/// Result of verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub passed: bool,
    pub details: String,
}

/// Run `cargo check` on the given crate path and return a VerificationResult.
pub fn verify_crate(crate_path: &Path) -> Result<VerificationResult> {
    anyhow::ensure!(crate_path.exists(), "crate path does not exist: {}", crate_path.display());
    let output = Command::new("cargo")
        .args(&["check", "--manifest-path", &format!("{}/Cargo.toml", crate_path.display())])
        .output()
        .context("failed to execute cargo check")?;
    let passed = output.status.success();
    let details = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(VerificationResult { passed, details })
}
