//! Invariant Rule: Text Encoding and Line Endings
//!
//! Validates that all Git-tracked text files conform to UTF-8 encoding
//! and deterministic line endings (LF for text, CRLF for batch/cmd scripts).

#![deny(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Known binary extensions to exclude from text encoding rules.
pub const BINARY_EXTENSIONS: &[&str] = &[
    "png",
    "jpg",
    "jpeg",
    "gif",
    "ico",
    "wasm",
    "dll",
    "exe",
    "bin",
    "db",
    "gguf",
    "si",
    "sissm",
    "sovereign",
    "zip",
    "zst",
];

/// Structured violation record for encoding and line ending defects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingViolation {
    pub file_path: PathBuf,
    pub issue: String,
}

impl std::fmt::Display for EncodingViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file_path.display(), self.issue)
    }
}

/// Audit a single file's raw bytes for text encoding and line ending invariants.
/// Returns `None` if the file is classified as binary, or if it passes all checks.
pub fn audit_file_encoding(path: &Path, bytes: &[u8]) -> Option<EncodingViolation> {
    if is_binary_extension(path) {
        return None;
    }

    // Check UTF-8 validity
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => {
            return Some(EncodingViolation {
                file_path: path.to_path_buf(),
                issue: "Not valid UTF-8".to_string(),
            });
        }
    };

    // Check for UTF-8 BOM
    if bytes.starts_with(b"\xef\xbb\xbf") {
        return Some(EncodingViolation {
            file_path: path.to_path_buf(),
            issue: "UTF-8 BOM detected (must be UTF-8 without BOM)".to_string(),
        });
    }

    let is_bat_or_cmd = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd"));

    let is_powershell = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ps1"));

    let has_crlf = text.contains("\r\n");
    let has_lf = text.replace("\r\n", "").contains('\n');

    if is_bat_or_cmd {
        if has_lf {
            return Some(EncodingViolation {
                file_path: path.to_path_buf(),
                issue: "Expected CRLF line endings in batch script, found bare LF".to_string(),
            });
        }
    } else if !is_powershell && has_crlf {
        return Some(EncodingViolation {
            file_path: path.to_path_buf(),
            issue: "Expected LF line endings, found CRLF".to_string(),
        });
    }

    None
}

/// Helper to determine if a file is a binary artifact by extension.
pub fn is_binary_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            let lower = ext.to_lowercase();
            BINARY_EXTENSIONS.contains(&lower.as_str())
        })
}

/// Audits all Git-tracked files in the repository for encoding and line ending compliance.
pub fn audit_tracked_encodings(repo_root: &Path) -> Result<Vec<EncodingViolation>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files"])
        .output()
        .map_err(|e| format!("Failed to spawn git ls-files: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "git ls-files failed with status: {}",
            output.status
        ));
    }

    let files_str = std::str::from_utf8(&output.stdout)
        .map_err(|e| format!("git ls-files output is not valid UTF-8: {e}"))?;

    let mut violations = Vec::new();

    for line in files_str.lines() {
        if line.is_empty() {
            continue;
        }

        let rel_path = Path::new(line);
        if is_binary_extension(rel_path) {
            continue;
        }

        let full_path = repo_root.join(rel_path);
        let bytes = match fs::read(&full_path) {
            Ok(b) => b,
            Err(_) => continue, // Ignore deleted or unreadable files
        };

        if let Some(violation) = audit_file_encoding(rel_path, &bytes) {
            violations.push(violation);
        }
    }

    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_utf8_lf() {
        let path = Path::new("src/lib.rs");
        let bytes = b"pub fn hello() {}\n";
        assert!(audit_file_encoding(path, bytes).is_none());
    }

    #[test]
    fn test_invalid_utf8() {
        let path = Path::new("src/bad.rs");
        let bytes = b"let x = \xFF\xFE;";
        let violation = audit_file_encoding(path, bytes).expect("should fail UTF-8");
        assert!(violation.issue.contains("Not valid UTF-8"));
    }

    #[test]
    fn test_utf8_bom_detected() {
        let path = Path::new("src/bom.rs");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"pub fn bom() {}\n");
        let violation = audit_file_encoding(path, &bytes).expect("should fail BOM");
        assert!(violation.issue.contains("BOM"));
    }

    #[test]
    fn test_crlf_in_standard_file_fails() {
        let path = Path::new("Cargo.toml");
        let bytes = b"[package]\r\nname = \"foo\"\r\n";
        let violation = audit_file_encoding(path, bytes).expect("should fail CRLF");
        assert!(violation.issue.contains("Expected LF"));
    }

    #[test]
    fn test_crlf_in_bat_passes() {
        let path = Path::new("build.bat");
        let bytes = b"@echo off\r\necho building\r\n";
        assert!(audit_file_encoding(path, bytes).is_none());
    }

    #[test]
    fn test_bare_lf_in_bat_fails() {
        let path = Path::new("build.bat");
        let bytes = b"@echo off\necho building\r\n";
        let violation = audit_file_encoding(path, bytes).expect("should fail bare LF");
        assert!(violation.issue.contains("Expected CRLF"));
    }

    #[test]
    fn test_binary_extension_ignored() {
        let path = Path::new("asset.png");
        let bytes = b"\x89PNG\r\n\x1a\n\x00\x00\xFF";
        assert!(audit_file_encoding(path, bytes).is_none());
    }
}
