// Batch Cratification CLI - Automated multi-crate normalization and certification
#![allow(ambient_authority)]

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use orchestration_plane::batch_runner::{BatchNormalizationRunner, BatchReport};

fn main() -> Result<()> {
    println!("===========================================================");
    println!("BATCH CRATIFICATION: Multi-Crate Legacy Normalization");
    println!("===========================================================\n");

    // Default to legacy fixture directory for batch testing
    let target_dir: PathBuf = env::args().nth(1).unwrap_or_else(|| "dev/canary_legacy_fixture/".to_string()).into();

    println!("Target: {}", target_dir.display());
    println!(".-----------------------------------------------------------.");

    let runner = BatchNormalizationRunner::new()?;
    
    let report = runner.run_batch(&target_dir)?;
    
    generate_markdown_report(&report);
    
    println!("\n=== NEXT STEPS ===");
    println!("Run workspace verification:");
    println!("  $ cargo check --workspace");
    println!("  $ cargo test --workspace");
    
    println!("\nRun governance audit:");
    println!("  $ cargo run -p cratify -- audit crates/");

    Ok(())
}

fn generate_markdown_report(report: &BatchReport) {
    println!("\n=== BATCH NORMALIZATION REPORT ===");
    println!("# Batch Normalization Results\n");

    println!("## Summary Statistics\n");
    println!("| Metric | Value |");
    println!("--------|-------");
    println!("**Total Crates Scanned** | {}", report.total_crates_scanned);
    println!("**Total Files Inspected** | {}", report.total_files_inspected);
    println!("**Unwrap Violations** | {}", report.unwrap_violations);
    println!("**Panic Violations** | {}", report.panic_violations);
    println!("**Success Rate** | {:.2}%", report.success_rate());

    println!("\n## Per-Crate Status\n");
    println!("| Crate | Files | Violations Fixed | Compliant |");
    println!("-------|-------|------------------|-----------");

    for (name, status) in &report.per_crate_status {
        let compliant_icon = if status.compliant { "(OK)" } else { "(NEEDS FIX)" };
        println!("| {} | {} | {} | {}", name, status.files_inspected, status.violations_found, compliant_icon);
    }

    println!("\n## Recommendations\n");
    if report.success_rate() < 100.0 {
        println!("Some crates still have violations.");
        println!("Consider reviewing non-compliant crates manually.");
    } else {
        println!("All crates are Cratify-certified!");
        println!("Ready for production deployment");
    }

    println!("\n--- End Report ---");
}
