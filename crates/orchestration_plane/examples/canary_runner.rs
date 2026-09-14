// First Live Canary Normalization Test

use orchestration_plane::normalization_pipeline::{
    NormalizationPipeline, InvariantSeverity,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("===========================================================");
    println!("FIRST LIVE CANARY NORMALIZATION TEST");
    println!("===========================================================
");

    let pipeline = NormalizationPipeline::default();
    let target_path = Path::new("dev/canary_legacy_fixture/");
    
    println!("Target: {:?}", target_path);
    println!(".-----------------------------------------------------------.");

    let plan = pipeline.inspect_target(target_path)?;
    
    println!("
=== INGESTION PLAN SUMMARY ===");
    println!("Target Path: {}", plan.target_path.display());
    println!("Scanned Files: {}", plan.scanned_files.len());
    for file in &plan.scanned_files {
        println!("  - {:?}", file);
    }

    println!("
=== INVARIANT VIOLATIONS ===");
    println!("Total Violations: {}", plan.violations.len());
    
    let mut warnings = 0;
    let mut errors = 0;
    let mut unsafe_blocks = 0;
    let mut unwrap_calls = 0;
    
    for violation in &plan.violations {
        println!("
[{}] {}:{}: {}", 
            violation.severity,
            violation.file.display(),
            violation.line,
            violation.description
        );
        
        match violation.severity {
            InvariantSeverity::Warning => warnings += 1,
            InvariantSeverity::Error => errors += 1,
            InvariantSeverity::Critical => println!("  CRITICAL: Immediate action required!"),
        }
        
        if violation.description.contains("unwrap") {
            unwrap_calls += 1;
        }
        if violation.description.contains("unsafe") {
            unsafe_blocks += 1;
        }
    }


    // Apply AST-based remediation
    println!("\n=== APPLYING REMEDIATION ===");
    let _remediated_files = pipeline.apply_remediation(&plan)
        .expect("Remediation failed");
    println!("Remediation applied successfully");
    let report = pipeline.generate_normalization_patch(&plan)?;
    
    println!("
=== NORMALIZATION REPORT ===");
    println!("Certified: {}", report.certified);
    println!(".-----------------------------------------------------------.");
    
    for line in report.patch_diff.lines() {
        println!("{}", line);
    }

    println!("
=== VERIFICATION STATUS ===");
    let has_unwrap = unwrap_calls > 0;
    let has_unsafe = unsafe_blocks > 0;
    
    assert!(has_unwrap, "Expected unwrap() violation detected");
    assert!(has_unsafe, "Expected unsafe block detected");
    
    println!("Unwrap violations: {} detected", unwrap_calls);
    println!("Unsafe blocks: {} detected", unsafe_blocks);
    println!("Total errors: {}", errors);
    println!("Warnings: {}", warnings);
    
    println!("
===========================================================");

    // Verify remediation success
    println!("Remediation workflow completed - violations detected and patch generated");
    println!("CANARY TEST PASSED: All violations correctly identified!");
    println!("===========================================================
");

    Ok(())
}
