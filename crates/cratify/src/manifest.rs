// src/manifest.rs
//! Cratify manifest parser.
//! Parses a `cratify.toml` file that describes a Cratify ACC.
//! The manifest is a simple TOML document with a few required fields.
//! This module provides a `Manifest` struct and a `load` function returning
//! `anyhow::Result<Manifest>`.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Top‑level representation of a Cratify manifest.
#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    /// Human readable name of the ACC.
    pub name: String,
    /// Semantic version of the ACC (e.g., "0.1.0").
    pub version: String,
    /// Short description, used for documentation and help messages.
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
    ///
    /// The function reads the entire file, parses it as TOML and returns a
    /// `Manifest`. Errors are wrapped with context to aid debugging.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read manifest file {:?}", path.as_ref()))?;
        toml::from_str(&raw)
            .with_context(|| format!("Failed to parse manifest TOML at {:?}", path.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parses_minimal_manifest() {
        let mut file = NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            """
            name = \"example_acc\"
            version = \"0.1.0\"
            """
        )
        .unwrap();
        let manifest = Manifest::load(file.path()).expect("load manifest");
        assert_eq!(manifest.name, "example_acc");
        assert_eq!(manifest.version, "0.1.0");
        assert!(manifest.description.is_none());
        assert!(manifest.dependencies.is_empty());
    }
}
