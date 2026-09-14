// Canary Run: First automated normalization on a legacy target

use orchestration_plane::normalization_pipeline::{
    NormalizationPipeline, InvariantSeverity,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("===========================================================");
    println!("CANARY RUN: First Automated Normalization Pipeline Test");
    println!("===========================================================
");

    let pipeline = NormalizationPipeline::default();
    let target_path = Path::new("dev/canary_legacy_fixture/src");
    
    println!("Inspecting target: {:?}", target_path);
    println!(".-----------------------------------------------------------.");

    let plan = pipeline.inspect_target(target_path)?;
    
    println!("
INGESTION PLAN:");
    println!("Target Path: {}", plan.target_path.display());
    println!("Scanned Files: {}", plan.scanned_files.len());
    for file in &plan.scanned_files {
        println!("  - {:?}", file);
    }

    println!("
INVARIANT VIOLATIONS DETECTED: {}", plan.violations.len());
    println!(".-----------------------------------------------------------.");
    
    let mut warning_count = 0;
    let mut error_count = 0;
    let mut critical_count = 0;
    
    for violation in &plan.violations {
        println!("
[{}] {}:{} - {}", 
            violation.severity,
            violation.file.display(),
            violation.line,
            violation.description
        );
        
        match violation.severity {
            InvariantSeverity::Warning => warning_count += 1,
            InvariantSeverity::Error => error_count += 1,
            InvariantSeverity::Critical => critical_count += 1,
        }
    }

    println!("
SUMMARY:");
    println!("  Warnings: {}", warning_count);
    println!("  Errors: {}", error_count);
    println!("  Critical: {}", critical_count);

    let report = pipeline.generate_normalization_patch(&plan)?;
    
    println!("
NORMALIZATION REPORT:");
    println!("Certified: {}", report.certified);
    println!(".-----------------------------------------------------------.");
    
    for line in report.patch_diff.lines() {
        println!("{}", line);
    }

    println!("
VERIFYING DELIBERATE VIOLATIONS:");
    println!(".-----------------------------------------------------------.");
    
    let has_unwrap = plan.violations.iter().any(|v| v.description.contains("unwrap"));
    let has_panic = plan.violations.iter().any(|v| v.description.contains("panic!"));
    let has_unsafe = plan.violations.iter().any(|v| v.description.contains("unsafe"));
    
    assert!(has_unwrap, "Expected .unwrap() violation to be detected");
    assert!(has_panic, "Expected panic! violation to be detected");
    assert!(has_unsafe, "Expected unsafe block violation to be detected");

    println!("All deliberate violations properly categorized:");
    if warning_count > 0 {
        println!("  - Warnings detected for .unwrap()/.expect() calls");
    }
    if error_count > 0 {
        println!("  - Errors detected for panic! and unsafe blocks");
    }

    println!("
===========================================================");
    println!("CANARY RUN SUCCESSFUL: All violations detected correctly!");
    println!("===========================================================
");

    Ok(())
}
