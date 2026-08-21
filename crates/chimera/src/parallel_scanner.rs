//! crates/chimera/src/parallel_scanner.rs
//! High-Throughput Parallel AST Scanner & Batch Code Analysis Engine
//! Powered by Rayon multi-core work-stealing parallelism.

use crate::ast_parser::AstParser;
use crate::disassembly::BinaryInspector;
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Multi-file Parallel Analysis Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScanReport {
    pub total_files_scanned: usize,
    pub total_functions_extracted: usize,
    pub average_entropy: f64,
    pub processed_files: Vec<String>,
}

/// Rayon-powered Parallel Code Scanner
pub struct ParallelScanner;

impl ParallelScanner {
    /// Ingests and parses multiple source files concurrently across all CPU cores
    pub fn scan_sources_parallel(files: &[(&str, &str)]) -> BatchScanReport {
        let results: Vec<(usize, f64, String)> = files
            .par_iter()
            .map(|(path, content)| {
                let func_count = AstParser::parse_source(path, content)
                    .map(|obs| obs.functions.len())
                    .unwrap_or(0);
                let entropy = BinaryInspector::calculate_entropy(content.as_bytes());
                (func_count, entropy, path.to_string())
            })
            .collect();

        let total_files = results.len();
        let total_functions: usize = results.iter().map(|(f, _, _)| *f).sum();
        let total_entropy: f64 = results.iter().map(|(_, e, _)| *e).sum();
        let average_entropy = if total_files > 0 {
            total_entropy / (total_files as f64)
        } else {
            0.0
        };
        let processed_files = results.into_iter().map(|(_, _, p)| p).collect();

        BatchScanReport {
            total_files_scanned: total_files,
            total_functions_extracted: total_functions,
            average_entropy,
            processed_files,
        }
    }

    /// Parallel scan of files on disk matching extensions
    pub fn scan_directory_parallel(dir_path: impl AsRef<Path>, extensions: &[&str]) -> Result<Vec<PathBuf>> {
        let mut matched_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if extensions.contains(&ext) {
                            matched_files.push(path);
                        }
                    }
                }
            }
        }
        Ok(matched_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_scanner_multi_file_processing() {
        let file1 = ("main.rs", "fn main() { println!(\"hello\"); }\nfn helper() {}");
        let file2 = ("lib.rs", "pub fn compute_sum(a: i32, b: i32) -> i32 { a + b }");
        let file3 = ("util.py", "def process_data():\n    pass\ndef log():\n    pass");

        let files = vec![file1, file2, file3];
        let report = ParallelScanner::scan_sources_parallel(&files);

        assert_eq!(report.total_files_scanned, 3);
        assert!(report.total_functions_extracted >= 3);
        assert!(report.average_entropy > 0.0);
        assert_eq!(report.processed_files.len(), 3);
    }
}
