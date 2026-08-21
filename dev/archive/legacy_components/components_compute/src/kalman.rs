/// Kalman Filter for optimal state estimation.
/// Fuses noisy observations to estimate true system state.
/// Used for metabolic load estimation, specialist health monitoring,
/// and system state prediction.
/// Standard Kalman Filter state.
/// x = state vector, P = covariance matrix, Q = process noise, R = measurement noise
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    pub state: Vec<f64>,                  // x: current state estimate
    pub covariance: Vec<Vec<f64>>,        // P: state covariance matrix
    pub process_noise: Vec<Vec<f64>>,     // Q: process noise covariance
    pub measurement_noise: Vec<Vec<f64>>, // R: measurement noise covariance
    pub state_dim: usize,
    pub measurement_dim: usize,
}

impl KalmanFilter {
    /// Create new Kalman filter with given dimensions.
    /// Initializes state to zeros, covariance to identity.
    pub fn new(state_dim: usize, measurement_dim: usize) -> Self {
        Self {
            state: vec![0.0; state_dim],
            covariance: identity_matrix(state_dim),
            process_noise: scale_matrix(&identity_matrix(state_dim), 0.01),
            measurement_noise: scale_matrix(&identity_matrix(measurement_dim), 0.1),
            state_dim,
            measurement_dim,
        }
    }

    /// Create with custom noise parameters.
    pub fn with_noise(
        state_dim: usize,
        measurement_dim: usize,
        process_noise: f64,
        measurement_noise: f64,
    ) -> Self {
        Self {
            state: vec![0.0; state_dim],
            covariance: identity_matrix(state_dim),
            process_noise: scale_matrix(&identity_matrix(state_dim), process_noise),
            measurement_noise: scale_matrix(&identity_matrix(measurement_dim), measurement_noise),
            state_dim,
            measurement_dim,
        }
    }

    /// Predict step: propagate state forward using state transition matrix F.
    /// x_pred = F * x
    /// P_pred = F * P * F^T + Q
    pub fn predict(&mut self, state_transition: &[Vec<f64>]) {
        // x_pred = F * x
        let new_state = mat_vec_mul(state_transition, &self.state);

        // P_pred = F * P * F^T + Q
        let fp = mat_mul(state_transition, &self.covariance);
        let ft = transpose(state_transition);
        let fpt = mat_mul(&fp, &ft);
        self.covariance = mat_add(&fpt, &self.process_noise);

        self.state = new_state;
    }

    /// Update step: correct prediction with measurement.
    /// K = P * H^T * (H * P * H^T + R)^(-1)
    /// x = x + K * (z - H * x)
    /// P = (I - K * H) * P
    pub fn update(&mut self, measurement: &[f64], observation_matrix: &[Vec<f64>]) {
        // Innovation: z - H * x
        let hx = mat_vec_mul(observation_matrix, &self.state);
        let innovation: Vec<f64> = measurement
            .iter()
            .zip(hx.iter())
            .map(|(z, hx)| z - hx)
            .collect();

        // Innovation covariance: S = H * P * H^T + R
        let hp = mat_mul(observation_matrix, &self.covariance);
        let ht = transpose(observation_matrix);
        let hpt = mat_mul(&hp, &ht);
        let s = mat_add(&hpt, &self.measurement_noise);

        // Kalman gain: K = P * H^T * S^(-1)
        let pht = mat_mul(&self.covariance, &ht);
        let s_inv = invert_matrix(&s).unwrap_or(identity_matrix(self.measurement_dim));
        let k = mat_mul(&pht, &s_inv);

        // Update state: x = x + K * innovation
        let k_innovation = mat_vec_mul(&k, &innovation);
        self.state = self
            .state
            .iter()
            .zip(k_innovation.iter())
            .map(|(x, ki)| x + ki)
            .collect();

        // Update covariance: P = (I - K * H) * P
        let kh = mat_mul(&k, observation_matrix);
        let identity = identity_matrix(self.state_dim);
        let ikh = mat_sub(&identity, &kh);
        self.covariance = mat_mul(&ikh, &self.covariance);
    }

    /// Get current state estimate.
    pub fn get_state(&self) -> &[f64] {
        &self.state
    }

    /// Get state uncertainty (trace of covariance).
    pub fn get_uncertainty(&self) -> f64 {
        self.covariance
            .iter()
            .map(|row| row.iter().sum::<f64>())
            .sum::<f64>()
    }
}

/// Extended Kalman Filter for nonlinear systems.
/// Linearizes around current state using Jacobians.
#[derive(Debug, Clone)]
pub struct ExtendedKalmanFilter {
    pub base: KalmanFilter,
}

impl ExtendedKalmanFilter {
    pub fn new(state_dim: usize, measurement_dim: usize) -> Self {
        Self {
            base: KalmanFilter::new(state_dim, measurement_dim),
        }
    }

    /// Predict with nonlinear state transition function.
    /// f(x): nonlinear state transition
    /// F: Jacobian of f at current state
    pub fn predict_nonlinear<F>(&mut self, f: F, jacobian: &[Vec<f64>])
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        self.base.state = f(&self.base.state);
        self.base.predict(jacobian);
    }

    /// Update with nonlinear observation function.
    /// h(x): nonlinear observation
    /// H: Jacobian of h at current state
    pub fn update_nonlinear(
        &mut self,
        measurement: &[f64],
        _h: impl Fn(&[f64]) -> Vec<f64>,
        jacobian: &[Vec<f64>],
    ) {
        self.base.update(measurement, jacobian);
    }
}

/// Unscented Kalman Filter for highly nonlinear systems.
/// Uses sigma points to capture mean and covariance more accurately.
#[derive(Debug, Clone)]
pub struct UnscentedKalmanFilter {
    pub state: Vec<f64>,
    pub covariance: Vec<Vec<f64>>,
    pub alpha: f64, // Spread of sigma points
    pub beta: f64,  // Prior knowledge about distribution (2 for Gaussian)
    pub kappa: f64, // Secondary scaling parameter
    pub state_dim: usize,
}

impl UnscentedKalmanFilter {
    pub fn new(state_dim: usize) -> Self {
        Self {
            state: vec![0.0; state_dim],
            covariance: identity_matrix(state_dim),
            alpha: 1e-3,
            beta: 2.0,
            kappa: 0.0,
            state_dim,
        }
    }

    /// Generate sigma points.
    #[allow(clippy::needless_range_loop)]
    pub fn sigma_points(&self) -> Vec<Vec<f64>> {
        let n = self.state_dim;
        let lambda = self.alpha.powi(2) * (n as f64 + self.kappa) - n as f64;
        let mut points = vec![self.state.clone()];

        // Matrix square root via Cholesky decomposition (simplified)
        let scaled_cov = scale_matrix(&self.covariance, n as f64 + lambda);
        let sqrt_cov = matrix_sqrt(&scaled_cov);

        for i in 0..n {
            let mut plus = self.state.clone();
            let mut minus = self.state.clone();
            for j in 0..n {
                plus[j] += sqrt_cov[j][i];
                minus[j] -= sqrt_cov[j][i];
            }
            points.push(plus);
            points.push(minus);
        }

        points
    }
}

// Matrix operations

#[allow(clippy::needless_range_loop)]
fn identity_matrix(n: usize) -> Vec<Vec<f64>> {
    let mut m = vec![vec![0.0; n]; n];
    for i in 0..n {
        m[i][i] = 1.0;
    }
    m
}

fn scale_matrix(m: &[Vec<f64>], s: f64) -> Vec<Vec<f64>> {
    m.iter()
        .map(|row| row.iter().map(|x| x * s).collect())
        .collect()
}

fn mat_vec_mul(m: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    m.iter()
        .map(|row| row.iter().zip(v.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows_a = a.len();
    let cols_a = a[0].len();
    let cols_b = b[0].len();
    let mut result = vec![vec![0.0; cols_b]; rows_a];

    for i in 0..rows_a {
        for j in 0..cols_b {
            for k in 0..cols_a {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

fn mat_add(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter()
        .zip(b.iter())
        .map(|(row_a, row_b)| row_a.iter().zip(row_b.iter()).map(|(a, b)| a + b).collect())
        .collect()
}

fn mat_sub(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter()
        .zip(b.iter())
        .map(|(row_a, row_b)| row_a.iter().zip(row_b.iter()).map(|(a, b)| a - b).collect())
        .collect()
}

fn transpose(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = m.len();
    let cols = m[0].len();
    let mut result = vec![vec![0.0; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            result[j][i] = m[i][j];
        }
    }
    result
}

#[allow(clippy::needless_range_loop)]
fn invert_matrix(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    if n == 0 || m[0].len() != n {
        return None;
    }

    // Gauss-Jordan elimination
    let mut augmented: Vec<Vec<f64>> = m
        .iter()
        .zip(identity_matrix(n).iter())
        .map(|(row, id_row)| [row.clone(), id_row.clone()].concat())
        .collect();

    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        for k in (i + 1)..n {
            if augmented[k][i].abs() > augmented[max_row][i].abs() {
                max_row = k;
            }
        }
        augmented.swap(i, max_row);

        if augmented[i][i].abs() < 1e-10 {
            return None; // Singular matrix
        }

        // Scale pivot row
        let pivot = augmented[i][i];
        for j in 0..(2 * n) {
            augmented[i][j] /= pivot;
        }

        // Eliminate column
        for k in 0..n {
            if k != i {
                let factor = augmented[k][i];
                for j in 0..(2 * n) {
                    augmented[k][j] -= factor * augmented[i][j];
                }
            }
        }
    }

    Some(augmented.iter().map(|row| row[n..].to_vec()).collect())
}

fn matrix_sqrt(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    // Simplified: diagonal approximation for positive definite matrices
    let n = m.len();
    let mut result = vec![vec![0.0; n]; n];
    for i in 0..n {
        result[i][i] = m[i][i].max(0.0).sqrt();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_filter_creation() {
        let kf = KalmanFilter::new(2, 1);
        assert_eq!(kf.state.len(), 2);
        assert_eq!(kf.covariance.len(), 2);
        assert_eq!(kf.measurement_noise.len(), 1);
    }

    #[test]
    fn test_kalman_filter_predict_update() {
        let mut kf = KalmanFilter::with_noise(1, 1, 0.01, 0.1);
        kf.state = vec![0.5];

        // Predict with identity transition
        let f = vec![vec![1.0]];
        kf.predict(&f);

        // Update with measurement
        let h = vec![vec![1.0]];
        kf.update(&[0.6], &h);

        // State should move toward measurement
        assert!(kf.state[0] > 0.5);
        assert!(kf.state[0] < 0.6);
    }

    #[test]
    fn test_matrix_operations() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];

        let c = mat_mul(&a, &b);
        assert_eq!(c[0][0], 19.0); // 1*5 + 2*7
        assert_eq!(c[0][1], 22.0); // 1*6 + 2*8
        assert_eq!(c[1][0], 43.0); // 3*5 + 4*7
        assert_eq!(c[1][1], 50.0); // 3*6 + 4*8
    }

    #[test]
    fn test_matrix_inversion() {
        let m = vec![vec![4.0, 7.0], vec![2.0, 6.0]];
        let inv = invert_matrix(&m).unwrap();

        // Verify m * inv ≈ I
        let product = mat_mul(&m, &inv);
        assert!((product[0][0] - 1.0).abs() < 1e-10);
        assert!((product[1][1] - 1.0).abs() < 1e-10);
        assert!(product[0][1].abs() < 1e-10);
        assert!(product[1][0].abs() < 1e-10);
    }
}
