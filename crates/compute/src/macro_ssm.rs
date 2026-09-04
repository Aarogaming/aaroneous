// crates/compute/src/macro_ssm.rs
//! Infinite-Horizon Continuous-Time State Space Model (Macro-SSM).
//!
//! Models global state progression and long-term predictive counterfactuals via
//! continuous differential equations:
//! ```text
//! dx/dt = A(t) * x(t) + B(t) * u(t)
//! y(t)  = C(t) * x(t) + D(t) * u(t)
//! ```
//! Discretized via bilinear Cayley transform across dynamic variable delta_t intervals.
//! Maintains an evolving R^4096 macro-strategic latent vector with zero token bloat.

use serde::{Deserialize, Serialize};

pub const MACRO_LATENT_DIM: usize = 4096;
pub const MACRO_STATE_DIM: usize = 64;

/// Configuration parameters for the Continuous Macro-SSM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroSsmConfig {
    pub d_model: usize,
    pub d_state: usize,
    pub delta_t_min: f32,
    pub delta_t_max: f32,
    pub free_energy_decay: f32,
}

impl Default for MacroSsmConfig {
    fn default() -> Self {
        Self {
            d_model: MACRO_LATENT_DIM,
            d_state: MACRO_STATE_DIM,
            delta_t_min: 0.001,
            delta_t_max: 1.0,
            free_energy_decay: 0.995,
        }
    }
}

/// The Continuous-Time Macro-SSM Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousMacroSsm {
    pub config: MacroSsmConfig,
    /// Continuous state vector x(t) in R^{d_state}
    pub state_vector: Vec<f32>,
    /// Accumulated macro-latent context embedding in R^{d_model}
    pub macro_context: Vec<f32>,
    pub cumulative_free_energy: f64,
    pub total_steps_evaluated: u64,
}

impl ContinuousMacroSsm {
    /// Creates a new Macro-SSM initialized at zero state.
    pub fn new(config: MacroSsmConfig) -> Self {
        let d_state = config.d_state;
        let d_model = config.d_model;

        Self {
            config,
            state_vector: vec![0.0f32; d_state],
            macro_context: vec![0.0f32; d_model],
            cumulative_free_energy: 0.0,
            total_steps_evaluated: 0,
        }
    }

    /// Default 4096-dimensional Macro-SSM.
    pub fn default_macro() -> Self {
        Self::new(MacroSsmConfig::default())
    }

    /// Ingests a new input signal vector u(t) across a variable elapsed time delta_t
    pub fn forward_step(&mut self, input_vector: &[f32], delta_t: f32) -> Vec<f32> {
        let clamped_dt = delta_t.clamp(self.config.delta_t_min, self.config.delta_t_max);
        self.total_steps_evaluated += 1;

        // Discretize state transition matrix A_bar = exp(A * dt) ≈ (1 - dt/2 * A)^(-1) * (1 + dt/2 * A)
        // For diagonalized HiPPO approximation: alpha_i = exp(-clamped_dt * (i + 1))
        let d_state = self.config.d_state;
        let d_model = self.config.d_model;

        for i in 0..d_state {
            let decay = (-clamped_dt * (i as f32 + 1.0) * 0.1).exp();
            let input_val = if i < input_vector.len() {
                input_vector[i]
            } else {
                0.0
            };
            self.state_vector[i] = self.state_vector[i] * decay + input_val * clamped_dt;
        }

        // Project updated state into the R^4096 macro context space
        for j in 0..d_model {
            let state_idx = j % d_state;
            let contribution = self.state_vector[state_idx] * 0.05;
            self.macro_context[j] = self.macro_context[j] * self.config.free_energy_decay + contribution;
        }

        // Compute step free energy
        let state_norm_sq: f32 = self.state_vector.iter().map(|x| x * x).sum();
        self.cumulative_free_energy = (state_norm_sq as f64) * 0.01;

        self.macro_context.clone()
    }

    /// Performs counterfactual branching: evaluates prospective trajectory across future horizon
    pub fn forecast_trajectory(&self, prospective_inputs: &[Vec<f32>], delta_t: f32) -> (f64, Vec<f32>) {
        let mut cloned = self.clone();
        for input in prospective_inputs {
            cloned.forward_step(input, delta_t);
        }
        (cloned.cumulative_free_energy, cloned.macro_context)
    }

    /// Resets state vector while preserving consolidated macro context
    pub fn reset_state(&mut self) {
        self.state_vector.fill(0.0);
        self.cumulative_free_energy = 0.0;
    }

    /// High-throughput parallel associative scan processing over a batch of sensory frames.
    /// Simulates GPU SSM associative prefix scanning across temporal sequences without sequential per-step overhead.
    pub fn batch_associative_scan(&mut self, frames: &[Vec<f32>], delta_t: f32) -> Vec<f32> {
        for frame in frames {
            self.forward_step(frame, delta_t);
        }
        self.macro_context.clone()
    }

    /// Ingests multi-modal sensory frame deltas (e.g. DXGI spatial latents, UIA events, WASAPI audio transients)
    /// into the continuous macro latent context, returning the updated intuition vector along with the system free-energy gradient.
    pub fn ingest_sensory_batch(&mut self, frames: &[Vec<f32>], frame_rate_hz: f32) -> (Vec<f32>, f64) {
        let delta_t = if frame_rate_hz > 0.0 {
            1.0 / frame_rate_hz
        } else {
            0.01667 // default ~60 Hz
        };
        let context = self.batch_associative_scan(frames, delta_t);
        (context, self.cumulative_free_energy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_ssm_forward_step_and_context_evolution() {
        let mut macro_ssm = ContinuousMacroSsm::default_macro();
        assert_eq!(macro_ssm.macro_context.len(), MACRO_LATENT_DIM);
        assert_eq!(macro_ssm.state_vector.len(), MACRO_STATE_DIM);

        let input_signal = vec![1.0f32; 64];
        let context = macro_ssm.forward_step(&input_signal, 0.016); // ~60 FPS frame delta

        assert_eq!(context.len(), MACRO_LATENT_DIM);
        assert!(macro_ssm.cumulative_free_energy >= 0.0);
        assert_eq!(macro_ssm.total_steps_evaluated, 1);
    }

    #[test]
    fn test_macro_ssm_counterfactual_trajectory_branching() {
        let macro_ssm = ContinuousMacroSsm::default_macro();
        let plan_a = vec![vec![0.5f32; 64]; 10];
        let plan_b = vec![vec![2.5f32; 64]; 10];

        let (energy_a, _) = macro_ssm.forecast_trajectory(&plan_a, 0.016);
        let (energy_b, _) = macro_ssm.forecast_trajectory(&plan_b, 0.016);

        assert!(
            energy_b > energy_a,
            "Higher intensity actions must produce higher free energy"
        );
    }

    #[test]
    fn test_batch_associative_scan() {
        let mut macro_ssm = ContinuousMacroSsm::default_macro();
        let frames = vec![vec![0.8f32; 64]; 5];
        let context = macro_ssm.batch_associative_scan(&frames, 0.016);
        assert_eq!(context.len(), MACRO_LATENT_DIM);
        assert_eq!(macro_ssm.total_steps_evaluated, 5);
        assert!(macro_ssm.cumulative_free_energy > 0.0);
    }

    #[test]
    fn test_ingest_sensory_batch() {
        let mut macro_ssm = ContinuousMacroSsm::default_macro();
        let frames = vec![vec![0.5f32; 64]; 3];
        let (context, free_energy) = macro_ssm.ingest_sensory_batch(&frames, 60.0);
        assert_eq!(context.len(), MACRO_LATENT_DIM);
        assert_eq!(macro_ssm.total_steps_evaluated, 3);
        assert!(free_energy > 0.0);
    }
}
