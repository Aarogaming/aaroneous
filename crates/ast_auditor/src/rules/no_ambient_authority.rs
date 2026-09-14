//! Invariant Rule: No Ambient Authority
//!
//! Enforces zero-ambient-authority across workspace domain modules.
//! Bans direct access to ambient system environment state (`std::env::*`)
//! and implicit current working directory/path resolutions outside of explicitly
//! whitelisted entrypoints marked `#![allow(ambient_authority)]`.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, ExprCall, ExprMethodCall, File, ItemUse, Meta, Path as SynPath, UseGroup, UsePath,
    UseRename, UseTree,
};

/// Structured violation record for ambient authority breaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbientViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub symbol: String,
    pub remediation: String,
}

impl std::fmt::Display for AmbientViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: [VIOLATION] Ambient authority access `{}` is prohibited. Remediation: {}",
            self.file_path.display(),
            self.line,
            self.column,
            self.symbol,
            self.remediation
        )
    }
}

/// Visitor that inspects AST trees for banned `std::env` and ambient paths.
pub struct AmbientAuthorityVisitor<'a> {
    pub file_path: &'a Path,
    pub is_exempt: bool,
    pub violations: Vec<AmbientViolation>,
}

impl<'a> AmbientAuthorityVisitor<'a> {
    pub fn new(file_path: &'a Path) -> Self {
        Self {
            file_path,
            is_exempt: false,
            violations: Vec::new(),
        }
    }

    fn check_inner_attributes(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            if let Meta::List(list) = &attr.meta {
                if list.path.is_ident("allow") {
                    let tokens = list.tokens.to_string();
                    if tokens.contains("ambient_authority") {
                        self.is_exempt = true;
                        return;
                    }
                }
            }
        }
    }

    fn push_violation<S: Spanned>(&mut self, node: &S, symbol: &str, remediation: &str) {
        let span = node.span();
        let start = span.start();
        self.violations.push(AmbientViolation {
            file_path: self.file_path.to_path_buf(),
            line: start.line,
            column: start.column + 1,
            symbol: symbol.to_string(),
            remediation: remediation.to_string(),
        });
    }

    fn inspect_use_tree(&mut self, tree: &UseTree, prefix: &str) {
        match tree {
            UseTree::Path(UsePath { ident, tree, .. }) => {
                let current = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{prefix}::{ident}")
                };
                self.inspect_use_tree(tree, &current);
            }
            UseTree::Name(name) => {
                let full = if prefix.is_empty() {
                    name.ident.to_string()
                } else {
                    format!("{prefix}::{}", name.ident)
                };
                self.check_use_path(&full, name);
            }
            UseTree::Rename(UseRename { ident, .. }) => {
                let full = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{prefix}::{ident}")
                };
                self.check_use_path(&full, tree);
            }
            UseTree::Glob(glob) => {
                let full = format!("{prefix}::*");
                if prefix == "std::env" || prefix.starts_with("std::env::") {
                    self.push_violation(
                        glob,
                        &full,
                        "Do not wildcard import `std::env`. Pass environment configurations via typed structs.",
                    );
                }
            }
            UseTree::Group(UseGroup { items, .. }) => {
                for item in items {
                    self.inspect_use_tree(item, prefix);
                }
            }
        }
    }

    fn check_use_path<S: Spanned>(&mut self, path_str: &str, span_node: &S) {
        if path_str == "std::env" || path_str.starts_with("std::env::") {
            self.push_violation(
                span_node,
                path_str,
                "Forbidden import of `std::env`. Inject typed configuration structs at entrypoints.",
            );
        }
    }

    fn is_banned_env_call(&self, path: &SynPath) -> Option<String> {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let path_str = segments.join("::");

        // Flag std::env::* calls
        if path_str.starts_with("std::env::") || path_str.starts_with("env::") {
            if let Some(last) = segments.last() {
                if matches!(
                    last.as_str(),
                    "var"
                        | "var_os"
                        | "set_var"
                        | "remove_var"
                        | "temp_dir"
                        | "current_dir"
                        | "args"
                        | "args_os"
                ) {
                    return Some(path_str);
                }
            }
        }

        // Implicit root/cwd queries
        if path_str == "std::path::Path::new(\".\")" || path_str == "PathBuf::from(\".\")" {
            return Some(path_str);
        }

        None
    }
}

impl<'ast> Visit<'ast> for AmbientAuthorityVisitor<'_> {
    fn visit_file(&mut self, node: &'ast File) {
        self.check_inner_attributes(&node.attrs);
        if self.is_exempt {
            return;
        }
        visit::visit_file(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        if self.is_exempt {
            return;
        }
        self.inspect_use_tree(&node.tree, "");
        visit::visit_item_use(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if self.is_exempt {
            return;
        }

        if let syn::Expr::Path(expr_path) = &*node.func {
            if let Some(symbol) = self.is_banned_env_call(&expr_path.path) {
                self.push_violation(
                    node,
                    &symbol,
                    "Ambient environment/filesystem lookup detected. Receive explicit config parameters in constructors.",
                );
            }
        }

        visit::visit_expr_call(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let prev_exempt = self.is_exempt;
        self.check_inner_attributes(&node.attrs);
        visit::visit_item_mod(self, node);
        self.is_exempt = prev_exempt;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev_exempt = self.is_exempt;
        self.check_inner_attributes(&node.attrs);
        visit::visit_item_fn(self, node);
        self.is_exempt = prev_exempt;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev_exempt = self.is_exempt;
        self.check_inner_attributes(&node.attrs);
        visit::visit_impl_item_fn(self, node);
        self.is_exempt = prev_exempt;
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if self.is_exempt {
            return;
        }

        let method = node.method.to_string();
        if method == "canonicalize" {
            self.push_violation(
                node,
                &format!(".{method}()"),
                "Implicit filesystem canonicalization creates ambient I/O dependencies. Provide normalized paths via WorkspacePathsConfig.",
            );
        }

        visit::visit_expr_method_call(self, node);
    }
}
