/// Bayesian Inference primitives.
/// Used for trust scoring, intent classification, and belief updates.

#[derive(Debug, Clone)]
pub struct BetaDistribution {
    pub alpha: f64, // successes + prior
    pub beta: f64,  // failures + prior
}

impl BetaDistribution {
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }

    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    pub fn variance(&self) -> f64 {
        let denom = (self.alpha + self.beta).powi(2) * (self.alpha + self.beta + 1.0);
        if denom == 0.0 { return 0.0; }
        self.alpha * self.beta / denom
    }

    /// Update belief with new evidence (success/failure)
    pub fn update(&mut self, success: bool) {
        if success { self.alpha += 1.0; } else { self.beta += 1.0; }
    }
}

/// Bayesian update from input vector [prior_alpha, prior_beta, evidence...]
pub fn bayesian_update(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    if input.len() < 2 { return Ok(vec![]); }
    let mut dist = BetaDistribution::new(input[0], input[1]);
    for &evidence in &input[2..] {
        dist.update(evidence > 0.5);
    }
    Ok(vec![dist.mean(), dist.variance(), dist.alpha, dist.beta])
}
