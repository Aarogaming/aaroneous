//! Invariant Rule: No Workspace Prefix Stutter
//!
//! Enforces that internal crates, dependencies, types, and IPC/shm channel handles
//! do not prepend redundant `aaroneous_*` or `aaroneous-` prefixes.
//! Prometheus, OpenTelemetry, and external metrics identifiers are strictly exempted.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, File, Ident, ItemFn, ItemMod, ItemStruct, LitStr, Meta};

const PREFIX_LOWER: &str = concat!("aar", "oneous_");
const PREFIX_SLASH: &str = concat!("aar", "oneous/");
const PREFIX_DASH: &str = concat!("aar", "oneous-");
const PREFIX_DOT: &str = concat!("aar", "oneous.");
const STEM_PASCAL: &str = concat!("Aar", "oneous");

/// Diagnostic violation for workspace prefix stutter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixStutterViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub target: String,
    pub remediation: String,
}

impl std::fmt::Display for PrefixStutterViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: [PREFIX_STUTTER] Redundant workspace prefix in `{}`. Remediation: {}",
            self.file_path.display(),
            self.line,
            self.column,
            self.target,
            self.remediation
        )
    }
}

/// Visitor that inspects AST items and string literals for prefix stutter.
pub struct PrefixStutterVisitor<'a> {
    pub file_path: &'a Path,
    pub is_exempt: bool,
    pub violations: Vec<PrefixStutterViolation>,
}

impl<'a> PrefixStutterVisitor<'a> {
    pub fn new(file_path: &'a Path) -> Self {
        Self {
            file_path,
            is_exempt: false,
            violations: Vec::new(),
        }
    }

    fn check_file_attributes(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            if let Meta::List(list) = &attr.meta {
                if list.path.is_ident("allow") {
                    let tokens = list.tokens.to_string();
                    if tokens.contains("workspace_prefix_stutter") {
                        self.is_exempt = true;
                        return;
                    }
                }
            }
        }
    }

    fn push_violation<S: Spanned>(&mut self, node: &S, target: &str, remediation: &str) {
        let span = node.span();
        let start = span.start();
        self.violations.push(PrefixStutterViolation {
            file_path: self.file_path.to_path_buf(),
            line: start.line,
            column: start.column + 1,
            target: target.to_string(),
            remediation: remediation.to_string(),
        });
    }

    /// Checks if a string literal is an exempted metric label (Prometheus/OpenTelemetry).
    pub fn is_exempt_metric_identifier(s: &str) -> bool {
        if !s.starts_with(PREFIX_LOWER) && !s.starts_with(PREFIX_DOT) {
            return false;
        }

        // Standard Prometheus suffixes or telemetry keywords
        s.ends_with("_total")
            || s.ends_with("_seconds")
            || s.ends_with("_bytes")
            || s.ends_with("_count")
            || s.ends_with("_bucket")
            || s.ends_with("_ratio")
            || s.ends_with("_info")
            || s.contains("uptime")
            || s.contains("telemetry")
            || s.contains("metric")
    }

    fn check_identifier(&mut self, ident: &Ident, kind: &str) {
        let name = ident.to_string();
        if name.starts_with(PREFIX_LOWER) || name.starts_with(STEM_PASCAL) {
            self.push_violation(
                ident,
                &name,
                &format!("Remove `{PREFIX_LOWER}` prefix stutter from {kind}. Use concise generic systems nomenclature."),
            );
        }
    }
}

impl<'ast> Visit<'ast> for PrefixStutterVisitor<'_> {
    fn visit_file(&mut self, node: &'ast File) {
        self.check_file_attributes(&node.attrs);
        if self.is_exempt {
            return;
        }
        visit::visit_file(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        if self.is_exempt {
            return;
        }
        self.check_identifier(&node.ident, "struct");
        visit::visit_item_struct(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if self.is_exempt {
            return;
        }
        self.check_identifier(&node.sig.ident, "function");
        visit::visit_item_fn(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        if self.is_exempt {
            return;
        }
        self.check_identifier(&node.ident, "module");
        visit::visit_item_mod(self, node);
    }

    fn visit_lit_str(&mut self, node: &'ast LitStr) {
        if self.is_exempt {
            return;
        }

        let val = node.value();
        if val.starts_with(PREFIX_LOWER)
            || val.starts_with(PREFIX_SLASH)
            || val.starts_with(PREFIX_DASH)
        {
            // Metrics and OpenTelemetry exemption
            if Self::is_exempt_metric_identifier(&val) {
                return;
            }

            // Exclude diagnostic log messages or display strings that simply contain descriptive sentences
            if val.contains(' ') {
                return;
            }

            self.push_violation(
                node,
                &format!("\"{val}\""),
                "IPC channel, named pipe, shared-memory identifier, or binary moniker contains prefix stutter. Strip prefix.",
            );
        }

        visit::visit_lit_str(self, node);
    }
}

/// Inspect a parsed `Cargo.toml` manifest for prefix stutter violations in package name or internal dependencies.
pub fn audit_manifest_stutter(
    manifest_path: &Path,
    manifest_toml: &toml::Value,
    violations: &mut Vec<PrefixStutterViolation>,
) {
    let pbuf = manifest_path.to_path_buf();

    // 1. Package name check
    if let Some(package) = manifest_toml.get("package") {
        if let Some(name) = package.get("name").and_then(|v| v.as_str()) {
            if name.starts_with(PREFIX_LOWER) || name.starts_with(PREFIX_DASH) {
                violations.push(PrefixStutterViolation {
                    file_path: pbuf.clone(),
                    line: 1,
                    column: 1,
                    target: format!("package.name = \"{name}\""),
                    remediation: format!(
                        "Rename crate to strip prefix stutter (e.g. `{}`)",
                        name.trim_start_matches(PREFIX_LOWER)
                            .trim_start_matches(PREFIX_DASH)
                    ),
                });
            }
        }
    }

    // 2. Dependencies check
    let dep_tables = [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "workspace.dependencies",
    ];
    for table_key in dep_tables {
        let table_opt = if table_key.starts_with("workspace.") {
            manifest_toml
                .get("workspace")
                .and_then(|w| w.get("dependencies"))
        } else {
            manifest_toml.get(table_key)
        };

        if let Some(table) = table_opt.and_then(|t| t.as_table()) {
            for (dep_name, dep_val) in table {
                let is_internal = match dep_val {
                    toml::Value::Table(t) => t.contains_key("path") || t.contains_key("workspace"),
                    _ => false,
                };

                if is_internal
                    && (dep_name.starts_with(PREFIX_LOWER) || dep_name.starts_with(PREFIX_DASH))
                {
                    violations.push(PrefixStutterViolation {
                        file_path: pbuf.clone(),
                        line: 1,
                        column: 1,
                        target: format!("{table_key}.{dep_name}"),
                        remediation: format!(
                            "Remove stutter prefix from internal dependency `{dep_name}`."
                        ),
                    });
                }
            }
        }
    }
}
