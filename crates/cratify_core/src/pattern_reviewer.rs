//! crates/ast_auditor/src/pattern_reviewer.rs
//! Continuous Conformance & Architectural Pattern Synthesis Reviewer.
//!
//! Evaluates workspace source files against declarative pattern specifications
//! codified in `registry/patterns/`, identifying positive adoptions and
//! opportunities for predecessor pattern synthesis.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Declarative specification for an architectural pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub origin_predecessor: String,
    pub description: String,
    pub target_crates: Vec<String>,
    pub detection: PatternDetection,
    pub recommendations: Vec<String>,
}

/// Detection rules for a pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDetection {
    pub recommended_markers: Vec<String>,
    pub anti_patterns: Vec<String>,
}

/// Category of an observation during pattern review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObservationType {
    /// The target source adopts the recommended architectural pattern.
    PositiveAdoption,
    /// An anti-pattern or legacy pattern was detected that can be modernized.
    AntiPatternFound,
    /// An opportunity to synthesize an architectural pattern was identified.
    Recommendation,
}

/// A specific finding recorded during pattern review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternObservation {
    pub pattern_id: String,
    pub pattern_name: String,
    pub file_path: String,
    pub line: usize,
    pub observation_type: ObservationType,
    pub message: String,
}

/// Aggregated report from a pattern conformance review.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatternReviewReport {
    pub patterns_evaluated: usize,
    pub files_scanned: usize,
    pub positive_adoptions: usize,
    pub opportunities_identified: usize,
    pub observations: Vec<PatternObservation>,
}

impl core::fmt::Display for PatternReviewReport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "\n=== ARCHITECTURAL PATTERN CONFORMANCE REPORT ===")?;
        writeln!(
            f,
            "Patterns Evaluated: {} | Files Scanned: {}",
            self.patterns_evaluated, self.files_scanned
        )?;
        writeln!(
            f,
            "Positive Pattern Adoptions: {} | Opportunities Identified: {}",
            self.positive_adoptions, self.opportunities_identified
        )?;
        writeln!(f, "--------------------------------------------------")?;

        for obs in &self.observations {
            let prefix = match obs.observation_type {
                ObservationType::PositiveAdoption => "[ADOPTION]",
                ObservationType::AntiPatternFound => "[OPPORTUNITY/RISK]",
                ObservationType::Recommendation => "[RECOMMENDATION]",
            };
            writeln!(
                f,
                "{} [{}] {}:{} - {}",
                prefix, obs.pattern_name, obs.file_path, obs.line, obs.message
            )?;
        }
        Ok(())
    }
}

impl PatternReviewReport {
    /// Formats diagnostic summary for display in console or logs.
    pub fn print_summary(&self) {
        print!("{self}");
    }
}

/// Loads pattern definitions from a directory containing pattern JSON specifications.
pub fn load_patterns_from_dir(dir: &Path) -> Result<Vec<PatternDefinition>, String> {
    let mut patterns = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(patterns);
    }

    let entries =
        fs::read_dir(dir).map_err(|e| format!("Failed to read pattern directory: {e}"))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            match serde_json::from_str::<PatternDefinition>(&content) {
                Ok(pattern) => patterns.push(pattern),
                Err(e) => {
                    eprintln!(
                        "[WARN] Failed to parse pattern definition {}: {e}",
                        path.display()
                    );
                }
            }
        }
    }

    patterns.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(patterns)
}

/// Reviews target files or directories against declarative architectural patterns.
pub fn review_patterns<P: AsRef<Path>>(
    target_paths: &[P],
    patterns: &[PatternDefinition],
) -> Result<PatternReviewReport, String> {
    let mut report = PatternReviewReport {
        patterns_evaluated: patterns.len(),
        ..Default::default()
    };

    for target in target_paths {
        let p = target.as_ref();
        if p.is_file() {
            if p.extension().is_some_and(|ext| ext == "rs") {
                review_source_file(p, patterns, &mut report)?;
            }
        } else if p.is_dir() {
            for entry in WalkDir::new(p).into_iter().filter_map(Result::ok) {
                let entry_path = entry.path();
                if entry_path.extension().is_some_and(|ext| ext == "rs") {
                    review_source_file(entry_path, patterns, &mut report)?;
                }
            }
        }
    }

    Ok(report)
}

/// Inspects an individual Rust source file against patterns.
fn review_source_file(
    file_path: &Path,
    patterns: &[PatternDefinition],
    report: &mut PatternReviewReport,
) -> Result<(), String> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Skip unreadable or binary files gracefully
    };

    report.files_scanned += 1;
    let path_str = file_path.to_string_lossy().to_string();

    let lines: Vec<&str> = content.lines().collect();

    for pattern in patterns {
        // Determine relevance by checking target_crates if specified
        let is_target_crate = pattern.target_crates.is_empty()
            || pattern
                .target_crates
                .iter()
                .any(|c| path_str.contains(c.as_str()));

        if !is_target_crate {
            continue;
        }

        // Check for positive pattern adoption markers
        let mut marker_found = false;
        for marker in &pattern.detection.recommended_markers {
            for (idx, line) in lines.iter().enumerate() {
                if line.contains(marker) {
                    report.positive_adoptions += 1;
                    report.observations.push(PatternObservation {
                        pattern_id: pattern.id.clone(),
                        pattern_name: pattern.name.clone(),
                        file_path: path_str.clone(),
                        line: idx + 1,
                        observation_type: ObservationType::PositiveAdoption,
                        message: format!(
                            "Adopts pattern '{}' from {} (matched '{}')",
                            pattern.name, pattern.origin_predecessor, marker
                        ),
                    });
                    marker_found = true;
                    break;
                }
            }
            if marker_found {
                break;
            }
        }

        // Check for anti-pattern markers
        for anti in &pattern.detection.anti_patterns {
            for (idx, line) in lines.iter().enumerate() {
                if line.contains(anti) {
                    report.opportunities_identified += 1;
                    let recommendation =
                        pattern.recommendations.first().cloned().unwrap_or_else(|| {
                            "Consider refactoring to recommended pattern.".to_string()
                        });

                    report.observations.push(PatternObservation {
                        pattern_id: pattern.id.clone(),
                        pattern_name: pattern.name.clone(),
                        file_path: path_str.clone(),
                        line: idx + 1,
                        observation_type: ObservationType::AntiPatternFound,
                        message: format!(
                            "Anti-pattern detected: '{}'. Recommendation: {}",
                            anti, recommendation
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Executes pattern review against specified paths using patterns found in `registry/patterns`.
pub fn run_pattern_review<P: AsRef<Path>>(
    target_paths: &[P],
    registry_path: Option<PathBuf>,
) -> Result<PatternReviewReport, String> {
    let patterns_dir = registry_path.unwrap_or_else(|| PathBuf::from("registry/patterns"));
    let patterns = load_patterns_from_dir(&patterns_dir)?;
    review_patterns(target_paths, &patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_pattern_loading_and_review() {
        let dir = tempdir().expect("tempdir");
        let patterns_dir = dir.path().join("patterns");
        fs::create_dir_all(&patterns_dir).expect("create patterns dir");

        let pattern_file = patterns_dir.join("test_pattern.json");
        let pattern_json = r#"{
            "id": "test_typestate",
            "name": "Test Typestate Pattern",
            "category": "state_safety",
            "origin_predecessor": "Embedded-HAL",
            "description": "Typestate testing pattern",
            "target_crates": ["test_crate"],
            "detection": {
                "recommended_markers": ["PhantomData"],
                "anti_patterns": ["is_quarantined: bool"]
            },
            "recommendations": ["Use PhantomData markers."]
        }"#;
        fs::write(&pattern_file, pattern_json).expect("write pattern");

        let patterns = load_patterns_from_dir(&patterns_dir).expect("load patterns");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].id, "test_typestate");

        // Create sample source file with positive adoption and anti-pattern
        let src_dir = dir.path().join("test_crate").join("src");
        fs::create_dir_all(&src_dir).expect("create src dir");
        let src_file = src_dir.join("lib.rs");
        let mut f = fs::File::create(&src_file).expect("create file");
        writeln!(f, "use core::marker::PhantomData;").unwrap();
        writeln!(f, "pub struct State<S> {{ _m: PhantomData<S> }}").unwrap();
        writeln!(f, "pub struct Legacy {{ pub is_quarantined: bool }}").unwrap();

        let report = review_patterns(&[src_dir], &patterns).expect("review patterns");
        assert_eq!(report.files_scanned, 1);
        assert_eq!(report.positive_adoptions, 1);
        assert_eq!(report.opportunities_identified, 1);
        assert_eq!(report.observations.len(), 2);
    }
}
