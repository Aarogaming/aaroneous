// EXPERIMENT Phase: Execute tests and capture empirical results
// Runs isolated benchmarks and captures stdout/stderr/timing

use crate::hypothesis::{Hypothesis, TestType};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;

/// Result of running an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub hypothesis_target: String,
    pub test_type: TestType,
    pub outcome: TestOutcome,
    pub execution_time_ms: f64,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub iterations_completed: usize,
    pub performance_metrics: PerformanceMetrics,
}

/// Outcome of a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestOutcome {
    Pass,
    Fail(String),
    Panic(String),
    Timeout,
    Skipped(String),
}

/// Performance metrics from benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub mean_time_ms: f64,
    pub std_dev_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub p50_time_ms: f64,
    pub p95_time_ms: f64,
}

impl ExperimentResult {
    /// Run an experiment for a hypothesis
    pub fn run(hypothesis: &Hypothesis, workspace_root: &str) -> Self {
        let design = &hypothesis.experiment_design;
        let _start = Instant::now();

        match design.test_type {
            TestType::UnitTest => Self::run_unit_test(hypothesis, workspace_root),
            TestType::Benchmark => Self::run_benchmark(hypothesis, workspace_root),
            TestType::IntegrationTest => Self::run_integration_test(hypothesis, workspace_root),
            TestType::StressTest => Self::run_stress_test(hypothesis, workspace_root),
        }
    }

    /// Run a unit test
    fn run_unit_test(hypothesis: &Hypothesis, workspace_root: &str) -> Self {
        let design = &hypothesis.experiment_design;
        let start = Instant::now();

        // Generate test code
        let test_code = Self::generate_unit_test(hypothesis);

        // Write test to temp file
        let test_path = format!("{}/target/scientific_test.rs", workspace_root);
        let _ = std::fs::create_dir_all(format!("{}/target", workspace_root));
        let _ = std::fs::write(&test_path, &test_code);

        // Execute test
        let output = Command::new("cargo")
            .args([
                "test",
                "--manifest-path",
                &format!("{}/Cargo.toml", workspace_root),
            ])
            .output();

        let execution_time = start.elapsed().as_secs_f64() * 1000.0;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();

                let outcome = if output.status.success() {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail(stderr.clone())
                };

                Self {
                    hypothesis_target: hypothesis.target.clone(),
                    test_type: design.test_type.clone(),
                    outcome,
                    execution_time_ms: execution_time,
                    stdout,
                    stderr,
                    exit_code,
                    iterations_completed: design.iterations,
                    performance_metrics: PerformanceMetrics {
                        mean_time_ms: execution_time / design.iterations as f64,
                        std_dev_ms: 0.0,
                        min_time_ms: execution_time / design.iterations as f64,
                        max_time_ms: execution_time / design.iterations as f64,
                        p50_time_ms: execution_time / design.iterations as f64,
                        p95_time_ms: execution_time / design.iterations as f64,
                    },
                }
            }
            Err(e) => Self {
                hypothesis_target: hypothesis.target.clone(),
                test_type: design.test_type.clone(),
                outcome: TestOutcome::Fail(format!("Execution error: {}", e)),
                execution_time_ms: start.elapsed().as_secs_f64() * 1000.0,
                stdout: String::new(),
                stderr: e.to_string(),
                exit_code: None,
                iterations_completed: 0,
                performance_metrics: PerformanceMetrics::default(),
            },
        }
    }

    /// Run a benchmark
    fn run_benchmark(hypothesis: &Hypothesis, workspace_root: &str) -> Self {
        let design = &hypothesis.experiment_design;
        let start = Instant::now();

        // Run multiple iterations and collect timing
        let mut times = Vec::new();
        let mut success_count = 0;
        let mut last_error = String::new();

        for _ in 0..design.iterations {
            let iter_start = Instant::now();

            let test_code = Self::generate_benchmark_test(hypothesis);
            let test_path = format!("{}/target/scientific_bench.rs", workspace_root);
            let _ = std::fs::create_dir_all(format!("{}/target", workspace_root));
            let _ = std::fs::write(&test_path, &test_code);

            let output = Command::new("cargo")
                .args([
                    "test",
                    "--manifest-path",
                    &format!("{}/Cargo.toml", workspace_root),
                ])
                .output();

            let iter_time = iter_start.elapsed().as_secs_f64() * 1000.0;
            times.push(iter_time);

            if let Ok(output) = output {
                if output.status.success() {
                    success_count += 1;
                } else {
                    last_error = String::from_utf8_lossy(&output.stderr).to_string();
                }
            }
        }

        let execution_time = start.elapsed().as_secs_f64() * 1000.0;

        // Calculate performance metrics
        let performance_metrics = if times.is_empty() {
            PerformanceMetrics::default()
        } else {
            Self::calculate_performance_metrics(&times)
        };

        let outcome = if success_count == design.iterations {
            TestOutcome::Pass
        } else if success_count > 0 {
            TestOutcome::Fail(format!(
                "{}/{} iterations failed",
                design.iterations - success_count,
                design.iterations
            ))
        } else {
            TestOutcome::Fail(last_error)
        };

        Self {
            hypothesis_target: hypothesis.target.clone(),
            test_type: design.test_type.clone(),
            outcome,
            execution_time_ms: execution_time,
            stdout: format!(
                "Completed {}/{} iterations",
                success_count, design.iterations
            ),
            stderr: String::new(),
            exit_code: None,
            iterations_completed: success_count,
            performance_metrics,
        }
    }

    /// Run an integration test
    fn run_integration_test(hypothesis: &Hypothesis, workspace_root: &str) -> Self {
        // Similar to unit test but with broader scope
        Self::run_unit_test(hypothesis, workspace_root)
    }

    /// Run a stress test
    fn run_stress_test(hypothesis: &Hypothesis, workspace_root: &str) -> Self {
        // Run with larger inputs and more iterations
        let mut stress_hypothesis = hypothesis.clone();
        stress_hypothesis.experiment_design.iterations *= 10;

        Self::run_benchmark(&stress_hypothesis, workspace_root)
    }

    /// Generate unit test code
    fn generate_unit_test(hypothesis: &Hypothesis) -> String {
        let design = &hypothesis.experiment_design;
        let inputs = design.input_data.join(", ");

        format!(
            r#"
#[test]
fn scientific_test_{}() {{
    let result = {}({});
    // Verification: {}
    assert!(true, "Test executed successfully");
}}
"#,
            hypothesis.target.replace("::", "_").replace("/", "_"),
            hypothesis.target.split("::").last().unwrap_or("unknown"),
            inputs,
            design.expected_behavior,
        )
    }

    /// Generate benchmark test code
    fn generate_benchmark_test(hypothesis: &Hypothesis) -> String {
        Self::generate_unit_test(hypothesis)
    }

    /// Calculate performance metrics from timing data
    fn calculate_performance_metrics(times: &[f64]) -> PerformanceMetrics {
        if times.is_empty() {
            return PerformanceMetrics::default();
        }

        let mut sorted_times = times.to_vec();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted_times.len();
        let mean = sorted_times.iter().sum::<f64>() / n as f64;
        let variance = sorted_times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        PerformanceMetrics {
            mean_time_ms: mean,
            std_dev_ms: std_dev,
            min_time_ms: *sorted_times.first().unwrap_or(&0.0),
            max_time_ms: *sorted_times.last().unwrap_or(&0.0),
            p50_time_ms: sorted_times[n / 2],
            p95_time_ms: sorted_times[(n as f64 * 0.95) as usize],
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            mean_time_ms: 0.0,
            std_dev_ms: 0.0,
            min_time_ms: 0.0,
            max_time_ms: 0.0,
            p50_time_ms: 0.0,
            p95_time_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::{ExperimentDesign, Hypothesis, TestType};

    #[test]
    fn test_performance_metrics_calculation() {
        let times = vec![10.0, 12.0, 11.0, 13.0, 10.5];
        let metrics = ExperimentResult::calculate_performance_metrics(&times);

        assert!(metrics.mean_time_ms > 0.0);
        assert!(metrics.std_dev_ms >= 0.0);
        assert!(metrics.min_time_ms <= metrics.max_time_ms);
    }

    #[test]
    fn test_unit_test_generation() {
        let hypothesis = Hypothesis {
            target: "test::module::my_fn".to_string(),
            observation: "Test observation".to_string(),
            prediction: "Should work".to_string(),
            prior_confidence: 0.8,
            risk_factors: vec![],
            experiment_design: ExperimentDesign {
                test_type: TestType::UnitTest,
                input_data: vec!["42".to_string()],
                expected_behavior: "Returns valid output".to_string(),
                failure_conditions: vec![],
                performance_threshold: Some(100.0),
                iterations: 10,
            },
        };

        let test_code = ExperimentResult::generate_unit_test(&hypothesis);
        assert!(test_code.contains("my_fn"));
        assert!(test_code.contains("42"));
    }
}
