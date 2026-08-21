/// Linear Algebra & Tensor primitives.
/// Used for embedding similarity, vector search, and semantic memory.
/// Compute cosine similarity between two halves of input vector
pub fn cosine_similarity(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    if input.len() < 2 {
        return Ok(vec![0.0]);
    }
    let mid = input.len() / 2;
    let (a, b) = input.split_at(mid);
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    let norm_b = b.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    let sim = if norm_a * norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    };
    Ok(vec![sim])
}

/// Matrix-vector multiplication (flattened row-major)
pub fn mat_vec_mul(matrix: &[f64], vec: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut result = vec![0.0; rows];
    for i in 0..rows {
        for j in 0..cols {
            result[i] += matrix[i * cols + j] * vec[j];
        }
    }
    result
}
