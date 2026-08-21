/// Graph Theory & Spectral Clustering primitives.
/// Used for constellation mapping, community detection, and centrality metrics.
/// Compute degree centrality from adjacency matrix (flattened)
pub fn degree_centrality(adj: &[f64], n: usize) -> Vec<f64> {
    let mut centrality = vec![0.0; n];
    for i in 0..n {
        for j in 0..n {
            centrality[i] += adj[i * n + j];
        }
        centrality[i] /= (n - 1).max(1) as f64;
    }
    centrality
}

/// Spectral clustering approximation (power iteration for largest eigenvalue)
pub fn spectral_cluster(adj: &[f64], n: usize, iterations: usize) -> Vec<f64> {
    let mut eigenvector = vec![1.0 / (n as f64).sqrt(); n];
    for _ in 0..iterations {
        let mut next = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                next[i] += adj[i * n + j] * eigenvector[j];
            }
        }
        let norm = next.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            eigenvector = next.iter().map(|x| x / norm).collect();
        }
    }
    eigenvector
}
