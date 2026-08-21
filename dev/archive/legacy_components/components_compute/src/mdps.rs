use rand::Rng;

/// Markov Decision Process (MDP) primitives.
/// Models state transitions and optimal policy selection.

#[derive(Debug, Clone)]
pub struct MarkovChain {
    pub states: usize,
    pub transition_matrix: Vec<Vec<f64>>,
}

impl MarkovChain {
    pub fn new(states: usize) -> Self {
        let matrix = vec![vec![1.0 / states as f64; states]; states];
        Self {
            states,
            transition_matrix: matrix,
        }
    }

    /// Normalize rows to ensure valid probability distribution
    pub fn normalize(&mut self) {
        for row in self.transition_matrix.iter_mut() {
            let sum: f64 = row.iter().sum();
            if sum > 0.0 {
                for val in row.iter_mut() {
                    *val /= sum;
                }
            }
        }
    }

    /// Simulate one transition step
    pub fn step(&self, current_state: usize, rng: &mut impl Rng) -> usize {
        let probs = &self.transition_matrix[current_state];
        let roll: f64 = rng.gen();
        let mut cumulative = 0.0;
        for (next, &p) in probs.iter().enumerate() {
            cumulative += p;
            if roll <= cumulative {
                return next;
            }
        }
        probs.len() - 1
    }
}

/// Compute Markov transition from input vector
pub fn markov_transition(input: &[f64], rng: &mut impl Rng) -> anyhow::Result<Vec<f64>> {
    if input.is_empty() {
        return Ok(vec![]);
    }
    let states = input.len().min(10);
    let mut mc = MarkovChain::new(states);
    // Seed transition matrix from input (simplified)
    for i in 0..states {
        for j in 0..states {
            mc.transition_matrix[i][j] =
                input[i.min(input.len() - 1)] * (1.0 - input[j.min(input.len() - 1)]);
        }
    }
    mc.normalize();

    let mut state = 0;
    let mut trajectory = vec![];
    for _ in 0..input.len() {
        state = mc.step(state, rng);
        trajectory.push(state as f64 / states as f64);
    }
    Ok(trajectory)
}
