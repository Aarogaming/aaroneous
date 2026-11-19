//! Counting rule: ambient wall-clock reads through `chrono`.
//!
//! AGENTS.md section 2 ("No Ambient Clock") bans `SystemTime::now()` and
//! `Instant::now()` in library code. `chrono::Utc::now()` and
//! `chrono::Local::now()` are the same defect behind a different type, and
//! `chrono` is unmaintained (RUSTSEC-2026-0543; see
//! `docs/CHRONO_DECISION_2026-11-18.md`). This module counts the call sites
//! so `cargo xtask check-chrono` can hold a per-package baseline that may
//! only shrink.
//!
//! A hit is any path expression whose last two segments are `Utc::now` or
//! `Local::now`. Matching on the path (not only on calls) also counts the
//! function-value form, `.unwrap_or_else(Utc::now)`. Both the bare
//! `Utc::now()` and the fully qualified `chrono::Utc::now()` match.
//!
//! Like `unwrap_panic`, the visitor skips `#[cfg(test)]` and `#[test]`
//! subtrees; callers exclude whole test files by path.
//!
//! Known limit: macro bodies are token streams that `syn` does not parse. As
//! a best effort the visitor re-parses a macro body as comma-separated
//! expressions (which covers `format!`, `println!`, `tracing::info!` argument
//! lists and `vec![]`), and silently skips bodies that do not parse that way
//! (for example `json!({ ... })`). A clock read hidden in such a body is not
//! counted.

use std::path::{Path, PathBuf};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ExprPath, ImplItemFn, ItemFn, ItemMod, Token, TraitItemFn};

use super::unwrap_panic::is_test_item;

/// One ambient wall-clock read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbientClockHit {
    pub file_path: PathBuf,
    pub line: usize,
    /// `"Utc::now"` or `"Local::now"`.
    pub kind: &'static str,
}

pub struct AmbientClockVisitor<'a> {
    pub file_path: &'a Path,
    pub hits: Vec<AmbientClockHit>,
    /// Nesting depth of `#[cfg(test)]` / `#[test]` items we are inside.
    test_depth: usize,
}

impl<'a> AmbientClockVisitor<'a> {
    pub fn new(file_path: &'a Path) -> Self {
        Self {
            file_path,
            hits: Vec::new(),
            test_depth: 0,
        }
    }

    fn enter_if_test<F: FnOnce(&mut Self)>(&mut self, attrs: &[Attribute], visit_inner: F) {
        let is_test = is_test_item(attrs);
        if is_test {
            self.test_depth += 1;
        }
        visit_inner(self);
        if is_test {
            self.test_depth -= 1;
        }
    }
}

impl<'ast> Visit<'ast> for AmbientClockVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        self.enter_if_test(&node.attrs, |this| visit::visit_item_mod(this, node));
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.enter_if_test(&node.attrs, |this| visit::visit_item_fn(this, node));
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.enter_if_test(&node.attrs, |this| visit::visit_impl_item_fn(this, node));
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        self.enter_if_test(&node.attrs, |this| visit::visit_trait_item_fn(this, node));
    }

    fn visit_expr_path(&mut self, node: &'ast ExprPath) {
        let segments = &node.path.segments;
        if self.test_depth == 0 && segments.len() >= 2 {
            let last = &segments[segments.len() - 1].ident;
            let owner = &segments[segments.len() - 2].ident;
            if last == "now" {
                let kind = if owner == "Utc" {
                    Some("Utc::now")
                } else if owner == "Local" {
                    Some("Local::now")
                } else {
                    None
                };
                if let Some(kind) = kind {
                    self.hits.push(AmbientClockHit {
                        file_path: self.file_path.to_path_buf(),
                        line: node.span().start().line,
                        kind,
                    });
                }
            }
        }
        visit::visit_expr_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Ok(args) = node.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
            for expr in &args {
                self.visit_expr(expr);
            }
        }
        visit::visit_macro(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(source: &str) -> Vec<AmbientClockHit> {
        let ast: syn::File = syn::parse_str(source).unwrap();
        let mut visitor = AmbientClockVisitor::new(Path::new("test.rs"));
        visitor.visit_file(&ast);
        visitor.hits
    }

    fn kinds(source: &str) -> Vec<&'static str> {
        hits(source).iter().map(|h| h.kind).collect()
    }

    #[test]
    fn counts_bare_and_qualified_calls() {
        let source = r#"
use chrono::{Local, Utc};
fn f() {
    let _a = Utc::now();
    let _b = chrono::Utc::now();
    let _c = Local::now();
    let _d = chrono::Local::now();
}
"#;
        assert_eq!(
            kinds(source),
            vec!["Utc::now", "Utc::now", "Local::now", "Local::now"]
        );
    }

    #[test]
    fn counts_function_value_form() {
        let source = r#"
fn f(x: Option<chrono::DateTime<chrono::Utc>>) -> chrono::DateTime<chrono::Utc> {
    x.unwrap_or_else(chrono::Utc::now)
}
"#;
        assert_eq!(kinds(source), vec!["Utc::now"]);
    }

    #[test]
    fn ignores_other_types_and_other_methods() {
        let source = r#"
fn f() {
    let _a = std::time::Instant::now();
    let _b = std::time::SystemTime::now();
    let _c = Utc::today();
    let _d = Utc.timestamp_opt(0, 0);
    let _e = now();
}
"#;
        assert!(hits(source).is_empty());
    }

    #[test]
    fn skips_cfg_test_module_and_test_fn() {
        let source = r#"
fn production() {
    let _ = chrono::Utc::now();
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let _ = chrono::Utc::now();
    }
}

#[test]
fn bare_test() {
    let _ = chrono::Local::now();
}
"#;
        let found = hits(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "Utc::now");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn counts_inside_comma_separated_macro_arguments() {
        let source = r#"
fn f() {
    let _ = format!("{}", chrono::Utc::now());
    println!("{} {}", 1, chrono::Local::now());
}
"#;
        assert_eq!(kinds(source), vec!["Utc::now", "Local::now"]);
    }

    #[test]
    fn unparsable_macro_bodies_are_skipped_not_fatal() {
        let source = r#"
fn f() {
    let _ = serde_json::json!({ "at": chrono::Utc::now().to_rfc3339() });
}
"#;
        // Documented limit: `json!({ ... })` is not a comma-separated
        // expression list, so the read inside it is not counted.
        assert!(hits(source).is_empty());
    }
}
