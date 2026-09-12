//! Automated Ambient AST Rewriter
//!
//! Transforms ambient authority operations (`path.canonicalize()`, `std::env::*`, `std::fs::*`)
//! into capability-passing and deterministic workspace primitives (`paths::normalize_path`, explicit configs).

use std::collections::HashSet;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, ExprCall, ExprMethodCall, File, Item, ItemUse};

/// Summary of AST rewriting transformations applied.
#[derive(Debug, Clone, Default)]
pub struct RewriteSummary {
    pub canonicalize_rewrites: usize,
    pub env_temp_dir_rewrites: usize,
    pub env_current_dir_rewrites: usize,
    pub env_var_rewrites: usize,
    pub injected_imports: HashSet<String>,
}

/// AST Visitor and Rewriter for purging ambient authority.
pub struct AmbientAstRewriter {
    pub summary: RewriteSummary,
    injected_normalize_path: bool,
    injected_workspace_paths: bool,
}

impl Default for AmbientAstRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientAstRewriter {
    pub fn new() -> Self {
        Self {
            summary: RewriteSummary::default(),
            injected_normalize_path: false,
            injected_workspace_paths: false,
        }
    }

    /// Rewrite a raw Rust source code string, returning the remediated code and rewrite summary.
    pub fn rewrite_source(&mut self, source: &str) -> Result<(String, RewriteSummary), syn::Error> {
        let mut syntax_tree: File = syn::parse_str(source)?;
        self.rewrite_ast(&mut syntax_tree);
        let formatted = prettyplease::unparse(&syntax_tree);
        Ok((formatted, self.summary.clone()))
    }

    /// Rewrite in-place an already parsed `syn::File`.
    pub fn rewrite_ast(&mut self, file: &mut File) {
        self.visit_file_mut(file);

        // Inject missing imports at the top of the file
        let mut new_items = Vec::new();

        if self.injected_normalize_path {
            let import_item: Item = syn::parse_quote! {
                use paths::normalize_path;
            };
            new_items.push(import_item);
            self.summary
                .injected_imports
                .insert("use paths::normalize_path;".to_string());
        }

        if self.injected_workspace_paths {
            let import_item: Item = syn::parse_quote! {
                use paths::WorkspacePaths;
            };
            new_items.push(import_item);
            self.summary
                .injected_imports
                .insert("use paths::WorkspacePaths;".to_string());
        }

        if !new_items.is_empty() {
            new_items.extend(file.items.clone());
            file.items = new_items;
        }
    }
}

impl VisitMut for AmbientAstRewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // First recurse into subexpressions
        visit_mut::visit_expr_mut(self, expr);

        // 1. Rewrite method calls: `path.canonicalize()` -> `paths::normalize_path(&path)`
        if let Expr::MethodCall(ExprMethodCall { receiver, method, args, .. }) = expr {
            if method == "canonicalize" && args.is_empty() {
                let rec = receiver.clone();
                *expr = syn::parse_quote! {
                    paths::normalize_path(&#rec)
                };
                self.summary.canonicalize_rewrites += 1;
                self.injected_normalize_path = true;
                return;
            }
        }

        // 2. Rewrite function calls: `std::env::*` or `env::*`
        if let Expr::Call(ExprCall { func, args, .. }) = expr {
            if let Expr::Path(expr_path) = &**func {
                let segments: Vec<String> = expr_path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                let path_str = segments.join("::");

                if path_str == "std::env::temp_dir" || path_str == "env::temp_dir" {
                    *expr = syn::parse_quote! {
                        paths::WorkspacePaths::default().cache()
                    };
                    self.summary.env_temp_dir_rewrites += 1;
                    self.injected_workspace_paths = true;
                    return;
                }

                if path_str == "std::env::current_dir" || path_str == "env::current_dir" {
                    *expr = syn::parse_quote! {
                        Ok(paths::WorkspacePaths::default().root().clone())
                    };
                    self.summary.env_current_dir_rewrites += 1;
                    self.injected_workspace_paths = true;
                    return;
                }

                if (path_str == "std::env::var" || path_str == "env::var") && !args.is_empty() {
                    let first_arg = &args[0];
                    *expr = syn::parse_quote! {
                        paths::FederationConfigRegistry::new().get(#first_arg).ok_or(std::env::VarError::NotPresent)
                    };
                    self.summary.env_var_rewrites += 1;
                    return;
                }
            }
        }
    }

    fn visit_item_use_mut(&mut self, node: &mut ItemUse) {
        // Strip out direct banned imports `use std::env;`
        visit_mut::visit_item_use_mut(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewriter_canonicalize_and_temp_dir() {
        let mut rewriter = AmbientAstRewriter::new();
        let source = r#"
            pub fn resolve_target(path: std::path::PathBuf) -> std::path::PathBuf {
                let temp = std::env::temp_dir();
                let clean = path.canonicalize();
                clean
            }
        "#;

        let (rewritten, summary) = rewriter.rewrite_source(source).unwrap();

        assert_eq!(summary.canonicalize_rewrites, 1);
        assert_eq!(summary.env_temp_dir_rewrites, 1);
        assert!(!rewritten.contains(".canonicalize()"));
        assert!(!rewritten.contains("std::env::temp_dir()"));
        assert!(rewritten.contains("paths::normalize_path(&path)"));
        assert!(rewritten.contains("paths::WorkspacePaths::default().cache()"));
        assert!(rewritten.contains("use paths::normalize_path;"));
        assert!(rewritten.contains("use paths::WorkspacePaths;"));

        // Verify clean parse
        let parsed: Result<File, _> = syn::parse_str(&rewritten);
        assert!(parsed.is_ok(), "Rewritten code must parse cleanly");
    }

    #[test]
    fn test_rewriter_current_dir() {
        let mut rewriter = AmbientAstRewriter::new();
        let source = r#"
            pub fn get_cwd() -> Result<std::path::PathBuf, std::io::Error> {
                std::env::current_dir()
            }
        "#;

        let (rewritten, summary) = rewriter.rewrite_source(source).unwrap();
        assert_eq!(summary.env_current_dir_rewrites, 1);
        assert!(!rewritten.contains("std::env::current_dir()"));
        assert!(rewritten.contains("paths::WorkspacePaths::default().root().clone()"));
    }
}
