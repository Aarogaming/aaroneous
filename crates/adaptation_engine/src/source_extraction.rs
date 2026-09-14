// src/source_extraction.rs
//! Universal source tree extraction engine.
//! Walks arbitrary source trees, parses source files via `inspect`, and produces `CrateSpec` items.
//! The engine is intentionally generic: callers can specify which file extensions to treat as source, and which AST extraction strategy to use.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use ast_auditor::inspect;

/// Strategy for extracting AST information from a source file.
#[derive(Debug, Clone, Copy)]
pub enum ParseStrategy {
    /// Use the canonical `inspect::inspect_code` implementation.
    Inspect,
}

/// Configuration for the source tree extraction process.
#[derive(Debug, Clone)]
pub struct SourceExtractionConfig {
    /// File extensions that should be considered source files (without leading dot).
    pub extensions: Vec<String>,
    /// Which strategy to employ for AST extraction.
    pub strategy: ParseStrategy,
}

/// Backward-compatible alias for `SourceExtractionConfig`.
pub type HarvestConfig = SourceExtractionConfig;

impl Default for SourceExtractionConfig {
    fn default() -> Self {
        Self {
            extensions: vec!["rs".to_string()],
            strategy: ParseStrategy::Inspect,
        }
    }
}

/// Minimal representation of a generated crate.
#[derive(Debug, Clone)]
pub struct CrateSpec {
    pub name: String,
    pub source_path: PathBuf,
    /// Unified AST representation regardless of the chosen strategy.
    pub code_info: CodeInfo,
}

/// Unified AST abstraction used by extraction results.
#[derive(Debug, Clone)]
pub struct CodeInfo {
    pub functions: Vec<inspect::FunctionInfo>,
    pub structs: Vec<inspect::StructInfo>,
    pub enums: Vec<inspect::EnumInfo>,
}

/// Extract components from a source directory tree using the supplied configuration.
/// Returns a vector of `CrateSpec` – one per matching file discovered.
pub fn extract_source_tree<P: AsRef<Path>>(src: P, config: &SourceExtractionConfig) -> Result<Vec<CrateSpec>> {
    let mut specs = Vec::new();
    for entry in WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if config.extensions.iter().any(|e| e == ext) {
                    let code_info = match config.strategy {
                        ParseStrategy::Inspect => {
                            let info = inspect::inspect_code(path).with_context(|| format!("Inspect failed on {:?}", path))?;
                            CodeInfo {
                                functions: info.functions,
                                structs: info.structs,
                                enums: info.enums,
                            }
                        }
                    };
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                    specs.push(CrateSpec { name, source_path: path.to_path_buf(), code_info });
                }
            }
        }
    }
    Ok(specs)
}

/// Backward-compatible alias for `extract_source_tree`.
pub use extract_source_tree as harvest;

/// Convenience wrapper using the default configuration (Rust files, parser).
pub fn extract_source_path<P: AsRef<Path>>(src: P) -> Result<Vec<CrateSpec>> {
    extract_source_tree(src, &SourceExtractionConfig::default())
}

/// Backward-compatible alias for `extract_source_path`.
pub use extract_source_path as harvest_path;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_extract_source_path_simple() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("example.rs");
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "pub fn hello() -> u32 {{ 42 }}\npub struct Data {{ id: u64 }}").unwrap();
        let specs = extract_source_path(dir.path()).expect("extract_source_path");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "example");
        assert_eq!(specs[0].code_info.functions.len(), 1);
        assert_eq!(specs[0].code_info.structs.len(), 1);
    }
}
