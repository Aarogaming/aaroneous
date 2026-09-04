//! crates/compute/src/bayesian.rs
//! Bayesian Inference and Belief Updating primitives.
//! Used for trust scoring, intent classification, sensor fusion, and adaptive state regulation.

use anyhow::{bail, Result};

/// Beta distribution conjugate prior for Bernoulli trials.
#[derive(Debug, Clone)]
pub struct BetaDistribution {
    pub alpha: f64, // successes + prior
    pub beta: f64,  // failures + prior
}

impl BetaDistribution {
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self {
            alpha: alpha.max(1e-6),
            beta: beta.max(1e-6),
        }
    }

    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    pub fn variance(&self) -> f64 {
        let total = self.alpha + self.beta;
        let denom = total.powi(2) * (total + 1.0);
        if denom == 0.0 {
            return 0.0;
        }
        (self.alpha * self.beta) / denom
    }

    /// Update belief with new evidence (success/failure)
    pub fn update(&mut self, success: bool) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }

    /// Updates belief with continuous positive evidence weight.
    pub fn update_weighted(&mut self, success_weight: f64, failure_weight: f64) {
        self.alpha += success_weight.max(0.0);
        self.beta += failure_weight.max(0.0);
    }

    /// Computes approximate 95% symmetric credible interval [lower, upper].
    pub fn credible_interval_95(&self) -> (f64, f64) {
        let m = self.mean();
        let s = self.variance().sqrt();
        let lower = (m - 1.96 * s).clamp(0.0, 1.0);
        let upper = (m + 1.96 * s).clamp(0.0, 1.0);
        (lower, upper)
    }
}

/// Gaussian (Normal-Normal) conjugate updater for continuous parameters with known noise variance.
#[derive(Debug, Clone)]
pub struct GaussianConjugate {
    pub mean: f64,
    pub variance: f64,
    pub observation_variance: f64,
}

impl GaussianConjugate {
    pub fn new(prior_mean: f64, prior_variance: f64, observation_variance: f64) -> Self {
        Self {
            mean: prior_mean,
            variance: prior_variance.max(1e-12),
            observation_variance: observation_variance.max(1e-12),
        }
    }

    /// Incorporates a single observation `x` into the posterior distribution.
    pub fn update(&mut self, observation: f64) {
        let prior_precision = 1.0 / self.variance;
        let obs_precision = 1.0 / self.observation_variance;
        let post_precision = prior_precision + obs_precision;
        let post_mean = (prior_precision * self.mean + obs_precision * observation) / post_precision;

        self.mean = post_mean;
        self.variance = 1.0 / post_precision;
    }

    /// Incorporates a batch of observations into the posterior distribution.
    pub fn update_batch(&mut self, observations: &[f64]) {
        if observations.is_empty() {
            return;
        }
        let n = observations.len() as f64;
        let sample_mean: f64 = observations.iter().sum::<f64>() / n;

        let prior_precision = 1.0 / self.variance;
        let batch_precision = n / self.observation_variance;
        let post_precision = prior_precision + batch_precision;
        let post_mean = (prior_precision * self.mean + batch_precision * sample_mean) / post_precision;

        self.mean = post_mean;
        self.variance = 1.0 / post_precision;
    }
}

/// Dirichlet distribution conjugate prior for categorical / multinomial observations.
#[derive(Debug, Clone)]
pub struct DirichletDistribution {
    pub alphas: Vec<f64>,
}

impl DirichletDistribution {
    pub fn new(alphas: &[f64]) -> Result<Self> {
        if alphas.is_empty() {
            bail!("Dirichlet alphas cannot be empty");
        }
        Ok(Self {
            alphas: alphas.iter().map(|&a| a.max(1e-6)).collect(),
        })
    }

    pub fn uniform(dimension: usize) -> Result<Self> {
        if dimension == 0 {
            bail!("Dimension must be > 0");
        }
        Ok(Self {
            alphas: vec![1.0; dimension],
        })
    }

    /// Computes the expected probability vector (mean).
    pub fn mean(&self) -> Vec<f64> {
        let sum: f64 = self.alphas.iter().sum();
        if sum == 0.0 {
            return vec![1.0 / self.alphas.len() as f64; self.alphas.len()];
        }
        self.alphas.iter().map(|&a| a / sum).collect()
    }

    /// Updates counts based on observed category index.
    pub fn update_category(&mut self, category_idx: usize, weight: f64) {
        if let Some(alpha) = self.alphas.get_mut(category_idx) {
            *alpha += weight.max(0.0);
        }
    }
}

/// Log-Odds belief representation for high-efficiency additive evidence updating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogOddsBelief {
    pub log_odds: f64,
}

impl LogOddsBelief {
    pub fn from_probability(prob: f64) -> Self {
        let p = prob.clamp(1e-9, 1.0 - 1e-9);
        Self {
            log_odds: (p / (1.0 - p)).ln(),
        }
    }

    pub fn to_probability(&self) -> f64 {
        1.0 / (1.0 + (-self.log_odds).exp())
    }

    /// Add log-likelihood ratio from independent evidence: log(P(E|H) / P(E|~H)).
    pub fn update_evidence(&mut self, log_likelihood_ratio: f64) {
        self.log_odds += log_likelihood_ratio;
    }
}

/// Bayesian update from input vector [prior_alpha, prior_beta, evidence...] (backwards-compatible).
pub fn bayesian_update(input: &[f64]) -> Result<Vec<f64>> {
    if input.len() < 2 {
        return Ok(vec![]);
    }
    let mut dist = BetaDistribution::new(input[0], input[1]);
    for &evidence in &input[2..] {
        dist.update(evidence > 0.5);
    }
    Ok(vec![dist.mean(), dist.variance(), dist.alpha, dist.beta])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_distribution_updating() {
        let mut beta = BetaDistribution::new(1.0, 1.0);
        assert_eq!(beta.mean(), 0.5);

        beta.update(true); // 2 successes, 1 failure -> mean = 2/3
        assert!((beta.mean() - 2.0 / 3.0).abs() < 1e-6);

        let (low, high) = beta.credible_interval_95();
        assert!(low < beta.mean() && high > beta.mean());
    }

    #[test]
    fn test_gaussian_conjugate_update() {
        // Prior: mean 10, variance 4; observation noise variance 1
        let mut gauss = GaussianConjugate::new(10.0, 4.0, 1.0);
        // Observe 12.0
        gauss.update(12.0);
        // Precision: 1/4 + 1 = 1.25 -> variance = 0.8
        // Mean: (10/4 + 12/1) / 1.25 = 14.5 / 1.25 = 11.6
        assert!((gauss.mean - 11.6).abs() < 1e-6);
        assert!((gauss.variance - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_log_odds_roundtrip() {
        let prob = 0.75;
        let belief = LogOddsBelief::from_probability(prob);
        assert!((belief.to_probability() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_dirichlet_multinomial() {
        let mut dir = DirichletDistribution::uniform(3).unwrap();
        assert_eq!(dir.mean(), vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]);

        dir.update_category(0, 3.0); // alphas: [4.0, 1.0, 1.0] -> sum = 6.0
        let m = dir.mean();
        assert!((m[0] - 4.0 / 6.0).abs() < 1e-6);
    }
}
