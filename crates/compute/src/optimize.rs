//! crates/compute/src/optimize.rs
//! Optimization primitives (Genetic Algorithms, Simulated Annealing, Gradient-free Search).
//! Used for resource allocation, SAB evolution, hyperparameter tuning, and scheduling optimization.

use anyhow::Result;
use rand::Rng;

/// Cooling schedule for Simulated Annealing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoolingSchedule {
    /// T_{k+1} = T_k * alpha (e.g. alpha = 0.95)
    Exponential(f64),
    /// T_{k+1} = T_0 - k * rate
    Linear(f64),
    /// T_{k+1} = T_0 / ln(1 + k)
    Logarithmic,
}

impl CoolingSchedule {
    pub fn next_temperature(&self, t_initial: f64, t_current: f64, step: usize) -> f64 {
        match *self {
            CoolingSchedule::Exponential(alpha) => (t_current * alpha).max(1e-8),
            CoolingSchedule::Linear(rate) => (t_initial - (step as f64) * rate).max(1e-8),
            CoolingSchedule::Logarithmic => {
                let denom = ((step + 1) as f64).ln().max(1e-4);
                (t_initial / denom).max(1e-8)
            }
        }
    }
}

/// Continuous Simulated Annealing step (backwards-compatible).
pub fn anneal_step(
    current: f64,
    temperature: f64,
    rng: &mut impl Rng,
    objective: impl Fn(f64) -> f64,
) -> f64 {
    let neighbor = current + rng.gen_range(-0.5..0.5);
    let delta = objective(neighbor) - objective(current);
    if delta > 0.0 || rng.gen::<f64>() < (-delta / temperature.max(1e-10)).exp() {
        neighbor
    } else {
        current
    }
}

/// Simulated Annealing optimizer for finding the minimum of a continuous objective function f(x).
pub fn simulated_anneal_minimize(
    initial: f64,
    initial_temp: f64,
    iterations: usize,
    schedule: CoolingSchedule,
    step_radius: f64,
    rng: &mut impl Rng,
    objective: impl Fn(f64) -> f64,
) -> (f64, f64) {
    let mut current = initial;
    let mut current_score = objective(current);
    let mut best = current;
    let mut best_score = current_score;
    let mut temp = initial_temp;

    for k in 0..iterations {
        let neighbor = current + rng.gen_range(-step_radius..step_radius);
        let neighbor_score = objective(neighbor);
        let delta = neighbor_score - current_score;

        // Minimization: accept if delta < 0 (better) or with Boltzmann probability if delta >= 0
        if delta < 0.0 || rng.gen::<f64>() < (-delta / temp).exp() {
            current = neighbor;
            current_score = neighbor_score;

            if current_score < best_score {
                best = current;
                best_score = current_score;
            }
        }

        temp = schedule.next_temperature(initial_temp, temp, k);
    }

    (best, best_score)
}

/// Single step of a simple genetic algorithm (backwards-compatible).
pub fn genetic_step(population: &[f64], rng: &mut impl Rng) -> Result<Vec<f64>> {
    if population.len() < 2 {
        return Ok(population.to_vec());
    }
    // Tournament selection (size 2)
    let mut offspring = Vec::with_capacity(population.len());
    for _ in 0..population.len() {
        let i1 = rng.gen_range(0..population.len());
        let i2 = rng.gen_range(0..population.len());
        let parent = if population[i1] > population[i2] {
            i1
        } else {
            i2
        };
        offspring.push(population[parent]);
    }
    // Crossover & mutation
    for i in 0..(offspring.len() / 2) {
        if rng.gen::<f64>() < 0.7 {
            let child = (offspring[i * 2] + offspring[i * 2 + 1]) / 2.0;
            offspring[i * 2] = child + rng.gen_range(-0.1..0.1);
        }
    }
    Ok(offspring)
}

/// Golden-Section Search for local minimum of unimodal 1D function on interval [a, b].
pub fn golden_section_search(
    mut a: f64,
    mut b: f64,
    tol: f64,
    max_iter: usize,
    f: impl Fn(f64) -> f64,
) -> f64 {
    let phi = (1.0 + 5.0f64.sqrt()) / 2.0;
    let resphi = 2.0 - phi;

    let mut c = a + resphi * (b - a);
    let mut d = b - resphi * (b - a);
    let mut fc = f(c);
    let mut fd = f(d);

    for _ in 0..max_iter {
        if (b - a).abs() < tol {
            break;
        }

        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = a + resphi * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = b - resphi * (b - a);
            fd = f(d);
        }
    }

    (a + b) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golden_section_quadratic() {
        // Minimize f(x) = (x - 3)^2 + 5 -> minimum at x = 3.0
        let f = |x: f64| (x - 3.0).powi(2) + 5.0;
        let min_x = golden_section_search(0.0, 10.0, 1e-5, 100, f);
        assert!((min_x - 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_simulated_annealing() {
        let mut rng = rand::thread_rng();
        // Minimize f(x) = (x - 2)^2
        let f = |x: f64| (x - 2.0).powi(2);
        let (best, score) = simulated_anneal_minimize(
            10.0,
            5.0,
            300,
            CoolingSchedule::Exponential(0.95),
            0.5,
            &mut rng,
            f,
        );
        assert!((best - 2.0).abs() < 1.0);
        assert!(score < 1.0);
    }
}
