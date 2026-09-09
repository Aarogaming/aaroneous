use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use toml_edit::Document;

/// Find the workspace root (directory containing Cargo.toml with a [workspace] table).
pub fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut dir = start
        .canonicalize()
        .with_context(|| format!("failed to canonicalize path {:?}", start))?;
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            let txt = fs::read_to_string(&cargo)
                .with_context(|| format!("failed to read {:?}", cargo))?;
            if txt.contains("[workspace]") {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("workspace root not found (no Cargo.toml with [workspace] above {:?})", start)
}

/// Insert a new member into the root Cargo.toml if it does not already exist.
pub fn add_workspace_member(root: &Path, member_path: &Path) -> Result<()> {
    let cargo_path = root.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_path)
        .with_context(|| format!("failed to read {:?}", cargo_path))?;
    let mut doc = content
        .parse::<Document>()
        .with_context(|| format!("failed to parse {:?} as TOML", cargo_path))?;
    let members = doc["workspace"]["members"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("missing [workspace].members array in {:?}", cargo_path))?;
    let rel = member_path
        .strip_prefix(root)
        .with_context(|| format!("{:?} is not under workspace root {:?}", member_path, root))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("path {:?} is not valid UTF-8", member_path))?
        .replace('\\', "/");
    if !members.iter().any(|v| v.as_str() == Some(&rel)) {
        members.push(rel);
        fs::write(&cargo_path, doc.to_string())
            .with_context(|| format!("failed to write {:?}", cargo_path))?;
    }
    Ok(())
}
