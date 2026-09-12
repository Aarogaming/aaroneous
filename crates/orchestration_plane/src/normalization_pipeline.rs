// Normalization Pipeline - Literal Systems Engineering Implementation
// Scans codebase for violations of ACC standards

use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantSeverity {
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for InvariantSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InvariantSeverity::Warning => write!(f, "Warning"),
            InvariantSeverity::Error => write!(f, "Error"),
            InvariantSeverity::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantViolation {
    pub file: PathBuf,
    pub line: usize,
    pub description: String,
    pub severity: InvariantSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionPlan {
    pub target_path: PathBuf,
    pub scanned_files: Vec<PathBuf>,
    pub violations: Vec<InvariantViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationReport {
    pub plan: IngestionPlan,
    pub patch_diff: String,
    pub certified: bool,
}

pub struct NormalizationPipeline {
    workspace_root: PathBuf,
}

impl Default for NormalizationPipeline {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl NormalizationPipeline {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Inspect target path and generate ingestion plan
    pub fn inspect_target(&self, path: &Path) -> Result<IngestionPlan, Box<dyn std::error::Error>> {
        let mut scanned_files = Vec::new();
        let mut violations = Vec::new();

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "rs" {
                        scanned_files.push(entry.path().to_path_buf());
                        
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            violations.extend(self.analyze_source(&content, entry.path()));
                        }
                    }
                }
            }
        }

        Ok(IngestionPlan { target_path: path.to_path_buf(), scanned_files, violations })
    }

    fn analyze_source(&self, source: &str, source_path: &Path) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        // Check for unwrap/expect calls
        if source.contains(".unwrap()") || source.contains(".expect()") {
            violations.push(InvariantViolation {
                file: source_path.to_path_buf(),
                line: 0,
                description: "Function contains .unwrap() or .expect() - use Result propagation".to_string(),
                severity: InvariantSeverity::Warning,
            });
        }

        // Check for panic! macro
        if source.contains("panic!") {
            violations.push(InvariantViolation {
                file: source_path.to_path_buf(),
                line: 0,
                description: "Contains panic! macro - use proper error handling".to_string(),
                severity: InvariantSeverity::Error,
            });
        }

        // Check for bare unsafe blocks
        if source.contains("unsafe ") {
            violations.push(InvariantViolation {
                file: source_path.to_path_buf(),
                line: 0,
                description: "Contains unsafe block - ensure safety documentation".to_string(),
                severity: InvariantSeverity::Error,
            });
        }

        violations
    }

    /// Apply AST-based remediation to violating files
    pub fn apply_remediation(&self, plan: &IngestionPlan) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        use mutation_engine::remediate_source;

        let mut remediated_files = Vec::new();

        for file_path in &plan.scanned_files {
            // Read source file
            let source = std::fs::read_to_string(file_path)?;

            // Apply mutation engine remediation
            let remediated = remediate_source(&source)
                .map_err(|e| format!("Failed to remediate {}: {}", file_path.display(), e))?;

            // Write back the remediated code (prettyplease not needed for simple replacements)
            std::fs::write(file_path, remediated)?;
            remediated_files.push(file_path.clone());
        }

        Ok(remediated_files)
    }

    /// Generate normalization patch for the plan
    pub fn generate_normalization_patch(&self, plan: &IngestionPlan) -> Result<NormalizationReport, Box<dyn std::error::Error>> {
        let mut diff_lines = Vec::new();
        
        for violation in &plan.violations {
            match violation.severity {
                InvariantSeverity::Warning => {
                    diff_lines.push(format!("[WARNING] {}: {}", violation.file.display(), violation.description));
                    diff_lines.push("- Consider using Result<T, E> instead".to_string());
                }
                InvariantSeverity::Error => {
                    diff_lines.push(format!("[ERROR] {}: {}", violation.file.display(), violation.description));
                    diff_lines.push("- Must be refactored to use safe error handling".to_string());
                }
                InvariantSeverity::Critical => {
                    diff_lines.push(format!("[CRITICAL] {}", violation.file.display()));
                    diff_lines.push("Immediate action required - system cannot be certified".to_string());
                }
            }
            diff_lines.push("".to_string());
        }

        let has_errors = plan.violations.iter().any(|v| v.severity == InvariantSeverity::Error || v.severity == InvariantSeverity::Critical);
        let certified = !has_errors;

        Ok(NormalizationReport { 
            plan: plan.clone(), 
            patch_diff: diff_lines.join("
"), 
            certified 
        })
    }
}

#[cfg(test)]
#[allow(ambient_authority)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_target_flags_unwrap() {
        let temp_dir = std::env::temp_dir().join("orchestration_plane_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("test_unwrap.rs");
        std::fs::write(
            &test_file,
            r#"fn test() {
    let x: Option<i32> = None;
    x.unwrap();
}"#,
        ).unwrap();

        let pipeline = NormalizationPipeline::default();
        let plan = pipeline.inspect_target(&temp_dir).unwrap();

        assert!(!plan.violations.is_empty(), "Expected at least one violation");
        let unwrap_violation = plan.violations.iter().find(|v| v.description.contains("unwrap")).unwrap();
        assert_eq!(unwrap_violation.severity, InvariantSeverity::Warning);

        let _ = std::fs::remove_dir_all(&temp_dir);
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
}
