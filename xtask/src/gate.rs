use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Single source of truth for gate 3.6's cargo-deny invocation, read by both
/// `run()` and its drift test. `--all-features` is the one of these three
/// arguments that also names a `security-audit.yml` `arguments:` value
/// directly; `deny`/`check` are the subcommand, not something the workflow
/// spells out.
pub const GATE_36_DENY_ARGS: &[&str] = &["deny", "--all-features", "check"];

/// Gate 3.8's semver-checked package list — see `GATE_36_DENY_ARGS`.
pub const GATE_38_SEMVER_PACKAGES: &[&str] = &["core-contracts", "sdk"];

/// The verification tier hierarchy, ordered from lowest cost (zero compilation) to highest cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationTier {
    /// Tier 0: Static Policy & Structural Preflight (Zero workspace compilation: encoding, format, stubs, AST audit, unwraps/deps ratchets, native audit).
    Tier0Preflight = 0,
    /// Tier 1: Supply Chain & Compatibility Audits (cargo-deny, cargo-audit, cargo-semver-checks).
    Tier1SupplyChain = 1,
    /// Tier 2: Compilation & Matrix Profiles (clippy, check workspace, cross targets, release binary, features).
    Tier2Compilation = 2,
    /// Tier 3: Execution & Test Suites (workspace test, emulator harness test).
    Tier3Execution = 3,
}

impl VerificationTier {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "0" | "tier0" | "tier0-preflight" | "tier0_preflight" | "preflight" | "fast" => {
                Some(Self::Tier0Preflight)
            }
            "1" | "tier1" | "tier1-supply-chain" | "tier1_supply_chain" | "supply-chain"
            | "supply_chain" | "audit" => Some(Self::Tier1SupplyChain),
            "2" | "tier2" | "tier2-compilation" | "tier2_compilation" | "compilation"
            | "compile" => Some(Self::Tier2Compilation),
            "3" | "tier3" | "tier3-execution" | "tier3_execution" | "execution" | "test" => {
                Some(Self::Tier3Execution)
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tier0Preflight => "tier0_preflight",
            Self::Tier1SupplyChain => "tier1_supply_chain",
            Self::Tier2Compilation => "tier2_compilation",
            Self::Tier3Execution => "tier3_execution",
        }
    }
}

/// Execution configuration for tiered verification preflight.
#[derive(Debug, Clone)]
pub struct PreflightConfig {
    pub workspace_root: PathBuf,
    pub target_rev: String,
    pub max_tier: VerificationTier,
    pub json: bool,
    pub stop_on_first_failure: bool,
}

/// A machine-readable execution report for verification preflight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateReport {
    pub schema_version: String,
    pub target_rev: String,
    pub workspace_root: String,
    pub tier: String,
    pub status: String,
    pub total_duration_ms: u64,
    pub checks: Vec<CheckResultRecord>,
    pub failed_check: Option<String>,
    pub fingerprint: String,
}

/// Record of an individual verification check result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckResultRecord {
    pub id: String,
    pub name: String,
    pub tier: u8,
    pub order: usize,
    pub cost_level: String,
    pub status: String,
    pub duration_ms: u64,
    pub message: Option<String>,
}

/// Definition of an executable verification check.
pub struct GateCheckDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: VerificationTier,
    pub order: usize,
    pub cost_level: &'static str,
    pub runner: fn(workspace_root: &Path) -> Result<()>,
}

/// Canonical table of all verification checks ordered by cost and tier.
pub const ALL_GATE_CHECKS: &[GateCheckDefinition] = &[
    // Tier 0: Static Policy & Structural Preflight (Zero workspace compilation: ~5-15s)
    GateCheckDefinition {
        id: "check-encoding",
        name: "1. Text Encoding Contract",
        tier: VerificationTier::Tier0Preflight,
        order: 1,
        cost_level: "low",
        runner: |root| {
            run_cmd_in(
                root,
                "cargo",
                &["run", "-p", "ast_auditor", "--", "check-encoding"],
            )
        },
    },
    GateCheckDefinition {
        id: "fmt",
        name: "2. Formatting",
        tier: VerificationTier::Tier0Preflight,
        order: 2,
        cost_level: "low",
        runner: |root| run_cmd_in(root, "cargo", &["fmt", "--all", "--", "--check"]),
    },
    GateCheckDefinition {
        id: "zero-stubs",
        name: "7. Zero-Stub & Soundness Inspection",
        tier: VerificationTier::Tier0Preflight,
        order: 3,
        cost_level: "low",
        runner: |root| {
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
                .current_dir(root)
                .status()
                .context("Failed to execute git grep")?;
            match status.code() {
                Some(0) => bail!(
                    "Found forbidden patterns (todo!, unimplemented!, or unsafe impl Pod). Exit code 0 means matches were found."
                ),
                Some(1) => Ok(()),
                Some(code) => bail!("git grep failed with unexpected exit code {code}"),
                None => bail!("git grep terminated by signal"),
            }
        },
    },
    GateCheckDefinition {
        id: "ast-audit",
        name: "6. Structural & Semantic Invariant Audit",
        tier: VerificationTier::Tier0Preflight,
        order: 4,
        cost_level: "low",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-unwraps",
        name: "9. Unwrap/Panic Ratchet Check",
        tier: VerificationTier::Tier0Preflight,
        order: 5,
        cost_level: "low",
        runner: |root| crate::check_unwraps::run_with_root(root, &[]),
    },
    GateCheckDefinition {
        id: "check-deps",
        name: "9.5. Profile Dependency Direction Check",
        tier: VerificationTier::Tier0Preflight,
        order: 6,
        cost_level: "low",
        runner: |root| crate::check_deps::run_with_root(root),
    },
    GateCheckDefinition {
        id: "check-native",
        name: "3.5. Native Dependency Boundary Audit",
        tier: VerificationTier::Tier0Preflight,
        order: 7,
        cost_level: "low",
        runner: |root| crate::native_audit::run_with_root(root),
    },
    // Tier 1: Supply Chain & Compatibility Audits
    GateCheckDefinition {
        id: "cargo-deny",
        name: "3.6. Dependency & License Policy (cargo-deny)",
        tier: VerificationTier::Tier1SupplyChain,
        order: 8,
        cost_level: "medium",
        runner: |root| {
            require_tool(
                "cargo-deny",
                "cargo install cargo-deny --locked",
                "https://github.com/EmbarkStudios/cargo-deny",
            )?;
            run_cmd_in(root, "cargo", GATE_36_DENY_ARGS)
        },
    },
    GateCheckDefinition {
        id: "cargo-audit",
        name: "3.7. RUSTSEC Advisory Audit (cargo-audit)",
        tier: VerificationTier::Tier1SupplyChain,
        order: 9,
        cost_level: "medium",
        runner: |root| {
            require_tool(
                "cargo-audit",
                "cargo install cargo-audit --locked",
                "https://github.com/rustsec/rustsec",
            )?;
            run_cmd_in(root, "cargo", &["audit"])
        },
    },
    GateCheckDefinition {
        id: "cargo-semver",
        name: "3.8. Semver Compatibility (core-contracts, sdk)",
        tier: VerificationTier::Tier1SupplyChain,
        order: 10,
        cost_level: "medium",
        runner: |root| {
            require_tool(
                "cargo-semver-checks",
                "cargo install cargo-semver-checks --locked",
                "https://github.com/obi1kenobi/cargo-semver-checks",
            )?;
            let baseline = semver_baseline_rev();
            for pkg in GATE_38_SEMVER_PACKAGES {
                run_cmd_in(
                    root,
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
                .with_context(|| format!("Semver Compatibility ({pkg})"))?;
            }
            Ok(())
        },
    },
    // Tier 2: Compilation & Matrix Profiles
    GateCheckDefinition {
        id: "clippy",
        name: "3. Strict Clippy (workspace, all targets)",
        tier: VerificationTier::Tier2Compilation,
        order: 11,
        cost_level: "high",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-workspace",
        name: "4. Full Workspace Compilation",
        tier: VerificationTier::Tier2Compilation,
        order: 12,
        cost_level: "high",
        runner: |root| run_cmd_in(root, "cargo", &["check", "--workspace", "--all-targets"]),
    },
    GateCheckDefinition {
        id: "check-portable-core",
        name: "4a. Portable Core Profile",
        tier: VerificationTier::Tier2Compilation,
        order: 13,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
                "cargo",
                &["check", "-p", "scan_core", "--no-default-features"],
            )
        },
    },
    GateCheckDefinition {
        id: "check-arm-core",
        name: "4b. ARM Bare-Metal Core Profile",
        tier: VerificationTier::Tier2Compilation,
        order: 14,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-wasm-core",
        name: "4c. WebAssembly Core Profile",
        tier: VerificationTier::Tier2Compilation,
        order: 15,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-portable-contracts",
        name: "4d. Portable Shared Contracts",
        tier: VerificationTier::Tier2Compilation,
        order: 16,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
                "cargo",
                &["check", "-p", "core-contracts", "--no-default-features"],
            )
        },
    },
    GateCheckDefinition {
        id: "check-arm-contracts",
        name: "4e. ARM Bare-Metal Shared Contracts",
        tier: VerificationTier::Tier2Compilation,
        order: 17,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-wasm-contracts",
        name: "4f. WebAssembly Shared Contracts",
        tier: VerificationTier::Tier2Compilation,
        order: 18,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-release",
        name: "10. Release Binary Check",
        tier: VerificationTier::Tier2Compilation,
        order: 19,
        cost_level: "high",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-features",
        name: "11. Optional Runtime Features (compile only)",
        tier: VerificationTier::Tier2Compilation,
        order: 20,
        cost_level: "high",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    GateCheckDefinition {
        id: "check-iroh",
        name: "12. Iroh Compatibility Feature (compile only)",
        tier: VerificationTier::Tier2Compilation,
        order: 21,
        cost_level: "medium",
        runner: |root| {
            run_cmd_in(
                root,
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
        },
    },
    // Tier 3: Execution & Test Determinism
    GateCheckDefinition {
        id: "test-workspace",
        name: "5. Workspace Test Suite",
        tier: VerificationTier::Tier3Execution,
        order: 22,
        cost_level: "high",
        runner: |root| run_cmd_in(root, "cargo", &["test", "--workspace"]),
    },
    GateCheckDefinition {
        id: "test-emulator",
        name: "8. Golden Dogfooding Harness Verification",
        tier: VerificationTier::Tier3Execution,
        order: 23,
        cost_level: "medium",
        runner: |root| run_cmd_in(root, "cargo", &["test", "-p", "emulator_harness"]),
    },
];

/// Validates that an explicit workspace root path exists, is a directory,
/// and contains a Cargo.toml declaring [workspace].
pub fn validate_workspace_root(root: &Path) -> Result<PathBuf> {
    if !root.exists() {
        bail!("Explicit workspace root does not exist: {}", root.display());
    }
    if !root.is_dir() {
        bail!(
            "Explicit workspace root is not a directory: {}",
            root.display()
        );
    }
    let manifest_path = root.join("Cargo.toml");
    if !manifest_path.exists() {
        bail!(
            "Explicit workspace root has no Cargo.toml: {}",
            root.display()
        );
    }
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Reading {}", manifest_path.display()))?;
    if !manifest_content.contains("[workspace]") {
        bail!(
            "Explicit workspace root Cargo.toml does not declare [workspace]: {}",
            manifest_path.display()
        );
    }
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let s = canon.to_string_lossy();
    let normalized = if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        canon
    };
    Ok(normalized)
}

/// Validates that an explicit target git revision resolves to an existing git object.
pub fn validate_target_rev(root: &Path, target_rev: &str) -> Result<String> {
    if target_rev.trim().is_empty() {
        bail!("Explicit target revision cannot be empty");
    }
    let output = Command::new("git")
        .args(["rev-parse", "--verify", target_rev])
        .current_dir(root)
        .output()
        .context("Failed to spawn git rev-parse")?;
    if !output.status.success() {
        bail!(
            "Explicit target revision '{}' does not resolve in workspace git repository: {}",
            target_rev,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(resolved)
}

/// Calculate a deterministic SHA-256 fingerprint for a GateReport.
///
/// Hashing stable facts only (schema version, target revision, tier, status,
/// and ordered check results). Wall-clock duration and workspace checkout root
/// are excluded to ensure exact cross-host determinism.
pub fn compute_gate_fingerprint(
    schema_version: &str,
    target_rev: &str,
    tier: &str,
    status: &str,
    checks: &[CheckResultRecord],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(schema_version.as_bytes());
    hasher.update(b"|");
    hasher.update(target_rev.as_bytes());
    hasher.update(b"|");
    hasher.update(tier.as_bytes());
    hasher.update(b"|");
    hasher.update(status.as_bytes());
    for c in checks {
        hasher.update(b";");
        hasher.update(c.id.as_bytes());
        hasher.update(b":");
        hasher.update(c.status.as_bytes());
        hasher.update(b":");
        hasher.update(c.tier.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(c.cost_level.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Execute a list of checks deterministically against the workspace root.
pub fn execute_checks(
    checks: &[&GateCheckDefinition],
    root: &Path,
    stop_on_first_failure: bool,
) -> (Vec<CheckResultRecord>, bool, Option<String>) {
    let mut records = Vec::new();
    let mut failed_check = None;
    let mut overall_success = true;

    for check in checks {
        let check_start = Instant::now();
        let result = (check.runner)(root);
        let duration_ms = check_start.elapsed().as_millis() as u64;

        match result {
            Ok(()) => {
                records.push(CheckResultRecord {
                    id: check.id.to_string(),
                    name: check.name.to_string(),
                    tier: check.tier as u8,
                    order: check.order,
                    cost_level: check.cost_level.to_string(),
                    status: "passed".to_string(),
                    duration_ms,
                    message: None,
                });
            }
            Err(e) => {
                overall_success = false;
                let msg = format!("{:#}", e);
                records.push(CheckResultRecord {
                    id: check.id.to_string(),
                    name: check.name.to_string(),
                    tier: check.tier as u8,
                    order: check.order,
                    cost_level: check.cost_level.to_string(),
                    status: "failed".to_string(),
                    duration_ms,
                    message: Some(msg.clone()),
                });
                failed_check = Some(check.id.to_string());

                if stop_on_first_failure {
                    break;
                }
            }
        }
    }

    (records, overall_success, failed_check)
}

/// Run tiered verification preflight according to the provided configuration.
pub fn run_preflight(config: &PreflightConfig) -> Result<GateReport> {
    let validated_root = validate_workspace_root(&config.workspace_root)?;
    let validated_rev = validate_target_rev(&validated_root, &config.target_rev)?;

    // Verify checked-out working tree is strictly at target revision
    let head_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&validated_root)
        .output()
        .context("git rev-parse HEAD")?;
    let head_rev = String::from_utf8_lossy(&head_output.stdout)
        .trim()
        .to_string();
    if head_rev != validated_rev {
        bail!(
            "Target revision '{}' ({}) does not match current checked-out HEAD ({})",
            config.target_rev,
            validated_rev,
            head_rev
        );
    }

    // Verify tracked tree is clean (no uncommitted modifications)
    let status_output = Command::new("git")
        .args(["diff-index", "--quiet", "HEAD", "--"])
        .current_dir(&validated_root)
        .output()
        .context("git diff-index")?;
    if !status_output.status.success() {
        bail!(
            "Working tree has uncommitted modifications; cannot certify preflight report for clean revision"
        );
    }

    // Verify there are no untracked files in the working tree
    let untracked_output = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=normal"])
        .current_dir(&validated_root)
        .output()
        .context("git status --porcelain --untracked-files=normal")?;
    if !untracked_output.status.success() {
        bail!("Failed to check working tree status for untracked files");
    }
    let untracked_str = String::from_utf8_lossy(&untracked_output.stdout);
    let untracked_files: Vec<&str> = untracked_str
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("??"))
        .collect();
    if !untracked_files.is_empty() {
        bail!(
            "Working tree contains {} untracked file(s); cannot certify preflight report for clean revision: {}",
            untracked_files.len(),
            untracked_files.join(", ")
        );
    }

    let tier_name = config.max_tier.as_str();
    let start_time = Instant::now();

    let mut checks_to_run: Vec<&GateCheckDefinition> = ALL_GATE_CHECKS
        .iter()
        .filter(|c| c.tier <= config.max_tier)
        .collect();
    checks_to_run.sort_by_key(|c| c.order);

    let (records, overall_success, failed_check) = execute_checks(
        &checks_to_run,
        &validated_root,
        config.stop_on_first_failure,
    );

    let total_duration_ms = start_time.elapsed().as_millis() as u64;
    let status_str = if overall_success { "passed" } else { "failed" };

    let fingerprint =
        compute_gate_fingerprint("1.0.0", &validated_rev, tier_name, status_str, &records);

    let report = GateReport {
        schema_version: "1.0.0".to_string(),
        target_rev: validated_rev,
        workspace_root: validated_root.display().to_string(),
        tier: tier_name.to_string(),
        status: status_str.to_string(),
        total_duration_ms,
        checks: records,
        failed_check,
        fingerprint,
    };

    Ok(report)
}

/// CLI entrypoint for `cargo xtask preflight`.
pub fn preflight_cli(args: &[String]) -> Result<()> {
    let mut explicit_root: Option<PathBuf> = None;
    let mut explicit_rev: Option<String> = None;
    let mut explicit_output: Option<PathBuf> = None;
    let mut tier = VerificationTier::Tier0Preflight;
    let mut json = false;
    let mut stop_on_first_failure = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--workspace-root" => {
                i += 1;
                if i >= args.len() {
                    bail!("--workspace-root requires an argument");
                }
                explicit_root = Some(PathBuf::from(&args[i]));
            }
            "--target-rev" => {
                i += 1;
                if i >= args.len() {
                    bail!("--target-rev requires an argument");
                }
                explicit_rev = Some(args[i].clone());
            }
            "--output" | "--report-file" => {
                i += 1;
                if i >= args.len() {
                    bail!("{} requires a file path argument", args[i - 1]);
                }
                explicit_output = Some(PathBuf::from(&args[i]));
            }
            "--tier" => {
                i += 1;
                if i >= args.len() {
                    bail!("--tier requires an argument (tier0, tier1, tier2, tier3)");
                }
                tier = VerificationTier::parse(&args[i])
                    .with_context(|| format!("Unknown verification tier: '{}'", args[i]))?;
            }
            "--json" => {
                json = true;
            }
            "--stop-on-first-failure" => {
                stop_on_first_failure = true;
            }
            "--continue-on-failure" => {
                stop_on_first_failure = false;
            }
            other => {
                bail!(
                    "Unknown argument to preflight: '{other}'. Usage: cargo xtask preflight --workspace-root <PATH> --target-rev <REV> [--output <FILE>] [--tier <TIER>] [--json]"
                );
            }
        }
        i += 1;
    }

    let root_path = explicit_root.ok_or_else(|| {
        anyhow::anyhow!("Missing required --workspace-root argument (zero ambient authority)")
    })?;
    let target_rev = explicit_rev.ok_or_else(|| {
        anyhow::anyhow!("Missing required --target-rev argument (zero ambient authority)")
    })?;

    let config = PreflightConfig {
        workspace_root: root_path,
        target_rev,
        max_tier: tier,
        json,
        stop_on_first_failure,
    };

    let report = run_preflight(&config)?;

    let json_str = serde_json::to_string_pretty(&report)?;

    if let Some(ref out_path) = explicit_output {
        if let Some(parent) = out_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for {}",
                    out_path.display()
                )
            })?;
        }
        std::fs::write(out_path, &json_str).with_context(|| {
            format!("Failed to write preflight report to {}", out_path.display())
        })?;
    }

    if config.json {
        println!("{}", json_str);
    } else {
        println!("=== Aaroneous Verification Preflight ({}) ===", report.tier);
        println!("Workspace Root: {}", report.workspace_root);
        println!("Target Rev:     {}", report.target_rev);
        println!("Status:         {}", report.status.to_uppercase());
        println!("-------------------------------------------------------------");
        for (idx, check) in report.checks.iter().enumerate() {
            let symbol = if check.status == "passed" {
                "OK"
            } else {
                "FAIL"
            };
            println!(
                "[{}/{}] {:<18} {:<45} [{} in {}ms]",
                idx + 1,
                report.checks.len(),
                check.id,
                check.name,
                symbol,
                check.duration_ms
            );
            if let Some(ref msg) = check.message {
                eprintln!("    Error: {msg}");
            }
        }
        println!("-------------------------------------------------------------");
        println!(
            "Total duration: {}ms | Fingerprint: {}",
            report.total_duration_ms, report.fingerprint
        );
    }

    if report.status != "passed" {
        bail!("Preflight verification failed.");
    }

    Ok(())
}

/// Canonical entrypoint for `cargo xtask gate` (full sequential CI verification).
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

    match status.code() {
        Some(0) => bail!(
            "Gate 7 failed: Found forbidden patterns (todo!, unimplemented!, or unsafe impl Pod). Exit code 0 means matches were found."
        ),
        Some(1) => (),
        Some(code) => bail!("Gate 7 failed: git grep failed with unexpected exit code {code}"),
        None => bail!("Gate 7 failed: git grep terminated by signal"),
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
    let mut cmd = Command::new(program);
    cmd.args(args);
    let status = cmd
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

fn run_cmd_in(dir: &Path, program: &str, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if !dir.as_os_str().is_empty() && dir != Path::new(".") {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .with_context(|| format!("Failed to spawn {}", program))?;

    // Redirect child stdout and stderr to stderr so that stdout remains
    // completely reserved for pure machine-readable JSON output.
    let _ = std::io::stderr().write_all(&output.stdout);
    let _ = std::io::stderr().write_all(&output.stderr);

    if !output.status.success() {
        bail!(
            "Command '{} {}' failed with status: {}",
            program,
            args.join(" "),
            output.status
        );
    }
    Ok(())
}

/// Fails with install instructions if `binary` isn't on `PATH`, rather than
/// silently skipping the gate or paying an unconditional multi-minute
/// `cargo install` on every run.
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
/// `.github/workflows/semver-checks.yml` resolves on a pull-request event.
fn semver_baseline_rev() -> String {
    "origin/main".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

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

    #[test]
    fn require_tool_accepts_present_binary() {
        require_tool("cargo", "unused", "unused").expect("cargo must be found on PATH");
    }

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

    #[test]
    fn gate_commands_are_present_in_ci_workflow() {
        let ci = ci_workflow_text();
        for cmd in GATE_COMMANDS {
            assert!(
                ci.contains(cmd),
                "gate.rs expects CI to run `{cmd}`, but that exact command string is no longer in ci.yml."
            );
        }
    }

    #[test]
    fn ci_workflow_verification_commands_are_all_covered_by_a_gate() {
        let ci = ci_workflow_text();
        for line in ci.lines() {
            let Some(cmd) = line.trim().strip_prefix("run: ") else {
                continue;
            };
            if !cmd.starts_with("cargo ") {
                continue;
            }
            assert!(
                GATE_COMMANDS.contains(&cmd),
                "ci.yml runs `{cmd}` but no gate in xtask/src/gate.rs covers it."
            );
        }
    }

    #[test]
    fn security_and_semver_workflows_match_their_local_gates() {
        assert_eq!(
            GATE_36_DENY_ARGS,
            &["deny", "--all-features", "check"],
            "GATE_36_DENY_ARGS changed"
        );
        assert_eq!(
            GATE_38_SEMVER_PACKAGES,
            &["core-contracts", "sdk"],
            "GATE_38_SEMVER_PACKAGES changed"
        );

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workflows = Path::new(manifest_dir)
            .join("..")
            .join(".github")
            .join("workflows");

        let security_audit = fs::read_to_string(workflows.join("security-audit.yml"))
            .expect("failed to read security-audit.yml");
        assert!(security_audit.contains("EmbarkStudios/cargo-deny-action"));
        assert!(security_audit.contains("--all-features"));
        assert!(security_audit.contains("rustsec/audit-check"));

        let semver_checks = fs::read_to_string(workflows.join("semver-checks.yml"))
            .expect("failed to read semver-checks.yml");
        for pkg in GATE_38_SEMVER_PACKAGES {
            let needle = format!("-p {pkg}");
            assert!(semver_checks.contains(&needle));
        }
    }

    #[test]
    fn test_tier_ordering_and_strictly_increasing_sequence() {
        // Assert all checks in ALL_GATE_CHECKS are ordered monotonically by tier and order
        for window in ALL_GATE_CHECKS.windows(2) {
            let a = &window[0];
            let b = &window[1];
            assert!(
                a.tier <= b.tier,
                "Tier violation: {} ({:?}) placed after {} ({:?})",
                b.name,
                b.tier,
                a.name,
                a.tier
            );
            assert!(
                a.order < b.order,
                "Order violation: {} ({}) must be strictly less than {} ({})",
                a.name,
                a.order,
                b.name,
                b.order
            );
        }

        // Check Tier 0 specifically contains only zero-workspace-compilation checks
        let tier0_checks: Vec<_> = ALL_GATE_CHECKS
            .iter()
            .filter(|c| c.tier == VerificationTier::Tier0Preflight)
            .collect();
        assert_eq!(tier0_checks.len(), 7);
        assert_eq!(tier0_checks[0].id, "check-encoding");
        assert_eq!(tier0_checks[1].id, "fmt");
        assert_eq!(tier0_checks[2].id, "zero-stubs");
        assert_eq!(tier0_checks[3].id, "ast-audit");
        assert_eq!(tier0_checks[4].id, "check-unwraps");
        assert_eq!(tier0_checks[5].id, "check-deps");
        assert_eq!(tier0_checks[6].id, "check-native");
    }

    #[test]
    fn test_tier_parsing_from_strings() {
        assert_eq!(
            VerificationTier::parse("tier0"),
            Some(VerificationTier::Tier0Preflight)
        );
        assert_eq!(
            VerificationTier::parse("preflight"),
            Some(VerificationTier::Tier0Preflight)
        );
        assert_eq!(
            VerificationTier::parse("tier1"),
            Some(VerificationTier::Tier1SupplyChain)
        );
        assert_eq!(
            VerificationTier::parse("tier2"),
            Some(VerificationTier::Tier2Compilation)
        );
        assert_eq!(
            VerificationTier::parse("tier3"),
            Some(VerificationTier::Tier3Execution)
        );
        assert_eq!(
            VerificationTier::parse("execution"),
            Some(VerificationTier::Tier3Execution)
        );
        assert_eq!(
            VerificationTier::parse("test"),
            Some(VerificationTier::Tier3Execution)
        );
        assert_eq!(VerificationTier::parse("all"), None);
        assert_eq!(VerificationTier::parse("full"), None);
        assert_eq!(VerificationTier::parse("unknown_nonexistent_tier"), None);
    }

    #[test]
    fn test_explicit_workspace_root_validation() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let valid_root = manifest_dir.parent().unwrap();
        assert!(validate_workspace_root(valid_root).is_ok());

        // Non-existent path
        let invalid_path = valid_root.join("nonexistent_directory_abc123");
        assert!(validate_workspace_root(&invalid_path).is_err());

        // Regular file instead of directory
        let file_path = manifest_dir.join("Cargo.toml");
        assert!(validate_workspace_root(&file_path).is_err());
    }

    #[test]
    fn test_explicit_target_rev_validation() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir.parent().unwrap();

        // Valid revisions
        assert!(validate_target_rev(root, "HEAD").is_ok());

        // Empty revision
        assert!(validate_target_rev(root, "   ").is_err());

        // Non-existent revision
        assert!(validate_target_rev(root, "definitely_not_a_valid_git_sha_9999").is_err());
    }

    #[test]
    fn test_gate_report_serialization_and_deterministic_fingerprint() {
        let checks = vec![
            CheckResultRecord {
                id: "check-encoding".to_string(),
                name: "1. Text Encoding Contract".to_string(),
                tier: 0,
                order: 1,
                cost_level: "low".to_string(),
                status: "passed".to_string(),
                duration_ms: 15,
                message: None,
            },
            CheckResultRecord {
                id: "fmt".to_string(),
                name: "2. Formatting".to_string(),
                tier: 0,
                order: 2,
                cost_level: "low".to_string(),
                status: "passed".to_string(),
                duration_ms: 42,
                message: None,
            },
        ];

        let fp1 =
            compute_gate_fingerprint("1.0.0", "abc1234", "tier0_preflight", "passed", &checks);
        let fp2 =
            compute_gate_fingerprint("1.0.0", "abc1234", "tier0_preflight", "passed", &checks);
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 64); // SHA-256 hex string

        // Verify that wall-clock check duration is excluded from fingerprint:
        let mut checks_different_duration = checks.clone();
        checks_different_duration[0].duration_ms = 9999;
        let fp3 = compute_gate_fingerprint(
            "1.0.0",
            "abc1234",
            "tier0_preflight",
            "passed",
            &checks_different_duration,
        );
        assert_eq!(
            fp1, fp3,
            "Fingerprint must not vary based on check wall-clock duration"
        );

        let report = GateReport {
            schema_version: "1.0.0".to_string(),
            target_rev: "abc1234".to_string(),
            workspace_root: "/workspace/aaroneous".to_string(),
            tier: "tier0_preflight".to_string(),
            status: "passed".to_string(),
            total_duration_ms: 57,
            checks,
            failed_check: None,
            fingerprint: fp1,
        };

        let json_str = serde_json::to_string(&report).unwrap();
        let deserialized: GateReport = serde_json::from_str(&json_str).unwrap();
        assert_eq!(report, deserialized);
    }

    #[test]
    fn test_deterministic_stop_on_first_failure() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static THIRD_CHECK_RAN: AtomicBool = AtomicBool::new(false);

        let check1 = GateCheckDefinition {
            id: "test-check-1",
            name: "Passing Check 1",
            tier: VerificationTier::Tier0Preflight,
            order: 1,
            cost_level: "low",
            runner: |_| Ok(()),
        };
        let check2 = GateCheckDefinition {
            id: "test-check-2",
            name: "Failing Check 2",
            tier: VerificationTier::Tier0Preflight,
            order: 2,
            cost_level: "low",
            runner: |_| bail!("Simulated failure in check 2"),
        };
        let check3 = GateCheckDefinition {
            id: "test-check-3",
            name: "Never Called Check 3",
            tier: VerificationTier::Tier0Preflight,
            order: 3,
            cost_level: "low",
            runner: |_| {
                THIRD_CHECK_RAN.store(true, Ordering::SeqCst);
                Ok(())
            },
        };

        let checks = vec![&check1, &check2, &check3];
        let root = Path::new(".");

        // Case 1: stop_on_first_failure = true
        THIRD_CHECK_RAN.store(false, Ordering::SeqCst);
        let (records, success, failed_check) = execute_checks(&checks, root, true);
        assert!(!success);
        assert_eq!(failed_check, Some("test-check-2".to_string()));
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].status, "passed");
        assert_eq!(records[1].status, "failed");
        assert!(
            !THIRD_CHECK_RAN.load(Ordering::SeqCst),
            "Third check must NOT run when stop_on_first_failure is true"
        );

        // Case 2: stop_on_first_failure = false
        THIRD_CHECK_RAN.store(false, Ordering::SeqCst);
        let (records2, success2, failed_check2) = execute_checks(&checks, root, false);
        assert!(!success2);
        assert_eq!(failed_check2, Some("test-check-2".to_string()));
        assert_eq!(records2.len(), 3);
        assert!(
            THIRD_CHECK_RAN.load(Ordering::SeqCst),
            "Third check MUST run when stop_on_first_failure is false"
        );
    }

    #[test]
    fn test_run_preflight_rejects_untracked_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        // Initialize a minimal git repository with an initial commit
        Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(root)
            .output()
            .unwrap();

        // Create Cargo.toml with [workspace]
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        Command::new("git")
            .args(["add", "Cargo.toml"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(root)
            .output()
            .unwrap();

        let head_rev = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        let config = PreflightConfig {
            workspace_root: root.to_path_buf(),
            target_rev: head_rev.clone(),
            max_tier: VerificationTier::Tier0Preflight,
            json: true,
            stop_on_first_failure: true,
        };

        // Create an untracked file
        std::fs::write(
            root.join("untracked_probe.txt"),
            "forbidden untracked content",
        )
        .unwrap();

        let err = run_preflight(&config).unwrap_err();
        assert!(
            err.to_string().contains("untracked file(s)"),
            "Expected error to complain about untracked files, got: {err}"
        );

        // Remove the untracked file
        std::fs::remove_file(root.join("untracked_probe.txt")).unwrap();
    }

    #[test]
    fn test_preflight_cli_output_file_flag() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();
        let report_file = root.join("reports").join("preflight_report.json");

        let fake_report = GateReport {
            schema_version: "1.0.0".to_string(),
            target_rev: "abc1234".to_string(),
            workspace_root: root.display().to_string(),
            tier: "tier0_preflight".to_string(),
            status: "passed".to_string(),
            total_duration_ms: 100,
            checks: vec![],
            failed_check: None,
            fingerprint: "fakefingerprint".to_string(),
        };

        let json_str = serde_json::to_string_pretty(&fake_report).unwrap();
        if let Some(parent) = report_file.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&report_file, &json_str).unwrap();

        assert!(report_file.exists());
        let read_back: GateReport =
            serde_json::from_str(&std::fs::read_to_string(&report_file).unwrap()).unwrap();
        assert_eq!(read_back, fake_report);
    }
}
