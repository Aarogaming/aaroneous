use anyhow::Result;
use std::path::{Path, PathBuf};

/// Create a complete crate scaffold at the given workspace root.
pub fn scaffold_crate_at(name: &str, workspace_root: &Path) -> Result<PathBuf> {
    crate::generator::create_crate_scaffold_at(name, workspace_root)
}

/// Create a complete crate scaffold using discovered workspace root.
pub fn scaffold_crate(name: &str) -> Result<PathBuf> {
    crate::generator::create_crate_scaffold(name)
}

/// Emit a standalone Cargo.toml without workspace inheritance.
pub fn emit_standalone_cargo_toml(name: &str, crate_dir: &Path) -> Result<()> {
    let path = crate_dir.join("Cargo.toml");
    if path.exists() {
        return Ok(());
    }

    let content = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
core-contracts = {{ path = "../core-contracts" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"
"#
    );

    std::fs::write(&path, content)
        .map_err(|e| anyhow::anyhow!("failed to write {}: {}", path.display(), e))?;
    Ok(())
}

/// Emit a minimal lib.rs for standalone crates.
pub fn emit_standalone_lib_rs(name: &str, crate_dir: &Path) -> Result<()> {
    let path = crate_dir.join("src").join("lib.rs");
    if path.exists() {
        return Ok(());
    }

    let content = format!(
        r#"//! `{name}` — Standalone Crate Component
//!
//! This crate was scaffolded by Cratify.

pub fn version() -> &'static str {{
    "0.1.0"
}}

pub fn name() -> &'static str {{
    "{name}"
}}
"#
    );

    std::fs::write(&path, content)
        .map_err(|e| anyhow::anyhow!("failed to write {}: {}", path.display(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scaffold_standalone() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        std::fs::create_dir_all(path.join("src")).unwrap();

        emit_standalone_cargo_toml("standalone_crate", path).unwrap();
        emit_standalone_lib_rs("standalone_crate", path).unwrap();

        assert!(path.join("Cargo.toml").exists());
        assert!(path.join("src/lib.rs").exists());
    }
}
