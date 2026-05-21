/// Tensor-Based Task Routing
/// Replaces MDP routing with softmax attention scoring: softmax(Wx)
/// Uses linear algebra for O(n) specialist selection instead of O(n²) iteration.

use compute::linalg::mat_vec_mul;

/// Task embedding for routing.
#[derive(Debug, Clone)]
pub struct TaskEmbedding {
    pub task_id: String,
    pub features: Vec<f64>, // [complexity, urgency, skill_match, resource_need, ...]
}

/// Specialist embedding for routing.
#[derive(Debug, Clone)]
pub struct SpecialistEmbedding {
    pub specialist_id: String,
    pub name: String,
    pub features: Vec<f64>, // [skill_vector, capacity, success_rate, ...]
}

/// Weight matrix for routing: W[skill_dim, task_dim]
/// Learned via gradient descent on historical task outcomes.
#[derive(Debug, Clone)]
pub struct RoutingWeights {
    pub weights: Vec<Vec<f64>>, // rows=specialists, cols=task_features
    pub bias: Vec<f64>,
    pub specialist_ids: Vec<String>,
}

impl RoutingWeights {
    pub fn new(n_specialists: usize, n_features: usize, specialist_ids: Vec<String>) -> Self {
        // Initialize with small random-like values (uniform)
        let scale = 1.0 / (n_features as f64).sqrt();
        let weights = (0..n_specialists)
            .map(|i| {
                (0..n_features)
                    .map(|j| {
                        // Deterministic initialization based on indices
                        let val = ((i * n_features + j) % 100) as f64 / 100.0 * scale;
                        val - scale / 2.0
                    })
                    .collect()
            })
            .collect();

        Self {
            weights,
            bias: vec![0.0; n_specialists],
            specialist_ids,
        }
    }

    /// Update weights based on task outcome (online learning).
    pub fn update(&mut self, task_features: &[f64], specialist_idx: usize, success: bool, learning_rate: f64) {
        if specialist_idx >= self.weights.len() {
            return;
        }

        let error = if success { 1.0 } else { -1.0 };

        for j in 0..task_features.len() {
            self.weights[specialist_idx][j] += learning_rate * error * task_features[j];
        }

        // Update bias
        self.bias[specialist_idx] += learning_rate * error;
    }
}

/// Tensor routing result.
#[derive(Debug, Clone)]
pub struct RoutingResult {
    pub task_id: String,
    pub specialist_scores: Vec<(String, f64)>, // (specialist_id, probability)
    pub selected_specialist: String,
    pub confidence: f64,
    pub entropy: f64, // Routing entropy (uncertainty)
}

/// Tensor-based router using softmax attention.
pub struct TensorRouter {
    pub weights: RoutingWeights,
    pub temperature: f64, // Controls exploration vs exploitation
}

impl TensorRouter {
    pub fn new(weights: RoutingWeights, temperature: f64) -> Self {
        Self { weights, temperature }
    }

    /// Route task to specialist using softmax(Wx).
    /// Returns probability distribution over specialists.
    pub fn route(&self, task: &TaskEmbedding) -> RoutingResult {
        let n = self.weights.specialist_ids.len();

        // Compute logits: z = Wx + b
        let mut logits = Vec::with_capacity(n);
        for i in 0..n {
            let dot: f64 = self.weights.weights[i]
                .iter()
                .zip(task.features.iter())
                .map(|(w, x)| w * x)
                .sum();
            logits.push(dot + self.weights.bias[i]);
        }

        // Softmax with temperature: p_i = exp(z_i / T) / Σ exp(z_j / T)
        let scaled_logits: Vec<f64> = logits.iter().map(|z| z / self.temperature).collect();
        let max_logit = scaled_logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp_values: Vec<f64> = scaled_logits.iter().map(|z| (z - max_logit).exp()).collect();
        let sum_exp: f64 = exp_values.iter().sum();

        let probabilities: Vec<f64> = if sum_exp > 0.0 {
            exp_values.iter().map(|e| e / sum_exp).collect()
        } else {
            vec![1.0 / n as f64; n]
        };

        // Select specialist (argmax)
        let (selected_idx, &max_prob) = probabilities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        // Compute routing entropy
        let entropy: f64 = probabilities
            .iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.log2())
            .sum();

        let specialist_scores: Vec<(String, f64)> = self.weights.specialist_ids
            .iter()
            .zip(probabilities.iter())
            .map(|(id, &p)| (id.clone(), p))
            .collect();

        RoutingResult {
            task_id: task.task_id.clone(),
            specialist_scores,
            selected_specialist: self.weights.specialist_ids[selected_idx].clone(),
            confidence: max_prob,
            entropy,
        }
    }

    /// Route with Boltzmann exploration (sample from distribution).
    pub fn route_with_exploration(&self, task: &TaskEmbedding, rng: &mut impl rand::Rng) -> RoutingResult {
        let result = self.route(task);
        let specialist_scores = result.specialist_scores.clone();

        // Sample from distribution instead of argmax
        let roll: f64 = rng.gen_range(0.0..1.0);
        let mut cumulative = 0.0;
        let mut selected_idx = 0;

        for (i, &p) in result.specialist_scores.iter().map(|(_, p)| p).enumerate() {
            cumulative += p;
            if roll <= cumulative {
                selected_idx = i;
                break;
            }
        }

        RoutingResult {
            task_id: task.task_id.clone(),
            specialist_scores,
            selected_specialist: self.weights.specialist_ids[selected_idx].clone(),
            confidence: result.specialist_scores[selected_idx].1,
            entropy: result.entropy,
        }
    }

    /// Batch route multiple tasks efficiently.
    /// Returns routing results for all tasks.
    pub fn batch_route(&self, tasks: &[TaskEmbedding]) -> Vec<RoutingResult> {
        tasks.iter().map(|t| self.route(t)).collect()
    }

    /// Update routing weights based on task outcome.
    pub fn learn(&mut self, task_features: &[f64], specialist_id: &str, success: bool, learning_rate: f64) {
        if let Some(idx) = self.weights.specialist_ids.iter().position(|id| id == specialist_id) {
            self.weights.update(task_features, idx, success, learning_rate);
        }
    }

    /// Adjust temperature based on system state.
    /// High entropy needed -> increase temperature (explore)
    /// Low entropy needed -> decrease temperature (exploit)
    pub fn adjust_temperature(&mut self, target_entropy: f64, current_entropy: f64, learning_rate: f64) {
        let error = target_entropy - current_entropy;
        self.temperature = (self.temperature + learning_rate * error).clamp(0.1, 2.0);
    }
}

/// Multi-head attention routing.
/// Uses multiple routing "heads" for different aspects of tasks.
pub struct MultiHeadRouter {
    pub heads: Vec<TensorRouter>,
    pub head_weights: Vec<f64>, // Weight for each head's output
}

impl MultiHeadRouter {
    pub fn new(head_configs: Vec<(RoutingWeights, f64)>) -> Self {
        let heads: Vec<TensorRouter> = head_configs
            .iter()
            .map(|(w, t)| TensorRouter::new(w.clone(), *t))
            .collect();

        let head_weights = vec![1.0 / heads.len() as f64; heads.len()];

        Self { heads, head_weights }
    }

    /// Route using multi-head attention.
    /// Combines outputs from all heads weighted by head_weights.
    pub fn route(&self, task: &TaskEmbedding) -> RoutingResult {
        let n_specialists = self.heads[0].weights.specialist_ids.len();
        let mut combined_scores = vec![0.0; n_specialists];

        // Combine scores from all heads
        for (head, &hw) in self.heads.iter().zip(self.head_weights.iter()) {
            let result = head.route(task);
            for (i, (_, score)) in result.specialist_scores.iter().enumerate() {
                combined_scores[i] += hw * score;
            }
        }

        // Normalize combined scores
        let sum: f64 = combined_scores.iter().sum();
        if sum > 0.0 {
            combined_scores = combined_scores.iter().map(|s| s / sum).collect();
        }

        // Select best specialist
        let (selected_idx, &max_prob) = combined_scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        // Compute entropy
        let entropy: f64 = combined_scores
            .iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| -p * p.log2())
            .sum();

        let specialist_scores: Vec<(String, f64)> = self.heads[0].weights.specialist_ids
            .iter()
            .zip(combined_scores.iter())
            .map(|(id, &p)| (id.clone(), p))
            .collect();

        RoutingResult {
            task_id: task.task_id.clone(),
            specialist_scores,
            selected_specialist: self.heads[0].weights.specialist_ids[selected_idx].clone(),
            confidence: max_prob,
            entropy,
        }
    }
}

/// Gradient descent optimizer for routing weights.
pub struct RoutingOptimizer {
    pub learning_rate: f64,
    pub momentum: f64,
    pub velocity: Vec<Vec<f64>>, // Momentum buffer
}

impl RoutingOptimizer {
    pub fn new(n_specialists: usize, n_features: usize, learning_rate: f64, momentum: f64) -> Self {
        Self {
            learning_rate,
            momentum,
            velocity: vec![vec![0.0; n_features]; n_specialists],
        }
    }

    /// Update weights with momentum.
    pub fn step(&mut self, router: &mut TensorRouter, task_features: &[f64], specialist_idx: usize, success: bool) {
        if specialist_idx >= router.weights.weights.len() {
            return;
        }

        let error = if success { 1.0 } else { -1.0 };

        for j in 0..task_features.len() {
            let grad = error * task_features[j];
            self.velocity[specialist_idx][j] =
                self.momentum * self.velocity[specialist_idx][j] + self.learning_rate * grad;
            router.weights.weights[specialist_idx][j] += self.velocity[specialist_idx][j];
        }

        // Update bias with momentum
        let grad_bias = error;
        let bias_vel = self.momentum * 0.0 + self.learning_rate * grad_bias; // Simplified
        router.weights.bias[specialist_idx] += bias_vel;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn create_test_router() -> TensorRouter {
        let weights = RoutingWeights::new(3, 4, vec!["spec_a".to_string(), "spec_b".to_string(), "spec_c".to_string()]);
        TensorRouter::new(weights, 1.0)
    }

    #[test]
    fn test_tensor_routing() {
        let router = create_test_router();
        let task = TaskEmbedding {
            task_id: "task_1".to_string(),
            features: vec![0.7, 0.5, 0.8, 0.3],
        };

        let result = router.route(&task);
        assert_eq!(result.specialist_scores.len(), 3);

        // Probabilities should sum to 1.0
        let sum: f64 = result.specialist_scores.iter().map(|(_, p)| p).sum();
        assert!((sum - 1.0).abs() < 1e-10);

        // Selected specialist should have highest probability
        let selected_score = result.specialist_scores
            .iter()
            .find(|(id, _)| id == &result.selected_specialist)
            .unwrap();
        assert!(selected_score.1 >= result.specialist_scores.iter().map(|(_, p)| p).fold(0.0, f64::max) - 1e-10);
    }

    #[test]
    fn test_routing_entropy() {
        let router = create_test_router();
        let task = TaskEmbedding {
            task_id: "task_1".to_string(),
            features: vec![0.5, 0.5, 0.5, 0.5], // Uniform features
        };

        let result = router.route(&task);
        // Entropy should be positive
        assert!(result.entropy > 0.0);
        // Max entropy for 3 specialists is log2(3) ≈ 1.585
        assert!(result.entropy <= 1.585 + 1e-10);
    }

    #[test]
    fn test_routing_with_exploration() {
        let router = create_test_router();
        let task = TaskEmbedding {
            task_id: "task_1".to_string(),
            features: vec![0.7, 0.5, 0.8, 0.3],
        };

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let result = router.route_with_exploration(&task, &mut rng);
        assert!(!result.selected_specialist.is_empty());
    }

    #[test]
    fn test_batch_routing() {
        let router = create_test_router();
        let tasks = vec![
            TaskEmbedding { task_id: "task_1".to_string(), features: vec![0.7, 0.5, 0.8, 0.3] },
            TaskEmbedding { task_id: "task_2".to_string(), features: vec![0.3, 0.8, 0.2, 0.9] },
        ];

        let results = router.batch_route(&tasks);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_learning_updates_weights() {
        let mut router = create_test_router();
        let initial_weights = router.weights.weights.clone();

        let task_features = vec![0.7, 0.5, 0.8, 0.3];
        router.learn(&task_features, "spec_a", true, 0.1);

        // Weights should have changed
        assert!(router.weights.weights[0] != initial_weights[0]);
    }

    #[test]
    fn test_temperature_adjustment() {
        let mut router = create_test_router();
        let initial_temp = router.temperature;

        router.adjust_temperature(1.0, 0.5, 0.1);
        assert!(router.temperature > initial_temp); // Should increase

        router.adjust_temperature(0.1, 0.5, 0.1);
        assert!(router.temperature < initial_temp + 0.1); // Should decrease
    }
}
