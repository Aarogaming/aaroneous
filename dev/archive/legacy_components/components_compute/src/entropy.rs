/// Information Theory & Entropy primitives.
/// Used for data distillation, boilerplate detection, and information density scoring.
/// Compute Shannon entropy of input distribution
pub fn shannon_entropy(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    if input.is_empty() {
        return Ok(vec![0.0]);
    }
    let sum: f64 = input.iter().sum();
    let mut entropy = 0.0;
    for &p in input {
        let prob = p / sum;
        if prob > 0.0 {
            entropy -= prob * prob.log2();
        }
    }
    // Normalize to [0, 1]
    let max_entropy = (input.len() as f64).log2();
    let normalized = if max_entropy == 0.0 {
        0.0
    } else {
        entropy / max_entropy
    };
    Ok(vec![entropy, normalized])
}

/// KL Divergence between two distributions (first half vs second half of input)
pub fn kl_divergence(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    if input.len() < 2 {
        return Ok(vec![0.0]);
    }
    let mid = input.len() / 2;
    let (p, q) = input.split_at(mid);
    let sum_p: f64 = p.iter().sum();
    let sum_q: f64 = q.iter().sum();
    let mut kl = 0.0;
    for i in 0..mid.min(q.len()) {
        let pi = p[i] / sum_p.max(1e-10);
        let qi = q[i] / sum_q.max(1e-10);
        if pi > 0.0 && qi > 0.0 {
            kl += pi * (pi / qi).ln();
        }
    }
    Ok(vec![kl])
}
