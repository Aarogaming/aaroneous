//! Cratify — Aaroneous Crate Component lifecycle automation.
//!
//! Provides AST inspection, static audit analysis, LLM translation routing,
//! crate generation, workspace harvesting, and cryptographic certification
//! for the ACC standard.

pub mod inspect;
pub mod audit;
pub mod translate;
pub mod generator;
pub mod harvest;
pub mod scaffold;
pub mod verify;
pub mod workspace;
pub mod fascia;
pub mod certify;
pub mod ring;
pub mod python_to_rust;
pub mod rules;

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use syn::visit::Visit;
use walkdir::WalkDir;

pub use self::rules::no_ambient_authority::{AmbientAuthorityVisitor, AmbientViolation};
pub use self::rules::no_workspace_prefix_stutter::{
    audit_manifest_stutter, PrefixStutterViolation, PrefixStutterVisitor,
};
pub use self::rules::zero_alloc_hot_path::{HotPathAllocViolation, HotPathAllocVisitor};

/// Unified audit report aggregating structural and semantic AST violations.
#[derive(Debug, Default)]
pub struct UnifiedAuditReport {
    pub files_scanned: usize,
    pub ambient_violations: Vec<AmbientViolation>,
    pub hot_path_violations: Vec<HotPathAllocViolation>,
    pub prefix_stutter_violations: Vec<PrefixStutterViolation>,
}

impl UnifiedAuditReport {
    pub fn has_failures(&self) -> bool {
        !self.ambient_violations.is_empty()
            || !self.hot_path_violations.is_empty()
            || !self.prefix_stutter_violations.is_empty()
    }

    pub fn print_diagnostics(&self) {
        for v in &self.ambient_violations {
            eprintln!("{v}");
        }
        for v in &self.hot_path_violations {
            eprintln!("{v}");
        }
        for v in &self.prefix_stutter_violations {
            eprintln!("{v}");
        }

        println!(
            "\n[CRATIFY AUDIT SUMMARY] Files scanned: {} | Violations: {} ({} ambient, {} hot-path allocations, {} prefix stutter)",
            self.files_scanned,
            self.ambient_violations.len()
                + self.hot_path_violations.len()
                + self.prefix_stutter_violations.len(),
            self.ambient_violations.len(),
            self.hot_path_violations.len(),
            self.prefix_stutter_violations.len()
        );
    }
}

/// Audit an individual Rust source file using syn visitors.
pub fn audit_source_file(
    path: &Path,
    report: &mut UnifiedAuditReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let syntax_tree = match syn::parse_file(&content) {
        Ok(tree) => tree,
        Err(err) => {
            eprintln!("[SYN PARSE ERROR] {}: {}", path.display(), err);
            return Ok(());
        }
    };

    report.files_scanned += 1;

    // 1. Audit Ambient Authority
    let mut ambient_visitor = AmbientAuthorityVisitor::new(path);
    ambient_visitor.visit_file(&syntax_tree);
    report.ambient_violations.extend(ambient_visitor.violations);

    // 2. Audit Hot Path Allocations
    let mut hot_path_visitor = HotPathAllocVisitor::new(path);
    hot_path_visitor.visit_file(&syntax_tree);
    report.hot_path_violations.extend(hot_path_visitor.violations);

    // 3. Audit Prefix Stutter in AST
    let mut stutter_visitor = PrefixStutterVisitor::new(path);
    stutter_visitor.visit_file(&syntax_tree);
    report
        .prefix_stutter_violations
        .extend(stutter_visitor.violations);

    Ok(())
}

/// Audit a Cargo.toml manifest file for prefix stutter.
pub fn audit_manifest_file(path: &Path, report: &mut UnifiedAuditReport) {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(manifest_val) = toml::from_str::<toml::Value>(&content) {
            report.files_scanned += 1;
            audit_manifest_stutter(path, &manifest_val, &mut report.prefix_stutter_violations);
        }
    }
}

/// Recursively traverses targeted paths, executes AST and manifest audits, and exits cleanly.
pub fn run_workspace_audit<I, P>(targets: I) -> Result<(), ExitCode>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut report = UnifiedAuditReport::default();

    for target in targets {
        let p = target.as_ref();
        if p.is_file() {
            if p.extension().is_some_and(|ext| ext == "rs") {
                let _ = audit_source_file(p, &mut report);
            } else if p.file_name().is_some_and(|name| name == "Cargo.toml") {
                audit_manifest_file(p, &mut report);
            }
        } else if p.is_dir() {
            for entry in WalkDir::new(p).into_iter().filter_map(Result::ok) {
                let entry_path = entry.path();
                if entry_path.extension().is_some_and(|ext| ext == "rs") {
                    let _ = audit_source_file(entry_path, &mut report);
                } else if entry_path.file_name().is_some_and(|name| name == "Cargo.toml") {
                    audit_manifest_file(entry_path, &mut report);
                }
            }
        }
    }

    report.print_diagnostics();

    if report.has_failures() {
        eprintln!("\nCRATIFY GATE FAILED: Repository invariant violations detected.");
        Err(ExitCode::FAILURE)
    } else {
        println!("\nCRATIFY GATE PASSED: All architectural AST invariants verified.");
        Ok(())
    }
}
