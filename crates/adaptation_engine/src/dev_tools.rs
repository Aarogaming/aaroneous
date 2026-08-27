//! crates/adaptation_engine/src/dev_tools.rs
//! Industrial-Grade Developer Power Tools, Diagnostic Parsing, and Patch Application Engine.
//! Provides workspace file tree exploration, compiler diagnostic extraction, and safe file patching with backup.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

/// File item in the workspace tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFileItem {
    pub path: PathBuf,
    pub relative_path: String,
    pub is_dir: bool,
    pub line_count: usize,
    pub file_extension: String,
}

/// Structured Compiler Diagnostic extracted from `cargo check`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerDiagnosticItem {
    pub message: String,
    pub level: String, // "error", "warning", "note"
    pub code: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub suggested_replacement: Option<String>,
}

/// Developer Workbench Engine
pub struct DevToolsEngine {
    workspace_root: PathBuf,
}

impl Default for DevToolsEngine {
    fn default() -> Self {
        Self {
            workspace_root: aaroneous_paths::WorkspacePaths::discover()
                .root()
                .to_path_buf(),
        }
    }
}

impl DevToolsEngine {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Recursively lists workspace files with line counts and file extensions
    pub fn scan_workspace_tree(&self, max_depth: usize) -> Vec<WorkspaceFileItem> {
        let mut items = Vec::new();
        self.scan_dir_recursive(&self.workspace_root, "", 0, max_depth, &mut items);
        items.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        items
    }

    fn scan_dir_recursive(
        &self,
        current_dir: &Path,
        rel_prefix: &str,
        depth: usize,
        max_depth: usize,
        items: &mut Vec<WorkspaceFileItem>,
    ) {
        if depth > max_depth {
            return;
        }

        if let Ok(entries) = fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Skip target, .git, and hidden directories
                if file_name == "target" || file_name == ".git" || file_name == ".gemini" {
                    continue;
                }

                let rel_path = if rel_prefix.is_empty() {
                    file_name.clone()
                } else {
                    format!("{}/{}", rel_prefix, file_name)
                };

                let is_dir = path.is_dir();
                let mut line_count = 0;
                let file_extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

                if !is_dir && (file_extension == "rs" || file_extension == "toml" || file_extension == "md" || file_extension == "py") {
                    if let Ok(file) = File::open(&path) {
                        line_count = BufReader::new(file).lines().count();
                    }
                }

                items.push(WorkspaceFileItem {
                    path: path.clone(),
                    relative_path: rel_path.clone(),
                    is_dir,
                    line_count,
                    file_extension,
                });

                if is_dir {
                    self.scan_dir_recursive(&path, &rel_path, depth + 1, max_depth, items);
                }
            }
        }
    }

    /// Runs `cargo check --message-format=json` and parses structured compiler diagnostics
    pub fn run_cargo_diagnostic_check(&self) -> Result<Vec<CompilerDiagnosticItem>> {
        let output = Command::new("cargo")
            .current_dir(&self.workspace_root)
            .args(["check", "--message-format=json", "-q"])
            .output()?;

        let mut diagnostics = Vec::new();
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if val["reason"] == "compiler-message" {
                    let msg_obj = &val["message"];
                    let message = msg_obj["message"].as_str().unwrap_or("").to_string();
                    let level = msg_obj["level"].as_str().unwrap_or("error").to_string();
                    let code = msg_obj["code"]["code"].as_str().map(|s| s.to_string());

                    let mut file_path = None;
                    let mut line_number = None;
                    let mut suggested_replacement = None;

                    if let Some(spans) = msg_obj["spans"].as_array() {
                        if let Some(primary_span) = spans.iter().find(|s| s["is_primary"].as_bool().unwrap_or(false)) {
                            file_path = primary_span["file_name"].as_str().map(|s| s.to_string());
                            line_number = primary_span["line_start"].as_u64().map(|n| n as usize);
                            suggested_replacement = primary_span["suggested_replacement"].as_str().map(|s| s.to_string());
                        }
                    }

                    diagnostics.push(CompilerDiagnosticItem {
                        message,
                        level,
                        code,
                        file_path,
                        line_number,
                        suggested_replacement,
                    });
                }
            }
        }

        Ok(diagnostics)
    }

    /// Safely writes updated code to disk with an automatic `.bak` backup
    pub fn apply_patch_to_file(&self, target_file: impl AsRef<Path>, new_content: &str) -> Result<PathBuf> {
        let path = target_file.as_ref();
        if !path.exists() {
            return Err(anyhow!("Target file does not exist: {}", path.display()));
        }

        // 1. Create backup
        let backup_path = path.with_extension(format!("bak.{}", crate::disassembly::BinaryInspector::calculate_entropy(new_content.as_bytes()) as u32));
        fs::copy(path, &backup_path)?;

        // 2. Write new content
        fs::write(path, new_content)?;

        Ok(backup_path)
    }

    /// Reverts a file from its backup
    pub fn revert_backup(&self, target_file: impl AsRef<Path>, backup_path: impl AsRef<Path>) -> Result<()> {
        let target = target_file.as_ref();
        let backup = backup_path.as_ref();
        if !backup.exists() {
            return Err(anyhow!("Backup file does not exist: {}", backup.display()));
        }
        fs::copy(backup, target)?;
        let _ = fs::remove_file(backup);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_workspace_tree() {
        let engine = DevToolsEngine::default();
        let items = engine.scan_workspace_tree(2);
        assert!(!items.is_empty());
        let has_crates = items.iter().any(|i| i.relative_path.starts_with("crates"));
        assert!(has_crates);
    }

    #[test]
    fn test_patch_application_and_backup_revert() {
        let temp_file = std::env::temp_dir().join(format!("dev_tool_test_{}.rs", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()));
        fs::write(&temp_file, "original content").unwrap();

        let engine = DevToolsEngine::default();
        let backup = engine.apply_patch_to_file(&temp_file, "patched content").unwrap();

        assert_eq!(fs::read_to_string(&temp_file).unwrap(), "patched content");
        assert!(backup.exists());

        // Revert
        engine.revert_backup(&temp_file, &backup).unwrap();
        assert_eq!(fs::read_to_string(&temp_file).unwrap(), "original content");

        let _ = fs::remove_file(temp_file);
    }
}
