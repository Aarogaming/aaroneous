//! xtask/src/workspace_metadata.rs
//! Scans workspace Cargo.toml files for `[package.metadata.capability]` sections
//! and generates a capability manifest JSON for MCP discovery and deployment profiling.
//!
//! Usage: `cargo xtask workspace-metadata [--output manifest.json]`
//!
//! Compliant with Aaroneous standards:
//! - Explicit path inputs (no ambient reads)
//! - No heap on hot paths (this is a build tool, not hot path)
//! - Output is deterministic JSON

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Aaroneous tool metadata in Cargo.toml `[package.metadata.capability.tool]`
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Crate-level Aaroneous metadata in Cargo.toml `[package.metadata.capability]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetadata {
    pub ring: Option<u8>,
    #[serde(default)]
    pub profile: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tool: Option<ToolMetadata>,
}

/// Full workspace manifest output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub workspace_root: String,
    pub crates: BTreeMap<String, CrateInfo>,
    pub tools: Vec<ToolDescriptor>,
    pub profiles: BTreeMap<String, Vec<String>>,
}

/// Per-crate entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub ring: Option<u8>,
    pub profile: Vec<String>,
    pub description: String,
    pub dependencies: Vec<String>,
}

/// Tool entry for MCP discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Scan a workspace and generate the capability manifest.
///
/// # Arguments
/// * `workspace_root` - Path to the workspace root (where the root Cargo.toml lives)
/// * `output_path` - Optional path to write the JSON manifest
pub fn scan_workspace(
    workspace_root: &Path,
    output_path: Option<&Path>,
) -> Result<WorkspaceManifest> {
    let root_toml = workspace_root.join("Cargo.toml");
    if !root_toml.exists() {
        bail!("No Cargo.toml found at {:?}", workspace_root);
    }

    let root_content = fs::read_to_string(&root_toml)?;
    let root_parsed: toml::Value = toml::from_str(&root_content)?;

    // Extract workspace members
    let members = root_parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| anyhow::anyhow!("No workspace.members found in root Cargo.toml"))?;

    let mut crates = BTreeMap::new();
    let mut tools = Vec::new();
    let mut profiles: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for member in members {
        let member_str = member.as_str().unwrap_or_default();
        let crate_path = workspace_root.join(member_str);
        let crate_toml = crate_path.join("Cargo.toml");

        if !crate_toml.exists() {
            continue;
        }

        let crate_content = fs::read_to_string(&crate_toml)?;
        let crate_parsed: toml::Value = toml::from_str(&crate_content)?;

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

        // Extract Aaroneous metadata
        let metadata = crate_parsed
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("capability"))
            .cloned();

        let cap_meta: Option<CrateMetadata> =
            metadata.and_then(|m| serde_json::from_value(serde_json::to_value(m).ok()?).ok());

        let ring = cap_meta.as_ref().and_then(|m| m.ring);
        let profile = cap_meta
            .as_ref()
            .map(|m| m.profile.clone())
            .unwrap_or_default();
        let description = cap_meta
            .as_ref()
            .map(|m| m.description.clone())
            .unwrap_or_default();

        // Track profiles
        for p in &profile {
            profiles.entry(p.clone()).or_default().push(name.clone());
        }

        // Extract dependencies (simplified - just names)
        let dependencies = crate_parsed
            .get("dependencies")
            .and_then(|d| d.as_table())
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default();

        crates.insert(
            name.clone(),
            CrateInfo {
                name: name.clone(),
                version,
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

    let manifest = WorkspaceManifest {
        workspace_root: workspace_root.to_string_lossy().to_string(),
        crates,
        tools,
        profiles,
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

/// Run the workspace-metadata subcommand
pub fn run(args: &[String]) -> Result<()> {
    let mut output_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                i += 1;
                if let Some(path) = args.get(i) {
                    output_path = Some(PathBuf::from(path));
                }
            }
            "--help" | "-h" => {
                println!("cargo xtask workspace-metadata");
                println!();
                println!("Scans workspace Cargo.toml files for [package.metadata.capability]");
                println!("sections and generates a capability manifest JSON.");
                println!();
                println!("Usage:");
                println!("  cargo xtask workspace-metadata [--output manifest.json]");
                println!();
                println!("Options:");
                println!("  --output, -o <path>  Write manifest to file (default: stdout)");
                println!("  --help, -h           Show this help");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // Find workspace root (look for root Cargo.toml with workspace section)
    let current_dir = std::env::current_dir()?;
    let workspace_root = find_workspace_root(&current_dir)?;

    scan_workspace(&workspace_root, output_path.as_deref())?;

    Ok(())
}

/// Walk up from the given directory to find the workspace root
fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let toml_path = current.join("Cargo.toml");
        if toml_path.exists() {
            let content = fs::read_to_string(&toml_path)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }
        if !current.pop() {
            bail!("Could not find workspace root with [workspace] section");
        }
    }
}
