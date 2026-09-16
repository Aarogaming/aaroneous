//! Native orchestration for Aaroneous's canonical repository verification gate.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, ExitCode};

pub fn run(root: &Path) -> Result<()> {
    run_command(
        root,
        "workspace compilation",
        "cargo",
        &["check", "--workspace", "--all-targets"],
    )?;
    crate::hardening::enforce_hardening_audit(root)?;
    run_command(root, "workspace tests", "cargo", &["test", "--workspace"])?;

    match crate::run_workspace_audit([
        root.join("core"),
        root.join("crates"),
        root.join("dev/emulator_harness"),
    ]) {
        Ok(()) => println!("PASS architectural syntax audit"),
        Err(code) => bail!("architectural syntax audit failed with {code:?}"),
    }

    let stub_pattern = r"(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)";
    let status = Command::new("git")
        .current_dir(root)
        .args([
            "grep",
            "-n",
            "-E",
            stub_pattern,
            "--",
            "crates/",
            "core/",
            "dev/",
        ])
        .status()
        .context("failed to run repository forbidden-pattern inspection")?;
    match status.code() {
        Some(1) => println!("PASS zero-stub and soundness inspection"),
        Some(code) => bail!("forbidden source patterns found or inspection failed with {code:?}"),
        None => bail!("forbidden-pattern inspection terminated without an exit status"),
    }

    run_command(
        root,
        "golden emulator harness",
        "cargo",
        &["test", "-p", "emulator_harness"],
    )
}

fn run_command(root: &Path, label: &str, program: &str, arguments: &[&str]) -> Result<()> {
    println!("=== {label} ===");
    let status = Command::new(program)
        .current_dir(root)
        .args(arguments)
        .status()
        .with_context(|| format!("failed to launch {program} for {label}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("{label} failed with {status}")
    }
}

pub fn exit_code(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => {
            println!("=== ALL REQUIRED NATIVE VERIFICATION GATES PASSED ===");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("native verification failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}
