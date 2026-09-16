//! Invariant Rule: Safety Comments for Unsafe Blocks
//!
//! Enforces documented safety rationale comments for unsafe blocks
//! across performance and microkernel modules.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use syn::ExprUnsafe;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyCommentViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub symbol: String,
}

impl std::fmt::Display for SafetyCommentViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: [VIOLATION] Unsafe block missing safety explanation comment.",
            self.file_path.display(),
            self.line
        )
    }
}

pub struct SafetyCommentVisitor<'a> {
    pub file_path: &'a Path,
    pub source_text: &'a str,
    pub violations: Vec<SafetyCommentViolation>,
}

impl<'a> SafetyCommentVisitor<'a> {
    pub fn new(file_path: &'a Path, source_text: &'a str) -> Self {
        Self {
            file_path,
            source_text,
            violations: Vec::new(),
        }
    }

    fn check_safety_comment<S: Spanned>(&mut self, node: &S) {
        let line = node.span().start().line;
        let lines: Vec<&str> = self.source_text.lines().collect();

        // Check 3 lines prior to unsafe block for SAFETY: comment
        let start_line = line.saturating_sub(4);
        let end_line = line.saturating_sub(1);

        let mut has_safety_comment = false;
        for l in start_line..=end_line {
            if let Some(text) = lines.get(l) {
                if text.contains("SAFETY:") || text.contains("Safety:") || text.contains("safety:")
                {
                    has_safety_comment = true;
                    break;
                }
            }
        }

        if !has_safety_comment {
            self.violations.push(SafetyCommentViolation {
                file_path: self.file_path.to_path_buf(),
                line,
                symbol: "unsafe".to_string(),
            });
        }
    }
}

impl<'ast> Visit<'ast> for SafetyCommentVisitor<'_> {
    fn visit_expr_unsafe(&mut self, node: &'ast ExprUnsafe) {
        self.check_safety_comment(node);
        visit::visit_expr_unsafe(self, node);
    }
}
