//! xtask/src/workspace_metadata.rs
//! Scans workspace Cargo.toml files for `[package.metadata.capability]` and
//! `[package.metadata.cratify]` sections and generates a strict capability manifest JSON
//! for external controllers (e.g. Foundry/Devtools) and deployment profiling.
//!
//! Compliant with Aaroneous standards:
//! - Explicit inputs: requires `--workspace-root` and `--git-revision` (zero ambient authority)
//! - Root/revision validation: rejects mismatch between supplied and resolved Git revision
//! - Gate contract: declares allowed native gate names and rejects undeclared gates
//! - Deterministic SHA-256 fingerprint over manifest facts
//! - Zero external Devtools dependencies

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Canonical schema version identifier for the Aaroneous capability manifest.
pub const SCHEMA_VERSION: &str = "aaroneous-capability-manifest-v1";

/// Aaroneous tool metadata in Cargo.toml `[package.metadata.capability.tool]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolMetadata {
    pub name: String,
    pub opcode: u16,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub consumes: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub requires_feature: Option<String>,
    #[serde(default)]
    pub ring: Option<u8>,
}

/// Crate-level Aaroneous capability metadata in Cargo.toml `[package.metadata.capability]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrateCapabilityMetadata {
    pub ring: Option<u8>,
    #[serde(default)]
    pub profile: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tool: Option<ToolMetadata>,
}

/// Description of an allowed native gate in Aaroneous.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateDescriptor {
    pub name: String,
    pub stage: String,
    pub description: String,
}

/// Static gate definition for the compile-time catalog.
pub struct StaticGate {
    pub name: &'static str,
    pub stage: &'static str,
    pub description: &'static str,
}

impl From<&StaticGate> for GateDescriptor {
    fn from(s: &StaticGate) -> Self {
        Self {
            name: s.name.to_string(),
            stage: s.stage.to_string(),
            description: s.description.to_string(),
        }
    }
}

/// Catalog of all allowed native gates in the Aaroneous sequential gate protocol.
pub const ALLOWED_GATES: &[StaticGate] = &[
    StaticGate {
        name: "check-encoding",
        stage: "1",
        description: "Text encoding contract (UTF-8 without BOM, LF line endings)",
    },
    StaticGate {
        name: "fmt",
        stage: "2",
        description: "Rust code formatting check (cargo fmt --all -- --check)",
    },
    StaticGate {
        name: "clippy",
        stage: "3",
        description: "Strict workspace Clippy check (cargo clippy --workspace --all-targets -- -D warnings)",
    },
    StaticGate {
        name: "check-native",
        stage: "3.5",
        description: "Native dependency boundary audit",
    },
    StaticGate {
        name: "check-deps-policy",
        stage: "3.6",
        description: "Dependency & license policy check (cargo-deny)",
    },
    StaticGate {
        name: "check-audit",
        stage: "3.7",
        description: "RUSTSEC security advisory audit (cargo-audit)",
    },
    StaticGate {
        name: "check-semver",
        stage: "3.8",
        description: "Semver compatibility check (cargo-semver-checks)",
    },
    StaticGate {
        name: "check-workspace",
        stage: "4",
        description: "Full workspace compilation check (cargo check --workspace --all-targets)",
    },
    StaticGate {
        name: "check-portable-core",
        stage: "4a",
        description: "Native-host portable core compilation (scan_core)",
    },
    StaticGate {
        name: "check-arm-core",
        stage: "4b",
        description: "ARM bare-metal portable core compilation (scan_core thumbv7em)",
    },
    StaticGate {
        name: "check-wasm-core",
        stage: "4c",
        description: "WebAssembly portable core compilation (scan_core wasm32)",
    },
    StaticGate {
        name: "check-portable-contracts",
        stage: "4d",
        description: "Native-host shared binary contracts compilation (core-contracts)",
    },
    StaticGate {
        name: "check-arm-contracts",
        stage: "4e",
        description: "ARM bare-metal shared binary contracts compilation (core-contracts thumbv7em)",
    },
    StaticGate {
        name: "check-wasm-contracts",
        stage: "4f",
        description: "WebAssembly shared binary contracts compilation (core-contracts wasm32)",
    },
    StaticGate {
        name: "test-workspace",
        stage: "5",
        description: "Full workspace test suite execution (cargo test --workspace)",
    },
    StaticGate {
        name: "ast-audit",
        stage: "6",
        description: "Structural and semantic invariant AST audit",
    },
    StaticGate {
        name: "check-stubs",
        stage: "7",
        description: "Zero-stub and soundness inspection (no todo!, unimplemented!, unsafe impl Pod)",
    },
    StaticGate {
        name: "test-emulator",
        stage: "8",
        description: "Golden dogfooding emulator harness verification",
    },
    StaticGate {
        name: "check-unwraps",
        stage: "9",
        description: "Unwrap/panic ratchet check",
    },
    StaticGate {
        name: "check-deps",
        stage: "9.5",
        description: "Profile dependency direction check",
    },
    StaticGate {
        name: "check-release",
        stage: "10",
        description: "Release binary compilation check (aaroneous, hypervisor)",
    },
    StaticGate {
        name: "check-optional-features",
        stage: "11",
        description: "Optional runtime features compilation check",
    },
    StaticGate {
        name: "check-iroh-feature",
        stage: "12",
        description: "Iroh P2P compatibility feature compilation check",
    },
    StaticGate {
        name: "gate",
        stage: "all",
        description: "Full sequential 12-stage gate verification suite",
    },
];

/// The declared artifact and output contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactContract {
    pub manifest_file: String,
    pub format: String,
    pub hashing_algorithm: String,
}

impl Default for ArtifactContract {
    fn default() -> Self {
        Self {
            manifest_file: "capability-manifest.json".to_string(),
            format: "json".to_string(),
            hashing_algorithm: "sha256".to_string(),
        }
    }
}

/// Per-crate entry in the capability manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub cratify_profile: Option<String>,
    pub ring: Option<u8>,
    pub profile: Vec<String>,
    pub description: String,
    pub dependencies: Vec<String>,
}

/// Tool entry for MCP discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub name: String,
    pub opcode: u16,
    pub description: String,
    pub category: String,
    pub consumes: Vec<String>,
    pub produces: Vec<String>,
    pub requires_feature: Option<String>,
    pub ring: Option<u8>,
    pub crate_name: String,
}

/// Complete schema-versioned workspace capability manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityManifest {
    pub schema_version: String,
    pub workspace_root: String,
    pub git_revision: String,
    pub crates: BTreeMap<String, CrateInfo>,
    pub tools: Vec<ToolDescriptor>,
    pub profiles: BTreeMap<String, Vec<String>>,
    pub cratify_profiles: BTreeMap<String, String>,
    pub allowed_gates: Vec<GateDescriptor>,
    pub artifact_contract: ArtifactContract,
    pub fingerprint: String,
}

/// View of the manifest used to compute the deterministic SHA-256 fingerprint.
#[derive(Serialize)]
pub struct FingerprintPayload<'a> {
    pub schema_version: &'a str,
    pub workspace_root: &'a str,
    pub git_revision: &'a str,
    pub crates: &'a BTreeMap<String, CrateInfo>,
    pub tools: &'a [ToolDescriptor],
    pub profiles: &'a BTreeMap<String, Vec<String>>,
    pub cratify_profiles: &'a BTreeMap<String, String>,
    pub allowed_gates: &'a [GateDescriptor],
    pub artifact_contract: &'a ArtifactContract,
}

impl<'a> From<&'a CapabilityManifest> for FingerprintPayload<'a> {
    fn from(m: &'a CapabilityManifest) -> Self {
        Self {
            schema_version: &m.schema_version,
            workspace_root: &m.workspace_root,
            git_revision: &m.git_revision,
            crates: &m.crates,
            tools: &m.tools,
            profiles: &m.profiles,
            cratify_profiles: &m.cratify_profiles,
            allowed_gates: &m.allowed_gates,
            artifact_contract: &m.artifact_contract,
        }
    }
}

/// Compute a deterministic SHA-256 fingerprint over manifest facts.
pub fn compute_fingerprint(payload: &FingerprintPayload<'_>) -> Result<String> {
    let json_bytes = serde_json::to_vec(payload)
        .context("failed to serialize manifest payload for fingerprint computation")?;
    let mut hasher = Sha256::new();
    hasher.update(&json_bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// Resolve the current Git commit SHA at the given workspace root.
pub fn resolve_git_revision(workspace_root: &Path) -> Result<String> {
    // 1. Try invoking git rev-parse HEAD
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace_root)
        .output();

    if let Ok(out) = output
        && out.status.success()
    {
        let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if is_valid_hex_sha(&sha) {
            return Ok(sha.to_lowercase());
        }
    }

    // 2. Fallback: inspect .git/HEAD directly
    let git_dir = workspace_root.join(".git");
    if git_dir.exists() {
        let head_path = git_dir.join("HEAD");
        if head_path.exists() {
            let head_content = fs::read_to_string(&head_path)
                .context("reading .git/HEAD")?
                .trim()
                .to_string();
            if let Some(stripped) = head_content.strip_prefix("ref: ") {
                let ref_path = stripped.trim();
                let full_ref = git_dir.join(ref_path);
                if full_ref.exists() {
                    let sha = fs::read_to_string(&full_ref)?.trim().to_string();
                    if is_valid_hex_sha(&sha) {
                        return Ok(sha.to_lowercase());
                    }
                }
            } else if is_valid_hex_sha(&head_content) {
                return Ok(head_content.to_lowercase());
            }
        }
    }

    bail!(
        "Failed to resolve git revision at {:?}: not a valid git repository or git rev-parse failed",
        workspace_root
    )
}

/// Validate that a string is a 40-character hexadecimal SHA.
pub fn is_valid_hex_sha(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate whether a gate name is declared in the allowed native gate catalog.
pub fn validate_gate_name(gate_name: &str) -> Result<()> {
    if ALLOWED_GATES.iter().any(|g| g.name == gate_name) {
        Ok(())
    } else {
        let allowed: Vec<&str> = ALLOWED_GATES.iter().map(|g| g.name).collect();
        bail!(
            "Undeclared gate '{}': allowed native gates are: {:?}",
            gate_name,
            allowed
        )
    }
}

/// Scan a workspace and generate the schema-versioned capability manifest.
///
/// # Arguments
/// * `workspace_root` - Explicit path to the workspace root
/// * `git_revision` - Explicit expected Git commit SHA (40-hex lowercase)
/// * `output_path` - Optional path to write the JSON manifest
pub fn scan_workspace(
    workspace_root: &Path,
    git_revision: &str,
    output_path: Option<&Path>,
) -> Result<CapabilityManifest> {
    if !workspace_root.exists() {
        bail!("Workspace root does not exist: {:?}", workspace_root);
    }

    let root_toml = workspace_root.join("Cargo.toml");
    if !root_toml.exists() {
        bail!(
            "No Cargo.toml found at workspace root: {:?}",
            workspace_root
        );
    }

    let normalized_expected_rev = git_revision.trim().to_lowercase();
    if !is_valid_hex_sha(&normalized_expected_rev) {
        bail!(
            "Invalid git revision '{}': must be a 40-character hexadecimal SHA",
            git_revision
        );
    }

    // Verify root and revision binding
    let actual_rev = resolve_git_revision(workspace_root)?;
    if actual_rev != normalized_expected_rev {
        bail!(
            "Workspace root and git revision mismatch: expected {}, but resolved {}",
            normalized_expected_rev,
            actual_rev
        );
    }

    let root_content = fs::read_to_string(&root_toml)
        .with_context(|| format!("reading {}", root_toml.display()))?;
    let root_parsed: toml::Value = toml::from_str(&root_content)
        .with_context(|| format!("parsing TOML in {}", root_toml.display()))?;

    let members = root_parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| anyhow::anyhow!("No workspace.members found in root Cargo.toml"))?;

    let mut crates = BTreeMap::new();
    let mut tools = Vec::new();
    let mut profiles: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut cratify_profiles: BTreeMap<String, String> = BTreeMap::new();

    for member in members {
        let member_str = member.as_str().unwrap_or_default();
        if member_str.trim().is_empty() {
            continue;
        }

        // Path bounds validation: member path must be relative to workspace root and strictly bounded
        let member_path = Path::new(member_str);
        if member_path.is_absolute()
            || member_str.contains("..")
            || member_str.starts_with('/')
            || member_str.starts_with('\\')
            || (member_str.len() > 1 && member_str.chars().nth(1) == Some(':'))
        {
            bail!(
                "Invalid workspace member path '{}': path must be relative to workspace root and cannot escape workspace bounds",
                member_str
            );
        }

        // Devtools boundary audit: Aaroneous cannot reference or contain Devtools components
        if member_str.to_lowercase().contains("devtools") {
            bail!(
                "Forbidden workspace member path '{}': Aaroneous cannot contain or reference Devtools components",
                member_str
            );
        }

        let crate_path = workspace_root.join(member_str);
        let crate_toml = crate_path.join("Cargo.toml");

        if !crate_toml.exists() {
            continue;
        }

        let crate_content = fs::read_to_string(&crate_toml)
            .with_context(|| format!("reading {}", crate_toml.display()))?;
        let crate_parsed: toml::Value = toml::from_str(&crate_content)
            .with_context(|| format!("parsing TOML in {}", crate_toml.display()))?;

        let package = crate_parsed
            .get("package")
            .cloned()
            .unwrap_or(toml::Value::Table(toml::map::Map::new()));
        let name = package
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or(member_str)
            .to_string();
        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        // 1. Extract Cratify profile ([package.metadata.cratify].profile)
        let cratify_profile = crate_parsed
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("cratify"))
            .and_then(|c| c.get("profile"))
            .and_then(|p| p.as_str())
            .map(String::from);

        if let Some(ref prof) = cratify_profile {
            cratify_profiles.insert(name.clone(), prof.clone());
        }

        // 2. Extract capability metadata ([package.metadata.capability])
        let cap_meta_val = crate_parsed
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("capability"))
            .cloned();

        let cap_meta: Option<CrateCapabilityMetadata> =
            cap_meta_val.and_then(|m| serde_json::from_value(serde_json::to_value(m).ok()?).ok());

        let ring = cap_meta.as_ref().and_then(|m| m.ring);
        let profile = cap_meta
            .as_ref()
            .map(|m| m.profile.clone())
            .unwrap_or_default();
        let description = cap_meta
            .as_ref()
            .map(|m| m.description.clone())
            .unwrap_or_else(|| {
                package
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or_default()
                    .to_string()
            });

        // Track capability profiles
        for p in &profile {
            profiles.entry(p.clone()).or_default().push(name.clone());
        }

        // Extract and audit dependencies (one-way boundary: no edge to Devtools allowed)
        let mut dependencies = Vec::new();
        if let Some(deps_table) = crate_parsed.get("dependencies").and_then(|d| d.as_table()) {
            for (dep_name, dep_val) in deps_table {
                dependencies.push(dep_name.clone());
                if dep_name.to_lowercase().contains("devtools") {
                    bail!(
                        "Forbidden dependency '{}' in crate '{}': Aaroneous cannot depend on Devtools",
                        dep_name,
                        name
                    );
                }
                if let Some(path_str) = dep_val.get("path").and_then(|p| p.as_str())
                    && path_str.to_lowercase().contains("devtools")
                {
                    bail!(
                        "Forbidden dependency path '{}' for crate '{}' -> '{}': Aaroneous cannot depend on Devtools",
                        path_str,
                        name,
                        dep_name
                    );
                }
            }
        }
        dependencies.sort();

        crates.insert(
            name.clone(),
            CrateInfo {
                name: name.clone(),
                version,
                path: member_str.replace('\\', "/"),
                cratify_profile,
                ring,
                profile,
                description,
                dependencies,
            },
        );

        // Extract tool metadata if present
        if let Some(tool) = cap_meta.and_then(|m| m.tool) {
            tools.push(ToolDescriptor {
                name: tool.name.clone(),
                opcode: tool.opcode,
                description: tool.description.clone(),
                category: tool.category.clone(),
                consumes: tool.consumes.clone(),
                produces: tool.produces.clone(),
                requires_feature: tool.requires_feature.clone(),
                ring: tool.ring,
                crate_name: name.clone(),
            });
        }
    }

    let allowed_gates: Vec<GateDescriptor> =
        ALLOWED_GATES.iter().map(GateDescriptor::from).collect();
    let artifact_contract = ArtifactContract::default();
    let normalized_root = workspace_root.to_string_lossy().replace('\\', "/");

    let payload = FingerprintPayload {
        schema_version: SCHEMA_VERSION,
        workspace_root: &normalized_root,
        git_revision: &normalized_expected_rev,
        crates: &crates,
        tools: &tools,
        profiles: &profiles,
        cratify_profiles: &cratify_profiles,
        allowed_gates: &allowed_gates,
        artifact_contract: &artifact_contract,
    };
    let fingerprint = compute_fingerprint(&payload)?;

    let manifest = CapabilityManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        workspace_root: normalized_root,
        git_revision: normalized_expected_rev,
        crates,
        tools,
        profiles,
        cratify_profiles,
        allowed_gates,
        artifact_contract,
        fingerprint,
    };

    // Write output
    let json = serde_json::to_string_pretty(&manifest)?;
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &json)?;
    } else {
        println!("{}", json);
    }

    Ok(manifest)
}

/// Run the workspace-metadata subcommand.
pub fn run(args: &[String]) -> Result<()> {
    let mut workspace_root: Option<PathBuf> = None;
    let mut git_revision: Option<String> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut check_gate: Option<String> = None;
    let mut check_schema_version: Option<String> = None;
    let mut expected_schema_version: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--workspace-root" => {
                i += 1;
                if let Some(path) = args.get(i) {
                    workspace_root = Some(PathBuf::from(path));
                }
            }
            "--git-revision" => {
                i += 1;
                if let Some(rev) = args.get(i) {
                    git_revision = Some(rev.clone());
                }
            }
            "--output" | "-o" => {
                i += 1;
                if let Some(path) = args.get(i) {
                    output_path = Some(PathBuf::from(path));
                }
            }
            "--check-gate" => {
                i += 1;
                if let Some(gate) = args.get(i) {
                    check_gate = Some(gate.clone());
                }
            }
            "--check-schema-version" => {
                i += 1;
                if let Some(ver) = args.get(i) {
                    check_schema_version = Some(ver.clone());
                }
            }
            "--expected-schema-version" | "--schema-version" => {
                i += 1;
                if let Some(ver) = args.get(i) {
                    expected_schema_version = Some(ver.clone());
                }
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => {
                bail!("Unknown option '{}'. Run with --help for usage.", other);
            }
        }
        i += 1;
    }

    // 0. If --check-schema-version was requested, validate against SCHEMA_VERSION and exit
    if let Some(ref ver) = check_schema_version {
        if ver != SCHEMA_VERSION {
            bail!(
                "Schema version mismatch: requested '{}', but current schema version is '{}'",
                ver,
                SCHEMA_VERSION
            );
        }
        println!("Schema version '{}' matches current specification.", ver);
        return Ok(());
    }

    // 1. If --check-gate was requested, validate the gate name and exit
    if let Some(ref gate_name) = check_gate {
        validate_gate_name(gate_name)?;
        println!("Gate '{}' is declared and allowed.", gate_name);
        return Ok(());
    }

    // 2. Strict inputs required for capability manifest generation
    let root = match workspace_root {
        Some(r) => r,
        None => {
            print_usage();
            bail!(
                "Missing required argument: --workspace-root <PATH> must be provided (zero ambient authority)"
            );
        }
    };

    let revision = match git_revision {
        Some(rev) => rev,
        None => {
            print_usage();
            bail!(
                "Missing required argument: --git-revision <HEX> must be provided (zero ambient authority)"
            );
        }
    };

    if let Some(ref expected_ver) = expected_schema_version
        && expected_ver != SCHEMA_VERSION
    {
        bail!(
            "Schema version mismatch: expected '{}', but manifest produces '{}'",
            expected_ver,
            SCHEMA_VERSION
        );
    }

    scan_workspace(&root, &revision, output_path.as_deref())?;

    Ok(())
}

fn print_usage() {
    println!("cargo xtask workspace-metadata");
    println!();
    println!("Scans workspace Cargo.toml files for [package.metadata.capability] and");
    println!(
        "[package.metadata.cratify] sections and generates a strict capability manifest JSON."
    );
    println!();
    println!("Usage:");
    println!(
        "  cargo xtask workspace-metadata --workspace-root <PATH> --git-revision <HEX> [--output <PATH>]"
    );
    println!("  cargo xtask workspace-metadata --check-gate <NAME>");
    println!("  cargo xtask workspace-metadata --check-schema-version <VERSION>");
    println!();
    println!("Options:");
    println!("  --workspace-root <PATH>           Explicit path to workspace root (required)");
    println!(
        "  --git-revision <HEX>              Explicit 40-character hex Git revision (required)"
    );
    println!("  --output, -o <PATH>               Write manifest to file (default: stdout)");
    println!(
        "  --check-gate <NAME>               Validate whether a gate name is declared and allowed"
    );
    println!("  --check-schema-version <VERSION>  Validate whether a schema version is recognized");
    println!(
        "  --expected-schema-version <VER>   Verify expected schema version during manifest export"
    );
    println!("  --help, -h                        Show this help");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_sha_format() {
        assert!(is_valid_hex_sha("1f8c4ed5b72e505a7d76cb04c35e80aa74fa2e55"));
        assert!(!is_valid_hex_sha("1f8c4ed5b72e505a7d76cb04c35e80aa74fa2e5")); // 39 chars
        assert!(!is_valid_hex_sha(
            "1f8c4ed5b72e505a7d76cb04c35e80aa74fa2e555"
        )); // 41 chars
        assert!(!is_valid_hex_sha(
            "1f8c4ed5b72e505a7d76cb04c35e80aa74fa2e5z"
        )); // invalid hex
    }

    #[test]
    fn allows_valid_gates_and_rejects_undeclared() {
        assert!(validate_gate_name("check-encoding").is_ok());
        assert!(validate_gate_name("fmt").is_ok());
        assert!(validate_gate_name("clippy").is_ok());
        assert!(validate_gate_name("gate").is_ok());
        assert!(validate_gate_name("invalid-gate-xyz").is_err());
    }

    #[test]
    fn cli_rejects_missing_inputs() {
        assert!(run(&[]).is_err());
        assert!(run(&["--workspace-root".to_string(), ".".to_string()]).is_err());
        assert!(run(&["--git-revision".to_string(), "abc".to_string()]).is_err());
    }

    #[test]
    fn cli_rejects_invalid_revision() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[workspace]\nmembers = []\n").unwrap();

        let result = run(&[
            "--workspace-root".to_string(),
            temp.path().to_string_lossy().to_string(),
            "--git-revision".to_string(),
            "short-sha".to_string(),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_check_gate_succeeds_for_declared_gate() {
        assert!(run(&["--check-gate".to_string(), "check-encoding".to_string()]).is_ok());
    }

    #[test]
    fn cli_check_gate_fails_for_undeclared_gate() {
        assert!(run(&["--check-gate".to_string(), "not-a-real-gate".to_string()]).is_err());
    }

    #[test]
    fn rejects_root_revision_mismatch_in_git_repo() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        // Initialize a real Git repo in tempdir
        let init_status = Command::new("git")
            .args(["init"])
            .current_dir(root)
            .status()
            .unwrap();
        assert!(init_status.success());

        // Configure dummy user for commit
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.local"])
            .current_dir(root)
            .status();

        let cargo_toml = root.join("Cargo.toml");
        fs::write(&cargo_toml, "[workspace]\nmembers = []\n").unwrap();

        let _ = Command::new("git")
            .args(["add", "Cargo.toml"])
            .current_dir(root)
            .status();
        let commit_status = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .status()
            .unwrap();
        assert!(commit_status.success());

        let actual_rev = resolve_git_revision(root).unwrap();

        // Passing a different 40-hex SHA must fail
        let fake_rev = "0123456789abcdef0123456789abcdef01234567";
        let result = scan_workspace(root, fake_rev, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("mismatch"));

        // Passing actual rev must succeed
        let manifest = scan_workspace(root, &actual_rev, None).unwrap();
        assert_eq!(manifest.schema_version, SCHEMA_VERSION);
        assert_eq!(manifest.git_revision, actual_rev);
        assert!(!manifest.fingerprint.is_empty());
    }

    #[test]
    fn deterministic_fingerprint_generation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let init_status = Command::new("git")
            .args(["init"])
            .current_dir(root)
            .status()
            .unwrap();
        assert!(init_status.success());

        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.local"])
            .current_dir(root)
            .status();

        // Create workspace structure
        let root_toml = root.join("Cargo.toml");
        fs::write(
            &root_toml,
            r#"[workspace]
members = ["crates/test_crate"]
"#,
        )
        .unwrap();

        let crate_dir = root.join("crates").join("test_crate");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "test_crate"
version = "0.1.0"
description = "Sample crate"

[package.metadata.cratify]
profile = "kernel"

[package.metadata.capability]
ring = 1
profile = ["scada", "deterministic"]

[dependencies]
"#,
        )
        .unwrap();

        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .status();

        let actual_rev = resolve_git_revision(root).unwrap();

        let manifest1 = scan_workspace(root, &actual_rev, None).unwrap();
        let manifest2 = scan_workspace(root, &actual_rev, None).unwrap();

        assert_eq!(manifest1.fingerprint, manifest2.fingerprint);
        assert_eq!(
            manifest1.crates["test_crate"].cratify_profile.as_deref(),
            Some("kernel")
        );
        assert_eq!(manifest1.cratify_profiles["test_crate"], "kernel");
        assert_eq!(manifest1.crates["test_crate"].ring, Some(1));
    }

    #[test]
    fn rejects_schema_version_mismatch() {
        assert!(
            run(&[
                "--check-schema-version".to_string(),
                SCHEMA_VERSION.to_string()
            ])
            .is_ok()
        );

        let bad_ver = run(&[
            "--check-schema-version".to_string(),
            "wrong-schema-v99".to_string(),
        ]);
        assert!(bad_ver.is_err());
        assert!(
            bad_ver
                .unwrap_err()
                .to_string()
                .contains("Schema version mismatch")
        );
    }

    #[test]
    fn rejects_path_bounds_escape() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let _ = Command::new("git")
            .args(["init"])
            .current_dir(root)
            .status()
            .unwrap();
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.local"])
            .current_dir(root)
            .status();

        // Workspace member with '..' escaping root
        let root_toml = root.join("Cargo.toml");
        fs::write(
            &root_toml,
            r#"[workspace]
members = ["../escaped_crate"]
"#,
        )
        .unwrap();

        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .status()
            .unwrap();

        let actual_rev = resolve_git_revision(root).unwrap();
        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("escape workspace bounds"));
    }

    #[test]
    fn rejects_devtools_dependency_edge() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let _ = Command::new("git")
            .args(["init"])
            .current_dir(root)
            .status()
            .unwrap();
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.local"])
            .current_dir(root)
            .status();

        let root_toml = root.join("Cargo.toml");
        fs::write(
            &root_toml,
            r#"[workspace]
members = ["crates/bad_crate"]
"#,
        )
        .unwrap();

        let crate_dir = root.join("crates").join("bad_crate");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "bad_crate"
version = "0.1.0"

[dependencies]
aaroneous-devtools = { path = "../../aaroneous-devtools" }
"#,
        )
        .unwrap();

        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .status()
            .unwrap();

        let actual_rev = resolve_git_revision(root).unwrap();
        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Aaroneous cannot depend on Devtools"));
    }
}
