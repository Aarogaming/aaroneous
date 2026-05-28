/// Predictive models: Kalman filter + Hidden Markov Model with Viterbi.
///
/// No neural nets or text generation — pure state estimation math.

// ── Kalman Filter (1D/2D state estimation) ────────────────────────────

/// Kalman filter for tracking position + velocity under noise.
/// Predicts pixel targets before the user's cursor arrives.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct KalmanFilter1D {
    /// State: [position, velocity]
    x: [f32; 2],
    /// Covariance matrix (2x2, row-major).
    p: [f32; 4],
    /// Process noise variance.
    q: f32,
    /// Measurement noise variance.
    r: f32,
}

impl KalmanFilter1D {
    pub fn new(initial_pos: f32, q: f32, r: f32) -> Self {
        Self {
            x: [initial_pos, 0.0],
            p: [1.0, 0.0, 0.0, 1.0],
            q,
            r,
        }
    }

    /// Predict next state (position = pos + vel, velocity unchanged).
    pub fn predict(&mut self) {
        let dt = 1.0f32;
        let p00 = self.p[0];
        let p01 = self.p[1];
        let p10 = self.p[2];
        let p11 = self.p[3];
        self.x[0] = self.x[0] + self.x[1] * dt;
        // P' = F * P * F^T + Q,  F = [[1, dt], [0, 1]]
        // F*P = [[p00+dt*p10, p01+dt*p11], [p10, p11]]
        // P' = (F*P) * F^T = [[p00+dt*p10 + dt*(p01+dt*p11), p01+dt*p11],
        //                     [p10 + dt*p11,                   p11]]
        let fp00 = p00 + dt * p10;
        let fp01 = p01 + dt * p11;
        self.p[0] = fp00 + dt * fp01 + self.q;  // +q on diagonal
        self.p[1] = fp01;
        self.p[2] = p10 + dt * p11;
        self.p[3] = p11 + self.q;
    }

    /// Update state with a noisy measurement.
    pub fn update(&mut self, measurement: f32) {
        let y = measurement - self.x[0];
        let s = self.p[0] + self.r;
        let k0 = self.p[0] / s;
        let k1 = self.p[2] / s;
        self.x[0] = self.x[0] + k0 * y;
        self.x[1] = self.x[1] + k1 * y;
        // Covariance update: P = (I - K*H)*P  (save originals before mutation)
        let p00 = self.p[0];
        let p01 = self.p[1];
        let p10 = self.p[2];
        let p11 = self.p[3];
        self.p[0] = (1.0 - k0) * p00;
        self.p[1] = (1.0 - k0) * p01;
        self.p[2] = -k1 * p00 + p10;
        self.p[3] = -k1 * p01 + p11;
    }

    pub fn position(&self) -> f32 { self.x[0] }
    pub fn velocity(&self) -> f32 { self.x[1] }
    pub fn predicted_next(&self) -> f32 { self.x[0] + self.x[1] }
}

// ── Hidden Markov Model with Viterbi Decoding ──────────────────────────

/// A single HMM state.
pub struct HmmState {
    pub id: usize,
    pub label: &'static str,
}

/// HMM: maps visible system actions to hidden user workflows.
/// Uses the Viterbi algorithm to find the most likely intent path.
pub struct HiddenMarkovModel {
    /// Number of hidden states.
    n_states: usize,
    /// Number of observable symbols.
    n_symbols: usize,
    /// Initial state probabilities [n_states].
    init_probs: Vec<f64>,
    /// Transition matrix [n_states x n_states] row-major.
    transitions: Vec<f64>,
    /// Emission matrix [n_states x n_symbols] row-major.
    emissions: Vec<f64>,
}

impl HiddenMarkovModel {
    pub fn new(
        init_probs: Vec<f64>,
        transitions: Vec<f64>,
        emissions: Vec<f64>,
    ) -> Result<Self, String> {
        let n_states = init_probs.len();
        if n_states == 0 { return Err("Need at least 1 state".into()); }
        if transitions.len() != n_states * n_states {
            return Err("Transition matrix size mismatch".into());
        }
        let n_symbols = if n_states > 0 { emissions.len() / n_states } else { 0 };
        if emissions.len() != n_states * n_symbols {
            return Err("Emission matrix size mismatch".into());
        }
        Ok(Self { n_states, n_symbols, init_probs, transitions, emissions })
    }

    /// Viterbi decoding: given observation sequence, find the most likely
    /// hidden state path. Returns (path, log_probability).
    pub fn viterbi(&self, observations: &[usize]) -> (Vec<usize>, f64) {
        let t = observations.len();
        if t == 0 { return (vec![], 0.0); }

        // Viterbi lattice: delta[t][i] = max prob of being in state i at time t
        let mut delta = vec![0.0f64; self.n_states * t];
        // Backpointers for traceback
        let mut psi = vec![0usize; self.n_states * t];

        // Initialization: delta[0][i] = init[i] * emit[i][obs[0]]
        for i in 0..self.n_states {
            delta[i] = self.init_probs[i] * self.emissions[i * self.n_symbols + observations[0]];
            psi[i] = 0;
        }

        // Recursion
        for time in 1..t {
            let obs = observations[time];
            for j in 0..self.n_states {
                let mut max_val = 0.0f64;
                let mut max_idx = 0usize;
                for i in 0..self.n_states {
                    let val = delta[(time - 1) * self.n_states + i]
                        * self.transitions[i * self.n_states + j];
                    if val > max_val {
                        max_val = val;
                        max_idx = i;
                    }
                }
                delta[time * self.n_states + j] = max_val * self.emissions[j * self.n_symbols + obs];
                psi[time * self.n_states + j] = max_idx;
            }
        }

        // Termination: find best final state
        let mut best_last = 0usize;
        let mut best_prob = 0.0f64;
        for i in 0..self.n_states {
            let prob = delta[(t - 1) * self.n_states + i];
            if prob > best_prob {
                best_prob = prob;
                best_last = i;
            }
        }

        // Traceback
        let mut path = vec![0usize; t];
        path[t - 1] = best_last;
        for time in (0..t - 1).rev() {
            path[time] = psi[(time + 1) * self.n_states + path[time + 1]];
        }

        (path, best_prob)
    }

    pub fn n_states(&self) -> usize { self.n_states }
    pub fn n_symbols(&self) -> usize { self.n_symbols }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_constant() {
        let mut kf = KalmanFilter1D::new(10.0, 0.01, 0.1);
        kf.predict();
        kf.update(10.5);
        let pos = kf.position();
        assert!((pos - 10.5).abs() < 0.5); // should track toward measurement
    }

    #[test]
    fn test_kalman_tracking() {
        let mut kf = KalmanFilter1D::new(0.0, 0.001, 0.01);
        for i in 1..=20 {
            kf.predict();
            kf.update(i as f32);
        }
        assert!((kf.position() - 20.0).abs() < 2.0, "pos {}", kf.position());
        assert!((kf.velocity() - 1.0).abs() < 0.3, "vel {}", kf.velocity());
    }

    #[test]
    fn test_kalman_predicted_next() {
        let mut kf = KalmanFilter1D::new(0.0, 0.01, 0.05);
        kf.predict();
        kf.update(2.0);
        kf.predict();
        kf.update(4.0);
        kf.predict();
        assert!(kf.predicted_next() > 3.0);
    }

    #[test]
    fn test_hmm_viterbi_simple() {
        // 2 states, 2 symbols
        // State 0: high prob of emitting symbol 0
        // State 1: high prob of emitting symbol 1
        let init = vec![0.5, 0.5];
        let trans = vec![0.8, 0.2, 0.2, 0.8]; // stay in same state usually
        let emit = vec![0.9, 0.1, 0.1, 0.9]; // state 0→sym0, state 1→sym1

        let hmm = HiddenMarkovModel::new(init, trans, emit).unwrap();
        let obs = vec![0, 0, 1, 1, 0];
        let (path, prob) = hmm.viterbi(&obs);
        assert_eq!(path.len(), 5);
        assert!(prob > 0.0);
        // First 2 observations likely state 0, next 2 state 1, last state 0
        assert_eq!(path[0], 0);
        assert_eq!(path[1], 0);
        assert_eq!(path[2], 1);
        assert_eq!(path[3], 1);
        assert_eq!(path[4], 0);
    }

    #[test]
    fn test_hmm_viterbi_empty() {
        let hmm = HiddenMarkovModel::new(vec![0.5], vec![1.0], vec![1.0]).unwrap();
        let (path, prob) = hmm.viterbi(&[]);
        assert!(path.is_empty());
        assert_eq!(prob, 0.0);
    }

    #[test]
    fn test_hmm_construction_errors() {
        assert!(HiddenMarkovModel::new(vec![], vec![], vec![]).is_err());
        // With 2 states, transitions must be 4 elements
        assert!(HiddenMarkovModel::new(vec![0.5, 0.5], vec![0.5], vec![1.0, 0.0, 0.0, 1.0]).is_err());
    }
}
