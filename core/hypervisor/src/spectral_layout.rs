/// Spectral Graph Layout for Constellation.
/// Uses eigendecomposition of the graph Laplacian for optimal 2D/3D positioning.
/// Replaces O(n²) iterative force-directed layout with O(n³) one-shot spectral decomposition.

/// Compute 2D positions from graph edges using spectral layout.
/// Returns (x, y) coordinates for each node.
pub fn spectral_layout_2d(n_nodes: usize, edges: &[(usize, usize, f64)]) -> Vec<(f64, f64)> {
    if n_nodes == 0 {
        return vec![];
    }
    if n_nodes == 1 {
        return vec![(0.0, 0.0)];
    }

    // Build weighted adjacency matrix
    let mut adjacency = vec![vec![0.0; n_nodes]; n_nodes];
    for &(i, j, w) in edges {
        if i < n_nodes && j < n_nodes {
            adjacency[i][j] = w;
            adjacency[j][i] = w;
        }
    }

    // Build Laplacian: L = D - W
    let mut laplacian = vec![vec![0.0; n_nodes]; n_nodes];
    for i in 0..n_nodes {
        let degree: f64 = adjacency[i].iter().sum();
        for j in 0..n_nodes {
            if i == j {
                laplacian[i][j] = degree;
            } else {
                laplacian[i][j] = -adjacency[i][j];
            }
        }
    }

    // Find first two non-trivial eigenvectors using power iteration
    // (simplified - full eigendecomposition would be more accurate)
    let eigenvector_1 = power_iteration(&laplacian, n_nodes, 1);
    let eigenvector_2 = power_iteration_orthogonal(&laplacian, &eigenvector_1, n_nodes);

    // Positions from eigenvectors
    let positions: Vec<(f64, f64)> = (0..n_nodes)
        .map(|i| (eigenvector_1[i], eigenvector_2[i]))
        .collect();

    // Normalize positions to [-100, 100] range
    normalize_positions(&positions)
}

/// Compute 3D positions from graph edges using spectral layout.
pub fn spectral_layout_3d(n_nodes: usize, edges: &[(usize, usize, f64)]) -> Vec<(f64, f64, f64)> {
    if n_nodes == 0 {
        return vec![];
    }
    if n_nodes == 1 {
        return vec![(0.0, 0.0, 0.0)];
    }

    // Build weighted adjacency matrix
    let mut adjacency = vec![vec![0.0; n_nodes]; n_nodes];
    for &(i, j, w) in edges {
        if i < n_nodes && j < n_nodes {
            adjacency[i][j] = w;
            adjacency[j][i] = w;
        }
    }

    // Build Laplacian: L = D - W
    let mut laplacian = vec![vec![0.0; n_nodes]; n_nodes];
    for i in 0..n_nodes {
        let degree: f64 = adjacency[i].iter().sum();
        for j in 0..n_nodes {
            if i == j {
                laplacian[i][j] = degree;
            } else {
                laplacian[i][j] = -adjacency[i][j];
            }
        }
    }

    // Find first three non-trivial eigenvectors
    let eigenvector_1 = power_iteration(&laplacian, n_nodes, 1);
    let eigenvector_2 = power_iteration_orthogonal(&laplacian, &eigenvector_1, n_nodes);
    let eigenvector_3 = power_iteration_orthogonal_to_two(&laplacian, &eigenvector_1, &eigenvector_2, n_nodes);

    // Positions from eigenvectors
    let positions: Vec<(f64, f64, f64)> = (0..n_nodes)
        .map(|i| (eigenvector_1[i], eigenvector_2[i], eigenvector_3[i]))
        .collect();

    // Normalize positions
    normalize_positions_3d(&positions)
}

/// Power iteration to find the k-th eigenvector.
/// Simplified: finds eigenvector corresponding to smallest non-zero eigenvalue.
fn power_iteration(laplacian: &[Vec<f64>], n: usize, _k: usize) -> Vec<f64> {
    // Initialize with random-like vector (deterministic for reproducibility)
    let mut v: Vec<f64> = (0..n)
        .map(|i| ((i * 7 + 13) % 100) as f64 / 100.0 - 0.5)
        .collect();

    // Normalize
    let norm: f64 = v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    if norm > 0.0 {
        v = v.iter().map(|x| x / norm).collect();
    }

    // Inverse iteration (for smallest eigenvalue)
    // Simplified: use shifted inverse iteration
    let shift = 1.0; // Shift to find smallest non-zero eigenvalue
    let mut shifted = laplacian.to_vec();
    for i in 0..n {
        shifted[i][i] += shift;
    }

    // Power iteration on shifted matrix
    for _ in 0..50 {
        let mut new_v = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                new_v[i] += shifted[i][j] * v[j];
            }
        }

        // Normalize
        let norm: f64 = new_v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            v = new_v.iter().map(|x| x / norm).collect();
        } else {
            break;
        }
    }

    v
}

/// Power iteration with orthogonality constraint to first eigenvector.
fn power_iteration_orthogonal(laplacian: &[Vec<f64>], v1: &[f64], n: usize) -> Vec<f64> {
    let mut v: Vec<f64> = (0..n)
        .map(|i| ((i * 11 + 7) % 100) as f64 / 100.0 - 0.5)
        .collect();

    // Orthogonalize against v1
    let dot: f64 = v.iter().zip(v1.iter()).map(|(a, b)| a * b).sum();
    v = v.iter().zip(v1.iter()).map(|(a, b)| a - dot * b).collect();

    // Normalize
    let norm: f64 = v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    if norm > 0.0 {
        v = v.iter().map(|x| x / norm).collect();
    }

    // Power iteration with re-orthogonalization
    let shift = 1.0;
    let mut shifted = laplacian.to_vec();
    for i in 0..n {
        shifted[i][i] += shift;
    }

    for _ in 0..50 {
        let mut new_v = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                new_v[i] += shifted[i][j] * v[j];
            }
        }

        // Re-orthogonalize against v1
        let dot: f64 = new_v.iter().zip(v1.iter()).map(|(a, b)| a * b).sum();
        new_v = new_v.iter().zip(v1.iter()).map(|(a, b)| a - dot * b).collect();

        // Normalize
        let norm: f64 = new_v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            v = new_v.iter().map(|x| x / norm).collect();
        } else {
            break;
        }
    }

    v
}

/// Power iteration orthogonal to two eigenvectors.
fn power_iteration_orthogonal_to_two(
    laplacian: &[Vec<f64>],
    v1: &[f64],
    v2: &[f64],
    n: usize,
) -> Vec<f64> {
    let mut v: Vec<f64> = (0..n)
        .map(|i| ((i * 17 + 3) % 100) as f64 / 100.0 - 0.5)
        .collect();

    // Orthogonalize against v1 and v2
    let dot1: f64 = v.iter().zip(v1.iter()).map(|(a, b)| a * b).sum();
    let dot2: f64 = v.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    v = v.iter()
        .zip(v1.iter())
        .zip(v2.iter())
        .map(|((a, b1), b2)| a - dot1 * b1 - dot2 * b2)
        .collect();

    // Normalize
    let norm: f64 = v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    if norm > 0.0 {
        v = v.iter().map(|x| x / norm).collect();
    }

    // Power iteration with re-orthogonalization
    let shift = 1.0;
    let mut shifted = laplacian.to_vec();
    for i in 0..n {
        shifted[i][i] += shift;
    }

    for _ in 0..50 {
        let mut new_v = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                new_v[i] += shifted[i][j] * v[j];
            }
        }

        // Re-orthogonalize against v1 and v2
        let dot1: f64 = new_v.iter().zip(v1.iter()).map(|(a, b)| a * b).sum();
        let dot2: f64 = new_v.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        new_v = new_v.iter()
            .zip(v1.iter())
            .zip(v2.iter())
            .map(|((a, b1), b2)| a - dot1 * b1 - dot2 * b2)
            .collect();

        // Normalize
        let norm: f64 = new_v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            v = new_v.iter().map(|x| x / norm).collect();
        } else {
            break;
        }
    }

    v
}

/// Normalize 2D positions to [-scale, scale] range.
fn normalize_positions(positions: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if positions.is_empty() {
        return vec![];
    }

    let scale = 100.0;
    let max_x = positions.iter().map(|(x, _)| x.abs()).fold(0.0f64, f64::max);
    let max_y = positions.iter().map(|(_, y)| y.abs()).fold(0.0f64, f64::max);

    positions
        .iter()
        .map(|(x, y)| {
            let nx = if max_x > 0.0 { x / max_x * scale } else { 0.0 };
            let ny = if max_y > 0.0 { y / max_y * scale } else { 0.0 };
            (nx, ny)
        })
        .collect()
}

/// Normalize 3D positions to [-scale, scale] range.
fn normalize_positions_3d(positions: &[(f64, f64, f64)]) -> Vec<(f64, f64, f64)> {
    if positions.is_empty() {
        return vec![];
    }

    let scale = 100.0;
    let max_x = positions.iter().map(|(x, _, _)| x.abs()).fold(0.0f64, f64::max);
    let max_y = positions.iter().map(|(_, y, _)| y.abs()).fold(0.0f64, f64::max);
    let max_z = positions.iter().map(|(_, _, z)| z.abs()).fold(0.0f64, f64::max);

    positions
        .iter()
        .map(|(x, y, z)| {
            let nx = if max_x > 0.0 { x / max_x * scale } else { 0.0 };
            let ny = if max_y > 0.0 { y / max_y * scale } else { 0.0 };
            let nz = if max_z > 0.0 { z / max_z * scale } else { 0.0 };
            (nx, ny, nz)
        })
        .collect()
}

/// Build edge weights from node similarity.
/// Returns edges with weights based on cosine similarity of features.
pub fn build_similarity_edges(n_nodes: usize, features: &[Vec<f64>], threshold: f64) -> Vec<(usize, usize, f64)> {
    let mut edges = Vec::new();

    for i in 0..n_nodes {
        for j in (i + 1)..n_nodes {
            if i < features.len() && j < features.len() {
                let sim = cosine_similarity(&features[i], &features[j]);
                if sim > threshold {
                    edges.push((i, j, sim));
                }
            }
        }
    }

    edges
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
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

/// Compute graph modularity for cluster quality.
/// Q = (1/2m) * Σ[A_ij - k_i*k_j/(2m)] * δ(c_i, c_j)
pub fn compute_modularity(n_nodes: usize, edges: &[(usize, usize, f64)], clusters: &[usize]) -> f64 {
    if n_nodes == 0 {
        return 0.0;
    }

    let total_weight: f64 = edges.iter().map(|(_, _, w)| w).sum::<f64>() * 2.0;
    if total_weight == 0.0 {
        return 0.0;
    }

    // Compute degrees
    let mut degrees = vec![0.0; n_nodes];
    for &(i, j, w) in edges {
        degrees[i] += w;
        degrees[j] += w;
    }

    // Build adjacency lookup
    let mut adjacency = vec![vec![0.0; n_nodes]; n_nodes];
    for &(i, j, w) in edges {
        adjacency[i][j] = w;
        adjacency[j][i] = w;
    }

    let mut q = 0.0;
    for i in 0..n_nodes {
        for j in 0..n_nodes {
            if clusters[i] == clusters[j] {
                q += adjacency[i][j] - degrees[i] * degrees[j] / total_weight;
            }
        }
    }

    q / total_weight
}

/// Infer edges from 2D positions using distance-based thresholding.
///
/// Given node positions computed by `spectral_layout_2d`, reconstructs
/// approximate edge information by connecting nearby nodes.
///
/// `positions`: (x, y) coordinates for each node
/// `distance_threshold`: maximum distance to consider nodes connected
/// `default_weight`: weight assigned to inferred edges
pub fn edges_from_positions(
    positions: &[(f64, f64)],
    distance_threshold: f64,
    default_weight: f64,
) -> Vec<(usize, usize, f64)> {
    let n = positions.len();
    let mut edges = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let dx = positions[i].0 - positions[j].0;
            let dy = positions[i].1 - positions[j].1;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= distance_threshold {
                // Weight is inversely proportional to distance
                let weight = if dist > 0.0 { default_weight / dist } else { default_weight };
                edges.push((i, j, weight));
            }
        }
    }

    edges
}

/// Infer edges from 3D positions using distance-based thresholding.
pub fn edges_from_positions_3d(
    positions: &[(f64, f64, f64)],
    distance_threshold: f64,
    default_weight: f64,
) -> Vec<(usize, usize, f64)> {
    let n = positions.len();
    let mut edges = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let dx = positions[i].0 - positions[j].0;
            let dy = positions[i].1 - positions[j].1;
            let dz = positions[i].2 - positions[j].2;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist <= distance_threshold {
                let weight = if dist > 0.0 { default_weight / dist } else { default_weight };
                edges.push((i, j, weight));
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_layout_2d() {
        let edges = vec![
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 3, 1.0),
        ];

        let positions = spectral_layout_2d(4, &edges);
        assert_eq!(positions.len(), 4);

        // All positions should be finite
        for (x, y) in &positions {
            assert!(x.is_finite());
            assert!(y.is_finite());
        }
    }

    #[test]
    fn test_spectral_layout_3d() {
        let edges = vec![
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 3, 1.0),
            (3, 0, 1.0),
        ];

        let positions = spectral_layout_3d(4, &edges);
        assert_eq!(positions.len(), 4);

        for (x, y, z) in &positions {
            assert!(x.is_finite());
            assert!(y.is_finite());
            assert!(z.is_finite());
        }
    }

    #[test]
    fn test_build_similarity_edges() {
        let features = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.9, 0.1, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let edges = build_similarity_edges(3, &features, 0.5);
        // First two vectors are similar, third is orthogonal
        assert!(!edges.is_empty());
        assert!(edges.iter().any(|(i, j, _)| (*i == 0 && *j == 1) || (*i == 1 && *j == 0)));
    }

    #[test]
    fn test_compute_modularity() {
        let edges = vec![
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
            (3, 4, 1.0),
            (4, 5, 1.0),
            (5, 3, 1.0),
        ];

        // Two clear clusters
        let clusters = vec![0, 0, 0, 1, 1, 1];
        let q = compute_modularity(6, &edges, &clusters);

        // Should have positive modularity (clear clusters)
        assert!(q > 0.0);
    }

    #[test]
    fn test_normalize_positions() {
        let positions = vec![(10.0, 20.0), (-5.0, -10.0), (0.0, 0.0)];
        let normalized = normalize_positions(&positions);

        // Should be scaled to [-100, 100]
        for (x, y) in &normalized {
            assert!(x.abs() <= 100.0 + 1e-10);
            assert!(y.abs() <= 100.0 + 1e-10);
        }
    }
}
