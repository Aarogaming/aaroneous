//! Static analysis engine for Cratify ACCs.
//!
//! Scans `CodeInfo` trees and `Cargo.toml` manifests to enforce microkernel
//! invariants, zero-copy structural compliance, and scaling-law constraints.
//! Returns a structured `AuditReport` listing violations and warnings.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::fs;

use crate::inspect::CodeInfo;

// ── Severity ──────────────────────────────────────────────────────────

/// Severity of an audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditSeverity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARNING"),
            Self::Info => write!(f, "INFO"),
        }
    }
}

// ── Rules ─────────────────────────────────────────────────────────────

/// Auditable invariant rules enforced by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditRule {
    /// Forbids `unsafe` blocks in ACC source.
    NoUnsafe,
    /// Forbids `.unwrap()` and `.expect()` calls (use `?` or `Result`).
    NoUnwrap,
    /// Forbids `panic!`, `unreachable!`, and `unimplemented!` macros.
    NoPanic,
    /// Forbids `println!` / `eprintln!` (use `tracing` instead).
    NoPrintln,
    /// Public structs should derive `bytemuck::Pod` for zero-copy compliance.
    ZeroCopyStructs,
    /// Forbidden crate dependencies that violate scaling-law constraints.
    ForbiddenDeps,
    /// Module exceeds recommended function count (scaling-law blast radius).
    ScalingLawBlastRadius,
}

impl fmt::Display for AuditRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUnsafe => write!(f, "no-unsafe"),
            Self::NoUnwrap => write!(f, "no-unwrap"),
            Self::NoPanic => write!(f, "no-panic"),
            Self::NoPrintln => write!(f, "no-println"),
            Self::ZeroCopyStructs => write!(f, "zero-copy-structs"),
            Self::ForbiddenDeps => write!(f, "forbidden-deps"),
            Self::ScalingLawBlastRadius => write!(f, "scaling-law-blast-radius"),
        }
    }
}

// ── Violation ─────────────────────────────────────────────────────────

/// A single audit finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditViolation {
    pub rule: AuditRule,
    pub severity: AuditSeverity,
    pub location: String,
    pub message: String,
}

impl fmt::Display for AuditViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}][{}] {}: {}",
            self.severity, self.rule, self.location, self.message
        )
    }
}

// ── Report ────────────────────────────────────────────────────────────

/// Structured audit report containing all findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub violations: Vec<AuditViolation>,
    pub files_scanned: usize,
    pub structs_scanned: usize,
    pub functions_scanned: usize,
}

impl AuditReport {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            files_scanned: 0,
            structs_scanned: 0,
            functions_scanned: 0,
        }
    }

    pub fn push(&mut self, v: AuditViolation) {
        self.violations.push(v);
    }

    /// True if any ERROR-severity violations exist.
    pub fn has_errors(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == AuditSeverity::Error)
    }

    /// Count by severity.
    pub fn count_by_severity(&self, sev: AuditSeverity) -> usize {
        self.violations.iter().filter(|v| v.severity == sev).count()
    }

    /// Filter violations by rule.
    pub fn violations_for_rule(&self, rule: AuditRule) -> Vec<&AuditViolation> {
        self.violations.iter().filter(|v| v.rule == rule).collect()
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        let errors = self.count_by_severity(AuditSeverity::Error);
        let warnings = self.count_by_severity(AuditSeverity::Warning);
        let infos = self.count_by_severity(AuditSeverity::Info);
        format!(
            "Audit complete: {} files, {} structs, {} functions scanned. \
             {} errors, {} warnings, {} info.",
            self.files_scanned,
            self.structs_scanned,
            self.functions_scanned,
            errors,
            warnings,
            infos,
        )
    }
}

impl Default for AuditReport {
    fn default() -> Self {
        Self::new()
    }
}

// ── Forbidden Dependencies ────────────────────────────────────────────

/// Crate names that violate scaling-law constraints when used unmanaged.
const FORBIDDEN_DEPS: &[&str] = &[
    "hyper",       // unmanaged HTTP stack — use reqwest or platform_bridge
    "actix-web",   // unmanaged web framework
    "rocket",      // unmanaged web framework
    "openssl",     // unmanaged TLS — use rustls via reqwest
    "native-tls",  // unmanaged TLS backend
    "glob",        // unmanaged filesystem walking — use walkdir
    "rand",        // unmanaged RNG — use getrandom orOsRng
];

// ── Audit Engine ──────────────────────────────────────────────────────

/// Configuration for an audit run.
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Rules to skip (empty = run all rules).
    pub skip_rules: Vec<AuditRule>,
    /// Maximum functions per file before triggering blast-radius warning.
    pub max_functions_per_file: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            skip_rules: Vec::new(),
            max_functions_per_file: 50,
        }
    }
}

/// Run a full static audit on a directory tree.
pub async fn run(path: PathBuf, strict: bool) -> Result<()> {
    let config = AuditConfig::default();
    let report = audit_path(&path, &config)?;

    println!("{}", report.summary());
    for v in &report.violations {
        println!("  {v}");
    }

    if strict && report.has_errors() {
        anyhow::bail!(
            "Audit failed with {} error(s) in strict mode.",
            report.count_by_severity(AuditSeverity::Error)
        );
    }

    Ok(())
}

/// Audit a single file's AST against all configured rules.
pub fn audit_code_info(
    file_label: &str,
    info: &CodeInfo,
    config: &AuditConfig,
) -> AuditReport {
    let mut report = AuditReport::new();
    report.files_scanned = 1;
    report.structs_scanned = info.structs.len();
    report.functions_scanned = info.functions.len();

    if !config.skip_rules.contains(&AuditRule::NoUnsafe) {
        audit_no_unsafe(file_label, info, &mut report);
    }
    if !config.skip_rules.contains(&AuditRule::NoUnwrap) {
        audit_no_unwrap(file_label, info, &mut report);
    }
    if !config.skip_rules.contains(&AuditRule::NoPanic) {
        audit_no_panic(file_label, info, &mut report);
    }
    if !config.skip_rules.contains(&AuditRule::NoPrintln) {
        audit_no_println(file_label, info, &mut report);
    }
    if !config.skip_rules.contains(&AuditRule::ZeroCopyStructs) {
        audit_zero_copy_structs(file_label, info, &mut report);
    }
    if !config.skip_rules.contains(&AuditRule::ScalingLawBlastRadius) {
        audit_blast_radius(file_label, info, config.max_functions_per_file, &mut report);
    }

    report
}

/// Audit a directory by inspecting every `.rs` file found.
pub fn audit_path(path: &Path, config: &AuditConfig) -> Result<AuditReport> {
    let mut combined = AuditReport::new();

    if path.is_file() {
        let info = crate::inspect::inspect_code(path)
            .with_context(|| format!("Failed to inspect {:?}", path))?;
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let sub = audit_code_info(&label, &info, config);
        combined.files_scanned += sub.files_scanned;
        combined.structs_scanned += sub.structs_scanned;
        combined.functions_scanned += sub.functions_scanned;
        combined.violations.extend(sub.violations);
        return Ok(combined);
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() && p.extension().map_or(false, |e| e == "rs") {
            let info = crate::inspect::inspect_code(p)
                .with_context(|| format!("Failed to inspect {:?}", p))?;
            let label = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
            let sub = audit_code_info(&label, &info, config);
            combined.files_scanned += sub.files_scanned;
            combined.structs_scanned += sub.structs_scanned;
            combined.functions_scanned += sub.functions_scanned;
            combined.violations.extend(sub.violations);
        }
    }

    // Check Cargo.toml for forbidden deps if present
    if !config.skip_rules.contains(&AuditRule::ForbiddenDeps) {
        let cargo_path = if path.is_dir() {
            path.join("Cargo.toml")
        } else {
            path.parent()
                .unwrap_or(path)
                .join("Cargo.toml")
        };
        if cargo_path.exists() {
            audit_forbidden_deps(&cargo_path, &mut combined)?;
        }
    }

    Ok(combined)
}

// ── Individual Rule Checkers ──────────────────────────────────────────

fn audit_no_unsafe(label: &str, info: &CodeInfo, report: &mut AuditReport) {
    for func in &info.functions {
        // Check function name for 'unsafe'
        if func.name.contains("unsafe") {
            report.push(AuditViolation {
                rule: AuditRule::NoUnsafe,
                severity: AuditSeverity::Error,
                location: format!("{label}::{}", func.name),
                message: "Function name contains 'unsafe' — forbidden in ACCs".into(),
            });
        }
        // Check function body for unsafe blocks (excluding strings/comments)
        let body = prepare_body(&func.body);
        if body.contains("unsafe") {
            report.push(AuditViolation {
                rule: AuditRule::NoUnsafe,
                severity: AuditSeverity::Error,
                location: format!("{label}::{}", func.name),
                message: "Function body contains 'unsafe' block — forbidden in ACCs".into(),
            });
        }
    }
    for st in &info.structs {
        if st.name.contains("UnsafeCell") || st.name.contains("RawPointer") {
            report.push(AuditViolation {
                rule: AuditRule::NoUnsafe,
                severity: AuditSeverity::Warning,
                location: format!("{label}::{}", st.name),
                message: "Struct name suggests unsafe interior mutability".into(),
            });
        }
    }
}

/// Pre-process a body string to strip comments and string literals before
/// pattern scanning, eliminating false positives from forbidden keywords
/// appearing inside comments or string content. Handles raw strings,
/// multi-hash raw strings, byte strings, character literals, and both
/// line and block comments.
fn strip_strings_and_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 1. Check for block comments /* ... */
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            continue;
        }

        // 2. Check for line comments // ...
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // 3. Check for raw strings: r#"..."#, r"...", r##"..."##, etc.
        //    Also handles br#"..."#, br"..."#, etc.
        if chars[i] == 'r'
            || (chars[i] == 'b'
                && i + 1 < chars.len()
                && chars[i + 1] == 'r')
        {
            let is_byte = chars[i] == 'b';
            let start_r = if is_byte { i + 1 } else { i };

            if start_r < chars.len() && chars[start_r] == 'r' {
                let mut hash_count = 0;
                let mut idx = start_r + 1;
                while idx < chars.len() && chars[idx] == '#' {
                    hash_count += 1;
                    idx += 1;
                }
                if idx < chars.len() && chars[idx] == '"' {
                    // Found start of raw string — skip to matching close
                    i = idx + 1; // skip opening quote
                    loop {
                        if i >= chars.len() {
                            break;
                        }
                        if chars[i] == '"' {
                            // Check matching hashes
                            let mut matched = true;
                            for h in 0..hash_count {
                                if i + 1 + h >= chars.len()
                                    || chars[i + 1 + h] != '#'
                                {
                                    matched = false;
                                    break;
                                }
                            }
                            if matched {
                                i += 1 + hash_count;
                                break;
                            }
                        }
                        i += 1;
                    }
                    // Insert placeholder space to prevent word joining
                    result.push(' ');
                    continue;
                }
            }
        }

        // 4. Check for byte string literals b"..."
        if chars[i] == 'b' && i + 1 < chars.len() && chars[i + 1] == '"' {
            i += 2; // skip b"
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2; // skip escape sequence
                    continue;
                }
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            result.push(' ');
            continue;
        }

        // 5. Check for standard string literals "..."
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2; // skip escape sequence (e.g. \", \\)
                    continue;
                }
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            result.push(' ');
            continue;
        }

        // 6. Check for character literals 'a', '\n'
        if chars[i] == '\'' {
            // Simple check to distinguish char literals from lifetimes
            if i + 2 < chars.len() && chars[i + 2] == '\'' && chars[i + 1] != '\\' {
                i += 3;
                result.push(' ');
                continue;
            } else if i + 3 < chars.len() && chars[i + 1] == '\\' && chars[i + 3] == '\'' {
                i += 4;
                result.push(' ');
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Collapse whitespace gaps introduced by `quote::quote!` tokenization.
/// `quote::quote!` emits `.unwrap ()` instead of `.unwrap()` — this
/// normalizes such gaps so downstream pattern matches work reliably.
fn normalize_body(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let chars: Vec<char> = body.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            // Skip spaces before ( ) ! ; , : . < > = { }
            if let Some(&next) = chars.get(i + 1) {
                if matches!(next, '(' | ')' | '!' | ';' | ',' | ':' | '.' | '<' | '>' | '=' | '{' | '}' | '|') {
                    continue;
                }
            }
            // Skip spaces after ( . < > { | = !
            if i > 0 {
                if let Some(&prev) = chars.get(i - 1) {
                    if matches!(prev, '(' | '.' | '<' | '>' | '{' | '|' | '=' | '!') {
                        continue;
                    }
                }
            }
        }
        out.push(ch);
    }
    out
}

/// Prepare a function body for audit pattern scanning: strip comments and
/// string literals, then normalize quote-tokenized whitespace.
fn prepare_body(body: &str) -> String {
    let stripped = strip_strings_and_comments(body);
    normalize_body(&stripped)
}

fn audit_no_unwrap(label: &str, info: &CodeInfo, report: &mut AuditReport) {
    for func in &info.functions {
        // Check parameter types for Option/Result
        for input in &func.inputs {
            if input.contains("Option<") || input.contains("Result<") {
                report.push(AuditViolation {
                    rule: AuditRule::NoUnwrap,
                    severity: AuditSeverity::Info,
                    location: format!("{label}::{}", func.name),
                    message: format!(
                        "Parameter `{}` uses Option/Result — ensure no .unwrap() in body",
                        input.split(':').next().unwrap_or("?").trim()
                    ),
                });
            }
        }
        // Check function body for .unwrap() / .expect() (excluding strings/comments)
        let body = prepare_body(&func.body);
        let has_unwrap = body.contains(".unwrap()") || body.contains(".expect(");
        if has_unwrap {
            report.push(AuditViolation {
                rule: AuditRule::NoUnwrap,
                severity: AuditSeverity::Error,
                location: format!("{label}::{}", func.name),
                message: "Function body contains .unwrap() or .expect() — use Result propagation".into(),
            });
        }
    }
}

fn audit_no_panic(label: &str, info: &CodeInfo, report: &mut AuditReport) {
    for func in &info.functions {
        // Check function name
        if func.name == "unreachable"
            || func.name == "unimplemented"
            || func.name == "todo"
        {
            report.push(AuditViolation {
                rule: AuditRule::NoPanic,
                severity: AuditSeverity::Error,
                location: format!("{label}::{}", func.name),
                message: format!(
                    "Function `{}` is a panic macro — forbidden in ACCs",
                    func.name
                ),
            });
        }
        // Check function body for panic macros (excluding strings/comments)
        let body = prepare_body(&func.body);
        let has_panic = body.contains("todo!")
            || body.contains("unreachable!")
            || body.contains("unimplemented!")
            || body.contains("panic!");
        if has_panic {
            report.push(AuditViolation {
                rule: AuditRule::NoPanic,
                severity: AuditSeverity::Error,
                location: format!("{label}::{}", func.name),
                message: "Function body contains panic macro — forbidden in ACCs".into(),
            });
        }
    }
}

fn audit_no_println(label: &str, info: &CodeInfo, report: &mut AuditReport) {
    for func in &info.functions {
        // Check function name
        if func.name == "println" || func.name == "eprintln" || func.name == "print" {
            report.push(AuditViolation {
                rule: AuditRule::NoPrintln,
                severity: AuditSeverity::Warning,
                location: format!("{label}::{}", func.name),
                message: "Direct print macro — use tracing instead".into(),
            });
        }
        // Check function body for print macros (excluding strings/comments)
        let body = prepare_body(&func.body);
        let has_print = body.contains("println!") || body.contains("eprintln!");
        if has_print {
            report.push(AuditViolation {
                rule: AuditRule::NoPrintln,
                severity: AuditSeverity::Warning,
                location: format!("{label}::{}", func.name),
                message: "Function body contains println!/eprintln! — use tracing instead".into(),
            });
        }
    }
}

fn audit_zero_copy_structs(label: &str, info: &CodeInfo, report: &mut AuditReport) {
    for st in &info.structs {
        if st.visibility == "pub" {
            // Heuristic: public structs with simple scalar fields should derive Pod
            let has_complex_fields = st.fields.iter().any(|f| {
                f.ty.contains("String")
                    || f.ty.contains("Vec<")
                    || f.ty.contains("Box<")
                    || f.ty.contains("HashMap")
                    || f.ty.contains("Rc<")
                    || f.ty.contains("Arc<")
            });

            if !has_complex_fields && !st.fields.is_empty() {
                report.push(AuditViolation {
                    rule: AuditRule::ZeroCopyStructs,
                    severity: AuditSeverity::Warning,
                    location: format!("{label}::{}", st.name),
                    message: format!(
                        "Public struct `{}` has only scalar fields but may be missing `#[derive(bytemuck::Pod)]`",
                        st.name
                    ),
                });
            }
        }
    }
}

fn audit_blast_radius(
    label: &str,
    info: &CodeInfo,
    max_functions: usize,
    report: &mut AuditReport,
) {
    if info.functions.len() > max_functions {
        report.push(AuditViolation {
            rule: AuditRule::ScalingLawBlastRadius,
            severity: AuditSeverity::Warning,
            location: label.to_string(),
            message: format!(
                "File has {} functions (limit {}) — consider splitting into sub-modules",
                info.functions.len(),
                max_functions
            ),
        });
    }
}

fn audit_forbidden_deps(cargo_path: &Path, report: &mut AuditReport) -> Result<()> {
    let content = fs::read_to_string(cargo_path)
        .with_context(|| format!("Failed to read {:?}", cargo_path))?;
    let doc: toml::Value = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {:?}", cargo_path))?;

    let label = cargo_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| cargo_path.display().to_string());

    // Collect all dependency names from [dependencies], [dev-dependencies], [build-dependencies]
    let mut all_deps: Vec<String> = Vec::new();
    for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = doc.get(*section).and_then(|v| v.as_table()) {
            all_deps.extend(deps.keys().cloned());
        }
    }

    for dep in &all_deps {
        if FORBIDDEN_DEPS.contains(&dep.as_str()) {
            report.push(AuditViolation {
                rule: AuditRule::ForbiddenDeps,
                severity: AuditSeverity::Error,
                location: label.clone(),
                message: format!(
                    "Forbidden dependency `{}` — violates scaling-law constraints",
                    dep
                ),
            });
        }
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect::{CodeInfo, FunctionInfo, StructInfo, StructField};

    fn make_code_info(functions: Vec<FunctionInfo>, structs: Vec<StructInfo>) -> CodeInfo {
        CodeInfo {
            functions,
            structs,
            enums: Vec::new(),
            ..Default::default()
        }
    }

    fn make_func(name: &str) -> FunctionInfo {
        FunctionInfo {
            name: name.to_string(),
            visibility: "pub".to_string(),
            inputs: Vec::new(),
            output: None,
            body: String::new(),
        }
    }

    fn make_struct(name: &str, fields: Vec<StructField>) -> StructInfo {
        StructInfo {
            name: name.to_string(),
            visibility: "pub".to_string(),
            fields,
        }
    }

    #[test]
    fn audit_clean_code_passes() {
        let info = make_code_info(
            vec![make_func("compute_score")],
            vec![make_struct("Point", vec![
                StructField { name: "x".into(), ty: "f64".into(), visibility: "pub".into() },
            ])],
        );
        let report = audit_code_info("clean.rs", &info, &AuditConfig::default());
        // Should have zero errors
        assert_eq!(report.count_by_severity(AuditSeverity::Error), 0);
    }

    #[test]
    fn audit_detects_unsafe_in_name() {
        let info = make_code_info(vec![make_func("do_unsafe_thing")], vec![]);
        let report = audit_code_info("bad.rs", &info, &AuditConfig::default());
        let violations = report.violations_for_rule(AuditRule::NoUnsafe);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, AuditSeverity::Error);
    }

    #[test]
    fn audit_detects_panic_macros() {
        let info = make_code_info(vec![make_func("unreachable")], vec![]);
        let report = audit_code_info("bad.rs", &info, &AuditConfig::default());
        let violations = report.violations_for_rule(AuditRule::NoPanic);
        assert!(!violations.is_empty());
    }

    #[test]
    fn audit_detects_print_macros() {
        let info = make_code_info(vec![make_func("println")], vec![]);
        let report = audit_code_info("bad.rs", &info, &AuditConfig::default());
        let violations = report.violations_for_rule(AuditRule::NoPrintln);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, AuditSeverity::Warning);
    }

    #[test]
    fn audit_detects_unwrap_params() {
        let info = make_code_info(
            vec![FunctionInfo {
                name: "process".into(),
                visibility: "pub".into(),
                inputs: vec!["data: Option<Vec<u8>>".into()],
                output: None,
                body: String::new(),
            }],
            vec![],
        );
        let report = audit_code_info("mod.rs", &info, &AuditConfig::default());
        let violations = report.violations_for_rule(AuditRule::NoUnwrap);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, AuditSeverity::Info);
    }

    #[test]
    fn audit_zero_copy_struct_flagged() {
        let info = make_code_info(
            vec![],
            vec![make_struct("Header", vec![
                StructField { name: "tag".into(), ty: "u32".into(), visibility: "pub".into() },
                StructField { name: "len".into(), ty: "u32".into(), visibility: "pub".into() },
            ])],
        );
        let report = audit_code_info("ipc.rs", &info, &AuditConfig::default());
        let violations = report.violations_for_rule(AuditRule::ZeroCopyStructs);
        assert!(!violations.is_empty());
    }

    #[test]
    fn audit_zero_copy_complex_struct_not_flagged() {
        let info = make_code_info(
            vec![],
            vec![make_struct("Config", vec![
                StructField { name: "name".into(), ty: "String".into(), visibility: "pub".into() },
                StructField { name: "items".into(), ty: "Vec<u8>".into(), visibility: "pub".into() },
            ])],
        );
        let report = audit_code_info("config.rs", &info, &AuditConfig::default());
        let violations = report.violations_for_rule(AuditRule::ZeroCopyStructs);
        assert!(violations.is_empty());
    }

    #[test]
    fn audit_blast_radius_triggers() {
        let funcs: Vec<FunctionInfo> = (0..60).map(|i| make_func(&format!("fn_{i}"))).collect();
        let info = make_code_info(funcs, vec![]);
        let config = AuditConfig { max_functions_per_file: 50, ..Default::default() };
        let report = audit_code_info("mega.rs", &info, &config);
        let violations = report.violations_for_rule(AuditRule::ScalingLawBlastRadius);
        assert!(!violations.is_empty());
    }

    #[test]
    fn audit_report_summary_format() {
        let mut report = AuditReport::new();
        report.files_scanned = 3;
        report.structs_scanned = 10;
        report.functions_scanned = 25;
        report.push(AuditViolation {
            rule: AuditRule::NoUnsafe,
            severity: AuditSeverity::Error,
            location: "test.rs".into(),
            message: "found unsafe".into(),
        });
        report.push(AuditViolation {
            rule: AuditRule::NoPrintln,
            severity: AuditSeverity::Warning,
            location: "test.rs".into(),
            message: "found println".into(),
        });
        assert!(report.has_errors());
        assert_eq!(report.count_by_severity(AuditSeverity::Error), 1);
        assert_eq!(report.count_by_severity(AuditSeverity::Warning), 1);
        let s = report.summary();
        assert!(s.contains("3 files"));
        assert!(s.contains("1 errors"));
    }

    #[test]
    fn audit_skip_rules() {
        let info = make_code_info(vec![make_func("do_unsafe_thing")], vec![]);
        let config = AuditConfig {
            skip_rules: vec![AuditRule::NoUnsafe],
            ..Default::default()
        };
        let report = audit_code_info("skipped.rs", &info, &config);
        let violations = report.violations_for_rule(AuditRule::NoUnsafe);
        assert!(violations.is_empty());
    }

    #[test]
    fn audit_forbidden_deps_detection() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_path,
            "[dependencies]\nhyper = \"1.0\"\nserde = { version = \"1.0\", features = [\"derive\"] }\n",
        )
        .unwrap();
        let mut report = AuditReport::new();
        audit_forbidden_deps(&cargo_path, &mut report).unwrap();
        let violations = report.violations_for_rule(AuditRule::ForbiddenDeps);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("hyper"));
    }
}
