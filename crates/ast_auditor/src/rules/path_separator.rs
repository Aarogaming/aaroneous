//! Invariant Rule: Cross-Platform Path Separator Normalization
//!
//! Flags unnormalized Windows backslash `\` path separators in Rust string literals
//! to ensure cross-platform compatibility on Linux/macOS runners.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use syn::LitStr;
use syn::visit::{self, Visit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSeparatorViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub literal: String,
}

impl std::fmt::Display for PathSeparatorViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: [VIOLATION] Unnormalized path literal containing backslash `{}`. Use forward slashes `/` or `Path::join`.",
            self.file_path.display(),
            self.line,
            self.literal
        )
    }
}

pub struct PathSeparatorVisitor<'a> {
    pub file_path: &'a Path,
    pub violations: Vec<PathSeparatorViolation>,
}

impl<'a> PathSeparatorVisitor<'a> {
    pub fn new(file_path: &'a Path) -> Self {
        Self {
            file_path,
            violations: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for PathSeparatorVisitor<'_> {
    fn visit_lit_str(&mut self, node: &'ast LitStr) {
        let val = node.value();
        // Flag string literals containing backslashes in path-like patterns (e.g. "foo\bar.txt")
        if val.contains('\\')
            && (val.contains(".rs")
                || val.contains(".json")
                || val.contains(".toml")
                || val.contains(".wgsl")
                || val.contains("crates\\")
                || val.contains("core\\")
                || val.contains("dev\\"))
        {
            self.violations.push(PathSeparatorViolation {
                file_path: self.file_path.to_path_buf(),
                line: node.span().start().line,
                literal: val,
            });
        }
        visit::visit_lit_str(self, node);
    }
}
