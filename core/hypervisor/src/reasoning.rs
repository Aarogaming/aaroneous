/// Reasoning scalers: test-time Viterbi depth adjustment, sparse
/// autoencoder feature steering for GGUF tensor maps, and state-space
/// model linear convolutions for long-range sequence memory.
use crate::predictive_models::HiddenMarkovModel;

// ── Test-Time Compute Scaler (Viterbi Depth) ─────────────────────────
// Expands Viterbi search depth when encountering novel environments.

#[derive(Debug, Clone)]
pub struct ViterbiDepthScaler {
    pub base_depth: usize,
    pub max_depth: usize,
    pub novelty_threshold: f64,
    pub current_depth: usize,
}

impl ViterbiDepthScaler {
    pub fn new(base: usize, max: usize, novelty: f64) -> Self {
        ViterbiDepthScaler {
            base_depth: base,
            max_depth: max,
            novelty_threshold: novelty,
            current_depth: base,
        }
    }

    /// Estimate novelty from HMM path probability.
    /// Low probability → novel environment → expand depth.
    pub fn estimate_novelty(prob: f64) -> f64 {
        if prob <= 0.0 {
            return 1.0;
        }
        (-prob.ln()).min(1.0)
    }

    /// Adjust search depth based on path probability from Viterbi.
    pub fn adjust(&mut self, viterbi_prob: f64) -> usize {
        let novelty = Self::estimate_novelty(viterbi_prob);
        if novelty > self.novelty_threshold {
            self.current_depth = self.max_depth;
        } else {
            self.current_depth = self.base_depth;
        }
        self.current_depth
    }

    /// Run Viterbi with depth-scaled search (repeat with expanded
    /// observation windows if novelty is high).
    pub fn decode_with_depth(
        &mut self,
        hmm: &HiddenMarkovModel,
        observations: &[usize],
    ) -> (Vec<usize>, f64) {
        let depth = self.current_depth;
        let window = observations.len().min(depth);
        let obs: Vec<usize> = observations.iter().copied().take(window).collect();
        let (path, prob) = hmm.viterbi(&obs);
        let novelty = Self::estimate_novelty(prob);
        if novelty > self.novelty_threshold && self.current_depth < self.max_depth {
            self.current_depth = (self.current_depth * 2).min(self.max_depth);
            let wider: Vec<usize> = observations
                .iter()
                .copied()
                .take(self.current_depth.min(observations.len()))
                .collect();
            let (deeper_path, deeper_prob) = hmm.viterbi(&wider);
            if deeper_prob > prob {
                return (deeper_path, deeper_prob);
            }
        }
        (path, prob)
    }
}

// ── Sparse-Autoencoder Feature Steering ──────────────────────────────
// Sparse dictionary to isolate and amplify action sub-features inside
// distilled GGUF tensor maps.

#[derive(Debug, Clone)]
pub struct SparseAutoencoderSteer {
    pub dict_size: usize,
    pub input_dim: usize,
    /// Dictionary atoms (dict_size × input_dim).
    pub dictionary: Vec<Vec<f64>>,
    /// Activation sparse codes for each atom.
    pub codes: Vec<f64>,
}

impl SparseAutoencoderSteer {
    pub fn new(dict_size: usize, input_dim: usize) -> Self {
        let dictionary = vec![vec![0.0; input_dim]; dict_size];
        // Initialize with random orthogonal-ish vectors
        for (i, atom) in dictionary.iter().enumerate() {
            let mut atom = atom.clone();
            let seed = (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
            for j in 0..input_dim {
                let h = (seed ^ (j as u64)).wrapping_mul(0xBF58476D1CE4E5B9);
                atom[j] = (h as f64 / u64::MAX as f64) * 2.0 - 1.0;
            }
        }
        SparseAutoencoderSteer {
            dict_size,
            input_dim,
            dictionary,
            codes: vec![0.0; dict_size],
        }
    }

    /// Encode input through sparse coding (greedy matching pursuit).
    pub fn encode(&mut self, input: &[f64]) -> Vec<f64> {
        let n = input.len().min(self.input_dim);
        let mut residual: Vec<f64> = input.iter().copied().take(n).collect();
        self.codes = vec![0.0; self.dict_size];

        for _ in 0..self.dict_size.min(16) {
            // Find best matching atom
            let mut best_idx = 0usize;
            let mut best_dot = 0.0f64;
            for (i, atom) in self.dictionary.iter().enumerate() {
                let dot: f64 = atom.iter().zip(&residual).map(|(a, r)| a * r).take(n).sum();
                let abs_dot = dot.abs();
                if abs_dot > best_dot {
                    best_dot = abs_dot;
                    best_idx = i;
                }
            }
            if best_dot < 1e-9 {
                break;
            }
            let atom = &self.dictionary[best_idx];
            let alpha = best_dot;
            self.codes[best_idx] += alpha;
            for j in 0..n {
                residual[j] -= alpha * atom[j];
            }
        }
        self.codes.clone()
    }

    /// Amplify specific feature indices for steering.
    pub fn steer(&mut self, feature_indices: &[usize], amplification: f64) {
        for &idx in feature_indices {
            if idx < self.dict_size {
                self.codes[idx] *= amplification;
            }
        }
    }

    /// Decode codes back to input space.
    pub fn decode(&self, codes: &[f64]) -> Vec<f64> {
        let n = self.input_dim;
        let mut output = vec![0.0; n];
        for (i, atom) in self.dictionary.iter().enumerate() {
            let c = if i < codes.len() { codes[i] } else { 0.0 };
            for j in 0..n {
                output[j] += c * atom[j];
            }
        }
        output
    }
}

// ── State-Space Model Linear Convolutions ────────────────────────────
// Bypasses O(N²) context windows with fixed-size linear state-space
// matrices (Mamba-style). Remembers user behavior across weeks.

#[derive(Debug, Clone)]
pub struct StateSpaceModel {
    pub state_dim: usize,
    pub input_dim: usize,
    /// State transition matrix A (state_dim × state_dim)
    pub a: Vec<f64>,
    /// Input projection B (state_dim × input_dim)
    pub b: Vec<f64>,
    /// Output projection C (input_dim × state_dim)
    pub c: Vec<f64>,
    /// Current state vector
    pub state: Vec<f64>,
    /// Decay factor (0-1): how fast old memories fade
    pub decay: f64,
}

impl StateSpaceModel {
    pub fn new(state_dim: usize, input_dim: usize, decay: f64) -> Self {
        // HiPPO-like initialization for A (legT)
        let mut a = vec![0.0; state_dim * state_dim];
        for i in 0..state_dim {
            for j in 0..state_dim {
                if i == j {
                    a[i * state_dim + j] = -0.5;
                } else if i > j {
                    a[i * state_dim + j] = -1.0;
                }
            }
        }
        // B: identity-like
        let mut b = vec![0.0; state_dim * input_dim];
        for i in 0..state_dim.min(input_dim) {
            b[i * input_dim + i] = 1.0;
        }
        // C: random projection
        let mut c = vec![0.0; input_dim * state_dim];
        for i in 0..input_dim {
            for j in 0..state_dim {
                c[i * state_dim + j] = (j & 1) as f64 * 2.0 - 1.0;
            }
        }
        StateSpaceModel {
            state_dim,
            input_dim,
            a,
            b,
            c,
            state: vec![0.0; state_dim],
            decay,
        }
    }

    /// Step the SSM: state' = A·state + B·input, output = C·state'
    pub fn step(&mut self, input: &[f64]) -> Vec<f64> {
        let n = input.len().min(self.input_dim);
        // state = A * state + B * input
        let mut new_state = vec![0.0; self.state_dim];
        for i in 0..self.state_dim {
            let mut s = 0.0;
            for j in 0..self.state_dim {
                s += self.a[i * self.state_dim + j] * self.state[j];
            }
            for j in 0..n {
                s += self.b[i * self.input_dim + j] * input[j];
            }
            new_state[i] = s * self.decay;
        }
        self.state = new_state;
        // output = C * state
        let mut output = vec![0.0; self.input_dim];
        for i in 0..self.input_dim {
            for j in 0..self.state_dim {
                output[i] += self.c[i * self.state_dim + j] * self.state[j];
            }
        }
        output
    }

    /// Process a sequence of inputs; returns outputs.
    pub fn process_sequence(&mut self, inputs: &[Vec<f64>]) -> Vec<Vec<f64>> {
        inputs.iter().map(|input| self.step(input)).collect()
    }

    /// Decay the state (simulate time passage without input).
    pub fn tick(&mut self) {
        for s in &mut self.state {
            *s *= self.decay;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viterbi_depth_adjust() {
        let mut scaler = ViterbiDepthScaler::new(5, 50, 0.3);
        // High probability → low novelty → base depth
        let depth = scaler.adjust(0.9);
        assert_eq!(depth, 5);
    }

    #[test]
    fn test_viterbi_depth_novelty() {
        let mut scaler = ViterbiDepthScaler::new(5, 50, 0.3);
        // Very low prob → high novelty → max depth
        let depth = scaler.adjust(1e-10);
        assert_eq!(depth, 50);
    }

    #[test]
    fn test_sae_encode_decode() {
        let mut sae = SparseAutoencoderSteer::new(16, 8);
        let input = vec![1.0, 0.5, -0.3, 0.8, 0.0, -0.7, 0.2, 0.9];
        let codes = sae.encode(&input);
        assert_eq!(codes.len(), 16);
        let reconstructed = sae.decode(&codes);
        assert_eq!(reconstructed.len(), 8);
    }

    #[test]
    fn test_sae_steer() {
        let mut sae = SparseAutoencoderSteer::new(8, 4);
        let input = vec![1.0, 0.0, -1.0, 0.5];
        let _ = sae.encode(&input);
        // Steering should not panic and codes should be non-negative after steering
        sae.steer(&[0, 1, 2], 2.0);
        // At minimum, codes changed (steer multiplied some by 2.0)
        let total: f64 = sae.codes.iter().sum();
        assert!(total >= 0.0);
    }

    #[test]
    fn test_ssm_step() {
        let mut ssm = StateSpaceModel::new(4, 2, 0.9);
        let input = vec![1.0, 0.5];
        let output = ssm.step(&input);
        assert_eq!(output.len(), 2);
    }

    #[test]
    fn test_ssm_sequence() {
        let mut ssm = StateSpaceModel::new(4, 2, 0.9);
        let inputs = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let outputs = ssm.process_sequence(&inputs);
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].len(), 2);
    }

    #[test]
    fn test_ssm_decay() {
        let mut ssm = StateSpaceModel::new(2, 1, 0.5);
        ssm.step(&[10.0]);
        assert!(ssm.state[0].abs() > 0.0);
        ssm.tick();
        // State should decay
        assert!(ssm.state[0].abs() < 10.0);
    }
}
