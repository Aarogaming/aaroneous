// VERIFY Phase: Falsification and confidence updates
// Uses compute::bayesian to update posterior confidence based on test results

use serde::{Serialize, Deserialize};
use compute::bayesian;
use crate::hypothesis::Hypothesis;
use crate::experiment::{ExperimentResult, TestOutcome};

/// Result of verification phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub hypothesis_target: String,
    pub prior_confidence: f64,
    pub posterior_confidence: f64,
    pub verdict: Verdict,
    pub evidence_summary: String,
    pub constellation_update: ConstellationUpdate,
}

/// Final verdict on a hypothesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verdict {
    Verified,      // Hypothesis confirmed
    Falsified,     // Hypothesis disproven
    Inconclusive,  // Insufficient evidence
    Unstable,      // Contradictory results
}

/// Update to send to constellation visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationUpdate {
    pub node_id: String,
    pub mass: f64,           // Based on execution complexity
    pub color: NodeColor,    // Based on verification result
    pub status: NodeStatus,  // Verified/Falsified/Unstable
    pub confidence: f64,     // Posterior confidence
}

/// Node color in constellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeColor {
    Green,   // Verified
    Red,     // Falsified
    Yellow,  // Inconclusive
    Purple,  // Unstable
    Blue,    // Untested
}

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Verified,
    Falsified,
    Inconclusive,
    Unstable,
    Pending,
}

/// Confidence update from Bayesian inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceUpdate {
    pub prior: f64,
    pub likelihood: f64,
    pub posterior: f64,
    pub evidence_count: usize,
}

impl VerificationResult {
    /// Verify a hypothesis based on experiment results
    pub fn verify(hypothesis: &Hypothesis, result: &ExperimentResult) -> Self {
        // Calculate likelihood based on experiment outcome
        let likelihood = Self::calculate_likelihood(result);
        
        // Bayesian update
        let prior = hypothesis.prior_confidence;
        let posterior = Self::bayesian_update(prior, likelihood);
        
        // Determine verdict
        let verdict = Self::determine_verdict(result, posterior);
        
        // Generate evidence summary
        let evidence_summary = Self::generate_evidence_summary(result, posterior);
        
        // Create constellation update
        let constellation_update = Self::create_constellation_update(
            &hypothesis.target,
            result,
            posterior,
            &verdict,
        );
        
        Self {
            hypothesis_target: hypothesis.target.clone(),
            prior_confidence: prior,
            posterior_confidence: posterior,
            verdict,
            evidence_summary,
            constellation_update,
        }
    }

    /// Calculate likelihood of hypothesis given experiment result
    fn calculate_likelihood(result: &ExperimentResult) -> f64 {
        match &result.outcome {
            TestOutcome::Pass => {
                // Strong evidence for hypothesis
                let base_likelihood = 0.9;
                // Adjust based on performance
                let perf_factor = if result.performance_metrics.mean_time_ms < 100.0 { 1.0 }
                    else if result.performance_metrics.mean_time_ms < 500.0 { 0.8 }
                    else { 0.6 };
                base_likelihood * perf_factor
            }
            TestOutcome::Fail(reason) => {
                // Evidence against hypothesis
                if reason.contains("assertion") {
                    0.1 // Strong falsification
                } else {
                    0.3 // Moderate falsification
                }
            }
            TestOutcome::Panic(_) => {
                0.05 // Very strong falsification
            }
            TestOutcome::Timeout => {
                0.2 // Moderate falsification (performance issue)
            }
            TestOutcome::Skipped(_) => {
                0.5 // No evidence either way
            }
        }
    }

    /// Bayesian update: posterior = (prior * likelihood) / normalizer
    fn bayesian_update(prior: f64, likelihood: f64) -> f64 {
        // Use compute::bayesian for proper Bayesian inference
        let prior_odds = prior / (1.0 - prior);
        let posterior_odds = prior_odds * (likelihood / (1.0 - likelihood));
        let posterior = posterior_odds / (1.0 + posterior_odds);
        
        posterior.clamp(0.01, 0.99)
    }

    /// Determine final verdict based on result and posterior
    fn determine_verdict(result: &ExperimentResult, posterior: f64) -> Verdict {
        match &result.outcome {
            TestOutcome::Pass => {
                if posterior > 0.8 {
                    Verdict::Verified
                } else {
                    Verdict::Inconclusive
                }
            }
            TestOutcome::Fail(_) | TestOutcome::Panic(_) => {
                if posterior < 0.3 {
                    Verdict::Falsified
                } else {
                    Verdict::Unstable
                }
            }
            TestOutcome::Timeout => Verdict::Unstable,
            TestOutcome::Skipped(_) => Verdict::Inconclusive,
        }
    }

    /// Generate human-readable evidence summary
    fn generate_evidence_summary(result: &ExperimentResult, posterior: f64) -> String {
        let outcome_str = match &result.outcome {
            TestOutcome::Pass => "PASSED",
            TestOutcome::Fail(r) => &format!("FAILED: {}", r),
            TestOutcome::Panic(r) => &format!("PANIC: {}", r),
            TestOutcome::Timeout => "TIMEOUT",
            TestOutcome::Skipped(r) => &format!("SKIPPED: {}", r),
        };
        
        format!(
            "Test {}: posterior={:.3}, execution_time={:.1}ms",
            outcome_str,
            posterior,
            result.execution_time_ms,
        )
    }

    /// Create constellation update from verification result
    fn create_constellation_update(
        target: &str,
        result: &ExperimentResult,
        posterior: f64,
        verdict: &Verdict,
    ) -> ConstellationUpdate {
        // Mass based on execution complexity (runtime)
        let mass = result.performance_metrics.mean_time_ms.max(1.0);
        
        // Color based on verdict
        let color = match verdict {
            Verdict::Verified => NodeColor::Green,
            Verdict::Falsified => NodeColor::Red,
            Verdict::Inconclusive => NodeColor::Yellow,
            Verdict::Unstable => NodeColor::Purple,
        };
        
        // Status
        let status = match verdict {
            Verdict::Verified => NodeStatus::Verified,
            Verdict::Falsified => NodeStatus::Falsified,
            Verdict::Inconclusive => NodeStatus::Inconclusive,
            Verdict::Unstable => NodeStatus::Unstable,
        };
        
        ConstellationUpdate {
            node_id: target.replace("::", "_").replace("/", "_"),
            mass,
            color,
            status,
            confidence: posterior,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::{Hypothesis, ExperimentDesign, TestType};
    use crate::experiment::{ExperimentResult, PerformanceMetrics};

    #[test]
    fn test_verification_pass() {
        let hypothesis = Hypothesis {
            target: "test::fn1".to_string(),
            observation: "Test".to_string(),
            prediction: "Should pass".to_string(),
            prior_confidence: 0.7,
            risk_factors: vec![],
            experiment_design: ExperimentDesign {
                test_type: TestType::UnitTest,
                input_data: vec![],
                expected_behavior: "Pass".to_string(),
                failure_conditions: vec![],
                performance_threshold: Some(100.0),
                iterations: 10,
            },
        };
        
        let result = ExperimentResult {
            hypothesis_target: "test::fn1".to_string(),
            test_type: TestType::UnitTest,
            outcome: TestOutcome::Pass,
            execution_time_ms: 50.0,
            stdout: "".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            iterations_completed: 10,
            performance_metrics: PerformanceMetrics {
                mean_time_ms: 5.0,
                std_dev_ms: 1.0,
                min_time_ms: 3.0,
                max_time_ms: 8.0,
                p50_time_ms: 5.0,
                p95_time_ms: 7.0,
            },
        };
        
        let verification = VerificationResult::verify(&hypothesis, &result);
        assert!(matches!(verification.verdict, Verdict::Verified));
        assert!(verification.posterior_confidence > hypothesis.prior_confidence);
    }

    #[test]
    fn test_verification_fail() {
        let hypothesis = Hypothesis {
            target: "test::fn2".to_string(),
            observation: "Test".to_string(),
            prediction: "Should pass".to_string(),
            prior_confidence: 0.8,
            risk_factors: vec![],
            experiment_design: ExperimentDesign {
                test_type: TestType::UnitTest,
                input_data: vec![],
                expected_behavior: "Pass".to_string(),
                failure_conditions: vec![],
                performance_threshold: Some(100.0),
                iterations: 10,
            },
        };
        
        let result = ExperimentResult {
            hypothesis_target: "test::fn2".to_string(),
            test_type: TestType::UnitTest,
            outcome: TestOutcome::Fail("Assertion failed".to_string()),
            execution_time_ms: 30.0,
            stdout: "".to_string(),
            stderr: "Assertion failed".to_string(),
            exit_code: Some(1),
            iterations_completed: 0,
            performance_metrics: PerformanceMetrics::default(),
        };
        
        let verification = VerificationResult::verify(&hypothesis, &result);
        assert!(matches!(verification.verdict, Verdict::Falsified | Verdict::Unstable));
        assert!(verification.posterior_confidence < hypothesis.prior_confidence);
    }

    #[test]
    fn test_bayesian_update() {
        let posterior = VerificationResult::bayesian_update(0.5, 0.9);
        assert!(posterior > 0.5);
        
        let posterior = VerificationResult::bayesian_update(0.5, 0.1);
        assert!(posterior < 0.5);
    }
}
