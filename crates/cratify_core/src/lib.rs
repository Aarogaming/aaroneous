//! AST Auditor — Static analysis and invariant verification engine.
//!
//! Provides AST inspection, architectural invariant audit analysis, and zero-ambient-authority
//! enforcement for sovereign workspace crates.

#![deny(unsafe_code)]

pub mod certify;
pub mod inspect;
pub mod pattern_reviewer;
pub mod rules;
pub mod verify;

pub use certify::*;
pub use inspect::*;
pub use pattern_reviewer::*;
pub use verify::*;

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use syn::visit::Visit;
use walkdir::WalkDir;

pub use self::rules::memory_geometry::{MemoryGeometryViolation, MemoryGeometryVisitor};
pub use self::rules::no_ambient_authority::{AmbientAuthorityVisitor, AmbientViolation};
pub use self::rules::no_workspace_prefix_stutter::{
    PrefixStutterViolation, PrefixStutterVisitor, audit_manifest_stutter,
};
pub use self::rules::safety_comments::{SafetyCommentViolation, SafetyCommentVisitor};
pub use self::rules::text_encoding::{
    EncodingViolation, audit_file_encoding, audit_tracked_encodings,
};
pub use self::rules::unwrap_panic::{UnwrapPanicHit, UnwrapPanicVisitor};
pub use self::rules::zero_alloc_hot_path::{HotPathAllocViolation, HotPathAllocVisitor};

/// Unified audit report aggregating structural and semantic AST violations.
#[derive(Debug, Default)]
pub struct UnifiedAuditReport {
    pub files_scanned: usize,
    pub hot_functions_scanned: usize,
    pub errors: Vec<String>,
    pub soundness_violations: Vec<rules::soundness::SoundnessViolation>,
    pub ambient_violations: Vec<AmbientViolation>,
    pub hot_path_violations: Vec<HotPathAllocViolation>,
    pub prefix_stutter_violations: Vec<PrefixStutterViolation>,
    pub memory_geometry_violations: Vec<MemoryGeometryViolation>,
    pub safety_comment_violations: Vec<SafetyCommentViolation>,
}

impl UnifiedAuditReport {
    pub fn has_failures(&self) -> bool {
        !self.errors.is_empty()
            || !self.soundness_violations.is_empty()
            || !self.ambient_violations.is_empty()
            || !self.hot_path_violations.is_empty()
            || !self.prefix_stutter_violations.is_empty()
            || !self.memory_geometry_violations.is_empty()
            || !self.safety_comment_violations.is_empty()
    }

    /// One human-readable line per finding, in the same wording
    /// [`print_diagnostics`](Self::print_diagnostics) prints, but returned
    /// instead of written to the process's stdout/stderr. Audit errors come
    /// first (prefixed `[AUDIT ERROR]`), then each violation.
    pub fn violation_messages(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        out.extend(self.errors.iter().map(|e| format!("[AUDIT ERROR] {e}")));
        out.extend(
            self.soundness_violations
                .iter()
                .map(|v| format!("{}:{}: {}", v.file_path.display(), v.line, v.rule)),
        );
        out.extend(self.ambient_violations.iter().map(ToString::to_string));
        out.extend(self.hot_path_violations.iter().map(ToString::to_string));
        out.extend(
            self.prefix_stutter_violations
                .iter()
                .map(ToString::to_string),
        );
        out.extend(
            self.memory_geometry_violations
                .iter()
                .map(ToString::to_string),
        );
        out.extend(
            self.safety_comment_violations
                .iter()
                .map(ToString::to_string),
        );
        out
    }

    pub fn print_diagnostics(&self) {
        for error in &self.errors {
            eprintln!("[AUDIT ERROR] {error}");
        }
        for violation in &self.soundness_violations {
            eprintln!(
                "{}:{}: {}",
                violation.file_path.display(),
                violation.line,
                violation.rule
            );
        }
        println!(
            "Annotated hot functions checked: {} (syntax only, not a transitive allocation proof)",
            self.hot_functions_scanned
        );
        for v in &self.ambient_violations {
            eprintln!("{v}");
        }
        for v in &self.hot_path_violations {
            eprintln!("{v}");
        }
        for v in &self.prefix_stutter_violations {
            eprintln!("{v}");
        }
        for v in &self.memory_geometry_violations {
            eprintln!("{v}");
        }
        for v in &self.safety_comment_violations {
            eprintln!("{v}");
        }

        println!(
            "\n[AST AUDIT SUMMARY] Files scanned: {} | Violations: {} ({} soundness, {} ambient, {} hot-path allocations, {} prefix stutter, {} memory geometry, {} missing safety comments)",
            self.files_scanned,
            self.soundness_violations.len()
                + self.ambient_violations.len()
                + self.hot_path_violations.len()
                + self.prefix_stutter_violations.len()
                + self.memory_geometry_violations.len()
                + self.safety_comment_violations.len(),
            self.soundness_violations.len(),
            self.ambient_violations.len(),
            self.hot_path_violations.len(),
            self.prefix_stutter_violations.len(),
            self.memory_geometry_violations.len(),
            self.safety_comment_violations.len()
        );
    }
}

/// Audit an individual Rust source file using syn visitors.
pub fn audit_source_file(
    path: &Path,
    report: &mut UnifiedAuditReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let syntax_tree = syn::parse_file(&content)?;

    report.files_scanned += 1;

    // 1. Audit Ambient Authority
    let mut ambient_visitor = AmbientAuthorityVisitor::new(path);
    ambient_visitor.visit_file(&syntax_tree);
    report.ambient_violations.extend(ambient_visitor.violations);

    // 2. Audit Hot Path Allocations
    let mut hot_path_visitor = HotPathAllocVisitor::new(path);
    hot_path_visitor.visit_file(&syntax_tree);
    report.hot_functions_scanned += hot_path_visitor.functions_scanned;
    report
        .hot_path_violations
        .extend(hot_path_visitor.violations);
    let mut soundness = rules::soundness::SoundnessVisitor {
        file_path: path,
        violations: Vec::new(),
    };
    soundness.visit_file(&syntax_tree);
    report.soundness_violations.extend(soundness.violations);

    // 3. Audit Prefix Stutter in AST
    let mut stutter_visitor = PrefixStutterVisitor::new(path);
    stutter_visitor.visit_file(&syntax_tree);
    report
        .prefix_stutter_violations
        .extend(stutter_visitor.violations);

    // 4. Audit Memory Geometry for Pod/Zeroable derives
    let mut geometry_visitor = MemoryGeometryVisitor::new(path);
    geometry_visitor.visit_file(&syntax_tree);
    report
        .memory_geometry_violations
        .extend(geometry_visitor.into_violations());

    // 5. Audit unsafe blocks for a documented SAFETY: rationale comment
    let mut safety_visitor = SafetyCommentVisitor::new(path, &content);
    safety_visitor.visit_file(&syntax_tree);
    report
        .safety_comment_violations
        .extend(safety_visitor.violations);

    Ok(())
}

/// Parse a single Rust source file and return every `.unwrap()`, `.expect(...)`,
/// `panic!(...)`, and `assert!(...)` occurrence found outside test code (see
/// `UnwrapPanicVisitor` for the exact exemption rules). Used by
/// `cargo xtask check-unwraps` to build its ratchet baseline.
pub fn scan_unwrap_panic_hits(
    path: &Path,
) -> Result<Vec<UnwrapPanicHit>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let syntax_tree = syn::parse_file(&content)?;
    let mut visitor = UnwrapPanicVisitor::new(path, &content);
    visitor.visit_file(&syntax_tree);
    Ok(visitor.hits)
}

/// Audit a Cargo.toml manifest file for prefix stutter.
pub fn audit_manifest_file(path: &Path, report: &mut UnifiedAuditReport) {
    match fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|text| toml::from_str::<toml::Value>(&text).map_err(|e| e.to_string()))
    {
        Ok(value) => {
            report.files_scanned += 1;
            audit_manifest_stutter(path, &value, &mut report.prefix_stutter_violations);
        }
        Err(error) => report.errors.push(format!("{}: {error}", path.display())),
    }
}

/// True for the only two file kinds the auditor examines: Rust source and
/// `Cargo.toml` manifests. Every other file (lockfiles, other TOML, Markdown,
/// YAML, text) is silently skipped by [`audit_entry`], so callers that hand
/// the auditor a list of changed files should filter with this first instead
/// of treating "nothing audited" as a failure.
pub fn is_auditable_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
        || path.file_name().is_some_and(|name| name == "Cargo.toml")
}

fn audit_entry(path: &Path, report: &mut UnifiedAuditReport) {
    if path.extension().is_some_and(|ext| ext == "rs") {
        if let Err(error) = audit_source_file(path, report) {
            report.errors.push(format!("{}: {error}", path.display()));
        }
    } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
        audit_manifest_file(path, report);
    }
}

/// Audit `targets` and return the structured report without printing
/// anything. This is the collection half of [`run_workspace_audit`]: it does
/// not add the "No source files were audited" error, so a caller can decide
/// whether an empty scope is a failure (the CLI gate does; a diff reviewer
/// whose diff touches no Rust does not).
///
/// Generated Cargo output, VCS metadata and JS dependencies are not source.
/// Explicit targets are never excluded; traversal/read/parse failures are
/// recorded in `errors` (fail closed).
pub fn audit_targets<I, P>(targets: I) -> UnifiedAuditReport
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut report = UnifiedAuditReport::default();
    for target in targets {
        let path = target.as_ref();
        if path.is_file() {
            audit_entry(path, &mut report);
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_entry(|entry| {
                entry.depth() == 0
                    || !entry.file_type().is_dir()
                    || !matches!(
                        entry.file_name().to_str(),
                        Some("target" | ".git" | "node_modules")
                    )
            }) {
                match entry {
                    Ok(entry) if entry.file_type().is_file() => {
                        audit_entry(entry.path(), &mut report)
                    }
                    Ok(_) => {}
                    Err(error) => report.errors.push(error.to_string()),
                }
            }
        } else {
            report
                .errors
                .push(format!("Missing or unreadable target: {}", path.display()));
        }
    }
    report
}

/// Generated Cargo output, VCS metadata and JS dependencies are not source.
/// Explicit targets are never excluded; traversal/read/parse failures fail closed.
pub fn run_workspace_audit<I, P>(targets: I) -> Result<(), ExitCode>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut report = audit_targets(targets);
    if report.files_scanned == 0 {
        report.errors.push("No source files were audited".into());
    }
    report.print_diagnostics();
    if report.has_failures() {
        Err(ExitCode::FAILURE)
    } else {
        println!("AUDIT GATE PASSED: Implemented syntax rules passed for the reported scope.");
        Ok(())
    }
}
