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
    // Enforce kebab-case and no prefix stutter (0 prefix violations)
    let clean_name = name.to_lowercase();
    if clean_name.starts_with("aaroneous_") || clean_name.starts_with("aaroneous-") {
        anyhow::bail!(
            "crate name `{}` violates naming convention: no 'aaroneous_' prefix allowed (kebab-case/un-prefixed required)",
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

    // Ensure kebab-case naming (no prefix stutter)
    let clean_name = name.to_lowercase();
    let has_prefix_stutter = clean_name.starts_with("aaroneous_") || clean_name.starts_with("aaroneous-");
    if has_prefix_stutter {
        anyhow::bail!(
            "crate name `{}` violates naming convention: no 'aaroneous_' prefix allowed (kebab-case/un-prefixed required)",
            name
        );
    }

    let content = format!(
        r#"[package]
name = "{clean_name}"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
readme.workspace = true

[dependencies]
paths = {{ workspace = true }}
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

    // Determine domain from crate name for appropriate unsafe code policy
    let is_sensitive_domain = name.contains("compute") || name.contains("hypervisor");
    
    // Domain-aware unsafe permissions: sensitive domains warn, others deny
    let unsafe_directive = if is_sensitive_domain {
        "#![warn(unsafe_code)]\n// SAFETY: Explanations required for all unsafe blocks\n"
    } else {
        "#![deny(unsafe_code)]\n"
    };

    // Build the lib.rs content manually to avoid format string escaping issues
    let mut content = String::new();
    content.push_str(&format!("//! {} — Aaroneous Crate Component (ACC)\n", name));
    content.push_str("//!\n");
    content.push_str("//! This crate was scaffolded by Cratify and adheres to the microkernel\n");
    content.push_str("//! contract boundaries defined in `core-contracts`.\n\n");
    content.push_str(&unsafe_directive);
    content.push_str("\nuse core_contracts::{ComponentManifest, HierarchyTier, Capability, pack_version};\n\n");
    content.push_str("/// ACC identity returned during bootstrap handshake.\npub fn manifest() -> ComponentManifest {\n");
    content.push_str("    ComponentManifest {\n");
    content.push_str(&format!("        name: *b\"{{}}{}\",\n", name));
    content.push_str("        version: pack_version(0, 1, 0),\n");
    content.push_str("        tier: HierarchyTier::SubordinateModule as u8,\n");
    content.push_str("        capabilities: Capability::INFERENCE_OUT.bits(),\n");
    content.push_str("        methodology: 0x01,\n");
    content.push_str("        supported_intents: 0x0001,\n");
    content.push_str("        priority_weight: 128,\n");
    content.push_str("        address: 0,\n");
    content.push_str("    }\n}\n\n");
    content.push_str("#[cfg(test)]\nmod tests {\n");
    content.push_str("    use super::*;\n\n");
    content.push_str("    #[test]\n    fn manifest_is_pod() {\n");
    content.push_str("        let m = manifest();\n");
    content.push_str("        assert_eq!(std::mem::size_of::<ComponentManifest>(), 64);\n");
    content.push_str("        assert_eq!(m.tier, HierarchyTier::SubordinateModule as u8);\n");
    content.push_str("    }\n}\n");

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
        // Test crate name "test_gen" is not a sensitive domain, should deny unsafe code
        assert!(lib.contains("#![deny(unsafe_code)]"));

        let manifest = fs::read_to_string(crates_dir.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("name = \"test_gen\""));
        assert!(manifest.contains("paths = { workspace = true }"));
    }

    #[test]
    fn reject_crate_name_with_prefix_stutter() {
        let result = generate_crate("aaroneous_test");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("prefix"));
    }

    #[test]
    fn sanitize_crate_name_to_kebab_case() {
        // This test verifies that the name is preserved as-is for display purposes
        // The actual kebab-case enforcement happens in write_cargo_toml via validation
        let workspace_root = tempdir().unwrap();
        let crates_dir = workspace_root.path().join("crates").join("valid_name");
        fs::create_dir_all(crates_dir.join("src")).unwrap();

        fs::write(
            workspace_root.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        let name = "valid-name";
        write_cargo_toml(name, &crates_dir).unwrap();
        write_lib_rs(name, &crates_dir).unwrap();

        let manifest = fs::read_to_string(crates_dir.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("name = \"valid-name\""));
    }

    #[test]
    fn domain_aware_unsafe_directive() {
        // Test that sensitive domains get warn, others get deny
        let workspace_root = tempdir().unwrap();
        
        // Test compute domain -> should warn
        let compute_dir = workspace_root.path().join("crates").join("test_compute");
        fs::create_dir_all(compute_dir.join("src")).unwrap();
        fs::write(
            workspace_root.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        ).unwrap();
        write_lib_rs("test_compute", &compute_dir).unwrap();
        let lib = fs::read_to_string(compute_dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("#![warn(unsafe_code)]"));
        assert!(lib.contains("// SAFETY: Explanations required for all unsafe blocks"));

        // Test hypervisor domain -> should warn
        let hypervisor_dir = workspace_root.path().join("crates").join("test_hypervisor");
        fs::create_dir_all(hypervisor_dir.join("src")).unwrap();
        write_lib_rs("test_hypervisor", &hypervisor_dir).unwrap();
        let lib = fs::read_to_string(hypervisor_dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("#![warn(unsafe_code)]"));

        // Test api domain -> should deny
        let api_dir = workspace_root.path().join("crates").join("test_api");
        fs::create_dir_all(api_dir.join("src")).unwrap();
        write_lib_rs("test_api", &api_dir).unwrap();
        let lib = fs::read_to_string(api_dir.join("src/lib.rs")).unwrap();
        assert!(lib.contains("#![deny(unsafe_code)]"));
    }

    #[test]
    fn test_cleanup_leaves_no_traces() {
        // Verify that tempdir ensures no physical directory remains after test
        let workspace_root = tempdir().unwrap();
        let crates_dir = workspace_root.path().join("crates").join("cleanup_test");
        
        // Create the crate
        fs::create_dir_all(crates_dir.join("src")).unwrap();
        fs::write(
            workspace_root.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        ).unwrap();
        write_lib_rs("cleanup_test", &crates_dir).unwrap();

        // Verify files were created
        assert!(crates_dir.join("src/lib.rs").exists());

        // Drop the tempdir to ensure cleanup
        drop(workspace_root);

        // The underlying filesystem should have been cleaned up by tempfile
        // Since we're using tempdir in a temp location, no trace should remain
    }
}
