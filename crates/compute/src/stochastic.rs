use rand::Rng;

/// Monte Carlo & MCMC simulation primitives.
/// Used for risk forecasting, metabolic prediction, and uncertainty quantification.
/// Run a Monte Carlo simulation over input parameters
pub fn monte_carlo_simulate(
    input: &[f64],
    iterations: usize,
    rng: &mut impl Rng,
) -> anyhow::Result<Vec<f64>> {
    if input.is_empty() {
        return Ok(vec![]);
    }

    let mut results = Vec::with_capacity(iterations);
    let mean: f64 = input.iter().sum::<f64>() / input.len() as f64;
    let variance: f64 = input.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / input.len() as f64;
    let std_dev = variance.sqrt();

    for _ in 0..iterations {
        // Sample from normal distribution (Box-Muller transform)
        let u1: f64 = rng.gen_range(1e-10..1.0);
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let sample = mean + std_dev * z;
        results.push(sample);
    }

    // Return summary statistics: [mean, std, p5, p50, p95]
    results.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = results.len();
    let sim_mean = results.iter().sum::<f64>() / n as f64;
    let sim_std = results
        .iter()
        .map(|x| (x - sim_mean).powi(2))
        .sum::<f64>()
        .sqrt();
    let p5 = results[(n as f64 * 0.05) as usize];
    let p50 = results[(n as f64 * 0.50) as usize];
    let p95 = results[(n as f64 * 0.95) as usize];

    Ok(vec![sim_mean, sim_std, p5, p50, p95])
}

/// Metropolis-Hastings MCMC step
pub fn metropolis_hastings_step(
    current: f64,
    proposal_std: f64,
    rng: &mut impl Rng,
    log_likelihood: impl Fn(f64) -> f64,
) -> f64 {
    let proposal = current + proposal_std * rng.gen::<f64>() * 2.0 - proposal_std;
    let log_alpha = log_likelihood(proposal) - log_likelihood(current);
    if rng.gen::<f64>() < log_alpha.exp() {
        proposal
    } else {
        current
    }
}
