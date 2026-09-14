use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use toml_edit::Document;

/// Find the workspace root (directory containing Cargo.toml with a [workspace] table).
pub fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut dir = paths::normalize_path(start);
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
    if !members.iter().any(|v| v.as_str() == Some(rel.as_str())) {
        members.push(rel);
        fs::write(&cargo_path, doc.to_string())
            .with_context(|| format!("failed to write {:?}", cargo_path))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_workspace_root() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[workspace]\nmembers = []\n").unwrap();

        let sub_dir = dir.path().join("crates").join("my_crate");
        fs::create_dir_all(&sub_dir).unwrap();

        let found = find_workspace_root(&sub_dir).unwrap();
        assert_eq!(found, paths::normalize_path(dir.path()));
    }

    #[test]
    fn test_find_workspace_root_missing() {
        let dir = tempdir().unwrap();
        let sub_dir = dir.path().join("some").join("nested");
        fs::create_dir_all(&sub_dir).unwrap();

        let res = find_workspace_root(&sub_dir);
        assert!(res.is_err());
    }

    #[test]
    fn test_add_workspace_member() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        let initial_toml = r#"[workspace]
members = [
    "crates/one",
]
"#;
        fs::write(&cargo_toml, initial_toml).unwrap();

        let member_path = dir.path().join("crates").join("two");
        add_workspace_member(dir.path(), &member_path).unwrap();

        let updated = fs::read_to_string(&cargo_toml).unwrap();
        assert!(updated.contains("crates/two"));

        // Adding again should be idempotent
        add_workspace_member(dir.path(), &member_path).unwrap();
        let count = updated.matches("crates/two").count();
        assert_eq!(count, 1);
    }
}
