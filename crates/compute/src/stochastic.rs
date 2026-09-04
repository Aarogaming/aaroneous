//! crates/compute/src/stochastic.rs
//! Monte Carlo & MCMC stochastic simulation primitives.
//! Used for risk forecasting, metabolic prediction, uncertainty quantification, and empirical confidence bounds.

use anyhow::Result;
use rand::Rng;
use std::cmp::Ordering;

/// Safely compares two f64 values without panicking on NaN.
fn safe_f64_cmp(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or_else(|| {
        if a.is_nan() && b.is_nan() {
            Ordering::Equal
        } else if a.is_nan() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    })
}

/// Detailed Summary Statistics for a Stochastic Simulation
#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloSummary {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p05: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p95: f64,
    pub interquartile_range: f64,
}

impl MonteCarloSummary {
    pub fn from_sorted_samples(sorted: &[f64]) -> Option<Self> {
        if sorted.is_empty() {
            return None;
        }
        let n = sorted.len();
        let mean = sorted.iter().sum::<f64>() / n as f64;
        let variance = sorted.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        let quantile = |p: f64| -> f64 {
            let idx = ((n as f64 - 1.0) * p).round() as usize;
            sorted[idx.min(n - 1)]
        };

        let p25 = quantile(0.25);
        let p75 = quantile(0.75);

        Some(Self {
            mean,
            std_dev,
            min: sorted[0],
            max: sorted[n - 1],
            p05: quantile(0.05),
            p25,
            median: quantile(0.50),
            p75,
            p95: quantile(0.95),
            interquartile_range: p75 - p25,
        })
    }
}

/// Run a Monte Carlo simulation over input parameters (backwards-compatible: [mean, std, p5, p50, p95]).
pub fn monte_carlo_simulate(
    input: &[f64],
    iterations: usize,
    rng: &mut impl Rng,
) -> Result<Vec<f64>> {
    if input.is_empty() || iterations == 0 {
        return Ok(vec![]);
    }

    let mut results = Vec::with_capacity(iterations);
    let mean: f64 = input.iter().sum::<f64>() / input.len() as f64;
    let variance: f64 = input.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / input.len() as f64;
    let std_dev = variance.sqrt().max(1e-12);

    for _ in 0..iterations {
        // Sample from normal distribution (Box-Muller transform)
        let u1: f64 = rng.gen_range(1e-10..1.0);
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let sample = mean + std_dev * z;
        results.push(sample);
    }

    // Zero-panic sorting
    results.sort_by(|&a, &b| safe_f64_cmp(a, b));
    let n = results.len();
    let sim_mean = results.iter().sum::<f64>() / n as f64;
    let sim_std = results
        .iter()
        .map(|x| (x - sim_mean).powi(2))
        .sum::<f64>()
        .sqrt()
        / (n as f64).sqrt();

    let p5 = results[(n as f64 * 0.05) as usize];
    let p50 = results[(n as f64 * 0.50) as usize];
    let p95 = results[(n as f64 * 0.95).min((n - 1) as f64) as usize];

    Ok(vec![sim_mean, sim_std, p5, p50, p95])
}

/// Bootstrap resampling to compute non-parametric empirical mean distribution.
pub fn bootstrap_resample(
    data: &[f64],
    num_resamples: usize,
    rng: &mut impl Rng,
) -> Vec<f64> {
    if data.is_empty() || num_resamples == 0 {
        return vec![];
    }
    let n = data.len();
    let mut resampled_means = Vec::with_capacity(num_resamples);

    for _ in 0..num_resamples {
        let mut sample_sum = 0.0;
        for _ in 0..n {
            let idx = rng.gen_range(0..n);
            sample_sum += data[idx];
        }
        resampled_means.push(sample_sum / n as f64);
    }

    resampled_means.sort_by(|&a, &b| safe_f64_cmp(a, b));
    resampled_means
}

/// Single step of Metropolis-Hastings MCMC (backwards-compatible).
pub fn metropolis_hastings_step(
    current: f64,
    proposal_std: f64,
    rng: &mut impl Rng,
    log_likelihood: impl Fn(f64) -> f64,
) -> f64 {
    let proposal = current + proposal_std * (rng.gen::<f64>() * 2.0 - 1.0);
    let log_alpha = log_likelihood(proposal) - log_likelihood(current);
    if rng.gen::<f64>() < log_alpha.exp() {
        proposal
    } else {
        current
    }
}

/// Output of an MCMC Sampling Chain
#[derive(Debug, Clone)]
pub struct McmcChain {
    pub samples: Vec<f64>,
    pub acceptance_rate: f64,
}

/// Runs a full Metropolis-Hastings MCMC chain with configurable burn-in and thinning.
pub fn run_mcmc_chain(
    initial: f64,
    steps: usize,
    burn_in: usize,
    thinning: usize,
    proposal_std: f64,
    rng: &mut impl Rng,
    log_posterior: impl Fn(f64) -> f64,
) -> McmcChain {
    let mut current = initial;
    let mut accepted_count = 0;
    let mut samples = Vec::new();
    let thin = thinning.max(1);

    for step in 0..(burn_in + steps) {
        let proposal = current + rng.gen_range(-proposal_std..proposal_std);
        let log_alpha = log_posterior(proposal) - log_posterior(current);

        let accepted = if log_alpha >= 0.0 {
            true
        } else {
            rng.gen::<f64>() < log_alpha.exp()
        };

        if accepted {
            current = proposal;
            if step >= burn_in {
                accepted_count += 1;
            }
        }

        if step >= burn_in && (step - burn_in).is_multiple_of(thin) {
            samples.push(current);
        }
    }

    let rate = if steps > 0 {
        accepted_count as f64 / steps as f64
    } else {
        0.0
    };

    McmcChain {
        samples,
        acceptance_rate: rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monte_carlo_summary() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let summary = MonteCarloSummary::from_sorted_samples(&sorted).unwrap();
        assert_eq!(summary.min, 1.0);
        assert_eq!(summary.max, 10.0);
        assert_eq!(summary.median, 6.0);
        assert!((summary.mean - 5.5).abs() < 1e-6);
    }

    #[test]
    fn test_bootstrap_resampling() {
        let mut rng = rand::thread_rng();
        let data = vec![10.0, 12.0, 11.0, 10.5, 11.5];
        let means = bootstrap_resample(&data, 100, &mut rng);
        assert_eq!(means.len(), 100);
        // Resampled means should stay close to empirical mean (11.0)
        assert!(means[0] >= 10.0);
        assert!(means[99] <= 12.0);
    }

    #[test]
    fn test_mcmc_chain_gaussian() {
        let mut rng = rand::thread_rng();
        // Target: standard normal distribution N(0, 1) -> log_p = -0.5 * x^2
        let log_p = |x: f64| -0.5 * x * x;
        let chain = run_mcmc_chain(0.0, 500, 100, 2, 0.5, &mut rng, log_p);
        assert!(!chain.samples.is_empty());
        assert!(chain.acceptance_rate > 0.1 && chain.acceptance_rate < 0.95);
    }
}
