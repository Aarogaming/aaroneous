// Thermodynamic Metabolic Governor
// Uses Free Energy Principle and thermodynamics for optimal resource allocation
// Replaces Monte Carlo prediction with variational free energy minimization

use crate::biology::SystemBiology;
use compute::thermodynamics::{FreeEnergyState, PhaseTransitionDetector, SystemPhase, boltzmann_distribution, gibbs_free_energy};
use compute::kalman::KalmanFilter;
use compute::mpc::ScalarMpc;

/// Configuration for the thermodynamic metabolic governor
#[derive(Debug, Clone)]
pub struct ThermodynamicGovernorConfig {
    pub target_free_energy: f64,      // Target F for system stability
    pub temperature_learning_rate: f64, // Learning rate for temperature adjustment
    pub process_noise: f64,           // Kalman filter process noise
    pub measurement_noise: f64,       // Kalman filter measurement noise
    pub mpc_prediction_horizon: usize, // MPC prediction horizon
    pub phase_transition_threshold: f64, // Critical susceptibility threshold
    pub entropy_weight: f64,          // Weight for entropy in free energy
    pub error_weight: f64,            // Weight for prediction error in free energy
}

impl Default for ThermodynamicGovernorConfig {
    fn default() -> Self {
        Self {
            target_free_energy: 0.1,
            temperature_learning_rate: 0.05,
            process_noise: 0.001,
            measurement_noise: 0.01,
            mpc_prediction_horizon: 10,
            phase_transition_threshold: 0.15,
            entropy_weight: 1.0,
            error_weight: 1.0,
        }
    }
}

/// Thermodynamic metabolic governor
pub struct ThermodynamicGovernor {
    pub config: ThermodynamicGovernorConfig,
    pub free_energy: FreeEnergyState,
    pub phase_detector: PhaseTransitionDetector,
    pub kalman: KalmanFilter,
    pub mpc: ScalarMpc,
    pub historical_load: Vec<f64>,
    pub max_history: usize,
}

impl ThermodynamicGovernor {
    pub fn new(config: ThermodynamicGovernorConfig) -> Self {
        let mut free_energy = FreeEnergyState::new(0.5, 0.5, 0.3);
        free_energy.temperature = 0.5; // Initial exploration rate

        let mut mpc = ScalarMpc::new(0.9, 0.1, config.target_free_energy);
        mpc.config.prediction_horizon = config.mpc_prediction_horizon;

        Self {
            config,
            free_energy,
            phase_detector: PhaseTransitionDetector::new(20, config.phase_transition_threshold),
            kalman: KalmanFilter::with_noise(1, 1, config.process_noise, config.measurement_noise),
            mpc,
            historical_load: Vec::new(),
            max_history: 50,
        }
    }

    /// Record a new metabolic load measurement
    pub fn record_load(&mut self, load: f64) {
        self.historical_load.push(load);
        if self.historical_load.len() > self.max_history {
            self.historical_load.remove(0);
        }

        // Update Kalman filter with new measurement
        let f = vec![vec![1.0]]; // Identity transition
        self.kalman.predict(&f);
        self.kalman.update(&[load], &vec![vec![1.0]]);

        // Update phase detector
        self.phase_detector.record(load);
    }

    /// Compute thermodynamic forecast using Free Energy Principle
    pub fn predict_metabolic_risk(&mut self) -> ThermodynamicForecast {
        if self.historical_load.is_empty() {
            return ThermodynamicForecast::default();
        }

        // Get smoothed state estimate from Kalman filter
        let estimated_load = self.kalman.get_state()[0];
        let estimation_uncertainty = self.kalman.get_uncertainty();

        // Compute prediction error (energy)
        let prediction_error = (estimated_load - self.config.target_free_energy).abs() * self.config.error_weight;

        // Compute entropy from historical variance
        let entropy = if self.historical_load.len() >= 2 {
            let mean: f64 = self.historical_load.iter().sum::<f64>() / self.historical_load.len() as f64;
            let variance: f64 = self.historical_load
                .iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>()
                / self.historical_load.len() as f64;
            (variance * self.config.entropy_weight).max(0.01)
        } else {
            0.3
        };

        // Update free energy state
        self.free_energy.update(prediction_error, entropy);
        self.free_energy.adjust_temperature(
            self.config.temperature_learning_rate,
            self.config.target_free_energy,
        );

        // Compute risk score from free energy
        let risk_score = self.free_energy.free_energy.clamp(0.0, 1.0);

        // Get phase classification
        let phase = self.phase_detector.get_phase();

        // MPC-optimal expression rate
        let mpc_rate = self.mpc.solve(estimated_load);

        // Boltzmann distribution over specialist priorities
        let specialist_energies: Vec<f64> = self.historical_load
            .iter()
            .rev()
            .take(5)
            .map(|&l| l as f64)
            .collect();
        let specialist_probs = boltzmann_distribution(&specialist_energies, self.free_energy.temperature);

        ThermodynamicForecast {
            predicted_mean: estimated_load,
            prediction_uncertainty: estimation_uncertainty,
            free_energy: self.free_energy.free_energy,
            temperature: self.free_energy.temperature,
            entropy: self.free_energy.entropy,
            risk_score,
            phase,
            recommended_expression_rate: mpc_rate as f32,
            specialist_probabilities: specialist_probs,
        }
    }

    /// Apply governor decision to the biology system
    pub fn apply_governance(&mut self, biology: &mut SystemBiology) -> ThermodynamicAction {
        let forecast = self.predict_metabolic_risk();

        let action = match forecast.phase {
            SystemPhase::Critical => {
                // Near phase transition: cautious adjustment
                let old_rate = biology.expression_rate;
                let new_rate = (biology.expression_rate * 0.85).max(0.2);
                biology.set_expression_rate(new_rate);
                ThermodynamicAction::CriticalAdjustment {
                    old_rate,
                    new_rate,
                    forecast: forecast.clone(),
                }
            }
            SystemPhase::Disordered => {
                // High exploration needed: increase temperature, reduce rate
                let old_rate = biology.expression_rate;
                let new_rate = (biology.expression_rate * 0.7).max(0.1);
                biology.set_expression_rate(new_rate);
                ThermodynamicAction::ExplorationMode {
                    old_rate,
                    new_rate,
                    forecast: forecast.clone(),
                }
            }
            SystemPhase::Ordered => {
                // Stable: exploit current configuration
                if forecast.risk_score < 0.3 && biology.expression_rate < 1.0 {
                    let old_rate = biology.expression_rate;
                    let new_rate = (biology.expression_rate + 0.05).min(1.0);
                    biology.set_expression_rate(new_rate);
                    ThermodynamicAction::ExploitationRecovery {
                        old_rate,
                        new_rate,
                        forecast: forecast.clone(),
                    }
                } else {
                    ThermodynamicAction::StableExploitation { forecast: forecast.clone() }
                }
            }
            SystemPhase::Mixed => {
                // Balanced: MPC-optimal control
                let old_rate = biology.expression_rate;
                let new_rate = forecast.recommended_expression_rate;
                biology.set_expression_rate(new_rate);
                ThermodynamicAction::MpcControl {
                    old_rate,
                    new_rate,
                    forecast: forecast.clone(),
                }
            }
            SystemPhase::Unknown => {
                ThermodynamicAction::InsufficientData { forecast: forecast.clone() }
            }
        };

        action
    }

    /// Compute Gibbs free energy for a resource allocation decision.
    /// Returns negative value if allocation is thermodynamically favorable.
    pub fn evaluate_allocation(
        &self,
        energy_cost: f64,    // Tokens required
        uncertainty_reduction: f64, // Expected entropy reduction
    ) -> f64 {
        gibbs_free_energy(
            energy_cost,
            uncertainty_reduction,
            self.free_energy.temperature,
        )
    }
}

/// Forecast result from thermodynamic prediction
#[derive(Debug, Clone)]
pub struct ThermodynamicForecast {
    pub predicted_mean: f64,
    pub prediction_uncertainty: f64,
    pub free_energy: f64,
    pub temperature: f64,           // Exploration/exploitation balance
    pub entropy: f64,               // System uncertainty
    pub risk_score: f64,            // 0.0-1.0, based on free energy
    pub phase: SystemPhase,         // Current system phase
    pub recommended_expression_rate: f32,
    pub specialist_probabilities: Vec<f64>, // Boltzmann distribution
}

impl Default for ThermodynamicForecast {
    fn default() -> Self {
        Self {
            predicted_mean: 0.5,
            prediction_uncertainty: 0.1,
            free_energy: 0.35,
            temperature: 0.5,
            entropy: 0.3,
            risk_score: 0.5,
            phase: SystemPhase::Unknown,
            recommended_expression_rate: 0.8,
            specialist_probabilities: vec![0.2, 0.2, 0.2, 0.2, 0.2],
        }
    }
}

/// Action taken by the thermodynamic governor
#[derive(Debug, Clone)]
pub enum ThermodynamicAction {
    CriticalAdjustment {
        old_rate: f32,
        new_rate: f32,
        forecast: ThermodynamicForecast,
    },
    ExplorationMode {
        old_rate: f32,
        new_rate: f32,
        forecast: ThermodynamicForecast,
    },
    ExploitationRecovery {
        old_rate: f32,
        new_rate: f32,
        forecast: ThermodynamicForecast,
    },
    StableExploitation {
        forecast: ThermodynamicForecast,
    },
    MpcControl {
        old_rate: f32,
        new_rate: f32,
        forecast: ThermodynamicForecast,
    },
    InsufficientData {
        forecast: ThermodynamicForecast,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biology::SystemBiology;

    #[test]
    fn test_governor_records_load() {
        let mut governor = ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default());
        governor.record_load(0.5);
        governor.record_load(0.6);
        governor.record_load(0.7);
        assert_eq!(governor.historical_load.len(), 3);
    }

    #[test]
    fn test_governor_predicts_risk() {
        let mut governor = ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default());
        for _ in 0..10 {
            governor.record_load(0.5);
        }
        let forecast = governor.predict_metabolic_risk();
        assert!(forecast.predicted_mean > 0.0);
        assert!(forecast.risk_score >= 0.0 && forecast.risk_score <= 1.0);
    }

    #[test]
    fn test_governor_phase_transitions() {
        let mut governor = ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default());
        
        // Stable phase
        for _ in 0..20 {
            governor.record_load(0.8);
        }
        assert_eq!(governor.phase_detector.get_phase(), SystemPhase::Ordered);

        // Transition to disordered
        for _ in 0..20 {
            governor.record_load(0.2);
        }
        // Should detect transition or disordered phase
        let phase = governor.phase_detector.get_phase();
        assert!(phase == SystemPhase::Disordered || phase == SystemPhase::Critical);
    }

    #[test]
    fn test_governor_apply_governance() {
        let mut governor = ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default());
        for _ in 0..20 {
            governor.record_load(0.9);
        }
        
        let mut biology = SystemBiology::new();
        let action = governor.apply_governance(&mut biology);
        
        match action {
            ThermodynamicAction::ExplorationMode { new_rate, .. } |
            ThermodynamicAction::CriticalAdjustment { new_rate, .. } |
            ThermodynamicAction::MpcControl { new_rate, .. } => {
                assert!(new_rate < 1.0);
            }
            _ => {} // Could be stable if variance is low
        }
    }

    #[test]
    fn test_evaluate_allocation() {
        let governor = ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default());
        
        // Favorable allocation: low cost, high uncertainty reduction
        let dg = governor.evaluate_allocation(-2.0, 1.0);
        assert!(dg < 0.0);

        // Unfavorable allocation: high cost, low uncertainty reduction
        let dg = governor.evaluate_allocation(5.0, 0.1);
        assert!(dg > 0.0);
    }
}
