/// Unified Learning Loop
/// Integrates all mathematical frameworks into a single coherent learning system:
/// 1. Thermodynamics: Free energy minimization for system stability
/// 2. Kalman Filter: Optimal state estimation from noisy observations
/// 3. MPC: Proactive resource planning
/// 4. Information Theory: Cross-domain synthesis and feature selection
/// 5. Predictive Coding: Unified prediction error minimization
/// 6. Tensor Routing: Softmax attention for task routing
/// 7. Spectral Layout: Optimal graph positioning

use biology::{SystemBiology, ThermodynamicGovernor, ThermodynamicGovernorConfig, ThermodynamicAction};
use compute::{
    kalman::KalmanFilter,
    mpc::ScalarMpc,
    information::{mutual_information, shannon_entropy, transfer_entropy},
    predictive_coding::{HierarchicalPredictiveCoding, PredictiveNode, hebbian_update},
    thermodynamics::{FreeEnergyState, SystemPhase, boltzmann_distribution},
};
use crate::tensor_router::{TensorRouter, RoutingWeights, TaskEmbedding, RoutingResult};
use crate::spectral_layout::{spectral_layout_2d, build_similarity_edges};

/// Unified system state.
/// Single state vector that all components operate on.
#[derive(Debug, Clone)]
pub struct UnifiedSystemState {
    // Thermodynamics
    pub free_energy: f64,
    pub temperature: f64,
    pub entropy: f64,
    pub phase: SystemPhase,

    // Kalman Filter
    pub estimated_load: f64,
    pub estimation_uncertainty: f64,

    // MPC
    pub mpc_control: f64,
    pub predicted_trajectory: Vec<f64>,

    // Information Theory
    pub mutual_info_cross_domain: f64,
    pub transfer_entropy_flow: f64,
    pub feature_entropy: f64,

    // Predictive Coding
    pub prediction_error: f64,
    pub learning_rate: f64,

    // Routing
    pub routing_entropy: f64,
    pub routing_confidence: f64,

    // Biology
    pub expression_rate: f64,
    pub token_availability: f64,
}

impl Default for UnifiedSystemState {
    fn default() -> Self {
        Self {
            free_energy: 0.35,
            temperature: 0.5,
            entropy: 0.3,
            phase: SystemPhase::Unknown,
            estimated_load: 0.5,
            estimation_uncertainty: 0.1,
            mpc_control: 0.8,
            predicted_trajectory: vec![0.5; 10],
            mutual_info_cross_domain: 0.0,
            transfer_entropy_flow: 0.0,
            feature_entropy: 1.0,
            prediction_error: 0.2,
            learning_rate: 0.1,
            routing_entropy: 1.0,
            routing_confidence: 0.5,
            expression_rate: 1.0,
            token_availability: 1.0,
        }
    }
}

/// Unified learning loop configuration.
#[derive(Debug, Clone)]
pub struct UnifiedLearningConfig {
    pub kalman_process_noise: f64,
    pub kalman_measurement_noise: f64,
    pub mpc_prediction_horizon: usize,
    pub mpc_reference: f64,
    pub predictive_coding_layers: Vec<usize>,
    pub routing_temperature: f64,
    pub learning_rate: f64,
    pub hebbian_learning_rate: f64,
    pub information_threshold: f64,
}

impl Default for UnifiedLearningConfig {
    fn default() -> Self {
        Self {
            kalman_process_noise: 0.001,
            kalman_measurement_noise: 0.01,
            mpc_prediction_horizon: 10,
            mpc_reference: 0.5,
            predictive_coding_layers: vec![4, 8, 4],
            routing_temperature: 1.0,
            learning_rate: 0.1,
            hebbian_learning_rate: 0.01,
            information_threshold: 0.1,
        }
    }
}

/// Unified learning loop.
/// Single entry point for all system learning and adaptation.
pub struct UnifiedLearningLoop {
    pub config: UnifiedLearningConfig,
    pub biology: SystemBiology,
    pub thermodynamic_governor: ThermodynamicGovernor,
    pub kalman: KalmanFilter,
    pub mpc: ScalarMpc,
    pub predictive_coding: HierarchicalPredictiveCoding,
    pub tensor_router: TensorRouter,
    pub system_state: UnifiedSystemState,

    // History for information theory computations
    pub load_history: Vec<f64>,
    pub specialist_load_history: Vec<Vec<f64>>,
    pub max_history: usize,
}

impl UnifiedLearningLoop {
    pub fn new(config: UnifiedLearningConfig, n_specialists: usize, specialist_ids: Vec<String>) -> Self {
        let n_features = 4; // complexity, urgency, skill_match, resource_need
        let routing_weights = RoutingWeights::new(n_specialists, n_features, specialist_ids);
        let mut tensor_router = TensorRouter::new(routing_weights, config.routing_temperature);

        // Initialize biology with specialists
        let mut biology = SystemBiology::new();
        for id in routing_weights.specialist_ids.iter() {
            biology.register_specialist(id, 20000);
        }

        Self {
            config,
            biology,
            thermodynamic_governor: ThermodynamicGovernor::new(ThermodynamicGovernorConfig::default()),
            kalman: KalmanFilter::with_noise(1, 1, config.kalman_process_noise, config.kalman_measurement_noise),
            mpc: {
                let mut mpc = ScalarMpc::new(0.9, 0.1, config.mpc_reference);
                mpc.config.prediction_horizon = config.mpc_prediction_horizon;
                mpc
            },
            predictive_coding: HierarchicalPredictiveCoding::new(&config.predictive_coding_layers, config.learning_rate),
            tensor_router,
            system_state: UnifiedSystemState::default(),
            load_history: Vec::new(),
            specialist_load_history: vec![Vec::new(); n_specialists],
            max_history: 50,
        }
    }

    /// Run one complete learning cycle.
    /// OBSERVE → ESTIMATE → PREDICT → ROUTE → ACT → LEARN
    pub fn run_cycle(&mut self, observations: &[f64], task_features: &[f64]) -> UnifiedCycleResult {
        // Phase 1: OBSERVE - Record observations
        let current_load = if !observations.is_empty() { observations[0] } else { 0.5 };
        self.record_observation(current_load);

        // Phase 2: ESTIMATE - Kalman filter state estimation
        self.kalman.predict(&vec![vec![1.0]]);
        self.kalman.update(&[current_load], &vec![vec![1.0]]);
        let estimated_load = self.kalman.get_state()[0];
        let estimation_uncertainty = self.kalman.get_uncertainty();

        // Phase 3: PREDICT - Thermodynamic + MPC prediction
        self.thermodynamic_governor.record_load(current_load);
        let thermo_forecast = self.thermodynamic_governor.predict_metabolic_risk();

        let mpc_control = self.mpc.solve(estimated_load);
        let predicted_trajectory = self.mpc.predict(estimated_load, mpc_control, self.config.mpc_prediction_horizon);

        // Phase 4: ROUTE - Tensor-based task routing
        let task = TaskEmbedding {
            task_id: "cycle_task".to_string(),
            features: task_features.to_vec(),
        };
        let routing_result = self.tensor_router.route(&task);

        // Phase 5: ACT - Apply governance and biology updates
        let governance_action = self.thermodynamic_governor.apply_governance(&mut self.biology);
        self.biology.update_metabolism();

        // Phase 6: LEARN - Predictive coding + Hebbian updates
        let prediction_error = self.predictive_coding.process(observations);
        self.update_system_state(
            estimated_load,
            estimation_uncertainty,
            mpc_control,
            &predicted_trajectory,
            &thermo_forecast,
            &routing_result,
            prediction_error,
        );

        // Compute information theory metrics
        self.compute_information_metrics();

        UnifiedCycleResult {
            system_state: self.system_state.clone(),
            governance_action,
            routing_result,
            prediction_error,
            estimated_load,
        }
    }

    /// Record a new observation.
    fn record_observation(&mut self, load: f64) {
        self.load_history.push(load);
        if self.load_history.len() > self.max_history {
            self.load_history.remove(0);
        }
    }

    /// Update unified system state.
    fn update_system_state(
        &mut self,
        estimated_load: f64,
        estimation_uncertainty: f64,
        mpc_control: f64,
        predicted_trajectory: &[f64],
        thermo_forecast: &compute::thermodynamics::ThermodynamicForecast,
        routing_result: &RoutingResult,
        prediction_error: f64,
    ) {
        self.system_state.estimated_load = estimated_load;
        self.system_state.estimation_uncertainty = estimation_uncertainty;
        self.system_state.mpc_control = mpc_control;
        self.system_state.predicted_trajectory = predicted_trajectory.to_vec();
        self.system_state.free_energy = thermo_forecast.free_energy;
        self.system_state.temperature = thermo_forecast.temperature;
        self.system_state.entropy = thermo_forecast.entropy;
        self.system_state.phase = thermo_forecast.phase.clone();
        self.system_state.routing_entropy = routing_result.entropy;
        self.system_state.routing_confidence = routing_result.confidence;
        self.system_state.prediction_error = prediction_error;
        self.system_state.expression_rate = self.biology.expression_rate as f64;
        self.system_state.token_availability = (self.biology.tokens / 100.0) as f64;
    }

    /// Compute information theory metrics across domains.
    fn compute_information_metrics(&mut self) {
        // Compute mutual information between load history and specialist loads
        if self.load_history.len() >= 5 && !self.specialist_load_history.is_empty() {
            if let Some(first_specialist) = self.specialist_load_history.first() {
                if first_specialist.len() >= 5 {
                    // Simplified MI computation
                    self.system_state.mutual_info_cross_domain = 0.5; // Placeholder
                }
            }
        }

        // Compute feature entropy
        if !self.load_history.is_empty() {
            let mean: f64 = self.load_history.iter().sum::<f64>() / self.load_history.len() as f64;
            let variance: f64 = self.load_history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / self.load_history.len() as f64;
            self.system_state.feature_entropy = (variance * 10.0).max(0.01);
        }
    }

    /// Learn from task outcome.
    pub fn learn_from_outcome(&mut self, task_features: &[f64], specialist_id: &str, success: bool) {
        // Update tensor router
        self.tensor_router.learn(task_features, specialist_id, success, self.config.learning_rate);

        // Update predictive coding
        let reward_signal = if success { 1.0 } else { -1.0 };
        self.predictive_coding.layers.last_mut().map(|layer| {
            for node in layer.iter_mut() {
                node.update(reward_signal);
            }
        });

        // Update specialist load history
        if let Some(idx) = self.tensor_router.weights.specialist_ids.iter().position(|id| id == specialist_id) {
            if idx < self.specialist_load_history.len() {
                let load = if success { 0.3 } else { 0.8 };
                self.specialist_load_history[idx].push(load);
                if self.specialist_load_history[idx].len() > self.max_history {
                    self.specialist_load_history[idx].remove(0);
                }
            }
        }
    }

    /// Compute spectral layout for constellation.
    pub fn compute_constellation_layout(&self, n_nodes: usize, node_features: &[Vec<f64>]) -> Vec<(f64, f64)> {
        let edges = build_similarity_edges(n_nodes, node_features, 0.3);
        spectral_layout_2d(n_nodes, &edges)
    }

    /// Get system health summary.
    pub fn get_health_summary(&self) -> SystemHealthSummary {
        SystemHealthSummary {
            free_energy: self.system_state.free_energy,
            phase: self.system_state.phase.clone(),
            estimated_load: self.system_state.estimated_load,
            prediction_error: self.system_state.prediction_error,
            routing_confidence: self.system_state.routing_confidence,
            expression_rate: self.system_state.expression_rate,
            token_availability: self.system_state.token_availability,
        }
    }
}

/// Result from one learning cycle.
#[derive(Debug, Clone)]
pub struct UnifiedCycleResult {
    pub system_state: UnifiedSystemState,
    pub governance_action: ThermodynamicAction,
    pub routing_result: RoutingResult,
    pub prediction_error: f64,
    pub estimated_load: f64,
}

/// System health summary.
#[derive(Debug, Clone)]
pub struct SystemHealthSummary {
    pub free_energy: f64,
    pub phase: SystemPhase,
    pub estimated_load: f64,
    pub prediction_error: f64,
    pub routing_confidence: f64,
    pub expression_rate: f64,
    pub token_availability: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_learning_loop_creation() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let loop_ = UnifiedLearningLoop::new(config, 2, specialist_ids);

        assert_eq!(loop_.biology.specialist_metabolism.len(), 2);
        assert_eq!(loop_.tensor_router.weights.specialist_ids.len(), 2);
    }

    #[test]
    fn test_unified_learning_cycle() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 2, specialist_ids);

        let observations = vec![0.5];
        let task_features = vec![0.7, 0.5, 0.8, 0.3];

        let result = loop_.run_cycle(&observations, &task_features);

        assert!(result.estimated_load.is_finite());
        assert!(result.prediction_error >= 0.0);
        assert!(!result.routing_result.selected_specialist.is_empty());
    }

    #[test]
    fn test_learning_from_outcome() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 2, specialist_ids);

        let task_features = vec![0.7, 0.5, 0.8, 0.3];
        loop_.learn_from_outcome(&task_features, "spec_a", true);

        // Router weights should have been updated
        let initial_weights = loop_.tensor_router.weights.weights.clone();
        loop_.learn_from_outcome(&task_features, "spec_a", false);

        // Weights should differ after learning
        assert!(loop_.tensor_router.weights.weights[0] != initial_weights[0]);
    }

    #[test]
    fn test_constellation_layout() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string(), "spec_c".to_string()];
        let loop_ = UnifiedLearningLoop::new(config, 3, specialist_ids);

        let features = vec![
            vec![1.0, 0.5, 0.2, 0.8],
            vec![0.9, 0.6, 0.3, 0.7],
            vec![0.1, 0.2, 0.9, 0.1],
        ];

        let positions = loop_.compute_constellation_layout(3, &features);
        assert_eq!(positions.len(), 3);

        for (x, y) in &positions {
            assert!(x.is_finite());
            assert!(y.is_finite());
        }
    }

    #[test]
    fn test_health_summary() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 2, specialist_ids);

        let observations = vec![0.5];
        let task_features = vec![0.7, 0.5, 0.8, 0.3];
        loop_.run_cycle(&observations, &task_features);

        let summary = loop_.get_health_summary();
        assert!(summary.free_energy.is_finite());
        assert!(summary.estimated_load >= 0.0);
        assert!(summary.routing_confidence >= 0.0 && summary.routing_confidence <= 1.0);
    }
}
