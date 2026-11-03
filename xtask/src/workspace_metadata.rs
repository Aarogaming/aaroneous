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
                } else if let Ok(packed_content) = fs::read_to_string(git_dir.join("packed-refs")) {
                    // Try reading from .git/packed-refs
                    for line in packed_content.lines() {
                        let line = line.trim();
                        if line.starts_with('#') || line.starts_with('^') || line.is_empty() {
                            continue;
                        }
                        let mut parts = line.split_whitespace();
                        match (parts.next(), parts.next()) {
                            (Some(sha), Some(name))
                                if name == ref_path && is_valid_hex_sha(sha) =>
                            {
                                return Ok(sha.to_lowercase());
                            }
                            _ => {}
                        }
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

fn check_no_devtools_deps(
    table: &toml::map::Map<String, toml::Value>,
    crate_name: &str,
    table_name: &str,
) -> Result<Vec<String>> {
    let mut deps = Vec::new();
    for (dep_name, dep_val) in table {
        deps.push(dep_name.clone());
        if dep_name.to_lowercase().contains("devtools") {
            bail!(
                "Forbidden dependency '{}' in [{}] for crate '{}': Aaroneous cannot depend on Devtools",
                dep_name,
                table_name,
                crate_name
            );
        }
        if let Some(path_str) = dep_val.get("path").and_then(|p| p.as_str())
            && path_str.to_lowercase().contains("devtools")
        {
            bail!(
                "Forbidden dependency path '{}' in [{}] for crate '{}' -> '{}': Aaroneous cannot depend on Devtools",
                path_str,
                table_name,
                crate_name,
                dep_name
            );
        }
    }
    Ok(deps)
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

    // Verify tracked files are clean (working tree binds content to git revision)
    let diff_status = Command::new("git")
        .args(["diff-index", "--quiet", "HEAD", "--"])
        .current_dir(workspace_root)
        .status()
        .context("executing git diff-index to verify tree cleanliness")?;
    if !diff_status.success() {
        bail!(
            "Working tree at {:?} has uncommitted modifications; cannot export capability manifest for revision {} with dirty tree",
            workspace_root,
            actual_rev
        );
    }

    // Verify absence of untracked files
    let status_output = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=normal"])
        .current_dir(workspace_root)
        .output()
        .context("executing git status to verify absence of untracked files")?;
    if !status_output.status.success() {
        bail!(
            "Failed to execute git status in {:?}: {}",
            workspace_root,
            String::from_utf8_lossy(&status_output.stderr)
        );
    }
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    if !status_str.trim().is_empty() {
        bail!(
            "Working tree at {:?} has untracked or modified files; cannot export capability manifest for revision {} with dirty tree:\n{}",
            workspace_root,
            actual_rev,
            status_str.trim()
        );
    }

    let root_content = fs::read_to_string(&root_toml)
        .with_context(|| format!("reading {}", root_toml.display()))?;
    let root_parsed: toml::Value = toml::from_str(&root_content)
        .with_context(|| format!("parsing TOML in {}", root_toml.display()))?;

    // Audit workspace-level dependencies for any devtools edge
    if let Some(ws_deps) = root_parsed
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        check_no_devtools_deps(ws_deps, "workspace", "workspace.dependencies")?;
    }

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
            bail!(
                "Workspace member '{}' does not have a Cargo.toml at {:?}",
                member_str,
                crate_toml
            );
        }

        // Verify member Cargo.toml is tracked in the git revision
        let rel_toml = format!(
            "{}/Cargo.toml",
            member_str.trim_matches('/').replace('\\', "/")
        );
        let ls_output = Command::new("git")
            .args(["ls-files", "--error-unmatch", &rel_toml])
            .current_dir(workspace_root)
            .output()
            .context("verifying member manifest is tracked in git")?;
        if !ls_output.status.success() {
            bail!(
                "Workspace member '{}' manifest '{}' is not tracked in git revision {}",
                member_str,
                rel_toml,
                actual_rev
            );
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

        // 1. Extract Cratify profile ([package.metadata.cratify].profile) strictly
        let cratify_profile = if let Some(meta) =
            crate_parsed.get("package").and_then(|p| p.get("metadata"))
        {
            if let Some(cratify) = meta.get("cratify") {
                if let Some(cratify_table) = cratify.as_table() {
                    match cratify_table.get("profile") {
                        Some(p) => match p.as_str() {
                            Some(s) => Some(s.to_string()),
                            None => bail!(
                                "Malformed [package.metadata.cratify].profile in crate '{}': expected string, got {:?}",
                                name,
                                p
                            ),
                        },
                        None => None,
                    }
                } else {
                    bail!(
                        "Malformed [package.metadata.cratify] in crate '{}': expected table, got {:?}",
                        name,
                        cratify
                    );
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ref prof) = cratify_profile {
            cratify_profiles.insert(name.clone(), prof.clone());
        }

        // 2. Extract capability metadata ([package.metadata.capability]) strictly
        let cap_meta_val = crate_parsed
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("capability"));

        let cap_meta: Option<CrateCapabilityMetadata> = match cap_meta_val {
            Some(val) => {
                let json_val = serde_json::to_value(val).with_context(|| {
                    format!(
                        "converting [package.metadata.capability] to json in crate '{}'",
                        name
                    )
                })?;
                let meta: CrateCapabilityMetadata =
                    serde_json::from_value(json_val).with_context(|| {
                        format!(
                            "Malformed [package.metadata.capability] in crate '{}'",
                            name
                        )
                    })?;
                Some(meta)
            }
            None => None,
        };

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

        // Extract and audit dependencies across all sections (one-way boundary: no edge to Devtools allowed)
        let mut dependencies = Vec::new();
        for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(table) = crate_parsed.get(table_name).and_then(|t| t.as_table()) {
                let found = check_no_devtools_deps(table, &name, table_name)?;
                if table_name == "dependencies" {
                    dependencies = found;
                }
            }
        }
        if let Some(targets) = crate_parsed.get("target").and_then(|t| t.as_table()) {
            for (tgt_name, tgt_val) in targets {
                if let Some(tgt_table) = tgt_val.as_table() {
                    for sub in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(table) = tgt_table.get(sub).and_then(|t| t.as_table()) {
                            let sec_name = format!("target.{}.{}", tgt_name, sub);
                            check_no_devtools_deps(table, &name, &sec_name)?;
                        }
                    }
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

    #[test]
    fn allowed_gates_match_gate_rs_catalog() {
        let names: Vec<&str> = ALLOWED_GATES.iter().map(|g| g.name).collect();
        assert!(names.contains(&"check-encoding"));
        assert!(names.contains(&"fmt"));
        assert!(names.contains(&"clippy"));
        assert!(names.contains(&"check-native"));
        assert!(names.contains(&"check-deps-policy"));
        assert!(names.contains(&"check-audit"));
        assert!(names.contains(&"check-semver"));
        assert!(names.contains(&"check-workspace"));
        assert!(names.contains(&"check-portable-core"));
        assert!(names.contains(&"test-workspace"));
        assert!(names.contains(&"ast-audit"));
        assert!(names.contains(&"check-stubs"));
        assert!(names.contains(&"test-emulator"));
        assert!(names.contains(&"check-unwraps"));
        assert!(names.contains(&"check-deps"));
        assert!(names.contains(&"check-release"));
        assert!(names.contains(&"check-optional-features"));
        assert!(names.contains(&"check-iroh-feature"));
        assert!(names.contains(&"gate"));
    }

    #[test]
    fn rejects_malformed_capability_metadata() {
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
        fs::write(&root_toml, "[workspace]\nmembers = [\"crates/bad_meta\"]\n").unwrap();

        let crate_dir = root.join("crates").join("bad_meta");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "bad_meta"
version = "0.1.0"

[package.metadata.capability]
ring = "not-a-number"
profile = ["test"]
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
        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Malformed [package.metadata.capability] in crate 'bad_meta'"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn rejects_dirty_tracked_tree() {
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
        fs::write(&root_toml, "[workspace]\nmembers = [\"crates/crate_a\"]\n").unwrap();

        let crate_dir = root.join("crates").join("crate_a");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "crate_a"
version = "0.1.0"
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

        // Manifest passes on clean commit
        assert!(scan_workspace(root, &actual_rev, None).is_ok());

        // Modify tracked file without committing
        fs::write(
            &crate_toml,
            r#"[package]
name = "crate_a"
version = "0.2.0"
"#,
        )
        .unwrap();

        // Manifest must fail because tracked tree is dirty
        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("uncommitted modifications"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn fingerprint_is_path_independent() {
        let temp1 = tempfile::tempdir().unwrap();
        let root1 = temp1.path();

        let _ = Command::new("git")
            .args(["init"])
            .current_dir(root1)
            .status()
            .unwrap();
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root1)
            .status();
        let _ = Command::new("git")
            .args(["config", "user.email", "test@test.local"])
            .current_dir(root1)
            .status();

        let root_toml = root1.join("Cargo.toml");
        fs::write(
            &root_toml,
            "[workspace]\nmembers = [\"crates/test_crate\"]\n",
        )
        .unwrap();

        let crate_dir = root1.join("crates").join("test_crate");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "test_crate"
version = "0.1.0"

[package.metadata.cratify]
profile = "control"
"#,
        )
        .unwrap();

        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(root1)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root1)
            .status();

        let rev = resolve_git_revision(root1).unwrap();

        let temp2 = tempfile::tempdir().unwrap();
        let clone_dir = temp2.path().join("cloned_checkout");
        let clone_status = Command::new("git")
            .args([
                "clone",
                root1.to_str().unwrap(),
                clone_dir.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(clone_status.success());

        let manifest1 = scan_workspace(root1, &rev, None).unwrap();
        let manifest2 = scan_workspace(&clone_dir, &rev, None).unwrap();

        // Fingerprints must be strictly identical even though root1 and clone_dir have completely different absolute paths
        assert_eq!(manifest1.fingerprint, manifest2.fingerprint);
    }

    #[test]
    fn rejects_missing_member_cargo_toml() {
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
            "[workspace]\nmembers = [\"crates/missing_toml\"]\n",
        )
        .unwrap();

        let crate_dir = root.join("crates").join("missing_toml");
        fs::create_dir_all(&crate_dir).unwrap();
        // Do NOT create Cargo.toml in crate_dir

        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(root)
            .status();

        let actual_rev = resolve_git_revision(root).unwrap();
        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not have a Cargo.toml"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn rejects_untracked_files_in_workspace() {
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
            "[workspace]\nmembers = [\"crates/valid_crate\"]\n",
        )
        .unwrap();

        let crate_dir = root.join("crates").join("valid_crate");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "valid_crate"
version = "0.1.0"

[package.metadata.cratify]
profile = "control"
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
        assert!(scan_workspace(root, &actual_rev, None).is_ok());

        // Create an untracked file in the workspace
        let untracked_file = root.join("untracked_probe.txt");
        fs::write(&untracked_file, "uncommitted contents").unwrap();

        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("untracked") || err.contains("dirty tree"),
            "Error was: {}",
            err
        );
    }

    #[test]
    fn rejects_untracked_member_cargo_toml() {
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

        // Commit root Cargo.toml that references crates/untracked_crate
        let root_toml = root.join("Cargo.toml");
        fs::write(
            &root_toml,
            "[workspace]\nmembers = [\"crates/untracked_crate\"]\n",
        )
        .unwrap();

        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init root"])
            .current_dir(root)
            .status();

        // Create crates/untracked_crate/Cargo.toml without committing it
        let crate_dir = root.join("crates").join("untracked_crate");
        fs::create_dir_all(&crate_dir).unwrap();
        let crate_toml = crate_dir.join("Cargo.toml");
        fs::write(
            &crate_toml,
            r#"[package]
name = "untracked_crate"
version = "0.1.0"

[package.metadata.cratify]
profile = "control"
"#,
        )
        .unwrap();

        let actual_rev = resolve_git_revision(root).unwrap();
        let result = scan_workspace(root, &actual_rev, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("untracked") || err.contains("not tracked in git"),
            "Error was: {}",
            err
        );
    }
}
