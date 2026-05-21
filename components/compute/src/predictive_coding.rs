/// Predictive Coding framework for unified learning.
/// Implements prediction error minimization across all system components.
/// Every component becomes a prediction machine that minimizes surprise.

/// Predictive Coding node.
/// Each node maintains a prediction and updates based on prediction error.
/// ε = observation - prediction
/// Δprediction = learning_rate * ε
#[derive(Debug, Clone)]
pub struct PredictiveNode {
    pub prediction: f64,
    pub precision: f64,      // Inverse variance (confidence in prediction)
    pub learning_rate: f64,
    pub prediction_error: f64,
    pub total_error: f64,    // Accumulated error for monitoring
    pub update_count: u64,
}

impl PredictiveNode {
    pub fn new(initial_prediction: f64, precision: f64, learning_rate: f64) -> Self {
        Self {
            prediction: initial_prediction,
            precision,
            learning_rate,
            prediction_error: 0.0,
            total_error: 0.0,
            update_count: 0,
        }
    }

    /// Update prediction based on new observation.
    /// Returns prediction error.
    pub fn update(&mut self, observation: f64) -> f64 {
        // Compute prediction error
        self.prediction_error = observation - self.prediction;

        // Weighted update by precision
        let weighted_error = self.prediction_error * self.precision;
        self.prediction += self.learning_rate * weighted_error;

        // Track statistics
        self.total_error += self.prediction_error.abs();
        self.update_count += 1;

        self.prediction_error
    }

    /// Get mean absolute error.
    pub fn mean_absolute_error(&self) -> f64 {
        if self.update_count == 0 {
            return 0.0;
        }
        self.total_error / self.update_count as f64
    }

    /// Adjust precision based on recent prediction accuracy.
    pub fn adapt_precision(&mut self, decay: f64) {
        // Higher error -> lower precision (less confident)
        let error_ratio = self.prediction_error.abs() / (self.prediction.abs() + 1e-10);
        self.precision = (self.precision * (1.0 - decay) + (1.0 - error_ratio) * decay).clamp(0.01, 10.0);
    }
}

/// Hierarchical Predictive Coding network.
/// Multiple layers of predictive nodes with top-down predictions and bottom-up errors.
#[derive(Debug, Clone)]
pub struct HierarchicalPredictiveCoding {
    pub layers: Vec<Vec<PredictiveNode>>,
    pub top_down_weights: Vec<Vec<Vec<f64>>>,  // weights[layer][node][prev_node]
    pub bottom_up_weights: Vec<Vec<Vec<f64>>>, // weights[layer][node][next_node]
    pub learning_rate: f64,
}

impl HierarchicalPredictiveCoding {
    pub fn new(layer_sizes: &[usize], learning_rate: f64) -> Self {
        let mut layers = Vec::new();
        for &size in layer_sizes {
            let layer: Vec<PredictiveNode> = (0..size)
                .map(|_| PredictiveNode::new(0.5, 1.0, learning_rate))
                .collect();
            layers.push(layer);
        }

        // Initialize weights (random would be better, but zeros for now)
        let mut top_down_weights = Vec::new();
        let mut bottom_up_weights = Vec::new();

        for i in 0..layer_sizes.len() {
            if i > 0 {
                let td: Vec<Vec<f64>> = (0..layer_sizes[i])
                    .map(|_| vec![1.0 / layer_sizes[i - 1] as f64; layer_sizes[i - 1]])
                    .collect();
                top_down_weights.push(td);
            }
            if i < layer_sizes.len() - 1 {
                let bu: Vec<Vec<f64>> = (0..layer_sizes[i])
                    .map(|_| vec![1.0 / layer_sizes[i + 1] as f64; layer_sizes[i + 1]])
                    .collect();
                bottom_up_weights.push(bu);
            }
        }

        Self {
            layers,
            top_down_weights,
            bottom_up_weights,
            learning_rate,
        }
    }

    /// Process observation through all layers.
    /// Returns total prediction error across all layers.
    pub fn process(&mut self, observation: &[f64]) -> f64 {
        if self.layers.is_empty() {
            return 0.0;
        }

        // Set input layer observation
        for (i, node) in self.layers[0].iter_mut().enumerate() {
            if i < observation.len() {
                node.update(observation[i]);
            }
        }

        // Propagate through layers
        let mut total_error = 0.0;

        for layer_idx in 1..self.layers.len() {
            // Collect predictions from previous layer first to avoid borrow conflict
            let prev_predictions: Vec<f64> = self.layers[layer_idx - 1]
                .iter()
                .map(|n| n.prediction)
                .collect();

            let current_layer = &mut self.layers[layer_idx];

            for (node_idx, node) in current_layer.iter_mut().enumerate() {
                if layer_idx - 1 < self.top_down_weights.len() && node_idx < self.top_down_weights[layer_idx - 1].len() {
                    // Top-down prediction
                    let weights = &self.top_down_weights[layer_idx - 1][node_idx];
                    let prediction: f64 = prev_predictions
                        .iter()
                        .zip(weights.iter())
                        .map(|(&prev_pred, &w)| prev_pred * w)
                        .sum();

                    // Use previous prediction as observation for this layer
                    let observation = prediction;
                    let error = node.update(observation);
                    total_error += error.abs();
                }
            }
        }

        total_error
    }

    /// Get prediction from top layer.
    pub fn get_top_prediction(&self) -> Vec<f64> {
        if self.layers.is_empty() {
            return vec![];
        }
        self.layers.last().unwrap().iter().map(|n| n.prediction).collect()
    }

    /// Get prediction errors from all layers.
    pub fn get_prediction_errors(&self) -> Vec<Vec<f64>> {
        self.layers.iter().map(|layer| layer.iter().map(|n| n.prediction_error).collect()).collect()
    }
}

/// Hebbian learning rule.
/// Δw = η * pre * post
/// "Neurons that fire together, wire together"
pub fn hebbian_update(weight: f64, pre_activity: f64, post_activity: f64, learning_rate: f64) -> f64 {
    weight + learning_rate * pre_activity * post_activity
}

/// Spike-Timing-Dependent Plasticity (STDP).
/// Strengthens connections when pre fires before post.
/// Weakens connections when post fires before pre.
pub fn stdp_update(
    weight: f64,
    pre_spike_time: f64,
    post_spike_time: f64,
    learning_rate: f64,
    time_constant: f64,
) -> f64 {
    let dt = post_spike_time - pre_spike_time;

    if dt > 0.0 {
        // Pre before post: potentiation
        weight + learning_rate * (-dt / time_constant).exp()
    } else {
        // Post before pre: depression
        weight - learning_rate * (dt / time_constant).exp()
    }
}

/// Neuromodulated learning.
/// Global reward signal modulates local learning rates.
/// Δw = η * reward * δ * pre
pub fn neuromodulated_hebbian(
    weight: f64,
    pre_activity: f64,
    post_activity: f64,
    reward: f64,          // Global reward signal (dopamine-like)
    base_learning_rate: f64,
) -> f64 {
    let modulated_lr = base_learning_rate * (1.0 + reward);
    hebbian_update(weight, pre_activity, post_activity, modulated_lr)
}

/// Predictive Coding controller.
/// Uses prediction error to drive action selection.
#[derive(Debug, Clone)]
pub struct PredictiveController {
    pub state_predictor: PredictiveNode,
    pub reward_predictor: PredictiveNode,
    pub action_values: Vec<f64>,
    pub exploration_rate: f64,
}

impl PredictiveController {
    pub fn new(n_actions: usize, exploration_rate: f64) -> Self {
        Self {
            state_predictor: PredictiveNode::new(0.5, 1.0, 0.1),
            reward_predictor: PredictiveNode::new(0.0, 1.0, 0.1),
            action_values: vec![0.5; n_actions],
            exploration_rate,
        }
    }

    /// Select action based on prediction errors.
    /// High prediction error -> explore more.
    pub fn select_action(&self, rng: &mut impl rand::Rng) -> usize {
        let total_error = self.state_predictor.mean_absolute_error()
            + self.reward_predictor.mean_absolute_error();

        // Boltzmann exploration based on prediction error
        if rng.gen::<f64>() < self.exploration_rate * total_error {
            // Explore: random action
            rng.gen_range(0..self.action_values.len())
        } else {
            // Exploit: best action
            self.action_values
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0)
        }
    }

    /// Update action values based on reward prediction error.
    pub fn update(&mut self, state: f64, reward: f64, action: usize) {
        self.state_predictor.update(state);
        let reward_error = self.reward_predictor.update(reward);

        // Update action value (Q-learning style)
        let alpha = 0.1;
        self.action_values[action] += alpha * reward_error;
    }
}

/// Free Energy Principle agent.
/// Minimizes variational free energy through action and perception.
#[derive(Debug, Clone)]
pub struct FreeEnergyAgent {
    pub generative_model: HierarchicalPredictiveCoding,
    pub action_policy: PredictiveController,
    pub temperature: f64,
    pub target_free_energy: f64,
}

impl FreeEnergyAgent {
    pub fn new(
        layer_sizes: &[usize],
        n_actions: usize,
        temperature: f64,
        target_free_energy: f64,
    ) -> Self {
        Self {
            generative_model: HierarchicalPredictiveCoding::new(layer_sizes, 0.1),
            action_policy: PredictiveController::new(n_actions, 0.1),
            temperature,
            target_free_energy,
        }
    }

    /// Perceive: update generative model with observation.
    pub fn perceive(&mut self, observation: &[f64]) -> f64 {
        self.generative_model.process(observation)
    }

    /// Act: select action to minimize expected free energy.
    pub fn act(&mut self, rng: &mut impl rand::Rng) -> usize {
        self.action_policy.select_action(rng)
    }

    /// Learn: update model based on reward.
    pub fn learn(&mut self, state: f64, reward: f64, action: usize) {
        self.action_policy.update(state, reward, action);
    }

    /// Compute expected free energy of an action.
    /// F = D_KL(Q(s'|a) || P(s')) - E_Q[ln P(o'|s')]
    /// First term: risk (divergence from preferred states)
    /// Second term: ambiguity (expected uncertainty)
    pub fn expected_free_energy(&self, action: usize, preferred_state: f64) -> f64 {
        // Simplified: risk + ambiguity
        let predicted_state = self.generative_model.get_top_prediction();
        if predicted_state.is_empty() {
            return f64::INFINITY;
        }

        let risk = (predicted_state[0] - preferred_state).powi(2);
        let ambiguity = self.generative_model
            .get_prediction_errors()
            .iter()
            .flat_map(|layer| layer.iter())
            .map(|e| e.abs())
            .sum::<f64>();

        risk - self.temperature * ambiguity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_predictive_node_update() {
        let mut node = PredictiveNode::new(0.5, 1.0, 0.1);
        let error = node.update(0.7);
        assert!((error - 0.2).abs() < 1e-10);
        assert!(node.prediction > 0.5); // Should move toward observation
    }

    #[test]
    fn test_hierarchical_predictive_coding() {
        let mut network = HierarchicalPredictiveCoding::new(&[3, 4, 2], 0.1);
        let observation = vec![0.5, 0.6, 0.7];
        let total_error = network.process(&observation);
        assert!(total_error >= 0.0);
    }

    #[test]
    fn test_hebbian_learning() {
        let weight = 0.5;
        let new_weight = hebbian_update(weight, 0.8, 0.6, 0.1);
        assert!(new_weight > weight); // Should increase
    }

    #[test]
    fn test_stdp() {
        let weight = 0.5;
        // Pre before post: potentiation
        let w1 = stdp_update(weight, 0.0, 0.1, 0.1, 0.02);
        assert!(w1 > weight);

        // Post before pre: depression
        let w2 = stdp_update(weight, 0.1, 0.0, 0.1, 0.02);
        assert!(w2 < weight);
    }

    #[test]
    fn test_predictive_controller() {
        let mut controller = PredictiveController::new(3, 0.2);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        controller.update(0.5, 1.0, 0);
        controller.update(0.6, 0.5, 1);

        // Action 0 should have higher value due to higher reward
        assert!(controller.action_values[0] > controller.action_values[1]);
    }

    #[test]
    fn test_free_energy_agent() {
        let mut agent = FreeEnergyAgent::new(&[2, 3, 1], 3, 0.5, 0.1);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let observation = vec![0.5, 0.6];
        let error = agent.perceive(&observation);
        assert!(error >= 0.0);

        let action = agent.act(&mut rng);
        assert!(action < 3);

        agent.learn(0.5, 1.0, action);
    }
}
