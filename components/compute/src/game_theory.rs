/// Game Theory primitives (Nash Equilibrium, Auction Mechanisms).
/// Used for multi-agent resource bidding, conflict resolution, and incentive alignment.

/// Approximate Nash equilibrium via iterative best response (2-player, 2-strategy)
pub fn nash_approx(payoffs: &[f64]) -> anyhow::Result<Vec<f64>> {
    // payoffs: [A1B1, A1B2, A2B1, A2B2] for player A
    if payoffs.len() < 4 { return Ok(vec![0.5, 0.5]); }
    let mut p = 0.5; // probability of playing strategy 1
    for _ in 0..100 {
        let ev1 = payoffs[0] * p + payoffs[1] * (1.0 - p);
        let ev2 = payoffs[2] * p + payoffs[3] * (1.0 - p);
        let diff = ev1 - ev2;
        p = 1.0 / (1.0 + (-diff).exp()); // Softmax best response
    }
    Ok(vec![p, 1.0 - p])
}

/// Vickrey auction (second-price sealed-bid)
pub fn vickrey_auction(bids: &[f64]) -> anyhow::Result<Vec<f64>> {
    if bids.len() < 2 { return Ok(vec![]); }
    let mut sorted = bids.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    Ok(vec![sorted[0], sorted[1]]) // [winner, price]
}
