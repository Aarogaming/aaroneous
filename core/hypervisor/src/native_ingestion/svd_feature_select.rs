/// Configuration for truncated SVD feature selection.
pub struct SvdConfig {
    /// Number of singular values to retain (target rank).
    pub target_rank: usize,
    /// Maximum Lanczos iterations.
    pub max_iterations: usize,
    /// Convergence tolerance for Rayleigh quotient.
    pub tolerance: f32,
}

impl Default for SvdConfig {
    fn default() -> Self {
        Self {
            target_rank: 16,
            max_iterations: 64,
            tolerance: 1e-5,
        }
    }
}

/// Compressed feature representation after SVD reduction.
#[repr(C)]
pub struct ReducedFeatures {
    /// Dominant singular values (sigma), length = target_rank.
    pub singular_values: Vec<f32>,
    /// Left singular vectors (U), flattened row-major.
    /// Shape: [rows * target_rank].
    pub u_matrix: Vec<f32>,
    /// Right singular vectors (V^T), flattened row-major.
    /// Shape: [target_rank * cols].
    pub vt_matrix: Vec<f32>,
    /// Original matrix dimensions.
    pub rows: usize,
    pub cols: usize,
    /// Rank actually achieved.
    pub achieved_rank: usize,
}

impl ReducedFeatures {
    /// Reconstruct the approximate matrix from SVD factors.
    ///
    /// Computes A_approx = U * diag(Sigma) * V^T
    /// Returns a flattened row-major matrix of shape [rows * cols].
    pub fn reconstruct(&self) -> Vec<f32> {
        let mut result = vec![0.0f32; self.rows * self.cols];
        for i in 0..self.rows {
            for j in 0..self.cols {
                let mut sum = 0.0f32;
                for k in 0..self.achieved_rank {
                    sum += self.u_matrix[i * self.achieved_rank + k]
                        * self.singular_values[k]
                        * self.vt_matrix[k * self.cols + j];
                }
                result[i * self.cols + j] = sum;
            }
        }
        result
    }

    /// Compute relative reconstruction error: ||A - A_approx||_F / ||A||_F
    ///
    /// `original` is the original flattened matrix (row-major, rows * cols).
    pub fn reconstruction_error(&self, original: &[f32]) -> f32 {
        let recon = self.reconstruct();
        let mut err_sq = 0.0f32;
        let mut norm_sq = 0.0f32;
        for idx in 0..original.len().min(recon.len()) {
            let diff = original[idx] - recon[idx];
            err_sq += diff * diff;
            norm_sq += original[idx] * original[idx];
        }
        if norm_sq > 0.0 {
            (err_sq / norm_sq).sqrt()
        } else {
            0.0
        }
    }
}

/// Truncated SVD via randomized power iteration.
///
/// Decomposes an M×N float matrix A into:
///   A ≈ U * Σ * V^T
///
/// where:
///   - U is M×k, columns are left singular vectors
///   - Σ is k×k diagonal of singular values
///   - V^T is k×N, rows are right singular vectors
///   - k = target_rank (≪ min(M, N))
///
/// Uses a randomized range-finder (Halko–Martinsson–Tropp) for
/// near-optimal rank-k approximation with O(MN·k) complexity.
pub struct SvdReducer {
    config: SvdConfig,
    rng_state: u64,
}

impl SvdReducer {
    pub fn new(config: SvdConfig) -> Self {
        Self {
            config,
            rng_state: 0x9E3779B97F4A7C15u64,
        }
    }

    /// Compute truncated SVD of a flattened row-major matrix.
    ///
    /// `matrix` must have length `rows * cols`.
    pub fn reduce(&mut self, matrix: &[f32], rows: usize, cols: usize) -> ReducedFeatures {
        let k = self.config.target_rank.min(rows).min(cols);
        if k == 0 {
            return ReducedFeatures {
                singular_values: vec![],
                u_matrix: vec![],
                vt_matrix: vec![],
                rows,
                cols,
                achieved_rank: 0,
            };
        }

        // 1. Random projection: Y = A * Omega    (Omega is cols × k Gaussian)
        let omega = self.gaussian_matrix(cols, k);
        let mut y = vec![0.0f32; rows * k];
        mat_mul(matrix, &omega, &mut y, rows, cols, k, false, false);

        // 2. QR factorization of Y to get Q (rows × k)
        let (q, _) = householder_qr(&mut y, rows, k);

        // 3. B = Q^T * A   (k × cols)
        let mut b = vec![0.0f32; k * cols];
        mat_mul(&q, matrix, &mut b, k, rows, cols, true, false);

        // 4. SVD of small B (k × cols) via power iteration
        let (u_small, sigma, vt) = self.power_iteration_svd(&b, k, cols);

        // 5. U = Q * U_small
        let mut u = vec![0.0f32; rows * k];
        mat_mul(&q, &u_small, &mut u, rows, k, k, false, false);

        let achieved = sigma.iter().filter(|&&s| s > 1e-8).count();

        ReducedFeatures {
            singular_values: sigma,
            u_matrix: u,
            vt_matrix: vt,
            rows,
            cols,
            achieved_rank: achieved,
        }
    }

    /// Randomized power-iteration SVD for a small k×cols matrix.
    fn power_iteration_svd(
        &mut self,
        b: &[f32],
        k: usize,
        cols: usize,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let mut u = vec![0.0f32; k * k];
        let mut sigma = vec![0.0f32; k];
        let mut vt = vec![0.0f32; k * cols];

        // Start with random k×k matrix
        let mut rnd = self.gaussian_matrix(k, k);
        for _iter in 0..self.config.max_iterations {
            let mut bt = vec![0.0f32; cols * k];
            transpose(b, &mut bt, k, cols);

            let mut temp = vec![0.0f32; k * k];
            mat_mul(b, &bt, &mut temp, k, cols, k, false, false);
            let mut temp2 = vec![0.0f32; k * k];
            mat_mul(&temp, &rnd, &mut temp2, k, k, k, false, true);
            // QR each iteration for numerical stability
            let (q, _r_mat) = householder_qr(&mut temp2, k, k);
            rnd = q;

            // Check convergence via Rayleigh quotient
            let mut rq_prev = 0.0f32;
            let mut converged = true;
            for i in 0..k.min(4) {
                let col = i * k;
                let num = dot(&rnd[col..col + k], &temp[col..col + k]);
                let den = dot(&rnd[col..col + k], &rnd[col..col + k]);
                let rq = if den > 1e-12 { num / den } else { 0.0 };
                if (rq - rq_prev).abs() > self.config.tolerance {
                    converged = false;
                }
                rq_prev = rq;
            }
            if converged {
                break;
            }
        }

        // Extract singular vectors via eigendecomposition of R = U^T * B * V
        // Simplified: take columns of rnd as approximate eigenvectors of B*B^T
        u.copy_from_slice(&rnd);

        // Σ = diagonal of U^T * B
        let mut ub = vec![0.0f32; k * cols];
        mat_mul(&u, b, &mut ub, k, k, cols, true, false);

        for i in 0..k {
            let row_start = i * cols;
            let norm = euclidean_norm(&ub[row_start..row_start + cols]);
            sigma[i] = norm;
            if norm > 1e-12 {
                for j in 0..cols {
                    vt[i * cols + j] = ub[row_start + j] / norm;
                }
            }
        }

        (u, sigma, vt)
    }

    /// Generate a Gaussian random matrix (mean=0, std=1) using the
    /// Box–Muller transform with a split-mix64 generator.
    fn gaussian_matrix(&mut self, rows: usize, cols: usize) -> Vec<f32> {
        let mut mat = vec![0.0f32; rows * cols];
        for v in mat.iter_mut() {
            let u1 = self.next_f32();
            let u2 = self.next_f32();
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;
            *v = r * theta.cos();
        }
        mat
    }

    fn next_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(0x9E3779B97F4A7C15);
        let x = self.rng_state;
        let f = (x >> 11) as f32 * (1.0 / (1u64 << 53) as f32);
        f.clamp(0.0, 1.0 - f32::EPSILON)
    }
}

// ── BLAS-like primitives (allocation-free, single-threaded) ─────────────

/// C = alpha * A * B + beta * C, where A is M×K, B is K×N.
/// Supports transposition via boolean flags.
#[allow(clippy::too_many_arguments)]
fn mat_mul(
    a: &[f32],
    b: &[f32],
    c: &mut [f32],
    m: usize,
    k: usize,
    n: usize,
    trans_a: bool,
    trans_b: bool,
) {
    let _a_rows = if trans_a { k } else { m };
    let a_cols = if trans_a { m } else { k };
    let b_rows = if trans_b { n } else { k };
    let b_cols = if trans_b { k } else { n };

    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0f32;
            for t in 0..k {
                let av = if trans_a {
                    a[t * a_cols + i]
                } else {
                    a[i * a_cols + t]
                };
                let bv = if trans_b {
                    b[j * b_rows + t]
                } else {
                    b[t * b_cols + j]
                };
                sum += av * bv;
            }
            c[i * n + j] = sum;
        }
    }
}

/// Transpose a K×N matrix to N×K.
fn transpose(src: &[f32], dst: &mut [f32], k: usize, n: usize) {
    for i in 0..k {
        for j in 0..n {
            dst[j * k + i] = src[i * n + j];
        }
    }
}

/// In-place Householder QR decomposition of an M×N matrix.
/// Returns Q (M×min(M,N)) and R (min(M,N)×N).
fn householder_qr(matrix: &mut [f32], m: usize, n: usize) -> (Vec<f32>, Vec<f32>) {
    let rank = m.min(n);
    let mut q = vec![0.0f32; m * rank];
    let mut r = vec![0.0f32; rank * n];

    // Copy matrix into workspace (we'll extract R from Q^T * A later)
    // For simplicity, build Q via modified Gram-Schmidt
    let mut v = vec![0.0f32; m];

    for i in 0..rank {
        // Copy column i
        for row in 0..m {
            v[row] = matrix[row * n + i];
        }

        // Orthogonalize against previous Q columns
        for j in 0..i {
            let mut proj = 0.0f32;
            for row in 0..m {
                proj += q[row * rank + j] * v[row];
            }
            r[j * n + i] = proj;
            for row in 0..m {
                v[row] -= proj * q[row * rank + j];
            }
        }

        let norm = euclidean_norm(&v);
        r[i * n + i] = norm;
        if norm > 1e-12 {
            let inv = 1.0 / norm;
            for row in 0..m {
                q[row * rank + i] = v[row] * inv;
            }
        }
    }

    (q, r)
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn euclidean_norm(x: &[f32]) -> f32 {
    dot(x, x).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svd_identity_like() {
        let mut reducer = SvdReducer::new(SvdConfig {
            target_rank: 4,
            max_iterations: 32,
            tolerance: 1e-4,
        });
        // 8×8 diagonal matrix: diag(4.0, 3.0, 2.0, 1.0, 0, 0, ...)
        let rows = 8;
        let cols = 8;
        let mut mat = vec![0.0f32; rows * cols];
        for i in 0..4 {
            mat[i * cols + i] = (4 - i) as f32;
        }
        let result = reducer.reduce(&mat, rows, cols);
        assert_eq!(result.rows, rows);
        assert_eq!(result.cols, cols);
        assert!(result.achieved_rank >= 4);
        // Top singular values should be close to 4, 3, 2, 1
        assert!((result.singular_values[0] - 4.0).abs() < 0.3); // tolerance due to randomization
        assert!((result.singular_values[1] - 3.0).abs() < 0.3);
    }

    #[test]
    fn test_svd_skinny_matrix() {
        let mut reducer = SvdReducer::new(SvdConfig {
            target_rank: 2,
            max_iterations: 16,
            tolerance: 1e-3,
        });
        // 32×4 matrix (more rows than cols)
        let rows = 32;
        let cols = 4;
        let mut mat = vec![0.0f32; rows * cols];
        for i in 0..rows {
            for j in 0..cols {
                mat[i * cols + j] = ((i * cols + j) % 10) as f32;
            }
        }
        let result = reducer.reduce(&mat, rows, cols);
        assert_eq!(result.achieved_rank, 2);
        assert_eq!(result.singular_values.len(), 2);
        assert_eq!(result.u_matrix.len(), rows * 2);
        assert_eq!(result.vt_matrix.len(), 2 * cols);
    }

    #[test]
    fn test_reconstruction_error() {
        let mut reducer = SvdReducer::new(SvdConfig {
            target_rank: 8,
            max_iterations: 48,
            tolerance: 1e-4,
        });
        let rows = 16;
        let cols = 16;
        let mut mat = vec![0.0f32; rows * cols];
        for i in 0..rows {
            for j in 0..cols {
                mat[i * cols + j] = ((i * 7 + j * 13) % 20) as f32;
            }
        }
        let result = reducer.reduce(&mat, rows, cols);

        // Reconstruct A ≈ U * Σ * V^T
        let mut recon = vec![0.0f32; rows * cols];
        for i in 0..rows {
            for j in 0..cols {
                let mut sum = 0.0f32;
                for k in 0..result.achieved_rank {
                    sum += result.u_matrix[i * result.achieved_rank + k]
                        * result.singular_values[k]
                        * result.vt_matrix[k * cols + j];
                }
                recon[i * cols + j] = sum;
            }
        }

        // Compute relative error ||A - recon||_F / ||A||_F
        let mut err_sq = 0.0f32;
        let mut norm_sq = 0.0f32;
        for idx in 0..rows * cols {
            let diff = mat[idx] - recon[idx];
            err_sq += diff * diff;
            norm_sq += mat[idx] * mat[idx];
        }
        let rel_err = (err_sq / norm_sq).sqrt();
        assert!(rel_err < 0.5); // Should capture at least half the variance with rank 8
    }

    #[test]
    fn test_zero_matrix() {
        let mut reducer = SvdReducer::new(SvdConfig::default());
        let mat = vec![0.0f32; 64];
        let result = reducer.reduce(&mat, 8, 8);
        assert_eq!(result.achieved_rank, 0);
    }

    #[test]
    fn test_small_under_rank() {
        let mut reducer = SvdReducer::new(SvdConfig {
            target_rank: 16,
            max_iterations: 16,
            tolerance: 1e-3,
        });
        // 3×4 constant matrix — rank is 1 (all rows identical)
        let mat = vec![1.0f32; 12];
        let result = reducer.reduce(&mat, 3, 4);
        assert!(result.achieved_rank <= 2); // near-constant -> rank ≈ 1
    }
}
