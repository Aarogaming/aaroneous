use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Validate a crate name: must be non-empty, start with a letter, and contain
/// only ASCII alphanumeric characters, hyphens, or underscores.
fn validate_crate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("crate name cannot be empty");
    }
    if !name.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
        anyhow::bail!(
            "crate name `{}` must start with an ASCII letter",
            name
        );
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

/// Generate a microkernel crate skeleton at `crates/<name>/`.
///
/// Emits:
/// - `Cargo.toml` with workspace-inherited fields + `core-contracts` dep
/// - `src/lib.rs` with canonical microkernel boilerplate
/// - `cratify.toml` ACC manifest
///
/// Returns an error if the crate directory already exists and contains files.
pub fn generate_crate(name: &str) -> Result<()> {
    validate_crate_name(name)?;

    let cwd =
        std::env::current_dir().context("failed to read current working directory")?;
    let root = crate::workspace::find_workspace_root(&cwd)
        .context("failed to locate workspace root")?;
    let crate_dir = root.join("crates").join(name);
    let src_dir = crate_dir.join("src");

    // Reject if crate directory already has a Cargo.toml (avoid silent overwrite)
    if crate_dir.join("Cargo.toml").exists() {
        anyhow::bail!(
            "crate `{}` already exists at {} — remove it first or choose a different name",
            name,
            crate_dir.display()
        );
    }

    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create {}", src_dir.display()))?;

    write_cargo_toml(name, &crate_dir)?;
    write_lib_rs(name, &crate_dir)?;
    write_cratify_toml(name, &crate_dir)?;

    Ok(())
}

fn write_cargo_toml(name: &str, crate_dir: &Path) -> Result<()> {
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

    fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_lib_rs(name: &str, crate_dir: &Path) -> Result<()> {
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

    fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_cratify_toml(name: &str, crate_dir: &Path) -> Result<()> {
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

    fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generate_crate_creates_expected_files() {
        let workspace_root = tempdir().unwrap();
        let crates_dir = workspace_root.path().join("crates").join("test_gen");
        fs::create_dir_all(crates_dir.join("src")).unwrap();

        // Create a fake workspace Cargo.toml
        fs::write(
            workspace_root.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        // Temporarily override cwd by calling the inner writers directly
        let name = "test_gen";
        write_cargo_toml(name, &crates_dir).unwrap();
        write_lib_rs(name, &crates_dir).unwrap();
        write_cratify_toml(name, &crates_dir).unwrap();

        assert!(crates_dir.join("Cargo.toml").exists());
        assert!(crates_dir.join("src/lib.rs").exists());
        assert!(crates_dir.join("cratify.toml").exists());

        let lib = fs::read_to_string(crates_dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("use core_contracts"));
        assert!(lib.contains("ComponentManifest"));

        let manifest = fs::read_to_string(crates_dir.join("cratify.toml")).unwrap();
        assert!(manifest.contains("test_gen"));
    }
}
