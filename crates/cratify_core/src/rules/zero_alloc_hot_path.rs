//! Invariant Rule: Zero Allocation on Hot Paths
//!
//! Enforces zero-heap allocation inside mission-critical execution loops,
//! frame ingestors, and kernel step transitions.
//! Detects `#[hot_path]` attributes on functions or `#![hot_path]` at the file level,
//! verifying that no heap containers or formatting macros are invoked.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ExprCall, ExprMacro, ExprMethodCall, File, ItemFn, Meta};

/// Diagnostic violation record for hot-path heap allocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotPathAllocViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub operation: String,
    pub reason: String,
}

impl std::fmt::Display for HotPathAllocViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: [HOT_PATH_ALLOC] Heap allocation `{}` banned on real-time hot paths: {}",
            self.file_path.display(),
            self.line,
            self.column,
            self.operation,
            self.reason
        )
    }
}

/// Visitor that audits functions marked as hot paths for dynamic heap operations.
pub struct HotPathAllocVisitor<'a> {
    pub file_path: &'a Path,
    pub file_is_hot_path: bool,
    pub functions_scanned: usize,
    pub in_hot_path_scope: bool,
    pub violations: Vec<HotPathAllocViolation>,
}

impl<'a> HotPathAllocVisitor<'a> {
    pub fn new(file_path: &'a Path) -> Self {
        Self {
            file_path,
            file_is_hot_path: false,
            functions_scanned: 0,
            in_hot_path_scope: false,
            violations: Vec::new(),
        }
    }

    fn check_file_attributes(&mut self, attrs: &[Attribute]) {
        self.file_is_hot_path = self.has_hot_path_attr(attrs);
    }
    fn has_hot_path_attr(&self, attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| match &attr.meta {
            Meta::Path(p) => p.is_ident("hot_path"),
            Meta::NameValue(value) if value.path.is_ident("doc") => matches!(&value.value, syn::Expr::Lit(lit) if matches!(&lit.lit, syn::Lit::Str(text) if text.value().trim() == "hot_path")),
            _ => false,
        })
    }

    fn push_violation<S: Spanned>(&mut self, node: &S, operation: &str, reason: &str) {
        let span = node.span();
        let start = span.start();
        self.violations.push(HotPathAllocViolation {
            file_path: self.file_path.to_path_buf(),
            line: start.line,
            column: start.column + 1,
            operation: operation.to_string(),
            reason: reason.to_string(),
        });
    }

    fn check_banned_callee(&mut self, expr_call: &ExprCall) {
        if let syn::Expr::Path(expr_path) = &*expr_call.func {
            let segments: Vec<String> = expr_path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            let call_str = segments.join("::");

            let tail = segments
                .iter()
                .rev()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("::");
            let banned = matches!(
                tail.as_str(),
                "String::new"
                    | "String::from"
                    | "String::with_capacity"
                    | "Vec::new"
                    | "Vec::with_capacity"
                    | "Box::new"
                    | "Box::pin"
                    | "Mutex::new"
                    | "RwLock::new"
                    | "Arc::new"
                    | "Rc::new"
                    | "std::sync::Arc::new"
                    | "alloc::sync::Arc::new"
                    | "std::rc::Rc::new"
                    | "alloc::rc::Rc::new"
                    | "HashMap::new"
                    | "BTreeMap::new"
                    | "HashSet::new"
                    | "BTreeSet::new"
            );

            if banned {
                self.push_violation(
                    expr_call,
                    &format!("{call_str}()"),
                    "Dynamic heap allocation is prohibited on hot paths. Use pre-allocated slices, arrays, or ring buffers.",
                );
            }
        }
    }

    fn check_banned_method(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();
        let banned = matches!(
            method_name.as_str(),
            "to_string"
                | "to_owned"
                | "clone"
                | "into_boxed_slice"
                | "collect"
                | "unwrap"
                | "expect"
                | "lock"
                | "read"
                | "write"
        );

        if banned {
            self.push_violation(
                method_call,
                &format!(".{method_name}()"),
                "Method introduces potential heap allocations or cloning. Use zero-copy borrows on hot paths.",
            );
        }
    }

    fn check_banned_macro(&mut self, expr_macro: &ExprMacro) {
        let macro_name = expr_macro
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();

        let banned = matches!(
            macro_name.as_str(),
            "panic" | "format" | "vec" | "println" | "eprintln" | "print" | "eprint"
        );

        if banned {
            self.push_violation(
                expr_macro,
                &format!("{macro_name}!"),
                "Macro generates dynamic formatting strings or allocates heap buffers. Use zero-alloc tracing or fixed buffers.",
            );
        }
    }
}

impl<'ast> Visit<'ast> for HotPathAllocVisitor<'_> {
    fn visit_file(&mut self, node: &'ast File) {
        self.check_file_attributes(&node.attrs);
        visit::visit_file(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let prev_scope = self.in_hot_path_scope;
        if self.file_is_hot_path || self.has_hot_path_attr(&node.attrs) {
            self.in_hot_path_scope = true;
            self.functions_scanned += 1;
        }

        visit::visit_item_fn(self, node);
        self.in_hot_path_scope = prev_scope;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev_scope = self.in_hot_path_scope;
        if self.file_is_hot_path || self.has_hot_path_attr(&node.attrs) {
            self.in_hot_path_scope = true;
            self.functions_scanned += 1;
        }

        visit::visit_impl_item_fn(self, node);
        self.in_hot_path_scope = prev_scope;
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if self.in_hot_path_scope {
            self.check_banned_callee(node);
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if self.in_hot_path_scope {
            self.check_banned_method(node);
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        if self.in_hot_path_scope {
            self.check_banned_macro(node);
        }
        visit::visit_expr_macro(self, node);
    }
}
