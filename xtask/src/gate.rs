use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    println!("=== 1. Text Encoding Contract ===");
    crate::encoding::run().context("Gate 1 failed: Text Encoding Contract")?;

    println!("=== 2. Formatting ===");
    run_cmd("cargo", &["fmt", "--all", "--", "--check"]).context("Gate 2 failed: Formatting")?;

    println!("=== 3. Strict Clippy (workspace) ===");
    run_cmd("cargo", &["clippy", "--workspace", "--", "-D", "warnings"])
        .context("Gate 3 failed: Strict Clippy")?;

    println!("=== 4. Full Workspace Compilation ===");
    run_cmd("cargo", &["check", "--workspace", "--all-targets"])
        .context("Gate 4 failed: Full Workspace Compilation")?;

    println!("=== 5. Workspace Test Suite ===");
    run_cmd("cargo", &["test", "--workspace"]).context("Gate 5 failed: Workspace Test Suite")?;

    println!("=== 6. Structural & Semantic Invariant Audit ===");
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
    .context("Gate 6 failed: Structural & Semantic Invariant Audit")?;

    println!("=== 7. Zero-Stub & Soundness Inspection ===");
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
            "Gate 7 failed: Found forbidden patterns (todo!, unimplemented!, or unsafe impl Pod). Exit code 0 means matches were found."
        );
    }

    println!("=== 8. Golden Dogfooding Harness Verification ===");
    run_cmd("cargo", &["test", "-p", "emulator_harness"])
        .context("Gate 8 failed: Golden Dogfooding Harness Verification")?;

    println!("=== 9. Unwrap/Panic Ratchet Check ===");
    crate::check_unwraps::run(&[]).context("Gate 9 failed: Unwrap/Panic Ratchet Check")?;

    println!("=== 10. Release Binary Check ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "--release",
            "--bin",
            "aaroneous",
            "--bin",
            "hypervisor",
        ],
    )
    .context("Gate 10 failed: Release Binary Check")?;

    println!("=== 11. Optional Runtime Features (compile only) ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "-p",
            "hypervisor",
            "--all-targets",
            "--features",
            "llama-gguf,gpu-metrics,fleet,testing,standalone",
        ],
    )
    .context("Gate 11 failed: Optional Runtime Features")?;

    println!("=== 12. Iroh Compatibility Feature (compile only) ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "-p",
            "hypervisor",
            "--all-targets",
            "--features",
            "p2p-iroh",
        ],
    )
    .context("Gate 12 failed: Iroh Compatibility Feature")?;

    println!("=== ALL REQUIRED GATES PASSED (CI-equivalent) ===");
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    /// The `run: cargo ...` lines `.github/workflows/ci.yml`'s `check-and-test`
    /// job declares directly, outside its "Canonical repository verification"
    /// step (`bash scripts/agent_check.sh`) — that step *is* `gate::run()`, so
    /// its own internal gates (compile check, workspace test, ast_auditor
    /// audit, forbidden-pattern scan, emulator_harness) are gate.rs's own
    /// pre-existing definition, not something mirrored from literal ci.yml
    /// text. This list exists so the remaining, separately-declared CI
    /// commands staying covered by a gate is checked by a test instead of
    /// kept true by hand — see queue item M18 in the private operations
    /// workspace, opened after a deprecation-migration commit (M7) passed
    /// every existing local gate but broke CI's "Strict Clippy" step, which
    /// no local gate ran at the time.
    const GATE_COMMANDS: &[&str] = &[
        "cargo run -p xtask -- check-encoding",
        "cargo fmt --all -- --check",
        "cargo clippy --workspace -- -D warnings",
        "cargo run -p xtask -- check-unwraps",
        "cargo check --release --bin aaroneous --bin hypervisor",
        "cargo check -p hypervisor --all-targets --features llama-gguf,gpu-metrics,fleet,testing,standalone",
        "cargo check -p hypervisor --all-targets --features p2p-iroh",
    ];

    fn ci_workflow_text() -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = Path::new(manifest_dir)
            .join("..")
            .join(".github")
            .join("workflows")
            .join("ci.yml");
        fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
    }

    /// Catches gate.rs drifting stale: a command this list claims CI runs,
    /// but ci.yml no longer contains verbatim.
    #[test]
    fn gate_commands_are_present_in_ci_workflow() {
        let ci = ci_workflow_text();
        for cmd in GATE_COMMANDS {
            assert!(
                ci.contains(cmd),
                "gate.rs expects CI to run `{cmd}`, but that exact command string is no \
                 longer in ci.yml. Either ci.yml changed (update GATE_COMMANDS and \
                 gate::run() to match) or this list has a stale entry."
            );
        }
    }

    /// Catches ci.yml drifting ahead: a `cargo ...` verification command CI
    /// runs that no local gate covers yet.
    #[test]
    fn ci_workflow_verification_commands_are_all_covered_by_a_gate() {
        let ci = ci_workflow_text();
        for line in ci.lines() {
            let Some(cmd) = line.trim().strip_prefix("run: ") else {
                continue;
            };
            // Only single-line `run: cargo ...` steps are gate.rs's concern —
            // "run: bash scripts/agent_check.sh" *is* gate.rs, and the
            // multi-line `run: |` shell/PowerShell blocks are environment
            // setup, not verification commands.
            if !cmd.starts_with("cargo ") {
                continue;
            }
            assert!(
                GATE_COMMANDS.contains(&cmd),
                "ci.yml runs `{cmd}` but no gate in xtask/src/gate.rs::run() covers it — \
                 a CI check was added or changed without a matching local gate. Add it to \
                 both gate::run() and GATE_COMMANDS."
            );
        }
    }
}
