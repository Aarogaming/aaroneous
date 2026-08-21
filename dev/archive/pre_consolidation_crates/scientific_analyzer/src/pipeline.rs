// Scientific Analysis Pipeline
// Orchestrates: OBSERVE → HYPOTHESIS → EXPERIMENT → VERIFY → CONSTELLATION

use crate::ast_parser::AstObservation;
use crate::experiment::ExperimentResult;
use crate::hypothesis::Hypothesis;
use crate::verifier::{ConstellationUpdate, VerificationResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Complete analysis report for a code file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub file_path: String,
    pub observation: AstObservation,
    pub hypotheses: Vec<Hypothesis>,
    pub experiments: Vec<ExperimentResult>,
    pub verifications: Vec<VerificationResult>,
    pub constellation_updates: Vec<ConstellationUpdate>,
    pub summary: PipelineSummary,
}

/// Summary of pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub total_functions_analyzed: usize,
    pub verified_count: usize,
    pub falsified_count: usize,
    pub unstable_count: usize,
    pub inconclusive_count: usize,
    pub avg_posterior_confidence: f64,
    pub total_execution_time_ms: f64,
}

/// Scientific analysis pipeline
pub struct ScientificPipeline {
    pub workspace_root: String,
}

impl ScientificPipeline {
    pub fn new(workspace_root: &str) -> Self {
        Self {
            workspace_root: workspace_root.to_string(),
        }
    }

    /// Run the full scientific analysis pipeline on a file
    pub async fn analyze_file(&self, file_path: &Path) -> anyhow::Result<AnalysisReport> {
        let start = std::time::Instant::now();

        // Phase 1: OBSERVE
        let observation = self.observe(file_path)?;

        // Phase 2: HYPOTHESIS
        let hypotheses = self.generate_hypotheses(&observation);

        // Phase 3: EXPERIMENT
        let mut experiments = Vec::new();
        for hypothesis in &hypotheses {
            let result = ExperimentResult::run(hypothesis, &self.workspace_root);
            experiments.push(result);
        }

        // Phase 4: VERIFY
        let mut verifications = Vec::new();
        for (hypothesis, experiment) in hypotheses.iter().zip(experiments.iter()) {
            let verification = VerificationResult::verify(hypothesis, experiment);
            verifications.push(verification);
        }

        // Phase 5: CONSTELLATION UPDATE
        let constellation_updates: Vec<ConstellationUpdate> = verifications
            .iter()
            .map(|v| v.constellation_update.clone())
            .collect();

        // Generate summary
        let summary = self.generate_summary(
            &observation,
            &verifications,
            start.elapsed().as_secs_f64() * 1000.0,
        );

        Ok(AnalysisReport {
            file_path: file_path.to_string_lossy().to_string(),
            observation,
            hypotheses,
            experiments,
            verifications,
            constellation_updates,
            summary,
        })
    }

    /// Phase 1: OBSERVE - Parse AST and extract structure
    fn observe(&self, file_path: &Path) -> anyhow::Result<AstObservation> {
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "rs" => AstObservation::parse_rust_file(file_path),
            "py" => AstObservation::parse_python_file(file_path),
            _ => anyhow::bail!("Unsupported file type: {}", extension),
        }
    }

    /// Phase 2: HYPOTHESIS - Generate testable hypotheses
    fn generate_hypotheses(&self, observation: &AstObservation) -> Vec<Hypothesis> {
        Hypothesis::from_observation(observation)
    }

    /// Generate pipeline summary
    fn generate_summary(
        &self,
        observation: &AstObservation,
        verifications: &[VerificationResult],
        total_time_ms: f64,
    ) -> PipelineSummary {
        let verified = verifications
            .iter()
            .filter(|v| matches!(v.verdict, crate::verifier::Verdict::Verified))
            .count();
        let falsified = verifications
            .iter()
            .filter(|v| matches!(v.verdict, crate::verifier::Verdict::Falsified))
            .count();
        let unstable = verifications
            .iter()
            .filter(|v| matches!(v.verdict, crate::verifier::Verdict::Unstable))
            .count();
        let inconclusive = verifications
            .iter()
            .filter(|v| matches!(v.verdict, crate::verifier::Verdict::Inconclusive))
            .count();

        let avg_confidence = if verifications.is_empty() {
            0.0
        } else {
            verifications
                .iter()
                .map(|v| v.posterior_confidence)
                .sum::<f64>()
                / verifications.len() as f64
        };

        PipelineSummary {
            total_functions_analyzed: observation.structures.len(),
            verified_count: verified,
            falsified_count: falsified,
            unstable_count: unstable,
            inconclusive_count: inconclusive,
            avg_posterior_confidence: avg_confidence,
            total_execution_time_ms: total_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        let mut file = std::fs::File::create(&file_path).unwrap();
        use std::io::Write;
        writeln!(file, "pub fn test_fn(x: i32) -> i32 {{").unwrap();
        writeln!(file, "    x + 1").unwrap();
        writeln!(file, "}}").unwrap();

        let pipeline = ScientificPipeline::new(".");
        let report = pipeline.analyze_file(&file_path).await.unwrap();

        assert_eq!(report.observation.structures.len(), 1);
        assert!(!report.hypotheses.is_empty());
        assert!(report.summary.total_functions_analyzed > 0);
    }

    #[tokio::test]
    async fn test_pipeline_python_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.py");
        let mut file = std::fs::File::create(&file_path).unwrap();
        use std::io::Write;
        writeln!(file, "def test_fn(x: int) -> int:").unwrap();
        writeln!(file, "    return x + 1").unwrap();

        let pipeline = ScientificPipeline::new(".");
        let report = pipeline.analyze_file(&file_path).await.unwrap();

        assert_eq!(report.observation.structures.len(), 1);
        assert!(!report.hypotheses.is_empty());
    }
}
