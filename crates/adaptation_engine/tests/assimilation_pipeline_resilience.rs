use adaptation_engine::*;
use std::path::Path;

// ============================================================================
// Best-Case Scenario Tests (High-Throughput, Clean AST & Promotion)
// ============================================================================

#[test]
fn test_best_case_multi_function_ast_dissection() {
    let code = r#"
pub fn compute_sum(a: i32, b: i32) -> i32 {
    a + b
}

pub fn compute_product(a: i32, b: i32) -> i32 {
    a * b
}

fn internal_helper() -> bool {
    true
}
"#;
    let obs = AdaptationEngine::inspect_source("pipeline_test.rs", code).unwrap();
    assert_eq!(obs.file_path, "pipeline_test.rs");
    assert_eq!(obs.functions.len(), 3);
    assert_eq!(obs.functions[0].name, "compute_sum");
    assert_eq!(obs.functions[1].name, "compute_product");
    assert_eq!(obs.functions[2].name, "internal_helper");
}

#[test]
fn test_best_case_pattern_rewrite_and_atomic_promotion() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = ShadowSandbox::with_dir(temp.path().join("shadow")).unwrap();

    let original = r#"
pub fn legacy_routine() {
    println!("processing payload");
}
"#;
    let (rewritten, patches) = AdaptationEngine::rewrite_pattern(
        "module.rs",
        original,
        "println!(\"processing payload\")",
        "tracing::info!(\"processing payload\")",
    )
    .unwrap();

    assert!(!patches.is_empty());
    assert!(rewritten.contains("tracing::info!"));

    // Write to shadow and promote to live target
    sandbox
        .write_shadow_file("module.rs", rewritten.as_bytes())
        .unwrap();
    let live_target = temp.path().join("live").join("module.rs");
    sandbox.promote_to_live("module.rs", &live_target).unwrap();

    assert!(live_target.exists());
    let promoted_content = std::fs::read_to_string(&live_target).unwrap();
    assert_eq!(promoted_content, rewritten);
}

#[test]
fn test_best_case_auto_wrapper_slug_sanitization() {
    let manifest = AutoWrapperEngine::inspect_target(
        Path::new("tools/bin/my-awesome-tool.exe"),
        Some("My Custom-Tool 1.0"),
    )
    .unwrap();

    assert_eq!(manifest.name, "My Custom-Tool 1.0");
    assert_eq!(manifest.slug, "my_custom_tool_1_0");
    assert_eq!(manifest.program_type, TargetProgramType::CliExecutable);
}

// ============================================================================
// Worst-Case Scenario Tests (Malformed Payload, Path Traversal, Corrupted Bytes)
// ============================================================================

#[test]
fn test_worst_case_path_traversal_custom_name_sanitization() {
    // Malicious custom name attempting directory traversal
    let manifest = AutoWrapperEngine::inspect_target(
        Path::new("bin/tool.exe"),
        Some("../../etc/passwd"),
    )
    .unwrap();

    // Custom name file_name() isolation converts "../../etc/passwd" -> "passwd"
    assert_eq!(manifest.name, "passwd");
    assert_eq!(manifest.slug, "passwd");
    assert_no_path_traversal(&manifest.slug);
}

#[test]
fn test_worst_case_corrupted_binary_dissection() {
    // Arbitrary unparseable noise bytes
    let noise_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x12, 0x34];
    let manifest = AdaptationEngine::inspect_binary("corrupted.bin", &noise_bytes).unwrap();

    assert_eq!(manifest.file_path, "corrupted.bin");
    assert_eq!(manifest.file_size_bytes, 8);
    // Should fallback cleanly without crashing
    assert!(manifest.sections.is_empty());
}

#[test]
fn test_worst_case_empty_source_dissection() {
    let obs = AdaptationEngine::inspect_source("empty_source.rs", "").unwrap();
    assert_eq!(obs.file_path, "empty_source.rs");
    assert!(obs.functions.is_empty());
}

#[test]
fn test_worst_case_non_existent_shadow_promotion() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = ShadowSandbox::with_dir(temp.path().join("shadow")).unwrap();
    let live_target = temp.path().join("live").join("nonexistent.rs");

    let result = sandbox.promote_to_live("nonexistent.rs", &live_target);
    assert!(result.is_err(), "Promoting non-existent shadow file must return Err");
}

#[test]
fn test_worst_case_pattern_rewrite_no_match() {
    let original = "fn simple() {}";
    let (rewritten, patches) = AdaptationEngine::rewrite_pattern(
        "simple.rs",
        original,
        "non_existent_pattern_string",
        "replacement",
    )
    .unwrap();

    assert_eq!(rewritten, original);
    assert!(patches.is_empty());
}

fn assert_no_path_traversal(slug: &str) {
    assert!(!slug.contains(".."));
    assert!(!slug.contains('/'));
    assert!(!slug.contains('\\'));
}
