//! Invariant Rule: Panics on Runtime Input
//!
//! AGENTS.md section 5 bans `.unwrap()`, `.expect(...)`, `panic!(...)`, and
//! `assert!(...)` on values derived from I/O, config, or model output,
//! except in tests, bootstrap entrypoints, `build.rs`, `debug_assert!`, and
//! call sites marked `// INFALLIBLE: <reason>` on the line immediately
//! above.
//!
//! This module counts occurrences of the four banned forms so that
//! `cargo xtask check-unwraps` can enforce a ratchet baseline over them (see
//! `xtask/src/check_unwraps.rs`). It deliberately does not attempt to prove
//! that a given call site actually receives runtime-derived input versus a
//! provably-infallible local invariant - narrowing that distinction is what
//! the `// INFALLIBLE:` escape hatch and human review are for. Skipping test
//! code is handled two ways: callers exclude whole files by path (`tests/`
//! directories, `*_test.rs`, `*_tests.rs`, `examples/`, `benches/`), and
//! this visitor additionally skips any `#[cfg(test)]`- or `#[test]`-annotated
//! item's subtree, so an inline `#[cfg(test)] mod tests { ... }` block
//! embedded in an otherwise-scanned file is not counted.

use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ImplItemFn, ItemFn, ItemMod, TraitItemFn};

/// One occurrence of a banned panicking form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrapPanicHit {
    pub file_path: PathBuf,
    pub line: usize,
    pub kind: &'static str,
}

pub struct UnwrapPanicVisitor<'a> {
    pub file_path: &'a Path,
    source_lines: Vec<&'a str>,
    pub hits: Vec<UnwrapPanicHit>,
    /// Nesting depth of `#[cfg(test)]` / `#[test]`-annotated items we are
    /// currently inside. Zero means "count normally."
    test_depth: usize,
}

impl<'a> UnwrapPanicVisitor<'a> {
    pub fn new(file_path: &'a Path, source_text: &'a str) -> Self {
        Self {
            file_path,
            source_lines: source_text.lines().collect(),
            hits: Vec::new(),
            test_depth: 0,
        }
    }

    /// True when the source line immediately preceding `node`'s span starts
    /// (after leading whitespace) with `// INFALLIBLE:`, matching AGENTS.md's
    /// documented exemption.
    fn has_infallible_marker<S: Spanned>(&self, node: &S) -> bool {
        let line = node.span().start().line; // 1-indexed
        if line < 2 {
            return false;
        }
        // Source line `line - 1` (1-indexed) is `self.source_lines[line - 2]`
        // (0-indexed).
        self.source_lines
            .get(line - 2)
            .is_some_and(|text| text.trim_start().starts_with("// INFALLIBLE:"))
    }

    fn record<S: Spanned>(&mut self, node: &S, kind: &'static str) {
        if self.test_depth > 0 {
            return;
        }
        if self.has_infallible_marker(node) {
            return;
        }
        self.hits.push(UnwrapPanicHit {
            file_path: self.file_path.to_path_buf(),
            line: node.span().start().line,
            kind,
        });
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

/// True if `attrs` contains `#[cfg(test)]` or a bare `#[test]`.
fn is_test_item(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("test") {
            return true;
        }
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

impl<'ast> Visit<'ast> for UnwrapPanicVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_item_mod(this, node));
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_item_fn(this, node));
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_impl_item_fn(this, node));
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_trait_item_fn(this, node));
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method = node.method.to_string();
        match method.as_str() {
            "unwrap" => self.record(node, "unwrap"),
            "expect" => self.record(node, "expect"),
            _ => {}
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(seg) = node.path.segments.last() {
            match seg.ident.to_string().as_str() {
                "panic" => self.record(node, "panic"),
                "assert" => self.record(node, "assert"),
                _ => {}
            }
        }
        visit::visit_macro(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(source: &str) -> Vec<UnwrapPanicHit> {
        let ast: syn::File = syn::parse_str(source).unwrap();
        let mut visitor = UnwrapPanicVisitor::new(Path::new("test.rs"), source);
        visitor.visit_file(&ast);
        visitor.hits
    }

    #[test]
    fn counts_unwrap_expect_panic_assert() {
        let source = r#"
fn f(x: Option<i32>, y: Result<i32, ()>) {
    let _ = x.unwrap();
    let _ = y.expect("boom");
    if x.is_none() {
        panic!("no value");
    }
    assert!(x.is_some());
}
"#;
        let found = hits(source);
        assert_eq!(found.len(), 4);
        let kinds: Vec<&str> = found.iter().map(|h| h.kind).collect();
        assert_eq!(kinds, vec!["unwrap", "expect", "panic", "assert"]);
    }

    #[test]
    fn does_not_count_unwrap_or_variants() {
        let source = r#"
fn f(x: Option<i32>) -> i32 {
    x.unwrap_or(0) + x.unwrap_or_else(|| 1) + x.unwrap_or_default()
}
"#;
        assert!(hits(source).is_empty());
    }

    #[test]
    fn does_not_count_debug_assert() {
        let source = r#"
fn f(x: i32) {
    debug_assert!(x > 0);
}
"#;
        assert!(hits(source).is_empty());
    }

    #[test]
    fn skips_cfg_test_module() {
        let source = r#"
fn production(x: Option<i32>) -> i32 {
    x.unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let x: Option<i32> = Some(1);
        assert!(x.unwrap() == 1);
    }
}
"#;
        assert!(hits(source).is_empty());
    }

    #[test]
    fn skips_bare_test_attribute_function() {
        let source = r#"
#[test]
fn t() {
    let x: Option<i32> = Some(1);
    x.unwrap();
}
"#;
        assert!(hits(source).is_empty());
    }

    #[test]
    fn counts_outside_test_module_in_same_file() {
        let source = r#"
fn production(x: Option<i32>) -> i32 {
    x.unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let x: Option<i32> = Some(1);
        x.unwrap();
    }
}
"#;
        let found = hits(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "unwrap");
    }

    #[test]
    fn infallible_marker_suppresses_the_next_line() {
        let source = r#"
fn f(x: Option<i32>) -> i32 {
    // INFALLIBLE: constructed two lines above from a literal.
    x.unwrap()
}
"#;
        assert!(hits(source).is_empty());
    }

    #[test]
    fn infallible_marker_covers_every_call_on_the_marked_line() {
        let source = r#"
fn f(x: Option<i32>, y: Option<i32>) -> i32 {
    // INFALLIBLE: constructed two lines above from a literal.
    x.unwrap() + y.unwrap()
}
"#;
        // Both `.unwrap()` calls share the marked line's line number, so
        // both are suppressed - the exemption is line-granular, not
        // node-granular.
        assert!(hits(source).is_empty());
    }

    #[test]
    fn infallible_marker_does_not_suppress_a_later_unmarked_line() {
        let source = r#"
fn f(x: Option<i32>, y: Option<i32>) -> i32 {
    // INFALLIBLE: constructed two lines above from a literal.
    let a = x.unwrap();
    let b = y.unwrap();
    a + b
}
"#;
        let found = hits(source);
        assert_eq!(found.len(), 1);
    }
}
