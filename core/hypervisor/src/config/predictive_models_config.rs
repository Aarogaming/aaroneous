/// Predictive Models Configuration
///
/// Externalizes Kalman filter and HMM parameters for runtime tuning
/// without recompilation.
///
/// This module provides configuration structs for:
/// - Kalman filter parameters (process noise, measurement noise, covariance)
/// - HMM parameters (states, symbols, transition matrix, emission matrix)
/// - Predictive model tuning (confidence thresholds, update rates)
/// - Observability configuration (telemetry, tracing, metrics)
/// - Runtime tuning (dynamic updates, persistence)
use serde::{Deserialize, Serialize};

/// Kalman filter configuration
///
/// Controls the Kalman filter behavior for state estimation.
/// Process noise (q) controls how much the filter trusts the model.
/// Measurement noise (r) controls how much the filter trusts measurements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanConfig {
    /// Process noise variance (q)
    /// Lower = trusts model more, Higher = trusts measurements more
    /// Thermal prediction: 0.001 - 0.1
    /// Load prediction: 0.0001 - 0.01
    /// Token prediction: 0.001 - 0.05
    pub process_noise_variance: f32,

    /// Measurement noise variance (r)
    /// Lower = trusts measurements more, Higher = trusts model more
    /// Thermal prediction: 0.01 - 1.0
    /// Load prediction: 0.001 - 0.1
    /// Token prediction: 0.01 - 0.5
    pub measurement_noise_variance: f32,

    /// Initial covariance matrix (P0)
    /// [position_uncertainty, velocity_uncertainty]
    /// [0, 0]
    pub initial_covariance: [f32; 4],

    /// Initial state - starting position estimate
    pub initial_position: f32,

    /// Initial velocity estimate
    pub initial_velocity: f32,
}

impl Default for KalmanConfig {
    fn default() -> Self {
        Self {
            process_noise_variance: 0.01,
            measurement_noise_variance: 0.1,
            initial_covariance: [1.0, 0.0, 0.0, 1.0],
            initial_position: 0.0,
            initial_velocity: 0.0,
        }
    }
}

/// Hidden Markov Model configuration
///
/// Controls the HMM behavior for intent prediction.
/// Defines the number of hidden states, observable symbols,
/// and the transition/emission matrices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HMMConfig {
    /// Number of hidden states
    /// 2 states: idle/active
    /// 3 states: idle/working/thinking
    /// 4 states: idle/working/thinking/creating
    pub num_states: usize,

    /// Number of observable symbols
    /// 2 symbols: low/high activity
    /// 3 symbols: low/medium/high activity
    pub num_symbols: usize,

    /// Initial state probabilities
    /// [prob_state_0, prob_state_1, ...]
    /// Sum must equal 1.0
    pub initial_state_probabilities: Vec<f64>,

    /// Transition matrix
    /// [from_state_0_to_state_0, from_state_0_to_state_1, ...]
    /// [from_state_1_to_state_0, from_state_1_to_state_1, ...]
    /// Each row must sum to 1.0
    pub transition_matrix: Vec<f64>,

    /// Emission matrix
    /// [state_0_emit_symbol_0, state_0_emit_symbol_1, ...]
    /// [state_1_emit_symbol_0, state_1_emit_symbol_1, ...]
    /// Each row must sum to 1.0
    pub emission_matrix: Vec<f64>,
}

impl Default for HMMConfig {
    fn default() -> Self {
        Self {
            num_states: 2,
            num_symbols: 2,
            initial_state_probabilities: vec![0.5, 0.5],
            transition_matrix: vec![0.8, 0.2, 0.2, 0.8],
            emission_matrix: vec![0.9, 0.1, 0.1, 0.9],
        }
    }
}

/// Predictive model tuning configuration
///
/// Controls the behavior of predictive models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveConfig {
    /// Enable/disable predictive models
    pub enabled: bool,

    /// Thermal prediction tuning
    pub thermal: ThermalConfig,

    /// Load prediction tuning
    pub load: LoadConfig,

    /// Token prediction tuning
    pub token: TokenConfig,

    /// Intent prediction tuning
    pub intent: IntentConfig,
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            thermal: ThermalConfig::default(),
            load: LoadConfig::default(),
            token: TokenConfig::default(),
            intent: IntentConfig::default(),
        }
    }
}

/// Thermal prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalConfig {
    /// Enable thermal prediction
    pub enabled: bool,
    /// Thermal prediction confidence threshold
    pub confidence_threshold: f64,
    /// Thermal prediction update rate (seconds)
    pub update_rate: f64,
}

impl Default for ThermalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            confidence_threshold: 0.8,
            update_rate: 1.0,
        }
    }
}

/// Load prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadConfig {
    /// Enable load prediction
    pub enabled: bool,
    /// Load prediction confidence threshold
    pub confidence_threshold: f64,
    /// Load prediction update rate (seconds)
    pub update_rate: f64,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            confidence_threshold: 0.8,
            update_rate: 1.0,
        }
    }
}

/// Token prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Enable token prediction
    pub enabled: bool,
    /// Token prediction confidence threshold
    pub confidence_threshold: f64,
    /// Token prediction update rate (seconds)
    pub update_rate: f64,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            confidence_threshold: 0.8,
            update_rate: 1.0,
        }
    }
}

/// Intent prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentConfig {
    /// Enable intent prediction
    pub enabled: bool,
    /// Intent prediction confidence threshold
    pub confidence_threshold: f64,
    /// Intent prediction update rate (seconds)
    pub update_rate: f64,
}

impl Default for IntentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            confidence_threshold: 0.7,
            update_rate: 1.0,
        }
    }
}

/// Observability configuration
///
/// Controls telemetry, tracing, and metrics collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable predictive telemetry logging
    pub telemetry_enabled: bool,
    /// Telemetry log level
    pub telemetry_log_level: String,
    /// Enable tracing spans
    pub tracing_enabled: bool,
    /// Enable metrics collection
    pub metrics_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            telemetry_enabled: true,
            telemetry_log_level: "debug".to_string(),
            tracing_enabled: true,
            metrics_enabled: true,
        }
    }
}

/// Runtime tuning configuration
///
/// Controls dynamic parameter updates and persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Enable runtime parameter updates
    pub dynamic_tuning_enabled: bool,
    /// Parameter update interval (seconds)
    pub parameter_update_interval: f64,
    /// Enable parameter persistence
    pub parameter_persistence_enabled: bool,
    /// Parameter persistence path
    pub parameter_persistence_path: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            dynamic_tuning_enabled: true,
            parameter_update_interval: 60.0,
            parameter_persistence_enabled: true,
            parameter_persistence_path: "config/predictive_models.json".to_string(),
        }
    }
}

/// Main predictive models configuration
///
/// Combines all predictive model configurations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PredictiveModelsConfig {
    /// Kalman filter configuration
    pub kalman: KalmanConfig,

    /// HMM configuration
    pub hmm: HMMConfig,

    /// Predictive model tuning
    pub predictive: PredictiveConfig,

    /// Observability configuration
    pub observability: ObservabilityConfig,

    /// Runtime tuning configuration
    pub runtime: RuntimeConfig,
}

impl PredictiveModelsConfig {
    /// Create a new configuration with custom parameters
    pub fn new(
        kalman: KalmanConfig,
        hmm: HMMConfig,
        predictive: PredictiveConfig,
        observability: ObservabilityConfig,
        runtime: RuntimeConfig,
    ) -> Self {
        Self {
            kalman,
            hmm,
            predictive,
            observability,
            runtime,
        }
    }

    /// Get the Kalman filter process noise variance
    pub fn process_noise_variance(&self) -> f32 {
        self.kalman.process_noise_variance
    }

    /// Get the Kalman filter measurement noise variance
    pub fn measurement_noise_variance(&self) -> f32 {
        self.kalman.measurement_noise_variance
    }

    /// Get the HMM number of states
    pub fn num_states(&self) -> usize {
        self.hmm.num_states
    }

    /// Get the HMM number of symbols
    pub fn num_symbols(&self) -> usize {
        self.hmm.num_symbols
    }

    /// Get the HMM initial state probabilities
    pub fn initial_state_probabilities(&self) -> &[f64] {
        &self.hmm.initial_state_probabilities
    }

    /// Get the HMM transition matrix
    pub fn transition_matrix(&self) -> &[f64] {
        &self.hmm.transition_matrix
    }

    /// Get the HMM emission matrix
    pub fn emission_matrix(&self) -> &[f64] {
        &self.hmm.emission_matrix
    }

    /// Get the thermal prediction configuration
    pub fn thermal(&self) -> &ThermalConfig {
        &self.predictive.thermal
    }

    /// Get the load prediction configuration
    pub fn load(&self) -> &LoadConfig {
        &self.predictive.load
    }

    /// Get the token prediction configuration
    pub fn token(&self) -> &TokenConfig {
        &self.predictive.token
    }

    /// Get the intent prediction configuration
    pub fn intent(&self) -> &IntentConfig {
        &self.predictive.intent
    }

    /// Get the observability configuration
    pub fn observability(&self) -> &ObservabilityConfig {
        &self.observability
    }

    /// Get the runtime configuration
    pub fn runtime(&self) -> &RuntimeConfig {
        &self.runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PredictiveModelsConfig::default();
        assert_eq!(config.kalman.process_noise_variance, 0.01);
        assert_eq!(config.kalman.measurement_noise_variance, 0.1);
        assert_eq!(config.hmm.num_states, 2);
        assert_eq!(config.hmm.num_symbols, 2);
        assert_eq!(config.hmm.initial_state_probabilities, vec![0.5, 0.5]);
        assert_eq!(config.hmm.transition_matrix, vec![0.8, 0.2, 0.2, 0.8]);
        assert_eq!(config.hmm.emission_matrix, vec![0.9, 0.1, 0.1, 0.9]);
    }

    #[test]
    fn test_custom_config() {
        let kalman = KalmanConfig {
            process_noise_variance: 0.001,
            measurement_noise_variance: 0.01,
            initial_covariance: [0.5, 0.0, 0.0, 0.5],
            initial_position: 10.0,
            initial_velocity: 0.0,
        };

        let hmm = HMMConfig {
            num_states: 3,
            num_symbols: 3,
            initial_state_probabilities: vec![0.3, 0.4, 0.3],
            transition_matrix: vec![0.7, 0.2, 0.1, 0.2, 0.7, 0.1, 0.1, 0.2],
            emission_matrix: vec![0.8, 0.1, 0.1, 0.1, 0.8, 0.1],
        };

        let predictive = PredictiveConfig {
            enabled: true,
            thermal: ThermalConfig {
                enabled: true,
                confidence_threshold: 0.8,
                update_rate: 1.0,
            },
            load: LoadConfig {
                enabled: true,
                confidence_threshold: 0.8,
                update_rate: 1.0,
            },
            token: TokenConfig {
                enabled: true,
                confidence_threshold: 0.8,
                update_rate: 1.0,
            },
            intent: IntentConfig {
                enabled: true,
                confidence_threshold: 0.7,
                update_rate: 1.0,
            },
        };

        let observability = ObservabilityConfig {
            telemetry_enabled: true,
            telemetry_log_level: "debug".to_string(),
            tracing_enabled: true,
            metrics_enabled: true,
        };

        let runtime = RuntimeConfig {
            dynamic_tuning_enabled: true,
            parameter_update_interval: 60.0,
            parameter_persistence_enabled: true,
            parameter_persistence_path: "config/predictive_models.json".to_string(),
        };

        let config = PredictiveModelsConfig::new(kalman, hmm, predictive, observability, runtime);

        assert_eq!(config.process_noise_variance(), 0.001);
        assert_eq!(config.measurement_noise_variance(), 0.01);
        assert_eq!(config.num_states(), 3);
        assert_eq!(config.num_symbols(), 3);
        assert_eq!(config.thermal().enabled, true);
        assert_eq!(config.thermal().confidence_threshold, 0.8);
    }
}
