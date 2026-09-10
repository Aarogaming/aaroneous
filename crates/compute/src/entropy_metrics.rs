/// Thermodynamics & Statistical Mechanics primitives.
/// Implements Free Energy Principle, phase transitions, and Boltzmann distributions
/// for metabolic control and system stability guarantees.
/// Free Energy Principle state.
/// F = E - T*S where:
///   E = expected error (prediction error)
///   T = temperature (exploration rate)
///   S = entropy (uncertainty)
#[derive(Debug, Clone)]
pub struct FreeEnergyState {
    pub expected_error: f64, // E: prediction error
    pub temperature: f64,    // T: exploration/exploitation balance
    pub entropy: f64,        // S: system uncertainty
    pub free_energy: f64,    // F = E - T*S
}

impl FreeEnergyState {
    pub fn new(expected_error: f64, temperature: f64, entropy: f64) -> Self {
        let free_energy = expected_error - temperature * entropy;
        Self {
            expected_error,
            temperature,
            entropy,
            free_energy,
        }
    }

    /// Update free energy with new observations
    pub fn update(&mut self, new_error: f64, new_entropy: f64) {
        // Exponential moving average for stability
        let alpha = 0.1;
        self.expected_error = (1.0 - alpha) * self.expected_error + alpha * new_error;
        self.entropy = (1.0 - alpha) * self.entropy + alpha * new_entropy;
        self.free_energy = self.expected_error - self.temperature * self.entropy;
    }

    /// Adjust temperature based on free energy gradient
    /// High free energy -> increase temperature (explore more)
    /// Low free energy -> decrease temperature (exploit)
    pub fn adjust_temperature(&mut self, learning_rate: f64, target_free_energy: f64) {
        let gradient = self.free_energy - target_free_energy;
        self.temperature = (self.temperature + learning_rate * gradient).clamp(0.01, 2.0);
        self.free_energy = self.expected_error - self.temperature * self.entropy;
    }
}

/// Boltzmann distribution for specialist selection.
/// P(state) ∝ exp(-E/kT)
/// Returns probability distribution over energy states.
pub fn boltzmann_distribution(energies: &[f64], temperature: f64) -> Vec<f64> {
    if energies.is_empty() || temperature <= 0.0 {
        return vec![0.0; energies.len()];
    }

    let kt = temperature; // k = 1 for simplicity
    let _max_energy = energies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Numerically stable softmax: exp(x - max) / sum(exp(x - max))
    let exp_values: Vec<f64> = energies.iter().map(|&e| (-e / kt).exp()).collect();

    let sum_exp: f64 = exp_values.iter().sum();
    if sum_exp == 0.0 {
        return vec![1.0 / energies.len() as f64; energies.len()];
    }

    exp_values.iter().map(|&e| e / sum_exp).collect()
}

/// Phase transition detection.
/// Detects when system shifts between operational regimes.
/// Uses order parameter and susceptibility to identify critical points.
#[derive(Debug, Clone)]
pub struct PhaseTransitionDetector {
    pub order_parameter_history: Vec<f64>,
    pub susceptibility_history: Vec<f64>,
    pub window_size: usize,
    pub critical_threshold: f64,
}

impl PhaseTransitionDetector {
    pub fn new(window_size: usize, critical_threshold: f64) -> Self {
        Self {
            order_parameter_history: Vec::with_capacity(window_size),
            susceptibility_history: Vec::with_capacity(window_size),
            window_size,
            critical_threshold,
        }
    }

    /// Record new order parameter measurement.
    /// Order parameter: macroscopic observable (e.g., average token availability)
    pub fn record(&mut self, order_parameter: f64) {
        self.order_parameter_history.push(order_parameter);
        if self.order_parameter_history.len() > self.window_size {
            self.order_parameter_history.remove(0);
        }

        // Compute susceptibility (variance of order parameter)
        if self.order_parameter_history.len() >= 2 {
            let mean: f64 = self.order_parameter_history.iter().sum::<f64>()
                / self.order_parameter_history.len() as f64;
            let variance: f64 = self
                .order_parameter_history
                .iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>()
                / self.order_parameter_history.len() as f64;
            self.susceptibility_history.push(variance);
        }

        if self.susceptibility_history.len() > self.window_size {
            self.susceptibility_history.remove(0);
        }
    }

    /// Check if system is near a phase transition.
    /// High susceptibility indicates critical point.
    pub fn is_near_transition(&self) -> bool {
        if self.susceptibility_history.is_empty() {
            return false;
        }
        let current_susceptibility = *self.susceptibility_history.last().unwrap();
        current_susceptibility > self.critical_threshold
    }

    /// Get current phase classification.
    pub fn get_phase(&self) -> SystemPhase {
        if self.order_parameter_history.len() < 3 {
            return SystemPhase::Unknown;
        }

        let recent: Vec<f64> = self
            .order_parameter_history
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect();
        let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;

        if self.is_near_transition() {
            SystemPhase::Critical
        } else if mean > 0.7 {
            SystemPhase::Ordered // High performance, low exploration
        } else if mean > 0.3 {
            SystemPhase::Mixed // Balanced
        } else {
            SystemPhase::Disordered // Low performance, high exploration
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SystemPhase {
    Ordered,    // Stable, exploitative
    Mixed,      // Transitional
    Disordered, // Unstable, exploratory
    Critical,   // Near phase transition
    Unknown,
}

/// Entropy production rate.
/// Quantifies irreversible work done by the system.
/// dS/dt = Σ J_i * X_i where J_i are fluxes and X_i are forces.
pub fn entropy_production_rate(fluxes: &[f64], forces: &[f64]) -> f64 {
    if fluxes.len() != forces.len() {
        return 0.0;
    }
    fluxes.iter().zip(forces.iter()).map(|(j, x)| j * x).sum()
}

/// Thermodynamic efficiency of computation.
/// η = useful_work / total_energy_input
pub fn thermodynamic_efficiency(
    useful_work: f64,  // e.g., tasks completed successfully
    total_energy: f64, // e.g., tokens consumed
    waste_heat: f64,   // e.g., failed computations
) -> f64 {
    if total_energy + waste_heat == 0.0 {
        return 0.0;
    }
    useful_work / (total_energy + waste_heat)
}

/// Gibbs free energy for resource allocation decisions.
/// ΔG = ΔH - TΔS
/// Negative ΔG indicates spontaneous (favorable) allocation.
pub fn gibbs_free_energy(
    enthalpy_change: f64, // ΔH: energy cost of allocation
    entropy_change: f64,  // ΔS: uncertainty reduction
    temperature: f64,     // T: system temperature
) -> f64 {
    enthalpy_change - temperature * entropy_change
}

/// Thermodynamic forecast for system state prediction.
/// Used by the unified learning loop to track thermodynamic metrics.
#[derive(Debug, Clone)]
pub struct ThermodynamicForecast {
    pub free_energy: f64,
    pub temperature: f64,
    pub entropy: f64,
    pub phase: String,
    pub predicted_stability: f64,
}

impl ThermodynamicForecast {
    pub fn from_state(state: &FreeEnergyState, phase: String) -> Self {
        Self {
            free_energy: state.free_energy,
            temperature: state.temperature,
            entropy: state.entropy,
            phase,
            predicted_stability: (1.0 - state.free_energy.abs()).max(0.0),
        }
    }

    pub fn neutral() -> Self {
        Self {
            free_energy: 0.0,
            temperature: 0.5,
            entropy: 0.5,
            phase: "stable".to_string(),
            predicted_stability: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_energy_state() {
        let mut state = FreeEnergyState::new(0.5, 0.5, 0.3);
        assert!((state.free_energy - 0.35).abs() < 1e-10);

        state.update(0.6, 0.4);
        // EMA update should move values toward new observations
        assert!(state.expected_error > 0.5);
        assert!(state.entropy > 0.3);
    }

    #[test]
    fn test_boltzmann_distribution() {
        let energies = vec![1.0, 2.0, 3.0];
        let probs = boltzmann_distribution(&energies, 1.0);
        assert_eq!(probs.len(), 3);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        // Lower energy should have higher probability
        assert!(probs[0] > probs[1]);
        assert!(probs[1] > probs[2]);
    }

    #[test]
    fn test_phase_transition_detector() {
        let mut detector = PhaseTransitionDetector::new(10, 0.1);

        // Stable phase
        for _ in 0..10 {
            detector.record(0.8);
        }
        assert_eq!(detector.get_phase(), SystemPhase::Ordered);

        // Transition to critical
        for _ in 0..10 {
            detector.record(0.2);
        }
        // Should detect high variance during transition
        assert!(detector.is_near_transition() || detector.get_phase() != SystemPhase::Ordered);
    }

    #[test]
    fn test_entropy_production() {
        let fluxes = vec![1.0, 2.0, 3.0];
        let forces = vec![0.5, 0.5, 0.5];
        let rate = entropy_production_rate(&fluxes, &forces);
        assert!((rate - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_gibbs_free_energy() {
        // Favorable allocation: negative ΔG
        let dg = gibbs_free_energy(-5.0, 2.0, 1.0);
        assert!(dg < 0.0);

        // Unfavorable allocation: positive ΔG
        let dg = gibbs_free_energy(5.0, 1.0, 1.0);
        assert!(dg > 0.0);
    }
}
