/// Model Predictive Control (MPC) for proactive resource planning.
/// Optimizes control actions over a prediction horizon subject to constraints.
/// Replaces reactive throttling with proactive resource allocation.

use crate::kalman::KalmanFilter;

/// MPC configuration.
#[derive(Debug, Clone)]
pub struct MpcConfig {
    pub prediction_horizon: usize,  // N: number of future steps to optimize
    pub control_horizon: usize,     // M: number of control moves to optimize (M <= N)
    pub state_weight: Vec<f64>,     // Q: state error penalty
    pub control_weight: Vec<f64>,   // R: control effort penalty
    pub terminal_weight: Vec<f64>,  // P: terminal state penalty
    pub control_limits: (Vec<f64>, Vec<f64>), // (u_min, u_max)
    pub state_limits: (Vec<f64>, Vec<f64>),   // (x_min, x_max)
}

impl Default for MpcConfig {
    fn default() -> Self {
        Self {
            prediction_horizon: 10,
            control_horizon: 5,
            state_weight: vec![1.0],
            control_weight: vec![0.1],
            terminal_weight: vec![1.0],
            control_limits: (vec![0.0], vec![1.0]),
            state_limits: (vec![0.0], vec![100.0]),
        }
    }
}

/// MPC state for linear systems.
/// x[k+1] = A * x[k] + B * u[k]
/// y[k] = C * x[k]
#[derive(Debug, Clone)]
pub struct LinearMpc {
    pub config: MpcConfig,
    pub state_dim: usize,
    pub control_dim: usize,
    pub output_dim: usize,
    pub a_matrix: Vec<Vec<f64>>,  // State transition
    pub b_matrix: Vec<Vec<f64>>,  // Control input
    pub c_matrix: Vec<Vec<f64>>,  // Output mapping
    pub reference: Vec<f64>,      // Target state/output
    pub kalman: Option<KalmanFilter>, // Optional state estimator
}

impl LinearMpc {
    pub fn new(
        config: MpcConfig,
        a_matrix: Vec<Vec<f64>>,
        b_matrix: Vec<Vec<f64>>,
        c_matrix: Vec<Vec<f64>>,
    ) -> Self {
        let state_dim = a_matrix.len();
        let control_dim = b_matrix[0].len();
        let output_dim = c_matrix.len();

        Self {
            config,
            state_dim,
            control_dim,
            output_dim,
            a_matrix,
            b_matrix,
            c_matrix,
            reference: vec![0.0; output_dim],
            kalman: None,
        }
    }

    /// Set reference trajectory.
    pub fn set_reference(&mut self, reference: Vec<f64>) {
        self.reference = reference;
    }

    /// Solve MPC optimization problem.
    /// Returns optimal control sequence u[0], u[1], ..., u[M-1]
    /// Uses quadratic programming with constraints.
    pub fn solve(&self, current_state: &[f64]) -> Vec<f64> {
        let n = self.config.prediction_horizon;
        let m = self.config.control_horizon;

        // Build prediction matrices
        let (phi, gamma) = self.build_prediction_matrices(n, m);

        // Build cost function: J = ||Y - R||^2_Q + ||U||^2_R
        let reference_traj = self.build_reference_trajectory(n);
        let optimal_controls = self.solve_qp(&phi, &gamma, current_state, &reference_traj);

        optimal_controls
    }

    /// Get first control action (receding horizon principle).
    pub fn get_control_action(&self, current_state: &[f64]) -> Vec<f64> {
        let controls = self.solve(current_state);
        controls[..self.control_dim].to_vec()
    }

    /// Predict future states given control sequence.
    pub fn predict_states(&self, initial_state: &[f64], controls: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let mut states = vec![initial_state.to_vec()];
        let mut current = initial_state.to_vec();

        for u in controls {
            let next = self.predict_step(&current, u);
            states.push(next.clone());
            current = next;
        }

        states
    }

    /// Single step prediction: x[k+1] = A*x[k] + B*u[k]
    fn predict_step(&self, state: &[f64], control: &[f64]) -> Vec<f64> {
        let mut next = vec![0.0; self.state_dim];

        for i in 0..self.state_dim {
            for j in 0..self.state_dim {
                next[i] += self.a_matrix[i][j] * state[j];
            }
            for j in 0..self.control_dim {
                next[i] += self.b_matrix[i][j] * control[j];
            }
        }

        // Apply state limits
        for i in 0..self.state_dim {
            next[i] = next[i].clamp(self.config.state_limits.0[i], self.config.state_limits.1[i]);
        }

        next
    }

    /// Build prediction matrices for horizon N.
    /// Y = Phi * x[0] + Gamma * U
    fn build_prediction_matrices(&self, n: usize, m: usize) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let output_steps = n * self.output_dim;
        let control_steps = m * self.control_dim;

        // Phi: output response to initial state
        let mut phi = vec![vec![0.0; self.state_dim]; output_steps];
        let mut a_power = identity_matrix(self.state_dim);

        for k in 0..n {
            a_power = mat_mul(&self.a_matrix, &a_power);
            let ca = mat_mul(&self.c_matrix, &a_power);
            for i in 0..self.output_dim {
                for j in 0..self.state_dim {
                    phi[k * self.output_dim + i][j] = ca[i][j];
                }
            }
        }

        // Gamma: output response to control inputs
        let mut gamma = vec![vec![0.0; control_steps]; output_steps];

        for k in 0..n {
            for j in 0..m.min(k + 1) {
                // Compute A^(k-j) * B
                let mut a_pow = identity_matrix(self.state_dim);
                for _ in 0..(k - j) {
                    a_pow = mat_mul(&self.a_matrix, &a_pow);
                }
                let ab = mat_mul(&a_pow, &self.b_matrix);
                let cab = mat_mul(&self.c_matrix, &ab);

                for i in 0..self.output_dim {
                    for l in 0..self.control_dim {
                        gamma[k * self.output_dim + i][j * self.control_dim + l] = cab[i][l];
                    }
                }
            }
        }

        (phi, gamma)
    }

    /// Build reference trajectory over prediction horizon.
    fn build_reference_trajectory(&self, n: usize) -> Vec<f64> {
        let mut traj = Vec::with_capacity(n * self.output_dim);
        for _ in 0..n {
            traj.extend_from_slice(&self.reference);
        }
        traj
    }

    /// Solve constrained QP using projected gradient descent.
    /// min ||Phi*x + Gamma*U - R||^2_Q + ||U||^2_R
    /// s.t. u_min <= u <= u_max
    fn solve_qp(
        &self,
        phi: &[Vec<f64>],
        gamma: &[Vec<f64>],
        x0: &[f64],
        reference: &[f64],
    ) -> Vec<f64> {
        let m = self.config.control_horizon;
        let control_steps = m * self.control_dim;
        let output_steps = reference.len();

        // Initialize control sequence
        let mut u = vec![0.0; control_steps];

        // Gradient descent with projection
        let learning_rate = 0.01;
        let max_iterations = 100;

        for _ in 0..max_iterations {
            // Compute predicted output: Y = Phi*x0 + Gamma*U
            let phi_x0 = mat_vec_mul(phi, x0);
            let gamma_u = mat_vec_mul(gamma, &u);
            let predicted: Vec<f64> = phi_x0
                .iter()
                .zip(gamma_u.iter())
                .map(|(a, b)| a + b)
                .collect();

            // Compute gradient: dJ/dU = 2*Gamma^T*Q*(Y-R) + 2*R*U
            let error: Vec<f64> = predicted
                .iter()
                .zip(reference.iter())
                .map(|(y, r)| y - r)
                .collect();

            let gamma_t = transpose(gamma);
            let q_error: Vec<f64> = error
                .iter()
                .zip(self.config.state_weight.iter().cycle())
                .map(|(e, q)| e * q)
                .collect();
            let gamma_t_q_error = mat_vec_mul(&gamma_t, &q_error);

            let r_u: Vec<f64> = u
                .iter()
                .zip(self.config.control_weight.iter().cycle())
                .map(|(ui, r)| ui * r)
                .collect();

            let gradient: Vec<f64> = gamma_t_q_error
                .iter()
                .zip(r_u.iter())
                .map(|(a, b)| 2.0 * (a + b))
                .collect();

            // Update with gradient step
            let mut new_u: Vec<f64> = u
                .iter()
                .zip(gradient.iter())
                .map(|(ui, gi)| ui - learning_rate * gi)
                .collect();

            // Project onto control limits
            for i in 0..control_steps {
                let idx = i % self.control_dim;
                new_u[i] = new_u[i].clamp(
                    self.config.control_limits.0[idx],
                    self.config.control_limits.1[idx],
                );
            }

            u = new_u;
        }

        u
    }
}

/// Simplified MPC for scalar systems.
/// x[k+1] = a*x[k] + b*u[k]
/// Useful for single-variable control like expression rate.
#[derive(Debug, Clone)]
pub struct ScalarMpc {
    pub a: f64,  // State transition coefficient
    pub b: f64,  // Control input coefficient
    pub config: MpcConfig,
    pub reference: f64,
}

impl ScalarMpc {
    pub fn new(a: f64, b: f64, reference: f64) -> Self {
        Self {
            a,
            b,
            config: MpcConfig::default(),
            reference,
        }
    }

    /// Solve scalar MPC problem.
    /// Returns optimal control u[0].
    pub fn solve(&self, current_state: f64) -> f64 {
        let n = self.config.prediction_horizon;

        // For scalar system, optimal control can be computed analytically
        // u* = -(b/a) * (x - r) * (1 - a^n) / (1 - a^(n+1))
        // Simplified: proportional control with preview

        let error = self.reference - current_state;
        let preview_gain = (1.0 - self.a.powi(n as i32)) / (1.0 - self.a.powi(n as i32 + 1));
        let u = self.b * error * preview_gain;

        // Apply control limits
        u.clamp(self.config.control_limits.0[0], self.config.control_limits.1[0])
    }

    /// Predict future states.
    pub fn predict(&self, current_state: f64, control: f64, steps: usize) -> Vec<f64> {
        let mut states = vec![current_state];
        let mut x = current_state;

        for _ in 0..steps {
            x = self.a * x + self.b * control;
            states.push(x);
        }

        states
    }
}

// Matrix operations (duplicated from kalman.rs for modularity)
fn identity_matrix(n: usize) -> Vec<Vec<f64>> {
    let mut m = vec![vec![0.0; n]; n];
    for i in 0..n {
        m[i][i] = 1.0;
    }
    m
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_mpc() {
        let mpc = ScalarMpc::new(0.9, 0.1, 50.0);
        let u = mpc.solve(30.0);
        assert!(u > 0.0); // Should apply positive control to reach reference

        let states = mpc.predict(30.0, u, 10);
        assert!(states.last().unwrap() > &30.0); // Should move toward reference
    }

    #[test]
    fn test_scalar_mpc_stability() {
        let mpc = ScalarMpc::new(0.8, 0.2, 1.0);
        let mut state = 0.0;

        for _ in 0..20 {
            let u = mpc.solve(state);
            state = mpc.a * state + mpc.b * u;
        }

        // Should converge toward reference
        assert!((state - 1.0).abs() < 0.5);
    }

    #[test]
    fn test_linear_mpc_creation() {
        let a = vec![vec![0.9]];
        let b = vec![vec![0.1]];
        let c = vec![vec![1.0]];

        let mpc = LinearMpc::new(MpcConfig::default(), a, b, c);
        assert_eq!(mpc.state_dim, 1);
        assert_eq!(mpc.control_dim, 1);
        assert_eq!(mpc.output_dim, 1);
    }
}
