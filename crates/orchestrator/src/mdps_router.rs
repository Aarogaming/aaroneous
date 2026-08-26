// MDP-Based Task Routing Engine
// Uses Markov Decision Processes to optimally route tasks to specialists

use rand::SeedableRng;
use serde::{Deserialize, Serialize};

/// Represents a task that needs to be routed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutableTask {
    pub id: String,
    pub task_type: TaskType,
    pub complexity: f64, // 0.0-1.0
    pub urgency: f64,    // 0.0-1.0
    pub required_skills: Vec<String>,
    pub estimated_cost: f64, // Token/resource cost estimate
}

/// Types of tasks that can be routed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskType {
    CodeGeneration,
    BugFix,
    Analysis,
    Refactor,
    TestCreation,
    Documentation,
    Ingestion,
    Custom(String),
}

/// Represents a specialist that can handle tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specialist {
    pub id: String,
    pub name: String,
    pub skills: Vec<String>,
    pub capacity: f64,     // Current capacity (0.0-1.0, 1.0 = fully available)
    pub success_rate: f64, // Historical success rate (0.0-1.0)
    pub avg_completion_time: f64, // Average time to complete tasks (seconds)
}

/// MDP state for task routing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutingState {
    pub task_complexity_bin: usize, // Discretized complexity (0-4)
    pub specialist_load_bin: usize, // Discretized load (0-4)
    pub urgency_bin: usize,         // Discretized urgency (0-4)
}

/// Action in the MDP: which specialist to assign
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutingAction {
    pub specialist_index: usize,
}

/// MDP-based routing engine
pub struct TaskRoutingEngine {
    pub specialists: Vec<Specialist>,
    pub transition_matrix: Vec<Vec<Vec<f64>>>, // [state][action][next_state]
    pub reward_matrix: Vec<Vec<f64>>,          // [state][action] -> expected reward
    pub value_function: Vec<f64>,              // V(s) for each state
    pub policy: Vec<usize>,                    // π(s) - best action for each state
    pub rng: rand::rngs::StdRng,
    pub learning_rate: f64,
    pub discount_factor: f64,
}

impl TaskRoutingEngine {
    pub fn new(specialists: Vec<Specialist>) -> Self {
        let num_states = 5 * 5 * 5; // complexity(5) * load(5) * urgency(5) = 125
        let num_actions = specialists.len();

        let mut engine = Self {
            specialists,
            transition_matrix: vec![vec![vec![0.0; num_states]; num_actions]; num_states],
            reward_matrix: vec![vec![0.0; num_actions]; num_states],
            value_function: vec![0.0; num_states],
            policy: vec![0; num_states],
            rng: rand::rngs::StdRng::from_entropy(),
            learning_rate: 0.1,
            discount_factor: 0.9,
        };

        engine.initialize_matrices();
        engine
    }

    /// Initialize transition and reward matrices with priors
    fn initialize_matrices(&mut self) {
        let num_states = self.transition_matrix.len();
        let num_actions = self.specialists.len();

        for s in 0..num_states {
            let state = self.decode_state(s);

            for a in 0..num_actions {
                if a >= self.specialists.len() {
                    continue;
                }

                let specialist = &self.specialists[a];

                // Calculate expected reward based on specialist capabilities
                let reward = self.calculate_expected_reward(&state, specialist, &[]);
                self.reward_matrix[s][a] = reward;

                // Initialize transition probabilities (uniform prior)
                for next_s in 0..num_states {
                    self.transition_matrix[s][a][next_s] = 1.0 / num_states as f64;
                }
            }
        }
    }

    /// Calculate expected reward for assigning a task to a specialist
    fn calculate_expected_reward(
        &self,
        state: &RoutingState,
        specialist: &Specialist,
        task_skills: &[String],
    ) -> f64 {
        let complexity_match =
            1.0 - (state.task_complexity_bin as f64 / 4.0 - specialist.success_rate).abs();
        let capacity_reward = specialist.capacity;
        let success_bonus = specialist.success_rate * 2.0;
        let urgency_penalty = if state.urgency_bin > 3 && specialist.avg_completion_time > 10.0 {
            -1.0
        } else {
            0.0
        };

        // Skill matching: bonus for matching skills, penalty for missing required skills
        let skill_score = if task_skills.is_empty() {
            0.0 // No skills required, neutral
        } else {
            let matched = task_skills
                .iter()
                .filter(|skill| specialist.skills.contains(skill))
                .count();
            let total = task_skills.len();
            if matched == total {
                1.0 // All skills matched
            } else if matched > 0 {
                0.5 // Partial match
            } else {
                -2.0 // No skills match — heavy penalty
            }
        };

        (complexity_match + capacity_reward + success_bonus + urgency_penalty + skill_score)
            .max(0.0)
    }

    /// Decode a state index into a RoutingState
    fn decode_state(&self, index: usize) -> RoutingState {
        let urgency_bin = index % 5;
        let specialist_load_bin = (index / 5) % 5;
        let task_complexity_bin = (index / 25) % 5;

        RoutingState {
            task_complexity_bin,
            specialist_load_bin,
            urgency_bin,
        }
    }

    /// Encode a RoutingState into an index
    fn encode_state(&self, state: &RoutingState) -> usize {
        state.task_complexity_bin + state.specialist_load_bin * 5 + state.urgency_bin * 25
    }

    /// Discretize a continuous value into a bin (0-4)
    fn discretize(value: f64) -> usize {
        (value * 4.0).round().clamp(0.0, 4.0) as usize
    }

    /// Find the optimal specialist for a task using value iteration
    pub fn find_optimal_specialist(&mut self, task: &RoutableTask) -> RoutingDecision {
        // Create current state from task characteristics
        let avg_load = if self.specialists.is_empty() {
            0.5
        } else {
            self.specialists
                .iter()
                .map(|s| 1.0 - s.capacity)
                .sum::<f64>()
                / self.specialists.len() as f64
        };

        let state = RoutingState {
            task_complexity_bin: Self::discretize(task.complexity),
            specialist_load_bin: Self::discretize(avg_load),
            urgency_bin: Self::discretize(task.urgency),
        };

        let state_idx = self.encode_state(&state);

        // Run value iteration to find optimal policy
        self.value_iteration(100);

        // Get best action from policy
        let best_action_idx = self.policy[state_idx];

        if best_action_idx < self.specialists.len() {
            let specialist = &self.specialists[best_action_idx];
            let expected_value = self.value_function[state_idx];

            // Calculate skill match score for the task
            let skill_score = if task.required_skills.is_empty() {
                0.0
            } else {
                let matched = task
                    .required_skills
                    .iter()
                    .filter(|skill| specialist.skills.contains(skill))
                    .count();
                let total = task.required_skills.len();
                if matched == total {
                    1.0
                } else if matched > 0 {
                    0.5
                } else {
                    -2.0
                }
            };

            // Normalize confidence to 0.0-1.0 range, adjusted by skill match
            let base_confidence = (expected_value / 10.0).clamp(0.0, 1.0);
            let confidence = (base_confidence + skill_score * 0.3).clamp(0.0, 1.0);

            RoutingDecision {
                specialist_id: specialist.id.clone(),
                specialist_name: specialist.name.clone(),
                confidence,
                expected_completion_time: specialist.avg_completion_time,
                reasoning: format!(
                    "Optimal for complexity={:.2}, urgency={:.2}, skills_matched={}/{}",
                    task.complexity,
                    task.urgency,
                    task.required_skills
                        .iter()
                        .filter(|s| specialist.skills.contains(s))
                        .count(),
                    task.required_skills.len()
                ),
            }
        } else {
            RoutingDecision {
                specialist_id: "fallback".to_string(),
                specialist_name: "Fallback Handler".to_string(),
                confidence: 0.3,
                expected_completion_time: 30.0,
                reasoning: "No suitable specialist found".to_string(),
            }
        }
    }

    /// Run value iteration to update the value function and policy
    #[allow(clippy::needless_range_loop)]
    fn value_iteration(&mut self, iterations: usize) {
        let num_states = self.value_function.len();
        let num_actions = self.specialists.len();

        for _ in 0..iterations {
            let mut new_values = vec![0.0; num_states];

            for s in 0..num_states {
                let mut best_value = f64::NEG_INFINITY;
                let mut best_action = 0;

                for a in 0..num_actions {
                    if a >= self.specialists.len() {
                        continue;
                    }

                    let expected_value = self.reward_matrix[s][a];
                    let future_value: f64 = (0..num_states)
                        .map(|next_s| {
                            self.transition_matrix[s][a][next_s] * self.value_function[next_s]
                        })
                        .sum();

                    let total_value = expected_value + self.discount_factor * future_value;

                    if total_value > best_value {
                        best_value = total_value;
                        best_action = a;
                    }
                }

                new_values[s] = best_value;
                self.policy[s] = best_action;
            }

            self.value_function = new_values;
        }
    }

    /// Update specialist metrics based on task outcome
    pub fn update_specialist_performance(
        &mut self,
        specialist_id: &str,
        success: bool,
        completion_time: f64,
    ) {
        if let Some(specialist) = self.specialists.iter_mut().find(|s| s.id == specialist_id) {
            // Update success rate with exponential moving average
            specialist.success_rate =
                specialist.success_rate * 0.9 + if success { 1.0 } else { 0.0 } * 0.1;

            // Update completion time with EMA
            specialist.avg_completion_time =
                specialist.avg_completion_time * 0.9 + completion_time * 0.1;

            // Update capacity (replenish after task completion)
            specialist.capacity = (specialist.capacity + 0.2).min(1.0);
        }
    }

    /// Update transition matrix from observed routing outcome (online learning)
    pub fn update_transition_matrix(
        &mut self,
        state: &RoutingState,
        action: &RoutingAction,
        next_state: &RoutingState,
    ) {
        let state_idx = self.encode_state(state);
        let next_state_idx = self.encode_state(next_state);
        let action_idx = action.specialist_index;

        if action_idx >= self.specialists.len() {
            return;
        }

        let num_states = self.transition_matrix.len();

        // Bayesian update: increase probability of observed transition
        // Decrease all other transitions proportionally
        let current_prob = self.transition_matrix[state_idx][action_idx][next_state_idx];
        let delta = self.learning_rate * (1.0 - current_prob);

        for s in 0..num_states {
            if s == next_state_idx {
                self.transition_matrix[state_idx][action_idx][s] += delta;
            } else {
                self.transition_matrix[state_idx][action_idx][s] *=
                    1.0 - self.learning_rate;
            }
        }

        // Normalize to maintain valid probability distribution
        let sum: f64 = self.transition_matrix[state_idx][action_idx]
            .iter()
            .sum();
        if sum > 0.0 {
            for s in 0..num_states {
                self.transition_matrix[state_idx][action_idx][s] /= sum;
            }
        }
    }

    /// Consume specialist capacity when assigning a task
    pub fn consume_capacity(&mut self, specialist_id: &str, cost: f64) {
        if let Some(specialist) = self.specialists.iter_mut().find(|s| s.id == specialist_id) {
            specialist.capacity = (specialist.capacity - cost).max(0.0);
        }
    }

    /// Add a new specialist to the routing engine
    pub fn add_specialist(&mut self, specialist: Specialist) {
        self.specialists.push(specialist);
        // Extend matrices to accommodate new action
        let num_states = self.transition_matrix.len();

        // Extend matrices
        for s in 0..num_states {
            self.reward_matrix[s].push(0.0);
            self.transition_matrix[s].push(vec![0.0; num_states]);
        }
    }

    /// Rebalances tasks across local specialists and remote swarm peers based on metabolic capacity
    pub fn balance_swarm_load(&self, peer_capacities: &[(&str, f64)]) -> Vec<(String, String)> {
        let mut offload_routes = Vec::new();
        // If average local specialist capacity is overloaded (< 0.5), find peers with capacity > 0.6
        let avg_local_capacity: f64 = if self.specialists.is_empty() {
            1.0
        } else {
            self.specialists.iter().map(|s| s.capacity).sum::<f64>() / (self.specialists.len() as f64)
        };

        if avg_local_capacity < 0.5 {
            for (peer_id, peer_cap) in peer_capacities {
                if *peer_cap > 0.6 {
                    offload_routes.push(("overloaded_local_task".to_string(), peer_id.to_string()));
                }
            }
        }

        offload_routes
    }
}

/// Decision output from the routing engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub specialist_id: String,
    pub specialist_name: String,
    pub confidence: f64, // 0.0-1.0
    pub expected_completion_time: f64,
    pub reasoning: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_specialists() -> Vec<Specialist> {
        vec![
            Specialist {
                id: "spec_1".to_string(),
                name: "Code Generator".to_string(),
                skills: vec!["rust".to_string(), "python".to_string()],
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 5.0,
            },
            Specialist {
                id: "spec_2".to_string(),
                name: "Bug Fixer".to_string(),
                skills: vec!["debugging".to_string()],
                capacity: 0.8,
                success_rate: 0.85,
                avg_completion_time: 8.0,
            },
        ]
    }

    #[test]
    fn test_routing_engine_creation() {
        let specialists = create_test_specialists();
        let engine = TaskRoutingEngine::new(specialists);
        assert_eq!(engine.specialists.len(), 2);
        assert_eq!(engine.value_function.len(), 125); // 5*5*5 states
    }

    #[test]
    fn test_find_optimal_specialist() {
        let specialists = create_test_specialists();
        let mut engine = TaskRoutingEngine::new(specialists);

        let task = RoutableTask {
            id: "task_1".to_string(),
            task_type: TaskType::CodeGeneration,
            complexity: 0.7,
            urgency: 0.5,
            required_skills: vec!["rust".to_string()],
            estimated_cost: 0.2,
        };

        let decision = engine.find_optimal_specialist(&task);
        assert!(!decision.specialist_id.is_empty());
        assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
    }

    #[test]
    fn test_update_specialist_performance() {
        let specialists = create_test_specialists();
        let mut engine = TaskRoutingEngine::new(specialists);

        engine.update_specialist_performance("spec_1", true, 4.5);

        let spec = engine
            .specialists
            .iter()
            .find(|s| s.id == "spec_1")
            .unwrap();
        assert!(spec.success_rate > 0.85); // Should have increased slightly
    }

    #[test]
    fn test_balance_swarm_load() {
        let mut overloaded_specs = create_test_specialists();
        overloaded_specs[0].capacity = 0.2;
        overloaded_specs[1].capacity = 0.3;

        let engine = TaskRoutingEngine::new(overloaded_specs);
        let peers = vec![("peer_node_alpha", 0.9), ("peer_node_beta", 0.4)];
        let routes = engine.balance_swarm_load(&peers);

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].1, "peer_node_alpha");
    }
}
