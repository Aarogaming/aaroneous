// Normalization Pipeline Integration Tests

use orchestration_plane::normalization_pipeline::{NormalizationPipeline, InvariantSeverity};
use std::path::PathBuf;
use std::fs;
use std::env;

#[test]
fn test_inspect_target_flags_unwrap() {
    // Create a temporary directory with test file containing unwrap
    let temp_dir = env::temp_dir().join("orchestration_plane_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test_unwrap.rs");
    fs::write(
        &test_file,
        r#"fn test() {
    let x: Option<i32> = None;
    x.unwrap();
}"#,
    ).unwrap();

    let pipeline = NormalizationPipeline::default();
    let plan = pipeline.inspect_target(&temp_dir).unwrap();

    // Verify unwrap violation was detected
    assert!(!plan.violations.is_empty(), "Expected at least one violation");
    let unwrap_violation = plan.violations.iter().find(|v| v.description.contains("unwrap")).unwrap();
    assert_eq!(unwrap_violation.severity, InvariantSeverity::Warning);

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_generate_normalization_patch() {
    let pipeline = NormalizationPipeline::default();
    let plan = IngestionPlan {
        target_path: PathBuf::from("/test/path"),
        scanned_files: vec![],
        violations: vec![InvariantViolation {
            file: PathBuf::from("/test/file.rs"),
            line: 10,
            description: "Test violation".to_string(),
            severity: InvariantSeverity::Warning,
        }],
    };

    let report = pipeline.generate_normalization_patch(&plan).unwrap();
    
    assert!(!report.patch_diff.is_empty());
    assert!(report.certified); // Warnings alone don't prevent certification
}
