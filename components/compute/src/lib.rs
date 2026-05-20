pub mod mdps;
pub mod stochastic;
pub mod bayesian;
pub mod graph;
pub mod linalg;
pub mod entropy;
pub mod optimize;
pub mod topology;
pub mod signal;
pub mod control;
pub mod game_theory;
pub mod automata;
pub mod thermodynamics;
pub mod kalman;
pub mod mpc;
pub mod information;
pub mod predictive_coding;
pub mod category;

use rand::SeedableRng;
use nervous_system::SharedMemorySynapse;

/// The central Compute Engine.
/// Exposes mathematical methodologies to the Synapse for zero-copy execution.
pub struct ComputeEngine {
    pub synapse: SharedMemorySynapse,
    pub rng: rand::rngs::StdRng,
}

impl ComputeEngine {
    pub fn new() -> Self {
        Self {
            synapse: SharedMemorySynapse::new("SAB_STORE", 1024 * 1024).unwrap(),
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    // Unified execution interface
    pub fn execute(&mut self, task: &str, input: &[f64]) -> anyhow::Result<Vec<f64>> {
        match task {
            "monte_carlo" => stochastic::monte_carlo_simulate(input, 1000, &mut self.rng),
            "markov" => mdps::markov_transition(input, &mut self.rng),
            "bayesian" => bayesian::bayesian_update(input),
            "entropy" => entropy::shannon_entropy(input),
            "cosine" => linalg::cosine_similarity(input),
            "pid" => control::pid_step(input),
            "fft" => signal::fft_basic(input),
            "nash" => game_theory::nash_approx(input),
            "optimize_ga" => optimize::genetic_step(input, &mut self.rng),
            "boltzmann" => {
                let n = input.len() - 1;
                let temperature = input[0];
                let energies = &input[1..];
                Ok(thermodynamics::boltzmann_distribution(energies, temperature))
            }
            "free_energy" => {
                if input.len() >= 3 {
                    Ok(vec![thermodynamics::FreeEnergyState::new(input[0], input[1], input[2]).free_energy])
                } else {
                    Ok(vec![0.0])
                }
            }
            "mutual_info" => {
                if input.len() >= 3 {
                    Ok(vec![information::mutual_information(
                        &[vec![input[0], input[1]], vec![input[2], 1.0 - input[0] - input[1] - input[2]]],
                        &[input[0] + input[1], input[2] + (1.0 - input[0] - input[1] - input[2])],
                        &[input[0] + input[2], input[1] + (1.0 - input[0] - input[1] - input[2])],
                    )])
                } else {
                    Ok(vec![0.0])
                }
            }
            _ => anyhow::bail!("Unknown compute task: {}", task),
        }
    }
}
