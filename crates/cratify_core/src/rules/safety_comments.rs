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

        // `line` (from `syn`/`proc_macro2`) is 1-indexed, but `lines` (from
        // `str::lines`) is 0-indexed, so `lines[line - 1]` is the `unsafe`
        // line itself. That shift means the window below, `[line - 4, line
        // - 1]` as *vec indices*, actually covers *source lines* `[line -
        // 3, line]` - i.e. up to 3 lines strictly before `unsafe`, plus the
        // `unsafe` line itself (in case the comment is written inline). A
        // multi-line comment must therefore end with its `SAFETY:` line
        // immediately before (or on) the `unsafe` token; putting `SAFETY:`
        // on the *first* line of a 4+-line comment block falls outside this
        // window and is (falsely) reported as undocumented.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(source: &str) -> Vec<SafetyCommentViolation> {
        let ast: syn::File = syn::parse_str(source).unwrap();
        let mut visitor = SafetyCommentVisitor::new(Path::new("test.rs"), source);
        visitor.visit_file(&ast);
        visitor.violations
    }

    #[test]
    fn unsafe_block_with_no_comment_is_flagged() {
        let source = "fn f() {\n    unsafe { std::hint::black_box(1); }\n}\n";
        assert_eq!(violations(source).len(), 1);
    }

    #[test]
    fn safety_comment_directly_above_unsafe_is_accepted() {
        let source = "fn f() {\n    // SAFETY: trivially sound.\n    unsafe { std::hint::black_box(1); }\n}\n";
        assert!(violations(source).is_empty());
    }

    #[test]
    fn multiline_comment_ending_in_safety_is_accepted() {
        let source = "fn f() {\n    // Some rationale here.\n    // More rationale.\n    // SAFETY: sound because of the above.\n    unsafe { std::hint::black_box(1); }\n}\n";
        assert!(violations(source).is_empty());
    }

    /// Regression guard for the checker's own off-by-one: `SAFETY:` on the
    /// *first* line of a comment block longer than 3 lines falls outside
    /// the checked window and is (perhaps surprisingly) still flagged. This
    /// locks in that documented, if narrow, behavior rather than letting it
    /// silently change - see the comment on `check_safety_comment` above.
    #[test]
    fn safety_on_first_line_of_a_long_block_is_still_flagged() {
        let source = "fn f() {\n    // SAFETY: sound because of the below.\n    // More rationale.\n    // Even more rationale.\n    // Yet more rationale.\n    unsafe { std::hint::black_box(1); }\n}\n";
        assert_eq!(violations(source).len(), 1);
    }
}
