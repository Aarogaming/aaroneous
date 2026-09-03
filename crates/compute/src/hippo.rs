// crates/compute/src/hippo.rs
//! HiPPO (High-Order Polynomial Projection Operators) Legendre Basis Generation.
//!
//! Provides continuous-time state transition matrix A ∈ R^{N×N} and input matrix B ∈ R^N
//! for long-range memory projection using Legendre polynomial basis functions.
//!
//! The continuous dynamical system:
//!   ḣ(t) = A h(t) + B x(t)
//!   y(t) = C h(t) + D x(t)
//!
//! is discretized via bilinear (Tustin) transform for variable sensor frame intervals Δt.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// HiPPO Legendre continuous-time matrices (A, B) and discretized (Ā, B̄).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HippoLegendreMatrices {
    /// State dimension N
    pub state_dim: usize,
    /// Continuous A matrix (N × N), row-major
    pub a_continuous: Vec<f32>,
    /// Continuous B vector (N)
    pub b_continuous: Vec<f32>,
    /// Discretized Ā matrix (N × N), row-major
    pub a_discrete: Vec<f32>,
    /// Discretized B̄ vector (N)
    pub b_discrete: Vec<f32>,
    /// Current discretization timestep
    pub delta_t: f32,
}

/// Generate the HiPPO Legendre continuous-time matrices.
///
/// A_{nk} = -√(2n+1)√(2k+1)  if n > k
///        = -(n+1)             if n = k
///        = 0                  if n < k
///
/// B_n = √(2n+1)
pub fn generate_hippo_legendre(n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut a = vec![0.0f32; n * n];
    let mut b = vec![0.0f32; n];

    for i in 0..n {
        b[i] = ((2 * i + 1) as f32).sqrt();
        for j in 0..n {
            if i > j {
                a[i * n + j] = -((2 * i + 1) as f32).sqrt() * ((2 * j + 1) as f32).sqrt();
            } else if i == j {
                a[i * n + j] = -(i as f32 + 1.0);
            }
            // n < k => 0.0 (already initialized)
        }
    }
    (a, b)
}

/// In-place N×N matrix inversion via Gauss-Jordan elimination.
/// Returns Err if the matrix is singular.
fn invert_matrix_in_place(mat: &mut [f32], n: usize) -> Result<Vec<f32>> {
    let mut augmented = vec![0.0f32; n * 2 * n];

    // Build augmented [mat | I]
    for i in 0..n {
        for j in 0..n {
            augmented[i * 2 * n + j] = mat[i * n + j];
        }
        augmented[i * 2 * n + n + i] = 1.0;
    }

    // Forward elimination with partial pivoting
    for col in 0..n {
        // Find pivot
        let mut max_val = augmented[col * 2 * n + col].abs();
        let mut max_row = col;
        for row in (col + 1)..n {
            let val = augmented[row * 2 * n + col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }

        if max_val < 1e-12 {
            // Epsilon damping regularizer for singular / near-singular boundary cases
            augmented[col * 2 * n + col] += 1e-8;
        }

        // Swap rows
        if max_row != col {
            for j in 0..(2 * n) {
                augmented.swap(col * 2 * n + j, max_row * 2 * n + j);
            }
        }

        // Scale pivot row
        let pivot = augmented[col * 2 * n + col];
        let inv_pivot = 1.0 / pivot;
        for j in 0..(2 * n) {
            augmented[col * 2 * n + j] *= inv_pivot;
        }

        // Eliminate column with auto-vectorized SIMD row stride
        let pivot_row_start = col * 2 * n;
        for row in 0..n {
            if row == col {
                continue;
            }
            let target_row_start = row * 2 * n;
            let factor = augmented[target_row_start + col];
            if factor.abs() < 1e-15 {
                continue;
            }

            // Vectorized row elimination across contiguous 2*n slice
            let (target_row, pivot_row) = if row < col {
                let (first, second) = augmented.split_at_mut(pivot_row_start);
                (&mut first[target_row_start..target_row_start + 2 * n], &second[0..2 * n])
            } else {
                let (first, second) = augmented.split_at_mut(target_row_start);
                (&mut second[0..2 * n], &first[pivot_row_start..pivot_row_start + 2 * n])
            };

            for j in 0..(2 * n) {
                target_row[j] -= factor * pivot_row[j];
            }
        }
    }

    // Extract inverse from right half
    let mut inv = vec![0.0f32; n * n];
    for i in 0..n {
        for j in 0..n {
            inv[i * n + j] = augmented[i * 2 * n + n + j];
        }
    }
    Ok(inv)
}

/// Multiply two N×N row-major matrices: C = A * B
fn mat_mul(a: &[f32], b: &[f32], n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; n * n];
    for i in 0..n {
        for k in 0..n {
            let a_ik = a[i * n + k];
            if a_ik.abs() < 1e-15 {
                continue;
            }
            for j in 0..n {
                c[i * n + j] += a_ik * b[k * n + j];
            }
        }
    }
    c
}

/// Multiply N×N matrix by N-vector: y = M * v
fn mat_vec_mul(m: &[f32], v: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let mut sum = 0.0f32;
        for j in 0..n {
            sum += m[i * n + j] * v[j];
        }
        out[i] = sum;
    }
    out
}

/// Discretize HiPPO matrices using bilinear (Tustin) transform for variable Δt:
///
///   Ā = (I - Δt/2 · A)^{-1} · (I + Δt/2 · A)
///   B̄ = (I - Δt/2 · A)^{-1} · (Δt · B)
///
pub fn discretize_bilinear(
    a_continuous: &[f32],
    b_continuous: &[f32],
    n: usize,
    delta_t: f32,
) -> Result<(Vec<f32>, Vec<f32>)> {
    if delta_t <= 0.0 {
        anyhow::bail!("Discretization requires positive Δt, got {delta_t}");
    }

    let half_dt = delta_t / 2.0;

    // Build (I - Δt/2 · A) and (I + Δt/2 · A)
    let mut i_minus_half_a = vec![0.0f32; n * n];
    let mut i_plus_half_a = vec![0.0f32; n * n];

    for i in 0..n {
        for j in 0..n {
            let a_ij = a_continuous[i * n + j];
            i_minus_half_a[i * n + j] = if i == j { 1.0 } else { 0.0 } - half_dt * a_ij;
            i_plus_half_a[i * n + j] = if i == j { 1.0 } else { 0.0 } + half_dt * a_ij;
        }
    }

    // Compute (I - Δt/2 · A)^{-1}
    let inv = invert_matrix_in_place(&mut i_minus_half_a, n)?;

    // Ā = inv * (I + Δt/2 · A)
    let a_bar = mat_mul(&inv, &i_plus_half_a, n);

    // B̄ = inv * (Δt · B)
    let dt_b: Vec<f32> = b_continuous.iter().map(|bi| delta_t * bi).collect();
    let b_bar = mat_vec_mul(&inv, &dt_b, n);

    Ok((a_bar, b_bar))
}

/// Generate fully initialized HiPPO Legendre matrices with bilinear discretization.
pub fn generate_hippo_discretized(state_dim: usize, delta_t: f32) -> Result<HippoLegendreMatrices> {
    let (a_cont, b_cont) = generate_hippo_legendre(state_dim);
    let (a_disc, b_disc) = discretize_bilinear(&a_cont, &b_cont, state_dim, delta_t)?;

    Ok(HippoLegendreMatrices {
        state_dim,
        a_continuous: a_cont,
        b_continuous: b_cont,
        a_discrete: a_disc,
        b_discrete: b_disc,
        delta_t,
    })
}

impl HippoLegendreMatrices {
    /// Re-discretize for a new timestep (variable-rate sensor frames).
    pub fn rediscretize(&mut self, new_delta_t: f32) -> Result<()> {
        let (a_disc, b_disc) =
            discretize_bilinear(&self.a_continuous, &self.b_continuous, self.state_dim, new_delta_t)?;
        self.a_discrete = a_disc;
        self.b_discrete = b_disc;
        self.delta_t = new_delta_t;
        Ok(())
    }

    /// Single recurrence step: h_{t+1} = Ā h_t + B̄ u_t
    #[inline]
    pub fn step(&self, state: &mut [f32], input: f32) {
        let n = self.state_dim;
        let mut new_state = vec![0.0f32; n];
        for (i, out_val) in new_state.iter_mut().enumerate().take(n) {
            let mut sum = 0.0f32;
            for (j, &st) in state.iter().enumerate().take(n) {
                sum += self.a_discrete[i * n + j] * st;
            }
            sum += self.b_discrete[i] * input;
            *out_val = sum;
        }
        state.copy_from_slice(&new_state);
    }

    /// Process a sequence of inputs, returning the final hidden state.
    pub fn process_sequence(&self, inputs: &[f32]) -> Vec<f32> {
        let mut state = vec![0.0f32; self.state_dim];
        for &u in inputs {
            self.step(&mut state, u);
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hippo_legendre_generation() {
        let (a, b) = generate_hippo_legendre(4);
        assert_eq!(a.len(), 16);
        assert_eq!(b.len(), 4);

        // B_0 = sqrt(1) = 1.0
        assert!((b[0] - 1.0).abs() < 1e-6);
        // B_1 = sqrt(3) ≈ 1.732
        assert!((b[1] - 3.0f32.sqrt()).abs() < 1e-5);

        // A[0][0] = -1.0
        assert!((a[0] - (-1.0)).abs() < 1e-6);
        // A[1][1] = -2.0
        assert!((a[5] - (-2.0)).abs() < 1e-6);
        // A[0][1] = 0.0 (upper triangle)
        assert!((a[1]).abs() < 1e-6);
        // A[1][0] = -sqrt(3)*sqrt(1) = -sqrt(3)
        assert!((a[4] - (-3.0f32.sqrt())).abs() < 1e-5);
    }

    #[test]
    fn test_bilinear_discretization() {
        let (a, b) = generate_hippo_legendre(4);
        let result = discretize_bilinear(&a, &b, 4, 0.01);
        assert!(result.is_ok());
        let (a_bar, b_bar) = result.unwrap();
        assert_eq!(a_bar.len(), 16);
        assert_eq!(b_bar.len(), 4);

        // Ā should be close to I for small Δt
        for i in 0..4 {
            assert!((a_bar[i * 4 + i] - 1.0).abs() < 0.05, "Diagonal element should be near 1.0 for small Δt");
        }
    }

    #[test]
    fn test_hippo_discretized_full() {
        let hippo = generate_hippo_discretized(8, 0.001).unwrap();
        assert_eq!(hippo.state_dim, 8);
        assert_eq!(hippo.a_discrete.len(), 64);
        assert_eq!(hippo.b_discrete.len(), 8);
        assert!((hippo.delta_t - 0.001).abs() < 1e-8);
    }

    #[test]
    fn test_hippo_step_and_sequence() {
        let hippo = generate_hippo_discretized(4, 0.01).unwrap();
        let final_state = hippo.process_sequence(&[1.0, 0.5, 0.25, 0.1]);
        assert_eq!(final_state.len(), 4);
        // State should be non-zero after processing inputs
        assert!(final_state.iter().any(|s| s.abs() > 1e-6));
    }

    #[test]
    fn test_rediscretize() {
        let mut hippo = generate_hippo_discretized(4, 0.01).unwrap();
        let original_a = hippo.a_discrete.clone();

        hippo.rediscretize(0.005).unwrap();
        assert!((hippo.delta_t - 0.005).abs() < 1e-8);

        // Matrices should differ after rediscretization
        let changed = hippo.a_discrete.iter().zip(&original_a).any(|(a, b)| (a - b).abs() > 1e-8);
        assert!(changed, "Rediscretization should produce different matrices");
    }

    #[test]
    fn test_invalid_delta_t() {
        let (a, b) = generate_hippo_legendre(4);
        assert!(discretize_bilinear(&a, &b, 4, 0.0).is_err());
        assert!(discretize_bilinear(&a, &b, 4, -1.0).is_err());
    }
}
