use anyhow::{Context, Result, bail};
use std::process::Command;

/// Single source of truth for gate 3.6's cargo-deny invocation, read by both
/// `run()` and its drift test. `--all-features` is the one of these three
/// arguments that also names a `security-audit.yml` `arguments:` value
/// directly; `deny`/`check` are the subcommand, not something the workflow
/// spells out.
const GATE_36_DENY_ARGS: &[&str] = &["deny", "--all-features", "check"];

/// Gate 3.8's semver-checked package list — see `GATE_36_DENY_ARGS`.
const GATE_38_SEMVER_PACKAGES: &[&str] = &["core-contracts", "sdk"];

pub fn run() -> Result<()> {
    println!("=== 1. Text Encoding Contract ===");
    crate::encoding::run().context("Gate 1 failed: Text Encoding Contract")?;

    println!("=== 2. Formatting ===");
    run_cmd("cargo", &["fmt", "--all", "--", "--check"]).context("Gate 2 failed: Formatting")?;

    println!("=== 3. Strict Clippy (workspace, all targets) ===");
    run_cmd(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )
    .context("Gate 3 failed: Strict Clippy")?;

    println!("=== 3.5. Native Dependency Boundary Audit ===");
    crate::native_audit::run().context("Gate 3.5 failed: Native Dependency Boundary")?;

    println!("=== 3.6. Dependency & License Policy (cargo-deny) ===");
    require_tool(
        "cargo-deny",
        "cargo install cargo-deny --locked",
        "https://github.com/EmbarkStudios/cargo-deny",
    )?;
    run_cmd("cargo", GATE_36_DENY_ARGS)
        .context("Gate 3.6 failed: Dependency & License Policy (cargo-deny)")?;

    println!("=== 3.7. RUSTSEC Advisory Audit (cargo-audit) ===");
    require_tool(
        "cargo-audit",
        "cargo install cargo-audit --locked",
        "https://github.com/rustsec/rustsec",
    )?;
    run_cmd("cargo", &["audit"])
        .context("Gate 3.7 failed: RUSTSEC Advisory Audit (cargo-audit)")?;

    println!("=== 3.8. Semver Compatibility (core-contracts, sdk) ===");
    require_tool(
        "cargo-semver-checks",
        "cargo install cargo-semver-checks --locked",
        "https://github.com/obi1kenobi/cargo-semver-checks",
    )?;
    let baseline = semver_baseline_rev();
    for pkg in GATE_38_SEMVER_PACKAGES {
        run_cmd(
            "cargo",
            &[
                "semver-checks",
                "check-release",
                "-p",
                pkg,
                "--baseline-rev",
                &baseline,
            ],
        )
        .with_context(|| format!("Gate 3.8 failed: Semver Compatibility ({pkg})"))?;
    }

    println!("=== 4. Full Workspace Compilation ===");
    run_cmd("cargo", &["check", "--workspace", "--all-targets"])
        .context("Gate 4 failed: Full Workspace Compilation")?;

    println!("=== 4a. Portable Core Profile ===");
    run_cmd(
        "cargo",
        &["check", "-p", "scan_core", "--no-default-features"],
    )
    .context("Gate 4a failed: Portable Core Profile")?;

    println!("=== 4b. ARM Bare-Metal Core Profile ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "-p",
            "scan_core",
            "--target",
            "thumbv7em-none-eabihf",
            "--no-default-features",
        ],
    )
    .context("Gate 4b failed: ARM Bare-Metal Core Profile")?;

    println!("=== 4c. WebAssembly Core Profile ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "-p",
            "scan_core",
            "--target",
            "wasm32-unknown-unknown",
            "--no-default-features",
        ],
    )
    .context("Gate 4c failed: WebAssembly Core Profile")?;

    println!("=== 4d. Portable Shared Contracts ===");
    run_cmd(
        "cargo",
        &["check", "-p", "core-contracts", "--no-default-features"],
    )
    .context("Gate 4d failed: Portable Shared Contracts")?;

    println!("=== 4e. ARM Bare-Metal Shared Contracts ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "-p",
            "core-contracts",
            "--target",
            "thumbv7em-none-eabihf",
            "--no-default-features",
        ],
    )
    .context("Gate 4e failed: ARM Bare-Metal Shared Contracts")?;

    println!("=== 4f. WebAssembly Shared Contracts ===");
    run_cmd(
        "cargo",
        &[
            "check",
            "-p",
            "core-contracts",
            "--target",
            "wasm32-unknown-unknown",
            "--no-default-features",
        ],
    )
    .context("Gate 4f failed: WebAssembly Shared Contracts")?;

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

    println!("=== 9.5. Profile Dependency Direction Check ===");
    crate::check_deps::run().context("Gate 9.5 failed: Profile Dependency Direction Check")?;

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

/// Fails with install instructions if `binary` isn't on `PATH`, rather than
/// silently skipping the gate or paying an unconditional multi-minute
/// `cargo install` on every run. These three tools (cargo-deny, cargo-audit,
/// cargo-semver-checks) are one-time local setup, same as clippy/rustfmt
/// components are assumed present already.
fn require_tool(binary: &str, install_cmd: &str, project_url: &str) -> Result<()> {
    match Command::new(binary).arg("--version").status() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!(
                "`{binary}` is not installed or not on PATH. Install it once with:\n    {install_cmd}\nSee: {project_url}"
            )
        }
        Err(e) => Err(e).with_context(|| format!("Failed to probe for `{binary}`")),
    }
}

/// Baseline git revision for `cargo-semver-checks`, matching what
/// `.github/workflows/semver-checks.yml` resolves on a pull-request event
/// (diff against the PR's base branch). A local run assumes the standard
/// `origin/main` remote/branch naming; fetch `main` first if this doesn't
/// resolve.
fn semver_baseline_rev() -> String {
    "origin/main".to_string()
}

#[cfg(test)]
mod tests {
    use super::{GATE_36_DENY_ARGS, GATE_38_SEMVER_PACKAGES, require_tool};
    use std::fs;
    use std::path::Path;

    /// A binary that cannot plausibly exist on `PATH` must fail with
    /// install instructions, not a generic spawn error — this is the error
    /// path a contributor missing one of the three security/semver tools
    /// actually hits.
    #[test]
    fn require_tool_reports_missing_binary_with_install_instructions() {
        let err = require_tool(
            "definitely-not-a-real-binary-xyz123",
            "cargo install the-thing",
            "https://example.invalid",
        )
        .expect_err("a nonexistent binary must fail require_tool");
        let message = format!("{err:#}");
        assert!(message.contains("cargo install the-thing"));
        assert!(message.contains("https://example.invalid"));
    }

    /// A binary that does exist (here, `cargo` itself, always present under
    /// the test harness) must pass through without error.
    #[test]
    fn require_tool_accepts_present_binary() {
        require_tool("cargo", "unused", "unused").expect("cargo must be found on PATH");
    }

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
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo check -p scan_core --no-default-features",
        "cargo check -p scan_core --target thumbv7em-none-eabihf --no-default-features",
        "cargo check -p scan_core --target wasm32-unknown-unknown --no-default-features",
        "cargo check -p core-contracts --no-default-features",
        "cargo check -p core-contracts --target thumbv7em-none-eabihf --no-default-features",
        "cargo check -p core-contracts --target wasm32-unknown-unknown --no-default-features",
        "cargo run -p xtask -- check-unwraps",
        "cargo run -p xtask -- check-deps",
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

    /// `security-audit.yml` and `semver-checks.yml` run cargo-deny,
    /// cargo-audit and cargo-semver-checks via dedicated GitHub Actions
    /// (`EmbarkStudios/cargo-deny-action`, `rustsec/audit-check`,
    /// `taiki-e/install-action@cargo-semver-checks` + `cargo semver-checks
    /// check-release`), not a literal `run: cargo ...` line — so they can't
    /// be checked by the `GATE_COMMANDS`/ci.yml machinery above.
    ///
    /// This is their drift guard, and it checks both directions:
    /// `assert_eq!` first pins `GATE_36_DENY_ARGS`/`GATE_38_SEMVER_PACKAGES`
    /// to the exact values this test also checks the workflow files
    /// against, so changing either constant in `run()` (e.g. dropping
    /// `--all-features`, or removing `sdk` from the semver-checked package
    /// list) fails this test immediately, forcing a deliberate update here
    /// alongside confirming the workflow file still matches — rather than
    /// deriving the workflow-file assertions from the constants directly
    /// (e.g. `.filter(|a| a.starts_with("--"))`), which would vacuously
    /// pass if the constant lost the flag entirely instead of catching it.
    /// An earlier version of this test held its own separately hand-typed
    /// literals with no link back to `run()`'s real arguments at all, which
    /// caught the workflow changing out from under the gate but not the
    /// reverse — a review caught this: see the exchange on
    /// Aarogaming/aaroneous#89, 2026-10-03. The action names
    /// (`EmbarkStudios/cargo-deny-action`, `rustsec/audit-check`) have no
    /// equivalent in `run()` at all, since `run()` calls the CLIs directly
    /// rather than through an Action, so those two stay literal checks of
    /// the workflow's own identity.
    #[test]
    fn security_and_semver_workflows_match_their_local_gates() {
        assert_eq!(
            GATE_36_DENY_ARGS,
            &["deny", "--all-features", "check"],
            "GATE_36_DENY_ARGS changed in gate.rs::run() — update this test's expected value \
             AND confirm security-audit.yml's cargo-deny-action arguments still match."
        );
        assert_eq!(
            GATE_38_SEMVER_PACKAGES,
            &["core-contracts", "sdk"],
            "GATE_38_SEMVER_PACKAGES changed in gate.rs::run() — update this test's expected \
             value AND confirm semver-checks.yml still checks the same package set."
        );

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workflows = Path::new(manifest_dir)
            .join("..")
            .join(".github")
            .join("workflows");

        let security_audit = fs::read_to_string(workflows.join("security-audit.yml"))
            .expect("failed to read security-audit.yml");
        assert!(
            security_audit.contains("EmbarkStudios/cargo-deny-action"),
            "security-audit.yml no longer uses EmbarkStudios/cargo-deny-action — gate 3.6 \
             (`cargo deny check`) in gate.rs::run() may no longer match what CI runs."
        );
        assert!(
            security_audit.contains("--all-features"),
            "security-audit.yml's cargo-deny arguments changed — update GATE_36_DENY_ARGS in \
             gate.rs to pass the same arguments."
        );
        assert!(
            security_audit.contains("rustsec/audit-check"),
            "security-audit.yml no longer uses rustsec/audit-check — gate 3.7 (`cargo audit`) \
             in gate.rs::run() may no longer match what CI runs."
        );

        let semver_checks = fs::read_to_string(workflows.join("semver-checks.yml"))
            .expect("failed to read semver-checks.yml");
        for pkg in GATE_38_SEMVER_PACKAGES {
            let needle = format!("-p {pkg}");
            assert!(
                semver_checks.contains(&needle),
                "semver-checks.yml no longer checks `{pkg}` — update GATE_38_SEMVER_PACKAGES \
                 in gate.rs or this workflow so they match."
            );
        }
    }
}
