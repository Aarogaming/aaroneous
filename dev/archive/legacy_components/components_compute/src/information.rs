/// Information Theory primitives.
/// Implements mutual information, transfer entropy, information bottleneck,
/// and rate-distortion theory for cross-domain synthesis and feature selection.
/// Shannon entropy of a discrete distribution.
/// H(X) = -Σ p(x) * log2(p(x))
pub fn shannon_entropy(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum()
}

/// Joint entropy of two variables.
/// H(X,Y) = -Σ p(x,y) * log2(p(x,y))
pub fn joint_entropy(joint_prob: &[Vec<f64>]) -> f64 {
    joint_prob
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum()
}

/// Conditional entropy H(Y|X) = H(X,Y) - H(X)
pub fn conditional_entropy(joint_prob: &[Vec<f64>], marginal_x: &[f64]) -> f64 {
    let h_xy = joint_entropy(joint_prob);
    let h_x = shannon_entropy(marginal_x);
    (h_xy - h_x).max(0.0)
}

/// Mutual Information I(X;Y) = H(X) + H(Y) - H(X,Y)
/// Quantifies shared information between two variables.
/// Used for cross-domain link detection and feature selection.
pub fn mutual_information(joint_prob: &[Vec<f64>], marginal_x: &[f64], marginal_y: &[f64]) -> f64 {
    let h_x = shannon_entropy(marginal_x);
    let h_y = shannon_entropy(marginal_y);
    let h_xy = joint_entropy(joint_prob);
    (h_x + h_y - h_xy).max(0.0)
}

/// Normalized mutual information NMI = 2*I(X;Y) / (H(X) + H(Y))
/// Returns value in [0, 1] for comparability across different variable pairs.
pub fn normalized_mutual_information(
    joint_prob: &[Vec<f64>],
    marginal_x: &[f64],
    marginal_y: &[f64],
) -> f64 {
    let mi = mutual_information(joint_prob, marginal_x, marginal_y);
    let h_x = shannon_entropy(marginal_x);
    let h_y = shannon_entropy(marginal_y);
    if h_x + h_y == 0.0 {
        return 0.0;
    }
    2.0 * mi / (h_x + h_y)
}

/// Kullback-Leibler divergence D_KL(P||Q) = Σ p(x) * log2(p(x)/q(x))
/// Measures how much P differs from Q.
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len());
    p.iter()
        .zip(q.iter())
        .filter(|(&pi, &qi)| pi > 0.0 && qi > 0.0)
        .map(|(&pi, &qi)| pi * (pi / qi).log2())
        .sum()
}

/// Jensen-Shannon divergence JS(P||Q) = 0.5*D_KL(P||M) + 0.5*D_KL(Q||M)
/// where M = 0.5*(P + Q). Symmetric and bounded [0, 1].
pub fn js_divergence(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len());
    let m: Vec<f64> = p
        .iter()
        .zip(q.iter())
        .map(|(&pi, &qi)| 0.5 * (pi + qi))
        .collect();
    0.5 * kl_divergence(p, &m) + 0.5 * kl_divergence(q, &m)
}

/// Transfer Entropy T_{X→Y} = Σ p(y_{t+1}, y_t^{(k)}, x_t^{(l)}) * log2(p(y_{t+1}|y_t^{(k)}, x_t^{(l)}) / p(y_{t+1}|y_t^{(k)}))
/// Measures directed information flow from X to Y.
/// Used for causal graph construction and specialist influence detection.
pub fn transfer_entropy(
    source_history: &[f64],
    target_history: &[f64],
    source_lag: usize,
    target_lag: usize,
) -> f64 {
    if source_history.len() <= source_lag || target_history.len() <= target_lag {
        return 0.0;
    }

    // Discretize continuous values into bins
    let n_bins = 5;
    let all_values: Vec<f64> = source_history
        .iter()
        .chain(target_history.iter())
        .cloned()
        .collect();
    let min_val = all_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = all_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;

    let discretize = |v: f64| -> usize {
        if range == 0.0 {
            return 0;
        }
        ((v - min_val) / range * (n_bins - 1) as f64).round() as usize % n_bins
    };

    // Build joint and conditional probabilities
    let mut joint_counts = vec![vec![0.0; n_bins * n_bins]; n_bins];
    let mut target_cond_counts = vec![0.0; n_bins * n_bins];
    let mut source_target_cond_counts = vec![0.0; n_bins * n_bins * n_bins];

    let min_len = source_history.len().min(target_history.len());
    let start_idx = source_lag.max(target_lag);

    for t in start_idx..(min_len - 1) {
        let y_next = discretize(target_history[t + 1]);
        let y_past = discretize(target_history[t]);
        let x_past = discretize(source_history[t]);

        joint_counts[y_next][y_past * n_bins + x_past] += 1.0;
        target_cond_counts[y_next * n_bins + y_past] += 1.0;
        source_target_cond_counts[y_next * n_bins * n_bins + y_past * n_bins + x_past] += 1.0;
    }

    // Normalize to probabilities
    let total: f64 = joint_counts.iter().map(|row| row.iter().sum::<f64>()).sum();
    if total == 0.0 {
        return 0.0;
    }

    let joint_prob: Vec<Vec<f64>> = joint_counts
        .iter()
        .map(|row| row.iter().map(|&c| c / total).collect())
        .collect();

    let target_cond_prob: Vec<f64> = target_cond_counts.iter().map(|&c| c / total).collect();
    let source_target_cond_prob: Vec<f64> = source_target_cond_counts
        .iter()
        .map(|&c| c / total)
        .collect();

    // Compute transfer entropy
    let mut te = 0.0;
    for y_next in 0..n_bins {
        for y_past in 0..n_bins {
            for x_past in 0..n_bins {
                let p_joint = joint_prob[y_next][y_past * n_bins + x_past];
                let p_y_given_y_past = if target_cond_prob[y_next * n_bins + y_past] > 0.0 {
                    p_joint / target_cond_prob[y_next * n_bins + y_past]
                } else {
                    0.0
                };
                let p_y_given_yp_xp =
                    source_target_cond_prob[y_next * n_bins * n_bins + y_past * n_bins + x_past];

                if p_joint > 0.0 && p_y_given_y_past > 0.0 {
                    te += p_joint * (p_y_given_yp_xp / p_y_given_y_past).log2();
                }
            }
        }
    }

    te.max(0.0)
}

/// Information Bottleneck: find compressed representation T of X that preserves information about Y.
/// min I(X;T) - β*I(T;Y)
/// Returns optimal compression level and preserved information.
pub fn information_bottleneck(
    ix_t: f64, // I(X;T): compression cost
    it_y: f64, // I(T;Y): relevance
    beta: f64, // Trade-off parameter
) -> f64 {
    // Objective: minimize compression while maximizing relevance
    ix_t - beta * it_y
}

/// Rate-Distortion function approximation.
/// R(D) = minimum rate to achieve distortion D.
/// For Gaussian source: R(D) = 0.5 * log2(σ²/D)
pub fn rate_distortion_gaussian(variance: f64, distortion: f64) -> f64 {
    if distortion <= 0.0 || variance <= 0.0 {
        return f64::INFINITY;
    }
    if distortion >= variance {
        return 0.0;
    }
    0.5 * (variance / distortion).log2()
}

/// Cross-entropy H(P,Q) = -Σ p(x) * log2(q(x))
/// Used for evaluating prediction quality.
pub fn cross_entropy(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len());
    p.iter()
        .zip(q.iter())
        .filter(|(&pi, &qi)| pi > 0.0 && qi > 0.0)
        .map(|(&pi, &qi)| -pi * qi.log2())
        .sum()
}

/// Perplexity = 2^H(P)
/// Effective number of outcomes. Lower is better.
pub fn perplexity(probabilities: &[f64]) -> f64 {
    let h = shannon_entropy(probabilities);
    2.0f64.powf(h)
}

/// Information density i(x;y) = log2(p(x,y) / (p(x)*p(y)))
/// Pointwise mutual information.
pub fn information_density(p_joint: f64, p_x: f64, p_y: f64) -> f64 {
    if p_joint > 0.0 && p_x > 0.0 && p_y > 0.0 {
        (p_joint / (p_x * p_y)).log2()
    } else {
        0.0
    }
}

/// Active Information Storage: information in a variable's own past.
/// AIS = I(X_t; X_t^{(k)}) = Σ p(x_t, x_t^{(k)}) * log2(p(x_t|x_t^{(k)}) / p(x_t))
pub fn active_information_storage(history: &[f64], lag: usize) -> f64 {
    if history.len() <= lag {
        return 0.0;
    }

    let n_bins = 5;
    let min_val = history.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = history.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;

    let discretize = |v: f64| -> usize {
        if range == 0.0 {
            return 0;
        }
        ((v - min_val) / range * (n_bins - 1) as f64).round() as usize % n_bins
    };

    let mut joint_counts = vec![vec![0.0; n_bins]; n_bins];
    let mut marginal_current = vec![0.0; n_bins];
    let mut marginal_past = vec![0.0; n_bins];

    for t in lag..(history.len() - 1) {
        let current = discretize(history[t + 1]);
        let past = discretize(history[t]);

        joint_counts[current][past] += 1.0;
        marginal_current[current] += 1.0;
        marginal_past[past] += 1.0;
    }

    let total: f64 = marginal_current.iter().sum();
    if total == 0.0 {
        return 0.0;
    }

    let mut ais = 0.0;
    for i in 0..n_bins {
        for j in 0..n_bins {
            let p_joint = joint_counts[i][j] / total;
            let p_current = marginal_current[i] / total;
            let p_past = marginal_past[j] / total;

            if p_joint > 0.0 && p_current > 0.0 && p_past > 0.0 {
                ais += p_joint * (p_joint / (p_current * p_past)).log2();
            }
        }
    }

    ais.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy() {
        let uniform = vec![0.25, 0.25, 0.25, 0.25];
        assert!((shannon_entropy(&uniform) - 2.0).abs() < 1e-10);

        let deterministic = vec![1.0, 0.0, 0.0];
        assert!((shannon_entropy(&deterministic) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_mutual_information() {
        // Perfect correlation
        let joint = vec![vec![0.5, 0.0], vec![0.0, 0.5]];
        let marg_x = vec![0.5, 0.5];
        let marg_y = vec![0.5, 0.5];
        let mi = mutual_information(&joint, &marg_x, &marg_y);
        assert!((mi - 1.0).abs() < 1e-10);

        // Independence
        let joint_ind = vec![vec![0.25, 0.25], vec![0.25, 0.25]];
        let mi_ind = mutual_information(&joint_ind, &marg_x, &marg_y);
        assert!(mi_ind < 1e-10);
    }

    #[test]
    fn test_kl_divergence() {
        let p = vec![0.5, 0.5];
        let q = vec![0.5, 0.5];
        assert!((kl_divergence(&p, &q) - 0.0).abs() < 1e-10);

        let p2 = vec![0.9, 0.1];
        let q2 = vec![0.5, 0.5];
        assert!(kl_divergence(&p2, &q2) > 0.0);
    }

    #[test]
    fn test_rate_distortion() {
        let r = rate_distortion_gaussian(1.0, 0.25);
        assert!((r - 1.0).abs() < 1e-10); // 0.5 * log2(4) = 1.0
    }

    #[test]
    fn test_perplexity() {
        let uniform = vec![0.25, 0.25, 0.25, 0.25];
        assert!((perplexity(&uniform) - 4.0).abs() < 1e-10);
    }
}
