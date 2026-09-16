use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    println!("=== 1. Full Workspace Compilation ===");
    run_cmd("cargo", &["check", "--workspace", "--all-targets"])
        .context("Gate 1 failed: Full Workspace Compilation")?;

    println!("=== 2. Workspace Test Suite ===");
    run_cmd("cargo", &["test", "--workspace"]).context("Gate 2 failed: Workspace Test Suite")?;

    println!("=== 3. Structural & Semantic Invariant Audit ===");
    run_cmd(
        "cargo",
        &[
            "run",
            "-p",
            "ast_auditor",
            "--",
            "audit",
            "core/",
            "crates/",
            "dev/emulator_harness/",
        ],
    )
    .context("Gate 3 failed: Structural & Semantic Invariant Audit")?;

    println!("=== 4. Zero-Stub & Soundness Inspection ===");
    let status = Command::new("git")
        .args([
            "grep",
            "-n",
            "-E",
            "(\\btodo!\\(|\\bunimplemented!\\(|unsafe impl.*Pod)",
            "--",
            "crates/",
            "core/",
            "dev/",
        ])
        .status()
        .context("Failed to execute git grep")?;

    if status.success() {
        bail!(
            "Gate 4 failed: Found forbidden patterns (todo!, unimplemented!, or unsafe impl Pod). Exit code 0 means matches were found."
        );
    }

    println!("=== 5. Golden Dogfooding Harness Verification ===");
    run_cmd("cargo", &["test", "-p", "emulator_harness"])
        .context("Gate 5 failed: Golden Dogfooding Harness Verification")?;

    println!("=== ALL REQUIRED GATES PASSED ===");
    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to spawn {}", program))?;

    if !status.success() {
        bail!(
            "Command '{} {}' failed with status: {}",
            program,
            args.join(" "),
            status
        );
    }
    Ok(())
}
