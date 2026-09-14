// Integration tests for Ambient AST Rewriter and Domain Grafter

use orchestration_plane::ast_transformer::AmbientAstRewriter;
use orchestration_plane::grafter::graft_module_to_crate;
use orchestration_plane::domain_classifier::{Domain, DomainClassifier};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_ast_transformer_rewrites_canonicalize_and_temp_dir() {
    let mut rewriter = AmbientAstRewriter::new();
    let raw_source = r#"
        pub fn process_artifact(file_path: std::path::PathBuf) -> std::path::PathBuf {
            let temp_workspace = std::env::temp_dir();
            let canonical_target = file_path.canonicalize();
            canonical_target
        }
    "#;

    let (rewritten, summary) = rewriter.rewrite_source(raw_source).expect("AST rewriting must succeed");

    assert_eq!(summary.canonicalize_rewrites, 1);
    assert_eq!(summary.env_temp_dir_rewrites, 1);

    // Assert that ambient authority calls are eliminated
    assert!(!rewritten.contains(".canonicalize()"));
    assert!(!rewritten.contains("std::env::temp_dir()"));

    // Assert deterministic replacements
    assert!(rewritten.contains("paths::normalize_path(&file_path)"));
    assert!(rewritten.contains("paths::WorkspacePaths::default().cache()"));

    // Assert that imports are auto-injected
    assert!(rewritten.contains("use paths::normalize_path;"));
    assert!(rewritten.contains("use paths::WorkspacePaths;"));

    // Confirm that the output parses cleanly as valid Rust AST
    let parsed: Result<syn::File, _> = syn::parse_str(&rewritten);
    assert!(parsed.is_ok(), "Rewritten code must parse cleanly with syn");
}

#[test]
fn test_pipeline_classify_rewrite_and_graft() {
    let classifier = DomainClassifier::default();
    let raw_source = r#"
        use candle::Tensor;

        pub struct SimdKernel {
            pub weights: Tensor,
        }

        impl SimdKernel {
            pub fn compute_activation(&self, path: std::path::PathBuf) -> Tensor {
                let _safe = path.canonicalize();
                self.weights.clone()
            }
        }
    "#;

    // 1. Classify
    let report = classifier.classify_source("simd_kernel.rs", raw_source).unwrap();
    assert_eq!(report.target_domain, Domain::Compute);

    // 2. Rewrite ambient calls
    let mut rewriter = AmbientAstRewriter::new();
    let (remediated, summary) = rewriter.rewrite_source(raw_source).unwrap();
    assert_eq!(summary.canonicalize_rewrites, 1);
    assert!(!remediated.contains(".canonicalize()"));

    // 3. Graft into mock crate
    let temp = tempdir().unwrap();
    let target_crate = temp.path().join("compute");
    fs::create_dir_all(target_crate.join("src")).unwrap();
    fs::write(target_crate.join("src/lib.rs"), "// Compute root\n").unwrap();

    let graft_report = graft_module_to_crate(&report, &remediated, &target_crate).unwrap();
    assert_eq!(graft_report.module_name, "simd_kernel");
    assert!(graft_report.destination_file.exists());

    let grafted_content = fs::read_to_string(&graft_report.destination_file).unwrap();
    assert!(grafted_content.contains("paths::normalize_path(&path)"));

    let lib_rs = fs::read_to_string(graft_report.updated_lib_rs.as_ref().unwrap()).unwrap();
    assert!(lib_rs.contains("pub mod simd_kernel;"));
}
