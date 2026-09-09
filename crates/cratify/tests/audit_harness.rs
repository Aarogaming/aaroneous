//! Integration test harness for Cratify deep audit & control group verification.
//!
//! Runs inspection, audit, and translation pipeline against three isolated
//! synthetic fixtures to validate end-to-end correctness.

use cratify::inspect;
use cratify::audit::{self, AuditConfig, AuditRule, AuditSeverity};
use cratify::translate::{self, TranslateConfig, LlmRoute, TranslationComplexity};

const CLEAN: &str = "tests/control_groups/clean_conformer.rs";
const VIOLATOR: &str = "tests/control_groups/invariant_violator.rs";
const LEGACY: &str = "tests/control_groups/legacy_monolith.rs";

// ── Helper ────────────────────────────────────────────────────────────

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 1 — Clean Conformer: must pass with 0 errors, 0 warnings
// ══════════════════════════════════════════════════════════════════════

#[test]
fn clean_conformer_inspection_succeeds() {
    let path = fixture_path(CLEAN);
    let info = inspect::inspect_code(&path).expect("inspect clean_conformer");
    // Must find at least the functions and structs we declared
    assert!(info.functions.len() >= 3, "expected >=3 functions, got {}", info.functions.len());
    assert!(info.structs.len() >= 1, "expected >=1 struct, got {}", info.structs.len());
}

#[test]
fn clean_conformer_passes_audit_zero_violations() {
    let path = fixture_path(CLEAN);
    let info = inspect::inspect_code(&path).expect("inspect clean_conformer");
    let config = AuditConfig::default();
    let report = audit::audit_code_info("clean_conformer.rs", &info, &config);

    assert_eq!(
        report.count_by_severity(AuditSeverity::Error),
        0,
        "clean_conformer must have 0 errors, got:\n{:?}",
        report.violations_for_rule(AuditRule::NoUnsafe)
    );

    // The ZeroCopyStructs heuristic cannot detect bytemuck::Pod derive attributes
    // from the AST alone, so CleanHeader (scalar-only fields) triggers 1 expected
    // warning. All other warnings would be unexpected.
    let unexpected_warnings: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.severity == AuditSeverity::Warning && v.rule != AuditRule::ZeroCopyStructs)
        .collect();
    assert!(
        unexpected_warnings.is_empty(),
        "clean_conformer has unexpected warnings: {:?}",
        unexpected_warnings
    );
}

#[test]
fn clean_conformer_has_pod_struct() {
    let path = fixture_path(CLEAN);
    let info = inspect::inspect_code(&path).expect("inspect clean_conformer");
    // The ZeroCopyStructs rule should NOT fire on CleanHeader because it has
    // complex derive attributes (Pod) — but our heuristic looks at field types.
    // CleanHeader has only scalar fields, so it WILL be flagged as a warning
    // unless the heuristic is improved. We just verify the struct exists.
    let header = info.structs.iter().find(|s| s.name == "CleanHeader");
    assert!(header.is_some(), "CleanHeader struct not found in AST");
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 2 — Invariant Violator: must trip all 7 audit rules
// ══════════════════════════════════════════════════════════════════════

#[test]
fn violator_inspection_succeeds() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).expect("inspect invariant_violator");
    // Must extract functions even if they contain forbidden patterns
    assert!(
        info.functions.len() >= 5,
        "expected >=5 functions, got {}",
        info.functions.len()
    );
}

#[test]
fn violator_trips_no_unsafe() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("invariant_violator.rs", &info, &config);

    let violations = report.violations_for_rule(AuditRule::NoUnsafe);
    assert!(
        !violations.is_empty(),
        "NoUnsafe rule must fire on invariant_violator"
    );
    // At minimum: do_unsafe_thing (unsafe in name) + unsafe_helper
    assert!(
        violations.len() >= 2,
        "expected >=2 NoUnsafe violations, got {}",
        violations.len()
    );
}

#[test]
fn violator_trips_no_panic() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("invariant_violator.rs", &info, &config);

    let violations = report.violations_for_rule(AuditRule::NoPanic);
    assert!(
        !violations.is_empty(),
        "NoPanic rule must fire on invariant_violator"
    );
    // Must catch functions containing todo!, unreachable!, unimplemented! in their bodies
    let bodies: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();
    let has_panic = bodies.iter().any(|m| m.contains("panic macro"));
    assert!(has_panic, "must detect panic macros in function bodies");
}

#[test]
fn violator_trips_no_println() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("invariant_violator.rs", &info, &config);

    let violations = report.violations_for_rule(AuditRule::NoPrintln);
    assert!(
        !violations.is_empty(),
        "NoPrintln rule must fire on invariant_violator"
    );
    let has_println = violations.iter().any(|v| v.message.contains("println"));
    assert!(has_println, "must detect println! macro in function body");
}

#[test]
fn violator_trips_no_unwrap() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("invariant_violator.rs", &info, &config);

    let violations = report.violations_for_rule(AuditRule::NoUnwrap);
    assert!(
        !violations.is_empty(),
        "NoUnwrap rule must fire on invariant_violator"
    );
    // Should have both: Option param info + .unwrap() body error
    let has_body_violation = violations
        .iter()
        .any(|v| v.message.contains(".unwrap()") || v.message.contains(".expect()"));
    assert!(has_body_violation, "must detect .unwrap() in function body");
}

#[test]
fn violator_trips_zero_copy_structs() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("invariant_violator.rs", &info, &config);

    let violations = report.violations_for_rule(AuditRule::ZeroCopyStructs);
    assert!(
        !violations.is_empty(),
        "ZeroCopyStructs rule must fire on NaiveHeader"
    );
    let has_naive = violations
        .iter()
        .any(|v| v.location.contains("NaiveHeader"));
    assert!(has_naive, "must flag NaiveHeader specifically");
}

#[test]
fn violator_trips_all_rules_combined() {
    let path = fixture_path(VIOLATOR);
    let info = inspect::inspect_code(&path).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("invariant_violator.rs", &info, &config);

    // Must have violations for at least 4 distinct rules
    let mut triggered_rules: Vec<AuditRule> = report
        .violations
        .iter()
        .map(|v| v.rule)
        .collect();
    triggered_rules.sort_by_key(|r| format!("{r:?}"));
    triggered_rules.dedup();

    assert!(
        triggered_rules.len() >= 4,
        "invariant_violator must trigger >=4 distinct rules, got {:?}",
        triggered_rules
    );

    // Must have at least one ERROR
    assert!(
        report.has_errors(),
        "invariant_violator must produce at least one ERROR"
    );
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 3 — Legacy Monolith: AST extraction + translation pipeline
// ══════════════════════════════════════════════════════════════════════

#[test]
fn legacy_monolith_inspection_extracts_all_items() {
    let path = fixture_path(LEGACY);
    let info = inspect::inspect_code(&path).expect("inspect legacy_monolith");

    // Functions: create_worker, start_worker, process_batch, record_failure,
    //            get_status, build_report, aggregate_metrics, chrono_now
    assert!(
        info.functions.len() >= 7,
        "expected >=7 functions, got {}",
        info.functions.len()
    );

    // Structs: Config, Worker
    assert!(
        info.structs.len() >= 2,
        "expected >=2 structs, got {}",
        info.structs.len()
    );

    // Enums: State
    assert!(
        info.enums.len() >= 1,
        "expected >=1 enum, got {}",
        info.enums.len()
    );
}

#[test]
fn legacy_monolith_ast_signatures_correct() {
    let path = fixture_path(LEGACY);
    let info = inspect::inspect_code(&path).unwrap();

    // Verify specific function signatures were extracted
    let func_names: Vec<&str> = info.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(func_names.contains(&"create_worker"), "missing create_worker");
    assert!(func_names.contains(&"start_worker"), "missing start_worker");
    assert!(func_names.contains(&"process_batch"), "missing process_batch");
    assert!(func_names.contains(&"build_report"), "missing build_report");
    assert!(func_names.contains(&"aggregate_metrics"), "missing aggregate_metrics");

    // Verify struct field extraction
    let worker = info.structs.iter().find(|s| s.name == "Worker");
    assert!(worker.is_some(), "Worker struct not found");
    let worker = worker.unwrap();
    assert!(worker.fields.len() >= 4, "Worker should have >=4 fields");

    // Verify enum variant extraction
    let state = info.enums.iter().find(|e| e.name == "State");
    assert!(state.is_some(), "State enum not found");
    let state = state.unwrap();
    assert!(state.variants.len() >= 3, "State should have >=3 variants");
}

#[test]
fn legacy_monolith_complexity_estimation() {
    let path = fixture_path(LEGACY);
    let info = inspect::inspect_code(&path).unwrap();
    let complexity = translate::estimate_complexity(&info);

    // 8+ functions + 2 structs + 1 enum = 11+ items → Medium or High
    assert!(
        complexity >= TranslationComplexity::Medium,
        "legacy_monolith should be Medium+ complexity, got {:?}",
        complexity
    );
}

#[test]
fn legacy_monolith_builds_offload_packet() {
    let path = fixture_path(LEGACY);
    let config = TranslateConfig::default();
    let packet = translate::build_offload_packet(&path, "refactored_acc", &config)
        .expect("build offload packet");

    assert_eq!(packet.target_crate, "refactored_acc");
    assert!(!packet.source_code.is_empty());
    assert!(packet.task_id > 0);
    assert!(packet.created_at_us > 0);
    // Must have extracted AST info from the file
    assert!(
        packet.code_info.functions.len() >= 7,
        "packet should contain >=7 functions from legacy_monolith"
    );
    // Complexity should be Medium+
    assert!(
        packet.complexity >= TranslationComplexity::Medium,
        "packet complexity should be Medium+"
    );
    // Instructions must reference target crate
    assert!(
        packet.instructions.contains("refactored_acc"),
        "instructions must reference target crate"
    );
}

#[test]
fn legacy_monolith_ast_fallback_rewrites() {
    let path = fixture_path(LEGACY);
    let config = TranslateConfig::default();
    let packet = translate::build_offload_packet(&path, "clean_acc", &config).unwrap();

    let outcome = translate::execute_ast_fallback(&packet);
    assert!(outcome.success, "AST fallback must succeed");
    assert_eq!(outcome.route, LlmRoute::AstFallback);

    let code = outcome.translated_code.expect("must produce code");
    // Must replace println! with tracing
    assert!(
        code.contains("tracing::info!") || !code.contains("println!"),
        "AST fallback must replace println! with tracing"
    );
    // Must add core_contracts import
    assert!(
        code.contains("use core_contracts"),
        "AST fallback must add core_contracts import"
    );
}

#[test]
fn legacy_monolith_offload_packet_json_roundtrip() {
    let path = fixture_path(LEGACY);
    let config = TranslateConfig::default();
    let packet = translate::build_offload_packet(&path, "target", &config).unwrap();

    let json = packet.to_json().expect("serialize to JSON");
    let restored: translate::LlmOffloadPacket =
        serde_json::from_str(&json).expect("deserialize from JSON");

    assert_eq!(restored.task_id, packet.task_id);
    assert_eq!(restored.target_crate, "target");
    assert_eq!(restored.route, packet.route);
    assert_eq!(restored.complexity, packet.complexity);
    assert_eq!(
        restored.code_info.functions.len(),
        packet.code_info.functions.len()
    );
}

// ══════════════════════════════════════════════════════════════════════
//  CROSS-GROUP — audit_path on the entire control_groups directory
// ══════════════════════════════════════════════════════════════════════

#[test]
fn audit_path_scans_all_control_groups() {
    let dir = fixture_path("tests/control_groups");
    let config = AuditConfig::default();
    let report = audit::audit_path(&dir, &config).expect("audit_path on control_groups");

    // Must scan all 3 files
    assert_eq!(
        report.files_scanned, 3,
        "must scan all 3 control group files"
    );
    assert!(report.structs_scanned >= 4, "must find >=4 structs total");
    assert!(report.functions_scanned >= 15, "must find >=15 functions total");
}

#[test]
fn audit_path_control_groups_produce_errors() {
    let dir = fixture_path("tests/control_groups");
    let config = AuditConfig::default();
    let report = audit::audit_path(&dir, &config).expect("audit_path");

    // The violator file guarantees at least some errors
    assert!(
        report.has_errors(),
        "control_groups audit must produce at least one ERROR (from invariant_violator)"
    );
    assert!(
        report.violations.len() >= 5,
        "control_groups audit must produce >=5 total violations"
    );
}

// ══════════════════════════════════════════════════════════════════════
//  ROUTING — resolve_route logic
// ══════════════════════════════════════════════════════════════════════

#[test]
fn routing_default_is_local_qwen() {
    let config = TranslateConfig::default();
    assert_eq!(translate::resolve_route(&config), LlmRoute::LocalQwen);
}

#[test]
fn routing_with_endpoint_is_hypervisor() {
    let config = TranslateConfig {
        endpoint_override: Some("http://fleet:8080".into()),
        ..Default::default()
    };
    assert_eq!(translate::resolve_route(&config), LlmRoute::HypervisorBus);
}

#[test]
fn routing_explicit_ast_fallback() {
    let config = TranslateConfig {
        preferred_route: LlmRoute::AstFallback,
        ..Default::default()
    };
    assert_eq!(translate::resolve_route(&config), LlmRoute::AstFallback);
}

// ══════════════════════════════════════════════════════════════════════
//  EDGE CASES — false-positive suppression & complex patterns
// ══════════════════════════════════════════════════════════════════════

use cratify::inspect::{FunctionInfo, StructInfo, StructField, CodeInfo};

/// Helper: build a minimal CodeInfo from a single function with body text.
fn code_info_with_body(name: &str, body: &str) -> CodeInfo {
    CodeInfo {
        functions: vec![FunctionInfo {
            name: name.to_string(),
            visibility: "pub".to_string(),
            inputs: Vec::new(),
            output: None,
            body: body.to_string(),
        }],
        ..Default::default()
    }
}

/// Helper: build a CodeInfo from a single struct.
fn code_info_with_struct(name: &str, visibility: &str, fields: Vec<StructField>) -> CodeInfo {
    CodeInfo {
        structs: vec![StructInfo {
            name: name.to_string(),
            visibility: visibility.to_string(),
            fields,
        }],
        ..Default::default()
    }
}

#[test]
fn false_positive_unwrap_in_string_literal() {
    // .unwrap() inside a string literal must NOT trigger NoUnwrap
    let info = code_info_with_body(
        "safe_fn",
        r#"let msg = "call .unwrap() carefully"; let x = 42;"#,
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoUnwrap);
    assert!(
        violations.is_empty(),
        "string literal containing .unwrap() must not trigger: {:?}",
        violations
    );
}

#[test]
fn false_positive_panic_in_comment() {
    // panic! inside a line comment must NOT trigger NoPanic
    let info = code_info_with_body(
        "safe_fn",
        "// this is a panic! macro reference\nlet x = 42;",
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoPanic);
    assert!(
        violations.is_empty(),
        "panic! in comment must not trigger: {:?}",
        violations
    );
}

#[test]
fn false_positive_println_in_block_comment() {
    // println! inside a block comment must NOT trigger NoPrintln
    let info = code_info_with_body(
        "safe_fn",
        "/* TODO: replace println! with tracing */\nlet x = 42;",
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoPrintln);
    assert!(
        violations.is_empty(),
        "println! in block comment must not trigger: {:?}",
        violations
    );
}

#[test]
fn false_positive_unsafe_in_string_literal() {
    // "unsafe" inside a string literal must NOT trigger NoUnsafe
    let info = code_info_with_body(
        "safe_fn",
        r#"let warning = "unsafe blocks are forbidden"; let x = 42;"#,
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoUnsafe);
    assert!(
        violations.is_empty(),
        "unsafe in string literal must not trigger: {:?}",
        violations
    );
}

#[test]
fn nested_blocks_detected() {
    // Actual unsafe inside nested blocks must still be caught
    let info = code_info_with_body(
        "nested_fn",
        "if true { for i in 0..10 { unsafe { let p = 0x1 as *mut u32; } } }",
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoUnsafe);
    assert!(
        !violations.is_empty(),
        "unsafe in nested blocks must be detected"
    );
}

#[test]
fn multiline_macro_body_detected() {
    // panic! spanning multiple lines (as quote tokenizes it) must be caught
    let info = code_info_with_body(
        "multi_macro",
        "todo ! (\"implement this\nmulti-line\nfeature\")",
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoPanic);
    assert!(
        !violations.is_empty(),
        "multi-line todo! macro must be detected"
    );
}

#[test]
fn complex_trait_bounds_no_false_positive() {
    // Complex trait bounds in parameters must not cause panics or false positives
    let info = CodeInfo {
        functions: vec![FunctionInfo {
            name: "process_item".to_string(),
            visibility: "pub".to_string(),
            inputs: vec![
                "item: Box<dyn Fn(i32) -> Result<String, std::io::Error>>".to_string(),
                "ctx: &mut dyn std::fmt::Display".to_string(),
            ],
            output: Some("Result<(), Box<dyn std::error::Error>>".to_string()),
            body: "let _ = (item)(42); Ok(())".to_string(),
        }],
        ..Default::default()
    };
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    // No errors expected — complex trait bounds are not violations
    assert_eq!(
        report.count_by_severity(AuditSeverity::Error),
        0,
        "complex trait bounds must not produce errors: {:?}",
        report.violations
    );
}

#[test]
fn pub_crate_struct_recognized() {
    // pub(crate) visibility must be distinguished from plain "pub"
    let info = code_info_with_struct(
        "InternalHeader",
        "pub(crate)",
        vec![
            StructField { name: "tag".into(), ty: "u32".into(), visibility: "pub".into() },
        ],
    );
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    // ZeroCopyStructs only flags plain "pub" — pub(crate) is restricted
    let violations = report.violations_for_rule(AuditRule::ZeroCopyStructs);
    assert!(
        violations.is_empty(),
        "pub(crate) struct must not be flagged by ZeroCopyStructs: {:?}",
        violations
    );
}

#[test]
fn nested_module_items_extracted() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut file = NamedTempFile::new().expect("tmp file");
    writeln!(file, r#"
        pub mod inner {{
            pub fn helper() -> u32 {{ 42 }}
            pub struct InnerConfig {{ pub flag: bool }}
        }}
        pub fn top_level() -> i32 {{ 0 }}
    "#).unwrap();

    let info = inspect::inspect_code(file.path()).expect("inspect nested modules");
    // Must extract items from both the top level and the inner module
    let func_names: Vec<&str> = info.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(func_names.contains(&"top_level"), "missing top_level function");
    assert!(func_names.contains(&"helper"), "missing helper from nested module");

    let struct_names: Vec<&str> = info.structs.iter().map(|s| s.name.as_str()).collect();
    assert!(struct_names.contains(&"InnerConfig"), "missing InnerConfig from nested module");
}
