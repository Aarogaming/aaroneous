//! crates/compute/src/linalg.rs
//! Linear Algebra & Tensor primitives.
//! Used for embedding similarity, vector search, dimensional projection, and semantic memory.

use anyhow::{bail, Result};

/// Compute dot product between two vectors of equal dimension.
pub fn dot_product(a: &[f64], b: &[f64]) -> Result<f64> {
    if a.len() != b.len() {
        bail!("Vector length mismatch for dot product: {} vs {}", a.len(), b.len());
    }
    Ok(a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum())
}

/// Computes L1 (Manhattan) norm of a vector.
pub fn vec_norm_l1(v: &[f64]) -> f64 {
    v.iter().map(|x| x.abs()).sum()
}

/// Computes L2 (Euclidean) norm of a vector.
pub fn vec_norm_l2(v: &[f64]) -> f64 {
    v.iter().map(|x| x.powi(2)).sum::<f64>().sqrt()
}

/// Computes L-infinity (Chebyshev / Maximum) norm of a vector.
pub fn vec_norm_linf(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |max, &x| max.max(x.abs()))
}

/// Computes vector addition (a + b).
pub fn vec_add(a: &[f64], b: &[f64]) -> Result<Vec<f64>> {
    if a.len() != b.len() {
        bail!("Vector length mismatch for addition: {} vs {}", a.len(), b.len());
    }
    Ok(a.iter().zip(b.iter()).map(|(&x, &y)| x + y).collect())
}

/// Computes vector subtraction (a - b).
pub fn vec_sub(a: &[f64], b: &[f64]) -> Result<Vec<f64>> {
    if a.len() != b.len() {
        bail!("Vector length mismatch for subtraction: {} vs {}", a.len(), b.len());
    }
    Ok(a.iter().zip(b.iter()).map(|(&x, &y)| x - y).collect())
}

/// Multiplies a vector by a scalar.
pub fn vec_scale(v: &[f64], scalar: f64) -> Vec<f64> {
    v.iter().map(|&x| x * scalar).collect()
}

/// Compute cosine similarity between two distinct vectors.
pub fn cosine_similarity_vectors(a: &[f64], b: &[f64]) -> Result<f64> {
    if a.len() != b.len() {
        bail!("Vector dimension mismatch for cosine similarity: {} vs {}", a.len(), b.len());
    }
    let norm_a = vec_norm_l2(a);
    let norm_b = vec_norm_l2(b);
    if norm_a == 0.0 || norm_b == 0.0 {
        return Ok(0.0);
    }
    let dot = dot_product(a, b)?;
    Ok(dot / (norm_a * norm_b))
}

/// Compute cosine similarity between two halves of an input vector (backwards-compatible).
pub fn cosine_similarity(input: &[f64]) -> Result<Vec<f64>> {
    if input.len() < 2 {
        return Ok(vec![0.0]);
    }
    let mid = input.len() / 2;
    let (a, b) = input.split_at(mid);
    let sim = cosine_similarity_vectors(a, b)?;
    Ok(vec![sim])
}

/// Matrix-vector multiplication (flattened row-major).
pub fn mat_vec_mul(matrix: &[f64], vec: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    if matrix.len() < rows * cols || vec.len() < cols {
        return vec![0.0; rows];
    }
    let mut result = vec![0.0; rows];
    for i in 0..rows {
        for j in 0..cols {
            result[i] += matrix[i * cols + j] * vec[j];
        }
    }
    result
}

/// Matrix-matrix multiplication (A: m x k, B: k x n -> C: m x n, flattened row-major).
pub fn mat_mul(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Result<Vec<f64>> {
    if a.len() < m * k {
        bail!("Matrix A size smaller than required dimensions {}x{}", m, k);
    }
    if b.len() < k * n {
        bail!("Matrix B size smaller than required dimensions {}x{}", k, n);
    }

    let mut c = vec![0.0; m * n];
    for i in 0..m {
        for p in 0..k {
            let a_val = a[i * k + p];
            for j in 0..n {
                c[i * n + j] += a_val * b[p * n + j];
            }
        }
    }
    Ok(c)
}

/// Multi-threaded chunked matrix-matrix multiplication for larger dimension matrices.
/// Spawns worker threads across row chunks using standard Rust scoped threads.
pub fn mat_mul_parallel(a: &[f64], b: &[f64], m: usize, k: usize, n: usize, num_threads: usize) -> Result<Vec<f64>> {
    if a.len() < m * k {
        bail!("Matrix A size smaller than required dimensions {}x{}", m, k);
    }
    if b.len() < k * n {
        bail!("Matrix B size smaller than required dimensions {}x{}", k, n);
    }

    if m == 0 || n == 0 || k == 0 {
        return Ok(Vec::new());
    }

    // For very small matrices, fallback directly to serial without thread spawn overhead
    if m < 16 || num_threads <= 1 {
        return mat_mul(a, b, m, k, n);
    }

    let mut c = vec![0.0; m * n];
    let actual_threads = num_threads.min(m).max(1);
    let chunk_rows = m.div_ceil(actual_threads);

    std::thread::scope(|s| {
        let mut row_start = 0;
        let mut chunks = Vec::with_capacity(actual_threads);

        let mut remainder = &mut c[..];
        while row_start < m && !remainder.is_empty() {
            let current_chunk_rows = chunk_rows.min(m - row_start);
            let chunk_len = current_chunk_rows * n;
            let (chunk, rest) = remainder.split_at_mut(chunk_len);
            remainder = rest;

            chunks.push((row_start, current_chunk_rows, chunk));
            row_start += current_chunk_rows;
        }

        for (start_r, count_r, out_slice) in chunks {
            s.spawn(move || {
                for r in 0..count_r {
                    let global_row = start_r + r;
                    for p in 0..k {
                        let a_val = a[global_row * k + p];
                        for j in 0..n {
                            out_slice[r * n + j] += a_val * b[p * n + j];
                        }
                    }
                }
            });
        }
    });

    Ok(c)
}

/// Transposes a matrix of dimension rows x cols.
pub fn mat_transpose(matrix: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut transposed = vec![0.0; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            transposed[j * rows + i] = matrix[i * cols + j];
        }
    }
    transposed
}

/// Computes outer product of two vectors (a of len m, b of len n -> m x n matrix).
pub fn vec_outer_product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut mat = Vec::with_capacity(a.len() * b.len());
    for &x in a {
        for &y in b {
            mat.push(x * y);
        }
    }
    mat
}

/// Numerically stable softmax with max subtraction.
pub fn softmax(vec: &[f64]) -> Vec<f64> {
    if vec.is_empty() {
        return vec![];
    }
    let max_val = vec.iter().fold(f64::NEG_INFINITY, |max, &x| max.max(x));
    let exps: Vec<f64> = vec.iter().map(|&x| (x - max_val).exp()).collect();
    let sum_exps: f64 = exps.iter().sum();
    if sum_exps == 0.0 {
        return vec![1.0 / vec.len() as f64; vec.len()];
    }
    exps.into_iter().map(|x| x / sum_exps).collect()
}

/// Projects vector `u` onto vector `v` (proj_v(u) = (u . v / v . v) * v).
pub fn project_vector(u: &[f64], v: &[f64]) -> Result<Vec<f64>> {
    let v_dot_v = dot_product(v, v)?;
    if v_dot_v == 0.0 {
        bail!("Cannot project onto zero vector");
    }
    let u_dot_v = dot_product(u, v)?;
    let scale = u_dot_v / v_dot_v;
    Ok(vec_scale(v, scale))
}

/// Gram-Schmidt orthonormalization for a set of linearly independent vectors.
pub fn gram_schmidt_orthogonalize(vectors: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(vectors.len());
    for v in vectors {
        let mut u = v.clone();
        for b in &basis {
            let proj = project_vector(v, b)?;
            u = vec_sub(&u, &proj)?;
        }
        let norm = vec_norm_l2(&u);
        if norm > 1e-12 {
            basis.push(vec_scale(&u, 1.0 / norm));
        }
    }
    Ok(basis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_product_and_norms() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let dot = dot_product(&a, &b).unwrap();
        assert_eq!(dot, 32.0);

        assert_eq!(vec_norm_l1(&[-3.0, 4.0]), 7.0);
        assert_eq!(vec_norm_l2(&[3.0, 4.0]), 5.0);
        assert_eq!(vec_norm_linf(&[-10.0, 5.0, 2.0]), 10.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = [1.0, 0.0];
        let b = [1.0, 0.0];
        assert_eq!(cosine_similarity_vectors(&a, &b).unwrap(), 1.0);

        let orthogonal = [0.0, 1.0];
        assert_eq!(cosine_similarity_vectors(&a, &orthogonal).unwrap(), 0.0);

        let halves = [1.0, 0.0, 1.0, 0.0];
        let res = cosine_similarity(&halves).unwrap();
        assert_eq!(res, vec![1.0]);
    }

    #[test]
    fn test_matrix_multiplication() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [2.0, 0.0, 1.0, 2.0];
        let c = mat_mul(&a, &b, 2, 2, 2).unwrap();
        assert_eq!(c, vec![4.0, 4.0, 10.0, 8.0]);
    }

    #[test]
    fn test_matrix_multiplication_parallel() {
        // Construct 32x32 identity matrices
        let n = 32;
        let mut a = vec![0.0; n * n];
        let mut b = vec![0.0; n * n];
        for i in 0..n {
            a[i * n + i] = 2.0;
            b[i * n + i] = 3.0;
        }

        let serial = mat_mul(&a, &b, n, n, n).unwrap();
        let parallel = mat_mul_parallel(&a, &b, n, n, n, 4).unwrap();
        assert_eq!(serial, parallel);
        assert_eq!(parallel[0], 6.0);
        assert_eq!(parallel[1], 0.0);
    }

    #[test]
    fn test_softmax_stability() {
        let input = [1000.0, 1001.0, 1002.0];
        let sm = softmax(&input);
        let sum: f64 = sm.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(sm[2] > sm[1] && sm[1] > sm[0]);
    }

    #[test]
    fn test_gram_schmidt() {
        let v1 = vec![3.0, 1.0];
        let v2 = vec![2.0, 2.0];
        let ortho = gram_schmidt_orthogonalize(&[v1, v2]).unwrap();
        assert_eq!(ortho.len(), 2);
        let dot = dot_product(&ortho[0], &ortho[1]).unwrap();
        assert!(dot.abs() < 1e-10);
    }
}
