// Batch Normalization Runner - Multi-crate legacy code remediation

use anyhow::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Report for batch normalization results
#[derive(Debug)]
pub struct BatchReport {
    pub total_crates_scanned: usize,
    pub total_files_inspected: usize,
    pub unwrap_violations: usize,
    pub panic_violations: usize,
    pub per_crate_status: HashMap<String, CrateStatus>,
}

impl BatchReport {
    pub fn new() -> Self {
        Self {
            total_crates_scanned: 0,
            total_files_inspected: 0,
            unwrap_violations: 0,
            panic_violations: 0,
            per_crate_status: HashMap::new(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_crates_scanned == 0 {
            return 0.0;
        }
        let compliant = self
            .per_crate_status
            .values()
            .filter(|s| s.compliant)
            .count();
        (compliant as f64 / self.total_crates_scanned as f64) * 100.0
    }
}

impl Default for BatchReport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct CrateStatus {
    pub name: String,
    pub files_inspected: usize,
    pub violations_found: usize,
    pub compliant: bool,
}

/// Batch Normalization Runner - orchestrates multi-crate inspection
#[derive(Default)]
pub struct BatchNormalizationRunner {}

impl BatchNormalizationRunner {
    /// Create new batch runner
    pub fn new() -> Result<Self, Error> {
        Ok(Self {})
    }

    /// Run batch inspection across all discovered crates
    pub fn run_batch(&self, target_dir: &Path) -> Result<BatchReport, Error> {
        let mut report = BatchReport::new();

        println!("===========================================================");
        println!("BATCH NORMALIZATION RUNNER");
        println!("===========================================================\n");

        println!("Target directory: {}", target_dir.display());
        println!(".-----------------------------------------------------------.");

        // Discover crates
        let crates = self.discover_crates(target_dir)?;

        if crates.is_empty() {
            eprintln!("No crates found in target directory");
            return Err(anyhow::anyhow!("No crates discovered"));
        }

        println!("\nDiscovered {} crates:", crates.len());
        for crate_path in &crates {
            println!("  - {}", crate_path.display());
        }

        // Process each crate
        for crate_path in &crates {
            let crate_name = crate_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            println!("\nProcessing crate: {}", crate_name);

            // Discover source files in this crate
            let src_dir = crate_path.join("src");
            if !src_dir.exists() {
                continue;
            }

            let mut files_inspected = 0;
            let mut violations_found = 0;

            for entry in std::fs::read_dir(&src_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().is_some_and(|e| e == "rs") {
                    files_inspected += 1;

                    // Read source file
                    let source = std::fs::read_to_string(&path)?;

                    // Count violations (inspection mode only)
                    if source.contains(".unwrap()") {
                        report.unwrap_violations += 1;
                        violations_found += 1;
                    }

                    if source.contains("panic!") {
                        report.panic_violations += 1;
                        violations_found += 1;
                    }

                    files_inspected += 1;
                }
            }

            report.total_files_inspected += files_inspected;
            report.per_crate_status.insert(
                crate_name.clone(),
                CrateStatus {
                    name: crate_name,
                    files_inspected,
                    violations_found,
                    compliant: violations_found == 0,
                },
            );

            println!("  Files inspected: {}", files_inspected);
            println!("  Violations found: {}", violations_found);
        }

        // Update totals
        report.total_crates_scanned = crates.len();

        println!("\n.-----------------------------------------------------------.");
        println!("\n=== BATCH SUMMARY ===");
        println!("Total crates scanned:   {}", report.total_crates_scanned);
        println!("Total files inspected:  {}", report.total_files_inspected);
        println!("Unwrap violations:      {}", report.unwrap_violations);
        println!("Panic violations:       {}", report.panic_violations);
        println!("Success rate:           {:.2}%", report.success_rate());

        Ok(report)
    }

    fn discover_crates(&self, target_dir: &Path) -> Result<Vec<PathBuf>, Error> {
        let mut crates = Vec::new();

        if !target_dir.exists() || !target_dir.is_dir() {
            return Err(anyhow::anyhow!("Target directory does not exist"));
        }

        // Look for immediate child directories with Cargo.toml
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let cargo_path = path.join("Cargo.toml");
                if cargo_path.exists() && cargo_path.is_file() {
                    crates.push(path);
                }
            } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
                // Also support single crate targets
                crates.push(path.parent().unwrap_or(Path::new("")).to_path_buf());
            }
        }

        // Sort alphabetically
        crates.sort();
        Ok(crates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_report_success_rate() {
        let mut report = BatchReport::new();
        report.total_crates_scanned = 10;

        // 8 compliant, 2 non-compliant
        for i in 0..8 {
            report.per_crate_status.insert(
                format!("crate_{}", i),
                CrateStatus {
                    name: format!("crate_{}", i),
                    files_inspected: 1,
                    violations_found: 0,
                    compliant: true,
                },
            );
        }

        assert_eq!(report.success_rate(), 80.0);
    }
}
