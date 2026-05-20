use rand::Rng;

/// Optimization primitives (Genetic Algorithms, Simulated Annealing, Gradient-free).
/// Used for resource allocation, SAB evolution, and hyperparameter tuning.

/// Single step of a simple genetic algorithm
pub fn genetic_step(population: &[f64], rng: &mut impl Rng) -> anyhow::Result<Vec<f64>> {
    if population.len() < 2 { return Ok(population.to_vec()); }
    // Tournament selection (size 2)
    let mut offspring = vec![];
    for _ in 0..population.len() {
        let i1 = rng.gen_range(0..population.len());
        let i2 = rng.gen_range(0..population.len());
        let parent = if population[i1] > population[i2] { i1 } else { i2 };
        offspring.push(population[parent]);
    }
    // Crossover & mutation
    for i in 0..offspring.len() / 2 {
        if rng.gen::<f64>() < 0.7 {
            let child = (offspring[i] + offspring[i + 1]) / 2.0;
            offspring[i] = child + rng.gen_range(-0.1..0.1);
        }
    }
    Ok(offspring)
}

/// Simulated annealing step
pub fn anneal_step(current: f64, temperature: f64, rng: &mut impl Rng, objective: impl Fn(f64) -> f64) -> f64 {
    let neighbor = current + rng.gen_range(-0.5..0.5);
    let delta = objective(neighbor) - objective(current);
    if delta > 0.0 || rng.gen::<f64>() < (-delta / temperature.max(1e-10)).exp() {
        neighbor
    } else {
        current
    }
}
