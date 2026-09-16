use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    println!("=== Running native text encoding audit via ast_auditor ===");
    let status = Command::new("cargo")
        .args(["run", "-p", "ast_auditor", "--", "check-encoding"])
        .status()
        .context("Failed to spawn cargo run -p ast_auditor -- check-encoding")?;

    if !status.success() {
        bail!("Native text encoding audit failed with status: {}", status);
    }

    Ok(())
}
