// src/verify.rs
use anyhow::{Result, Context};
use tokio::process::Command;

pub async fn run(crate_name: &str) -> Result<()> {
    if crate_name.is_empty() {
        anyhow::bail!("crate_name cannot be empty");
    }
    let mut cmd = Command::new("cargo");
    cmd.arg("check").arg("-p").arg(crate_name).arg("--quiet");
    let output = cmd
        .output()
        .await
        .with_context(|| format!("failed to execute cargo for crate `{}`", crate_name))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo check failed for crate `{}`: {}", crate_name, stderr);
    }
    Ok(())
}
