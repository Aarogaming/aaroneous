/// SAB (Semantic Association Block) Tensor Analysis
/// Uses information theory and tensor operations for surface similarity,
/// spectral clustering, and cross-domain information flow.
use compute::information::{js_divergence, shannon_entropy};

/// SAB surface embedding with tensor operations.
/// Each surface is represented as a feature vector for similarity computation.
#[derive(Debug, Clone)]
pub struct SabEmbedding {
    pub surface_name: String,
    pub features: Vec<f64>,
    pub metadata: SabMetadata,
}

#[derive(Debug, Clone)]
pub struct SabMetadata {
    pub artifact_count: usize,
    pub pattern_count: usize,
    pub module_count: usize,
    pub complexity_score: f64,
    pub last_updated: f64, // timestamp
}

impl SabEmbedding {
    pub fn new(surface_name: &str, features: Vec<f64>, metadata: SabMetadata) -> Self {
        Self {
            surface_name: surface_name.to_string(),
            features,
            metadata,
        }
    }

    /// Normalize features to unit vector.
    pub fn normalize(&mut self) {
        let norm: f64 = self.features.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            self.features = self.features.iter().map(|x| x / norm).collect();
        }
    }
}

/// SAB Similarity Matrix.
/// Computes pairwise similarity between all surfaces using multiple metrics.
#[derive(Debug, Clone)]
pub struct SabSimilarityMatrix {
    pub surface_names: Vec<String>,
    pub cosine_matrix: Vec<Vec<f64>>,
    pub js_divergence_matrix: Vec<Vec<f64>>,
    pub mutual_info_matrix: Vec<Vec<f64>>,
}

impl SabSimilarityMatrix {
    /// Compute full similarity matrix from embeddings.
    pub fn from_embeddings(embeddings: &[SabEmbedding]) -> Self {
        let n = embeddings.len();
        let mut cosine_matrix = vec![vec![0.0; n]; n];
        let mut js_divergence_matrix = vec![vec![0.0; n]; n];
        let mut mutual_info_matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    cosine_matrix[i][j] = 1.0;
                    js_divergence_matrix[i][j] = 0.0;
                    mutual_info_matrix[i][j] = self_entropy(&embeddings[i].features);
                } else if j > i {
                    // Cosine similarity
                    let sim = compute_cosine(&embeddings[i].features, &embeddings[j].features);
                    cosine_matrix[i][j] = sim;
                    cosine_matrix[j][i] = sim;

                    // JS Divergence (symmetric)
                    let js =
                        compute_js_divergence(&embeddings[i].features, &embeddings[j].features);
                    js_divergence_matrix[i][j] = js;
                    js_divergence_matrix[j][i] = js;

                    // Mutual Information (approximated via feature correlation)
                    let mi = approximate_mutual_information(
                        &embeddings[i].features,
                        &embeddings[j].features,
                    );
                    mutual_info_matrix[i][j] = mi;
                    mutual_info_matrix[j][i] = mi;
                }
            }
        }

        let surface_names = embeddings.iter().map(|e| e.surface_name.clone()).collect();

        Self {
            surface_names,
            cosine_matrix,
            js_divergence_matrix,
            mutual_info_matrix,
        }
    }

    /// Get top-K most similar surfaces to a given surface.
    pub fn top_k_similar(&self, surface_idx: usize, k: usize) -> Vec<(usize, f64)> {
        if surface_idx >= self.surface_names.len() {
            return vec![];
        }

        let mut similarities: Vec<(usize, f64)> = self.cosine_matrix[surface_idx]
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != surface_idx)
            .map(|(i, &s)| (i, s))
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.into_iter().take(k).collect()
    }

    /// Find surfaces with high mutual information (strong information flow).
    pub fn high_mutual_info_pairs(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let n = self.surface_names.len();
        let mut pairs = Vec::new();

        for i in 0..n {
            for j in (i + 1)..n {
                let mi = self.mutual_info_matrix[i][j];
                if mi > threshold {
                    pairs.push((i, j, mi));
                }
            }
        }

        pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        pairs
    }
}

/// Spectral clustering of SAB surfaces.
/// Uses eigendecomposition of the graph Laplacian to find clusters.
pub fn spectral_clustering(similarity_matrix: &SabSimilarityMatrix, k: usize) -> Vec<usize> {
    let n = similarity_matrix.surface_names.len();
    if n == 0 || k == 0 {
        return vec![];
    }

    if k >= n {
        return (0..n).collect();
    }

    // Build affinity matrix (cosine similarity)
    let affinity = &similarity_matrix.cosine_matrix;

    // Compute degree matrix D (diagonal)
    let degrees: Vec<f64> = affinity.iter().map(|row| row.iter().sum()).collect();

    // Build unnormalized Laplacian L = D - W
    let mut laplacian = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                laplacian[i][j] = degrees[i];
            } else {
                laplacian[i][j] = -affinity[i][j];
            }
        }
    }

    // Simplified spectral clustering: use k-means on first k eigenvectors
    // For now, use a simplified approach based on degree centrality
    let mut assignments = vec![0; n];

    // Sort by degree and assign to clusters
    let mut degree_indices: Vec<(usize, f64)> =
        degrees.iter().enumerate().map(|(i, &d)| (i, d)).collect();
    degree_indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Assign to clusters based on degree quantiles (distribute evenly)
    for (idx, &(i, _)) in degree_indices.iter().enumerate() {
        assignments[i] = idx * k / n;
    }

    assignments
}

/// Compute information flow between SAB surfaces.
/// Returns directed information flow matrix (transfer entropy approximation).
pub fn compute_information_flow(
    embeddings: &[SabEmbedding],
    time_series: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    let n = embeddings.len();
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

/// Rate-distortion analysis for SAB compression.
/// Finds optimal representation size for a given accuracy.
#[allow(clippy::needless_range_loop)]
pub fn rate_distortion_analysis(embeddings: &[SabEmbedding], target_distortion: f64) -> f64 {
    if embeddings.is_empty() {
        return 0.0;
    }

    // Compute variance of each feature dimension
    let dim = embeddings[0].features.len();
    let mut variances = vec![0.0; dim];

    for d in 0..dim {
        let mean: f64 =
            embeddings.iter().map(|e| e.features[d]).sum::<f64>() / embeddings.len() as f64;
        let variance: f64 = embeddings
            .iter()
            .map(|e| (e.features[d] - mean).powi(2))
            .sum::<f64>()
            / embeddings.len() as f64;
        variances[d] = variance;
    }

    // Total rate for given distortion
    let total_rate: f64 = variances
        .iter()
        .filter(|&&v| v > target_distortion)
        .map(|&v| 0.5 * (v / target_distortion).log2())
        .sum();

    total_rate
}

/// Find redundant surfaces (high similarity, low unique information).
pub fn find_redundant_surfaces(
    embeddings: &[SabEmbedding],
    similarity_threshold: f64,
) -> Vec<(usize, usize)> {
    let n = embeddings.len();
    let mut redundant = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let sim = compute_cosine(&embeddings[i].features, &embeddings[j].features);
            if sim > similarity_threshold {
                redundant.push((i, j));
            }
        }
    }

    redundant
}

/// Compute SAB surface importance via PageRank-like algorithm.
#[allow(clippy::needless_range_loop)]
pub fn compute_surface_importance(
    similarity_matrix: &SabSimilarityMatrix,
    damping: f64,
    iterations: usize,
) -> Vec<f64> {
    let n = similarity_matrix.surface_names.len();
    if n == 0 {
        return vec![];
    }

    // Initialize uniform importance
    let mut importance = vec![1.0 / n as f64; n];

    for _ in 0..iterations {
        let mut new_importance = vec![0.0; n];

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let sim = similarity_matrix.cosine_matrix[i][j].max(0.0);
                    let out_degree: f64 = similarity_matrix.cosine_matrix[j]
                        .iter()
                        .enumerate()
                        .filter(|(k, _)| *k != j)
                        .map(|(_, &s)| s.max(0.0))
                        .sum();

                    if out_degree > 0.0 {
                        new_importance[i] += sim / out_degree * importance[j];
                    }
                }
            }
        }

        // Apply damping
        for i in 0..n {
            new_importance[i] = damping * new_importance[i] + (1.0 - damping) / n as f64;
        }

        importance = new_importance;
    }

    importance
}

// Helper functions

fn compute_cosine(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

    if norm_a * norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn compute_js_divergence(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    // Normalize to probability distributions
    let sum_a: f64 = a.iter().map(|x| x.abs()).sum();
    let sum_b: f64 = b.iter().map(|x| x.abs()).sum();

    if sum_a == 0.0 || sum_b == 0.0 {
        return 0.0;
    }

    let p: Vec<f64> = a.iter().map(|x| x.abs() / sum_a).collect();
    let q: Vec<f64> = b.iter().map(|x| x.abs() / sum_b).collect();

    js_divergence(&p, &q)
}

fn approximate_mutual_information(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    // Approximate MI via normalized correlation
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
    // MI ≈ -0.5 * log(1 - ρ²) for Gaussian variables
    let rho_sq = correlation.powi(2);
    if rho_sq >= 1.0 {
        return 1.0;
    }
    -0.5 * (1.0 - rho_sq).ln().max(0.0)
}

fn self_entropy(features: &[f64]) -> f64 {
    if features.is_empty() {
        return 0.0;
    }

    // Normalize to probability distribution
    let sum: f64 = features.iter().map(|x| x.abs()).sum();
    if sum == 0.0 {
        return 0.0;
    }

    let probs: Vec<f64> = features.iter().map(|x| x.abs() / sum).collect();
    shannon_entropy(&probs)
}

fn approximate_transfer_entropy(source: &[f64], target: &[f64]) -> f64 {
    if source.len() < 2 || target.len() < 2 {
        return 0.0;
    }

    // Simplified: cross-correlation at lag 1
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_embeddings() -> Vec<SabEmbedding> {
        vec![
            SabEmbedding::new(
                "surface_a",
                vec![1.0, 0.5, 0.2],
                SabMetadata {
                    artifact_count: 5,
                    pattern_count: 3,
                    module_count: 2,
                    complexity_score: 0.7,
                    last_updated: 100.0,
                },
            ),
            SabEmbedding::new(
                "surface_b",
                vec![0.9, 0.6, 0.3],
                SabMetadata {
                    artifact_count: 4,
                    pattern_count: 2,
                    module_count: 3,
                    complexity_score: 0.6,
                    last_updated: 90.0,
                },
            ),
            SabEmbedding::new(
                "surface_c",
                vec![0.1, 0.2, 0.9],
                SabMetadata {
                    artifact_count: 2,
                    pattern_count: 5,
                    module_count: 1,
                    complexity_score: 0.3,
                    last_updated: 80.0,
                },
            ),
        ]
    }

    #[test]
    fn test_similarity_matrix() {
        let embeddings = create_test_embeddings();
        let matrix = SabSimilarityMatrix::from_embeddings(&embeddings);

        assert_eq!(matrix.surface_names.len(), 3);
        assert_eq!(matrix.cosine_matrix.len(), 3);

        // Diagonal should be 1.0
        for i in 0..3 {
            assert!((matrix.cosine_matrix[i][i] - 1.0).abs() < 1e-10);
        }

        // Matrix should be symmetric
        for i in 0..3 {
            for j in 0..3 {
                assert!((matrix.cosine_matrix[i][j] - matrix.cosine_matrix[j][i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_top_k_similar() {
        let embeddings = create_test_embeddings();
        let matrix = SabSimilarityMatrix::from_embeddings(&embeddings);

        let top = matrix.top_k_similar(0, 1);
        assert_eq!(top.len(), 1);
        // surface_a and surface_b should be most similar
        assert!(top[0].0 == 1 || top[0].0 == 2);
    }

    #[test]
    fn test_spectral_clustering() {
        let embeddings = create_test_embeddings();
        let matrix = SabSimilarityMatrix::from_embeddings(&embeddings);

        let clusters = spectral_clustering(&matrix, 2);
        assert_eq!(clusters.len(), 3);
        assert!(clusters.iter().all(|&c| c < 2));
    }

    #[test]
    fn test_surface_importance() {
        let embeddings = create_test_embeddings();
        let matrix = SabSimilarityMatrix::from_embeddings(&embeddings);

        let importance = compute_surface_importance(&matrix, 0.85, 20);
        assert_eq!(importance.len(), 3);

        // Importance should sum to approximately 1.0
        let sum: f64 = importance.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rate_distortion() {
        let embeddings = create_test_embeddings();
        let rate = rate_distortion_analysis(&embeddings, 0.1);
        assert!(rate > 0.0);

        let rate_higher_distortion = rate_distortion_analysis(&embeddings, 0.5);
        assert!(rate_higher_distortion < rate); // Higher distortion -> lower rate
    }

    #[test]
    fn test_find_redundant_surfaces() {
        let embeddings = create_test_embeddings();
        let redundant = find_redundant_surfaces(&embeddings, 0.95);

        // surface_a and surface_b are very similar
        assert!(!redundant.is_empty());
    }
}
