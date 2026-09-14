use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Top‑level representation of a Cratify manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Manifest {
    /// Human readable name of the ACC.
    pub name: String,
    /// Semantic version of the ACC (e.g., "0.1.0").
    pub version: String,
    /// Short description, used for documentation and help messages.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional list of dependent crate names.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Optional map of feature flags.
    #[serde(default)]
    pub features: std::collections::HashMap<String, Vec<String>>,
}

impl Manifest {
    /// Load and parse a manifest from `path`.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read manifest file {:?}", path.as_ref()))?;
        Self::parse(&raw)
            .with_context(|| format!("Failed to parse manifest TOML at {:?}", path.as_ref()))
    }

    /// Parse manifest from TOML string.
    pub fn parse(content: &str) -> Result<Self> {
        toml::from_str(content).context("failed to parse TOML manifest")
    }

    /// Serialize manifest to a TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string(self).context("failed to serialize manifest to TOML")
    }
}

/// Emit cratify.toml manifest for a new crate.
pub fn emit_manifest(name: &str, crate_dir: &Path) -> Result<()> {
    crate::generator::emit_cratify_toml(name, crate_dir)
}

/// Parse and validate a cratify.toml manifest from file.
pub fn parse_manifest(path: &Path) -> Result<Manifest> {
    Manifest::load(path)
}

/// Parse a cratify.toml manifest from string.
pub fn parse_manifest_str(content: &str) -> Result<Manifest> {
    Manifest::parse(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_minimal_manifest() {
        let raw = r#"
name = "example_acc"
version = "0.1.0"
"#;
        let manifest = Manifest::parse(raw).expect("load manifest");
        assert_eq!(manifest.name, "example_acc");
        assert_eq!(manifest.version, "0.1.0");
        assert!(manifest.description.is_none());
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_load_and_emit_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        emit_manifest("test_acc", path).unwrap();

        let manifest = parse_manifest(&path.join("cratify.toml")).unwrap();
        assert_eq!(manifest.name, "test_acc");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.dependencies, vec!["core-contracts"]);
    }
}
