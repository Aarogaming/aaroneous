use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Validate a crate name: must be non-empty, start with a letter, and contain
/// only ASCII alphanumeric characters, hyphens, or underscores.
pub fn validate_crate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("crate name cannot be empty");
    }
    if !name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        anyhow::bail!("crate name `{}` must start with an ASCII letter", name);
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "crate name `{}` contains invalid characters (only ASCII alphanumeric, hyphens, underscores allowed)",
            name
        );
    }
    if name.len() > 64 {
        anyhow::bail!(
            "crate name `{}` exceeds maximum length of 64 characters",
            name
        );
    }
    Ok(())
}

/// Generate a microkernel crate skeleton at `crates/<name>/` inside the specified workspace root.
pub fn create_crate_scaffold_at(name: &str, workspace_root: &Path) -> Result<PathBuf> {
    validate_crate_name(name)?;

    let crate_dir = workspace_root.join("crates").join(name);
    let src_dir = crate_dir.join("src");

    if crate_dir.join("Cargo.toml").exists() {
        anyhow::bail!(
            "crate `{}` already exists at {} — remove it first or choose a different name",
            name,
            crate_dir.display()
        );
    }

    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create {}", src_dir.display()))?;

    emit_cargo_toml(name, &crate_dir)?;
    emit_lib_rs(name, &crate_dir)?;
    emit_cratify_toml(name, &crate_dir)?;

    Ok(crate_dir)
}

/// Generate a microkernel crate skeleton at `crates/<name>/` using discovered workspace root.
pub fn create_crate_scaffold(name: &str) -> Result<PathBuf> {
    let root = paths::WorkspacePaths::default().root().to_path_buf();
    create_crate_scaffold_at(name, &root)
}

/// Backwards-compatible alias for `create_crate_scaffold`.
pub fn generate_crate(name: &str) -> Result<()> {
    create_crate_scaffold(name).map(|_| ())
}

/// Emit Cargo.toml with workspace-inherited fields and core-contracts dependency.
pub fn emit_cargo_toml(name: &str, crate_dir: &Path) -> Result<()> {
    let path = crate_dir.join("Cargo.toml");
    if path.exists() {
        return Ok(());
    }

    let content = format!(
        r#"[package]
name = "{name}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
description.workspace = true
license.workspace = true
repository.workspace = true
keywords.workspace = true
categories.workspace = true
readme.workspace = true

[dependencies]
core-contracts = {{ path = "../core-contracts" }}
serde = {{ workspace = true }}
serde_json = {{ workspace = true }}
anyhow = {{ workspace = true }}
"#
    );

    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Emit src/lib.rs for a newly scaffolded ACC crate.
pub fn emit_lib_rs(name: &str, crate_dir: &Path) -> Result<()> {
    let path = crate_dir.join("src").join("lib.rs");
    if path.exists() {
        return Ok(());
    }

    let content = format!(
        r#"//! `{name}` — Aaroneous Crate Component (ACC)
//!
//! This crate was scaffolded by Cratify and adheres to the microkernel
//! contract boundaries defined in `core-contracts`.

use core_contracts::{{ComponentManifest, HierarchyTier, Capability, pack_version}};

/// ACC identity returned during bootstrap handshake.
pub fn manifest() -> ComponentManifest {{
    ComponentManifest {{
        name: *b"{name:<32}",
        version: pack_version(0, 1, 0),
        tier: HierarchyTier::SubordinateModule as u8,
        capabilities: Capability::INFERENCE_OUT.bits(),
        methodology: 0x01,
        supported_intents: 0x0001,
        priority_weight: 128,
        address: 0,
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn manifest_is_pod() {{
        let m = manifest();
        assert_eq!(std::mem::size_of::<ComponentManifest>(), 64);
        assert_eq!(m.tier, HierarchyTier::SubordinateModule as u8);
    }}
}}
"#
    );

    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Emit cratify.toml manifest for a newly scaffolded ACC crate.
pub fn emit_cratify_toml(name: &str, crate_dir: &Path) -> Result<()> {
    let path = crate_dir.join("cratify.toml");
    if path.exists() {
        return Ok(());
    }

    let content = format!(
        r#"name = "{name}"
version = "0.1.0"
description = "Aaroneous Crate Component scaffolded by Cratify"
dependencies = ["core-contracts"]
"#
    );

    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_crate_scaffold_at() {
        let ws = tempdir().unwrap();
        let ws_root = ws.path();
        let crate_dir = create_crate_scaffold_at("test_scaffold", ws_root).unwrap();

        assert!(crate_dir.join("Cargo.toml").exists());
        assert!(crate_dir.join("src").join("lib.rs").exists());
        assert!(crate_dir.join("cratify.toml").exists());

        let lib_content = fs::read_to_string(crate_dir.join("src").join("lib.rs")).unwrap();
        assert!(lib_content.contains("ComponentManifest"));
    }
}
