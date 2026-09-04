//! crates/compute/src/entropy.rs
//! Information Theory & Entropy primitives.
//! Used for data distillation, boilerplate detection, language modeling, and information density scoring.

use anyhow::{bail, Result};

/// Supported logarithmic bases for information measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogBase {
    /// Base 2 (bits / shannons)
    Two,
    /// Base e (nats)
    Natural,
    /// Base 10 (hartleys / bans)
    Ten,
}

impl LogBase {
    fn log(&self, x: f64) -> f64 {
        match self {
            LogBase::Two => x.log2(),
            LogBase::Natural => x.ln(),
            LogBase::Ten => x.log10(),
        }
    }
}

/// Computes Shannon entropy of an unnormalized or normalized frequency vector with a chosen base.
pub fn entropy_with_base(input: &[f64], base: LogBase) -> Result<f64> {
    if input.is_empty() {
        return Ok(0.0);
    }
    let sum: f64 = input.iter().sum();
    if sum <= 0.0 {
        return Ok(0.0);
    }
    let mut entropy = 0.0;
    for &p in input {
        if p > 0.0 {
            let prob = p / sum;
            entropy -= prob * base.log(prob);
        }
    }
    Ok(entropy.max(0.0))
}

/// Compute Shannon entropy of input distribution (base-2, returns [entropy, normalized_entropy]).
pub fn shannon_entropy(input: &[f64]) -> Result<Vec<f64>> {
    if input.is_empty() {
        return Ok(vec![0.0, 0.0]);
    }
    let entropy = entropy_with_base(input, LogBase::Two)?;
    let max_entropy = (input.len() as f64).log2();
    let normalized = if max_entropy > 0.0 {
        (entropy / max_entropy).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Ok(vec![entropy, normalized])
}

/// Compute Shannon entropy of raw binary byte slices (0.0 = completely homogeneous, 8.0 = maximum entropy / encrypted).
pub fn byte_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0.0f64; 256];
    for &b in data {
        counts[b as usize] += 1.0;
    }
    entropy_with_base(&counts, LogBase::Two).unwrap_or(0.0)
}

/// Computes Cross Entropy H(P, Q) = - \sum P(x) * ln(Q(x)).
pub fn cross_entropy(p: &[f64], q: &[f64]) -> Result<f64> {
    if p.len() != q.len() {
        bail!("Dimension mismatch for cross entropy: {} vs {}", p.len(), q.len());
    }
    let sum_p: f64 = p.iter().sum();
    let sum_q: f64 = q.iter().sum();
    if sum_p <= 0.0 || sum_q <= 0.0 {
        bail!("Distributions must have positive sum");
    }

    let mut ce = 0.0;
    for i in 0..p.len() {
        let pi = p[i] / sum_p;
        let qi = (q[i] / sum_q).max(1e-15);
        if pi > 0.0 {
            ce -= pi * qi.ln();
        }
    }
    Ok(ce)
}

/// KL Divergence D_KL(P || Q) between two distinct distributions.
pub fn kl_divergence_distributions(p: &[f64], q: &[f64]) -> Result<f64> {
    if p.len() != q.len() {
        bail!("Dimension mismatch for KL divergence: {} vs {}", p.len(), q.len());
    }
    let sum_p: f64 = p.iter().sum();
    let sum_q: f64 = q.iter().sum();
    if sum_p <= 0.0 || sum_q <= 0.0 {
        bail!("Distributions must have positive sum");
    }

    let mut kl = 0.0;
    for i in 0..p.len() {
        let pi = p[i] / sum_p;
        let qi = (q[i] / sum_q).max(1e-15);
        if pi > 0.0 {
            kl += pi * (pi / qi).ln();
        }
    }
    Ok(kl.max(0.0))
}

/// KL Divergence between two distributions (first half vs second half of input, backwards-compatible).
pub fn kl_divergence(input: &[f64]) -> Result<Vec<f64>> {
    if input.len() < 2 {
        return Ok(vec![0.0]);
    }
    let mid = input.len() / 2;
    let (p, q) = input.split_at(mid);
    let kl = kl_divergence_distributions(p, q)?;
    Ok(vec![kl])
}

/// Symmetric, bounded [0, 1] Jensen-Shannon Divergence JSD(P || Q) = 0.5 * (KL(P || M) + KL(Q || M)) where M = 0.5 * (P + Q).
pub fn jensen_shannon_divergence(p: &[f64], q: &[f64]) -> Result<f64> {
    if p.len() != q.len() {
        bail!("Dimension mismatch for JSD: {} vs {}", p.len(), q.len());
    }
    let sum_p: f64 = p.iter().sum();
    let sum_q: f64 = q.iter().sum();
    if sum_p <= 0.0 || sum_q <= 0.0 {
        bail!("Distributions must have positive sum");
    }

    let p_norm: Vec<f64> = p.iter().map(|&x| x / sum_p).collect();
    let q_norm: Vec<f64> = q.iter().map(|&x| x / sum_q).collect();
    let m: Vec<f64> = p_norm.iter().zip(q_norm.iter()).map(|(&x, &y)| 0.5 * (x + y)).collect();

    let kl_pm = kl_divergence_distributions(&p_norm, &m)?;
    let kl_qm = kl_divergence_distributions(&q_norm, &m)?;
    Ok((0.5 * (kl_pm + kl_qm)).max(0.0))
}

/// Mutual Information I(X; Y) computed from joint probability matrix (flattened row-major).
pub fn mutual_information(joint_p: &[f64], rows: usize, cols: usize) -> Result<f64> {
    if joint_p.len() != rows * cols {
        bail!("Joint probability matrix size mismatch: {} vs {}", joint_p.len(), rows * cols);
    }
    let total: f64 = joint_p.iter().sum();
    if total <= 0.0 {
        return Ok(0.0);
    }

    let mut marginal_x = vec![0.0; rows];
    let mut marginal_y = vec![0.0; cols];
    for i in 0..rows {
        for j in 0..cols {
            let p_xy = joint_p[i * cols + j] / total;
            marginal_x[i] += p_xy;
            marginal_y[j] += p_xy;
        }
    }

    let mut mi = 0.0;
    for i in 0..rows {
        for j in 0..cols {
            let p_xy = joint_p[i * cols + j] / total;
            if p_xy > 0.0 {
                let px = marginal_x[i];
                let py = marginal_y[j];
                if px > 0.0 && py > 0.0 {
                    mi += p_xy * (p_xy / (px * py)).ln();
                }
            }
        }
    }
    Ok(mi.max(0.0))
}

/// Perplexity of a probability distribution 2^{H(P)}.
pub fn perplexity(input: &[f64]) -> Result<f64> {
    let entropy = entropy_with_base(input, LogBase::Two)?;
    Ok(2.0f64.powf(entropy))
}

/// Renyi entropy of order alpha (alpha > 0, alpha != 1).
pub fn renyi_entropy(input: &[f64], alpha: f64) -> Result<f64> {
    if (alpha - 1.0).abs() < 1e-6 {
        return entropy_with_base(input, LogBase::Natural);
    }
    if alpha <= 0.0 {
        bail!("Renyi entropy alpha must be > 0");
    }
    let sum: f64 = input.iter().sum();
    if sum <= 0.0 {
        return Ok(0.0);
    }
    let sum_p_alpha: f64 = input.iter().map(|&x| (x / sum).powf(alpha)).sum();
    if sum_p_alpha <= 0.0 {
        return Ok(0.0);
    }
    Ok((1.0 / (1.0 - alpha)) * sum_p_alpha.ln())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy() {
        let uniform = [0.25, 0.25, 0.25, 0.25];
        let ent = shannon_entropy(&uniform).unwrap();
        assert!((ent[0] - 2.0).abs() < 1e-6);
        assert!((ent[1] - 1.0).abs() < 1e-6);

        let deterministic = [1.0, 0.0, 0.0, 0.0];
        let ent_det = shannon_entropy(&deterministic).unwrap();
        assert_eq!(ent_det[0], 0.0);
        assert_eq!(ent_det[1], 0.0);
    }

    #[test]
    fn test_kl_and_js_divergence() {
        let p = [0.5, 0.5];
        let q = [0.5, 0.5];
        let kl_identical = kl_divergence_distributions(&p, &q).unwrap();
        assert!(kl_identical < 1e-9);

        let jsd_identical = jensen_shannon_divergence(&p, &q).unwrap();
        assert!(jsd_identical < 1e-9);

        let p_diff = [0.9, 0.1];
        let q_diff = [0.1, 0.9];
        let jsd = jensen_shannon_divergence(&p_diff, &q_diff).unwrap();
        assert!(jsd > 0.0 && jsd <= 1.0);
    }

    #[test]
    fn test_perplexity() {
        let uniform_4 = [0.25, 0.25, 0.25, 0.25];
        let perp = perplexity(&uniform_4).unwrap();
        assert!((perp - 4.0).abs() < 1e-6);
    }
}
