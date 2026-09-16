//! Repository-wide hardening baseline checks.

use anyhow::{Context, Result, bail};
use quote::ToTokens;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::visit::Visit;
use walkdir::WalkDir;

const LENSES: [&str; 20] = [
    "H01", "H02", "H03", "H04", "H05", "H06", "H07", "H08", "H09", "H10", "H11", "H12", "H13",
    "H14", "H15", "H16", "H17", "H18", "H19", "H20",
];

const RETIRED_ROOT_DOCUMENTS: [&str; 6] = [
    "TODO.md",
    "MASTER_ROADMAP.md",
    "RELEASE_NOTES.md",
    "STRATEGIC_VISION.md",
    "WHAT_EXISTS_TODAY.md",
    "TEXT_ENCODING.md",
];

#[derive(Debug, Serialize)]
pub struct HardeningReport {
    pub checks: BTreeMap<String, bool>,
    pub errors: Vec<String>,
    pub matrix_lenses: Vec<String>,
    pub retired_root_documents_present: Vec<String>,
    pub broken_relative_links: Vec<BrokenLink>,
    pub source_inventory: SourceInventory,
    pub tooling: ToolingInventory,
}

#[derive(Debug, Serialize)]
pub struct BrokenLink {
    pub file: String,
    pub target: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SourceInventory {
    pub rust_files: usize,
    pub unsafe_blocks: usize,
    pub extern_c_symbols: usize,
    pub unwrap_calls: usize,
    pub expect_calls: usize,
    pub ambient_environment_calls: usize,
    pub canonicalize_calls: usize,
    pub todo_macros: usize,
    pub unimplemented_macros: usize,
}

#[derive(Debug, Serialize)]
pub struct ToolingInventory {
    pub cargo_audit_available: bool,
}

pub fn run_hardening_audit(root: &Path) -> Result<HardeningReport> {
    let matrix = read_required(root, "docs/HARDENING_AUDIT_MATRIX.md")?;
    let ci = read_required(root, ".github/workflows/ci.yml")?;
    let gate = read_required(root, "docs/VERIFICATION.md")?;
    let matrix_lenses = matrix_lenses(&matrix);
    let retired = RETIRED_ROOT_DOCUMENTS
        .iter()
        .filter(|name| root.join(name).exists())
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let broken_links = broken_relative_links(root)?;

    let required_command = "cargo run -p ast_auditor -- hardening --check";
    let mut checks = BTreeMap::new();
    checks.insert(
        "matrix_covers_all_lenses".to_owned(),
        matrix_lenses
            == LENSES
                .iter()
                .map(|lens| (*lens).to_owned())
                .collect::<Vec<_>>(),
    );
    checks.insert(
        "ci_runs_hardening_audit".to_owned(),
        ci.contains(required_command),
    );
    checks.insert(
        "native_verification_is_documented".to_owned(),
        gate.contains(required_command),
    );
    checks.insert(
        "retired_root_documents_absent".to_owned(),
        retired.is_empty(),
    );
    checks.insert(
        "tooling_boundary_policy_present".to_owned(),
        root.join("docs/architecture/TOOLING_BOUNDARY_POLICY.md")
            .is_file(),
    );
    checks.insert(
        "active_document_links_valid".to_owned(),
        broken_links.is_empty(),
    );

    let errors = checks
        .iter()
        .filter_map(|(name, passed)| (!passed).then_some(name.clone()))
        .collect();

    Ok(HardeningReport {
        checks,
        errors,
        matrix_lenses,
        retired_root_documents_present: retired,
        broken_relative_links: broken_links,
        source_inventory: source_inventory(root)?,
        tooling: ToolingInventory {
            cargo_audit_available: Command::new("cargo")
                .args(["audit", "--version"])
                .output()
                .is_ok_and(|output| output.status.success()),
        },
    })
}

pub fn enforce_hardening_audit(root: &Path) -> Result<HardeningReport> {
    let report = run_hardening_audit(root)?;
    if !report.errors.is_empty() {
        bail!("hardening baseline failed: {}", report.errors.join(", "));
    }
    Ok(report)
}

fn read_required(root: &Path, relative: &str) -> Result<String> {
    fs::read_to_string(root.join(relative))
        .with_context(|| format!("failed to read required file {relative}"))
}

fn matrix_lenses(matrix: &str) -> Vec<String> {
    let mut lenses = Vec::new();
    for token in matrix.split(|character: char| !character.is_ascii_alphanumeric()) {
        if token.len() == 3
            && token.starts_with('H')
            && token[1..]
                .chars()
                .all(|character| character.is_ascii_digit())
            && !lenses.iter().any(|existing| existing == token)
        {
            lenses.push(token.to_owned());
        }
    }
    lenses.sort();
    lenses
}

fn broken_relative_links(root: &Path) -> Result<Vec<BrokenLink>> {
    let mut broken = Vec::new();
    for path in active_markdown_files(root)? {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for target in markdown_targets(&text) {
            if is_external_target(&target) {
                continue;
            }
            if !path
                .parent()
                .is_some_and(|parent| parent.join(&target).exists())
            {
                broken.push(BrokenLink {
                    file: path
                        .strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    target,
                });
            }
        }
    }
    Ok(broken)
}

fn active_markdown_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let docs = root.join("docs");
    for entry in WalkDir::new(&docs)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || entry.file_name().to_str() != Some("archive"))
    {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            files.push(entry.into_path());
        }
    }
    for name in [
        "README.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "AGENTS.md",
        "CHANGELOG.md",
    ] {
        let path = root.join(name);
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn markdown_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut remainder = text;
    while let Some(start) = remainder.find("](") {
        let candidate = &remainder[start + 2..];
        let Some(end) = candidate.find(')') else {
            break;
        };
        let target = candidate[..end].split('#').next().unwrap_or_default();
        if !target.is_empty() {
            targets.push(target.to_owned());
        }
        remainder = &candidate[end + 1..];
    }
    targets
}

fn is_external_target(target: &str) -> bool {
    ["http:", "https:", "mailto:", "file:"]
        .iter()
        .any(|prefix| target.starts_with(prefix))
}

fn source_inventory(root: &Path) -> Result<SourceInventory> {
    let mut inventory = SourceInventory::default();
    for directory in ["core", "crates", "dev"] {
        let path = root.join(directory);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(path).into_iter().filter_entry(|entry| {
            entry.depth() == 0 || entry.file_name().to_str() != Some("target")
        }) {
            let entry = entry?;
            if !entry.file_type().is_file()
                || entry.path().extension().is_none_or(|ext| ext != "rs")
            {
                continue;
            }
            let text = fs::read_to_string(entry.path())
                .with_context(|| format!("failed to read {}", entry.path().display()))?;
            inventory.rust_files += 1;
            let syntax = syn::parse_file(&text)
                .with_context(|| format!("failed to parse {}", entry.path().display()))?;
            let mut visitor = SourceInventoryVisitor {
                inventory: &mut inventory,
            };
            visitor.visit_file(&syntax);
        }
    }
    Ok(inventory)
}

struct SourceInventoryVisitor<'a> {
    inventory: &'a mut SourceInventory,
}

impl Visit<'_> for SourceInventoryVisitor<'_> {
    fn visit_expr_unsafe(&mut self, expression: &'_ syn::ExprUnsafe) {
        self.inventory.unsafe_blocks += 1;
        syn::visit::visit_expr_unsafe(self, expression);
    }

    fn visit_item_fn(&mut self, item: &'_ syn::ItemFn) {
        if is_c_abi(&item.sig.abi) {
            self.inventory.extern_c_symbols += 1;
        }
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_foreign_item_fn(&mut self, item: &'_ syn::ForeignItemFn) {
        if is_c_abi(&item.sig.abi) {
            self.inventory.extern_c_symbols += 1;
        }
        syn::visit::visit_foreign_item_fn(self, item);
    }

    fn visit_expr_method_call(&mut self, expression: &'_ syn::ExprMethodCall) {
        match expression.method.to_string().as_str() {
            "unwrap" => self.inventory.unwrap_calls += 1,
            "expect" => self.inventory.expect_calls += 1,
            "canonicalize" => self.inventory.canonicalize_calls += 1,
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'_ syn::ExprCall) {
        let callee = expression
            .func
            .to_token_stream()
            .to_string()
            .replace(' ', "");
        if matches!(
            callee.as_str(),
            "std::env::var"
                | "std::env::var_os"
                | "std::env::set_var"
                | "std::env::remove_var"
                | "std::env::temp_dir"
                | "std::env::current_dir"
        ) {
            self.inventory.ambient_environment_calls += 1;
        }
        syn::visit::visit_expr_call(self, expression);
    }

    fn visit_macro(&mut self, macro_call: &'_ syn::Macro) {
        if let Some(segment) = macro_call.path.segments.last() {
            match segment.ident.to_string().as_str() {
                "todo" => self.inventory.todo_macros += 1,
                "unimplemented" => self.inventory.unimplemented_macros += 1,
                _ => {}
            }
        }
        syn::visit::visit_macro(self, macro_call);
    }
}

fn is_c_abi(abi: &Option<syn::Abi>) -> bool {
    abi.as_ref()
        .and_then(|abi| abi.name.as_ref())
        .is_some_and(|name| name.value() == "C")
}

#[cfg(test)]
mod tests {
    use super::{SourceInventory, SourceInventoryVisitor, markdown_targets, matrix_lenses};
    use syn::visit::Visit;

    #[test]
    fn extracts_unique_sorted_lenses() {
        assert_eq!(matrix_lenses("H20 H01 H20 H02"), ["H01", "H02", "H20"]);
    }

    #[test]
    fn extracts_relative_markdown_targets() {
        assert_eq!(
            markdown_targets("[local](docs/WORKLIST.md) [anchor](docs/README.md#contents)"),
            ["docs/WORKLIST.md", "docs/README.md"]
        );
    }

    #[test]
    fn inventories_exported_c_functions() {
        let syntax = syn::parse_file("pub unsafe extern \"C\" fn entry() {}").unwrap();
        let mut inventory = SourceInventory::default();
        SourceInventoryVisitor {
            inventory: &mut inventory,
        }
        .visit_file(&syntax);
        assert_eq!(inventory.extern_c_symbols, 1);
    }
}
