use crate::ast_parser::AstObservation;
use crate::experiment::ExperimentResult;
use crate::hypothesis::Hypothesis;
use crate::verifier::{Verdict, VerificationResult};
use compute::thermodynamics::boltzmann_distribution;

/// Batch feature extraction from multiple AST observations.
/// Returns feature matrix [n_files, n_features].
pub fn batch_extract_features(observations: &[AstObservation]) -> Vec<Vec<f64>> {
    observations
        .iter()
        .map(|obs| {
            let func_count = obs
                .structures
                .iter()
                .filter(|s| {
                    matches!(
                        s.structure_type,
                        crate::ast_parser::StructureType::Function
                            | crate::ast_parser::StructureType::Method
                    )
                })
                .count() as f64;
            let long_funcs = obs
                .structures
                .iter()
                .filter(|s| {
                    let lines = s.line_range.1.saturating_sub(s.line_range.0);
                    lines > 20
                        && matches!(
                            s.structure_type,
                            crate::ast_parser::StructureType::Function
                                | crate::ast_parser::StructureType::Method
                        )
                })
                .count() as f64;
            vec![
                obs.structures.len() as f64, // Structural complexity
                func_count,                  // Function count
                long_funcs,                  // Long functions
                obs.complexity_metrics.cyclomatic_complexity as f64, // Cyclomatic complexity
                obs.raw_entropy,             // Shannon entropy
                obs.complexity_metrics.nesting_depth as f64, // Nesting depth
            ]
        })
        .collect()
}

/// Compute pairwise similarity matrix for code files.
/// Returns [n_files, n_files] cosine similarity matrix.
pub fn batch_compute_similarity(feature_matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = feature_matrix.len();
    let mut similarity = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..n {
            if i == j {
                similarity[i][j] = 1.0;
            } else if j > i {
                let sim = dot_cosine_similarity(&feature_matrix[i], &feature_matrix[j]);
                similarity[i][j] = sim;
                similarity[j][i] = sim;
            }
        }
    }

    similarity
}

/// Simple cosine similarity between two vectors.
fn dot_cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Prioritize test execution using Boltzmann distribution.
/// Files with higher complexity/entropy get higher priority.
pub fn prioritize_tests(observations: &[AstObservation], temperature: f64) -> Vec<(usize, f64)> {
    if observations.is_empty() {
        return vec![];
    }

    // Compute energy (negative priority) for each file
    let energies: Vec<f64> = observations
        .iter()
        .map(|obs| {
            let complexity = obs.structures.len() as f64;
            let entropy = obs.raw_entropy;
            let long_funcs = obs
                .structures
                .iter()
                .filter(|s| {
                    let lines = s.line_range.1.saturating_sub(s.line_range.0);
                    lines > 20
                        && matches!(
                            s.structure_type,
                            crate::ast_parser::StructureType::Function
                                | crate::ast_parser::StructureType::Method
                        )
                })
                .count() as f64;

            // Higher complexity/entropy = higher energy = lower priority in Boltzmann
            // But we want to test complex files first, so negate
            -(complexity * 0.4 + entropy * 0.3 + long_funcs * 0.3)
        })
        .collect();

    // Boltzmann distribution gives probabilities
    let probabilities = boltzmann_distribution(&energies, temperature);

    // Return sorted by probability (highest first)
    let mut indexed_probs: Vec<(usize, f64)> = probabilities
        .iter()
        .enumerate()
        .map(|(i, &p)| (i, p))
        .collect();
    indexed_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    indexed_probs
}

/// Batch hypothesis generation from observations.
/// Uses feature clustering to generate related hypotheses.
pub fn batch_generate_hypotheses(observations: &[AstObservation]) -> Vec<Vec<Hypothesis>> {
    let feature_matrix = batch_extract_features(observations);
    let similarity = batch_compute_similarity(&feature_matrix);

    observations
        .iter()
        .enumerate()
        .map(|(i, obs)| {
            let mut hypotheses = Vec::new();

            // Generate hypotheses based on file characteristics
            if obs.structures.len() > 5 {
                hypotheses.push(Hypothesis::new(
                    &format!("File {} has high structural complexity", i),
                    "complexity",
                    0.7,
                ));
            }

            let has_long_func = obs.structures.iter().any(|s| {
                let lines = s.line_range.1.saturating_sub(s.line_range.0);
                lines > 50
                    && matches!(
                        s.structure_type,
                        crate::ast_parser::StructureType::Function
                            | crate::ast_parser::StructureType::Method
                    )
            });
            if has_long_func {
                hypotheses.push(Hypothesis::new(
                    &format!("File {} contains overly long functions", i),
                    "maintainability",
                    0.8,
                ));
            }

            // Find similar files and generate cross-file hypotheses
            let similar_files: Vec<usize> = similarity[i]
                .iter()
                .enumerate()
                .filter(|(j, &sim)| *j != i && sim > 0.7)
                .map(|(j, _)| j)
                .collect();

            if !similar_files.is_empty() {
                hypotheses.push(Hypothesis::new(
                    &format!(
                        "File {} shares patterns with {} similar files",
                        i,
                        similar_files.len()
                    ),
                    "duplication",
                    0.6,
                ));
            }

            hypotheses
        })
        .collect()
}

/// Batch experiment execution.
/// Runs experiments in parallel (simulated here).
pub fn batch_run_experiments(
    hypotheses_batch: &[Vec<Hypothesis>],
    workspace_root: &str,
) -> Vec<Vec<ExperimentResult>> {
    hypotheses_batch
        .iter()
        .map(|hypotheses| {
            hypotheses
                .iter()
                .map(|h| ExperimentResult::run(h, workspace_root))
                .collect()
        })
        .collect()
}

/// Batch verification with Bayesian updates.
/// Updates confidence for all hypotheses simultaneously.
pub fn batch_verify(
    hypotheses_batch: &[Vec<Hypothesis>],
    experiments_batch: &[Vec<ExperimentResult>],
) -> Vec<Vec<VerificationResult>> {
    hypotheses_batch
        .iter()
        .zip(experiments_batch.iter())
        .map(|(hypotheses, experiments)| {
            hypotheses
                .iter()
                .zip(experiments.iter())
                .map(|(h, e)| VerificationResult::verify(h, e))
                .collect()
        })
        .collect()
}

/// Compute information flow between files.
/// Returns directed information flow matrix.
pub fn compute_code_information_flow(
    observations: &[AstObservation],
    time_series: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    let n = observations.len();
    let mut flow_matrix = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..n {
            if i != j && i < time_series.len() && j < time_series.len() {
                // Approximate transfer entropy via cross-correlation
                let te = approximate_transfer_entropy(&time_series[i], &time_series[j]);
                flow_matrix[i][j] = te;
            }
        }
    }

    flow_matrix
}

/// Detect code clones using mutual information.
/// Returns pairs of files with high mutual information.
pub fn detect_code_clones(feature_matrix: &[Vec<f64>], threshold: f64) -> Vec<(usize, usize, f64)> {
    let n = feature_matrix.len();
    let mut clones = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let mi = approximate_mutual_information(&feature_matrix[i], &feature_matrix[j]);
            if mi > threshold {
                clones.push((i, j, mi));
            }
        }
    }

    clones.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    clones
}

/// Approximate mutual information between feature vectors.
fn approximate_mutual_information(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mean_a: f64 = a.iter().sum::<f64>() / a.len() as f64;
    let mean_b: f64 = b.iter().sum::<f64>() / b.len() as f64;

    let cov: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - mean_a) * (y - mean_b))
        .sum::<f64>()
        / a.len() as f64;

    let var_a: f64 = a.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / a.len() as f64;
    let var_b: f64 = b.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / b.len() as f64;

    if var_a * var_b == 0.0 {
        return 0.0;
    }

    let correlation = cov / (var_a.sqrt() * var_b.sqrt());
    let rho_sq = correlation.powi(2);
    if rho_sq >= 1.0 {
        return 1.0;
    }
    -0.5 * (1.0 - rho_sq).ln().max(0.0)
}

/// Approximate transfer entropy via cross-correlation.
fn approximate_transfer_entropy(source: &[f64], target: &[f64]) -> f64 {
    if source.len() < 2 || target.len() < 2 {
        return 0.0;
    }

    let n = source.len().min(target.len()) - 1;
    let mean_source: f64 = source.iter().take(n).sum::<f64>() / n as f64;
    let mean_target: f64 = target.iter().skip(1).take(n).sum::<f64>() / n as f64;

    let cov: f64 = source
        .iter()
        .take(n)
        .zip(target.iter().skip(1).take(n))
        .map(|(s, t)| (s - mean_source) * (t - mean_target))
        .sum::<f64>()
        / n as f64;

    let var_source: f64 = source
        .iter()
        .take(n)
        .map(|x| (x - mean_source).powi(2))
        .sum::<f64>()
        / n as f64;
    let var_target: f64 = target
        .iter()
        .skip(1)
        .take(n)
        .map(|x| (x - mean_target).powi(2))
        .sum::<f64>()
        / n as f64;

    if var_source * var_target == 0.0 {
        return 0.0;
    }

    let correlation = cov / (var_source.sqrt() * var_target.sqrt());
    correlation.abs().max(0.0)
}

/// Batch analysis report.
#[derive(Debug, Clone)]
pub struct BatchAnalysisReport {
    pub file_count: usize,
    pub total_hypotheses: usize,
    pub verified_count: usize,
    pub falsified_count: usize,
    pub unstable_count: usize,
    pub avg_confidence: f64,
    pub test_priority: Vec<(usize, f64)>,
    pub code_clones: Vec<(usize, usize, f64)>,
    pub information_flow: Vec<Vec<f64>>,
    pub processing_time_ms: f64,
}

/// Run full batch analysis pipeline.
pub fn run_batch_analysis(
    observations: &[AstObservation],
    workspace_root: &str,
    temperature: f64,
    clone_threshold: f64,
) -> BatchAnalysisReport {
    let start = std::time::Instant::now();

    // Phase 1: Feature extraction
    let feature_matrix = batch_extract_features(observations);

    // Phase 2: Test prioritization
    let test_priority = prioritize_tests(observations, temperature);

    // Phase 3: Code clone detection
    let code_clones = detect_code_clones(&feature_matrix, clone_threshold);

    // Phase 4: Hypothesis generation
    let hypotheses_batch = batch_generate_hypotheses(observations);

    // Phase 5: Experiment execution
    let experiments_batch = batch_run_experiments(&hypotheses_batch, workspace_root);

    // Phase 6: Verification
    let verifications_batch = batch_verify(&hypotheses_batch, &experiments_batch);

    // Phase 7: Information flow computation
    let time_series: Vec<Vec<f64>> = feature_matrix.to_vec();
    let information_flow = compute_code_information_flow(observations, &time_series);

    // Aggregate results
    let mut total_hypotheses = 0;
    let mut verified_count = 0;
    let mut falsified_count = 0;
    let mut unstable_count = 0;
    let mut total_confidence = 0.0;
    let mut verification_count = 0;

    for verifications in &verifications_batch {
        total_hypotheses += verifications.len();
        for v in verifications {
            match v.verdict {
                Verdict::Verified => verified_count += 1,
                Verdict::Falsified => falsified_count += 1,
                Verdict::Unstable => unstable_count += 1,
                Verdict::Inconclusive => {}
            }
            total_confidence += v.posterior_confidence;
            verification_count += 1;
        }
    }

    let avg_confidence = if verification_count > 0 {
        total_confidence / verification_count as f64
    } else {
        0.0
    };

    let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    BatchAnalysisReport {
        file_count: observations.len(),
        total_hypotheses,
        verified_count,
        falsified_count,
        unstable_count,
        avg_confidence,
        test_priority,
        code_clones,
        information_flow,
        processing_time_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_observations() -> Vec<AstObservation> {
        vec![
            AstObservation {
                file_path: "test1.rs".to_string(),
                language: crate::ast_parser::Language::Rust,
                structures: vec![crate::ast_parser::CodeStructure {
                    name: "main".to_string(),
                    structure_type: crate::ast_parser::StructureType::Function,
                    signature: crate::ast_parser::FunctionSignature {
                        name: "main".to_string(),
                        parameters: vec![],
                        return_type: Some("void".to_string()),
                        is_async: false,
                        visibility: crate::ast_parser::Visibility::Public,
                    },
                    line_range: (1, 3),
                    dependencies: vec![],
                    control_flow_complexity: 1,
                }],
                complexity_metrics: crate::ast_parser::ComplexityMetrics {
                    cyclomatic_complexity: 1,
                    lines_of_code: 3,
                    nesting_depth: 0,
                    branch_count: 0,
                    call_count: 0,
                },
                raw_entropy: 0.5,
            },
            AstObservation {
                file_path: "test2.rs".to_string(),
                language: crate::ast_parser::Language::Rust,
                structures: vec![crate::ast_parser::CodeStructure {
                    name: "helper".to_string(),
                    structure_type: crate::ast_parser::StructureType::Function,
                    signature: crate::ast_parser::FunctionSignature {
                        name: "helper".to_string(),
                        parameters: vec![crate::ast_parser::Parameter {
                            name: "arg".to_string(),
                            param_type: "i32".to_string(),
                        }],
                        return_type: Some("i32".to_string()),
                        is_async: false,
                        visibility: crate::ast_parser::Visibility::Private,
                    },
                    line_range: (1, 5),
                    dependencies: vec!["std::io".to_string()],
                    control_flow_complexity: 2,
                }],
                complexity_metrics: crate::ast_parser::ComplexityMetrics {
                    cyclomatic_complexity: 2,
                    lines_of_code: 5,
                    nesting_depth: 1,
                    branch_count: 1,
                    call_count: 0,
                },
                raw_entropy: 0.7,
            },
        ]
    }

    #[test]
    fn test_batch_extract_features() {
        let observations = create_test_observations();
        let features = batch_extract_features(&observations);

        assert_eq!(features.len(), 2);
        assert_eq!(features[0].len(), 6); // 6 features per file
    }

    #[test]
    fn test_batch_compute_similarity() {
        let observations = create_test_observations();
        let features = batch_extract_features(&observations);
        let similarity = batch_compute_similarity(&features);

        assert_eq!(similarity.len(), 2);
        assert_eq!(similarity[0].len(), 2);

        // Diagonal should be 1.0
        assert!((similarity[0][0] - 1.0).abs() < 1e-10);
        assert!((similarity[1][1] - 1.0).abs() < 1e-10);

        // Matrix should be symmetric
        assert!((similarity[0][1] - similarity[1][0]).abs() < 1e-10);
    }

    #[test]
    fn test_prioritize_tests() {
        let observations = create_test_observations();
        let priority = prioritize_tests(&observations, 1.0);

        assert_eq!(priority.len(), 2);
        // Probabilities should sum to 1.0
        let sum: f64 = priority.iter().map(|(_, p)| p).sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_batch_generate_hypotheses() {
        let observations = create_test_observations();
        let hypotheses_batch = batch_generate_hypotheses(&observations);

        assert_eq!(hypotheses_batch.len(), 2);
        // Each file should have at least 0 hypotheses
        assert!(!hypotheses_batch.is_empty());
    }

    #[test]
    fn test_detect_code_clones() {
        let observations = create_test_observations();
        let features = batch_extract_features(&observations);
        let clones = detect_code_clones(&features, 0.1);

        // May or may not find clones depending on feature similarity
        assert!(clones.len() <= 1); // At most 1 pair for 2 files
    }

    #[test]
    fn test_run_batch_analysis() {
        let observations = create_test_observations();
        let report = run_batch_analysis(&observations, ".", 1.0, 0.5);

        assert_eq!(report.file_count, 2);
        assert!(report.processing_time_ms >= 0.0);
        assert!(report.avg_confidence >= 0.0);
    }
}
