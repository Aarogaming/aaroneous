//! crates/compute/src/game_theory.rs
//! Game Theory primitives (Nash Equilibrium, Auction Mechanisms, Shapley Values).
//! Used for multi-agent resource bidding, conflict resolution, consensus, and incentive alignment.

use anyhow::{bail, Result};
use std::cmp::Ordering;

/// Safely compares two f64 values without panicking on NaN.
pub fn safe_f64_cmp(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or_else(|| {
        if a.is_nan() && b.is_nan() {
            Ordering::Equal
        } else if a.is_nan() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    })
}

/// Approximate Nash equilibrium via iterative best response (2-player, 2-strategy, backwards-compatible).
pub fn nash_approx(payoffs: &[f64]) -> Result<Vec<f64>> {
    // payoffs: [A1B1, A1B2, A2B1, A2B2] for player A
    if payoffs.len() < 4 {
        return Ok(vec![0.5, 0.5]);
    }
    let mut p = 0.5; // probability of playing strategy 1
    for _ in 0..100 {
        let ev1 = payoffs[0] * p + payoffs[1] * (1.0 - p);
        let ev2 = payoffs[2] * p + payoffs[3] * (1.0 - p);
        let diff = ev1 - ev2;
        p = 1.0 / (1.0 + (-diff).exp()); // Softmax best response
    }
    Ok(vec![p, 1.0 - p])
}

/// Vickrey auction (second-price sealed-bid, backwards-compatible [winner_amount, price]).
pub fn vickrey_auction(bids: &[f64]) -> Result<Vec<f64>> {
    if bids.len() < 2 {
        return Ok(vec![]);
    }
    let mut sorted = bids.to_vec();
    sorted.sort_by(|&a, &b| safe_f64_cmp(b, a));
    Ok(vec![sorted[0], sorted[1]]) // [highest bid, clearing price]
}

/// Structured Bid in an Auction
#[derive(Debug, Clone, PartialEq)]
pub struct AuctionBid {
    pub bidder_id: String,
    pub amount: f64,
}

/// Result of a Second-Price Vickrey Auction
#[derive(Debug, Clone, PartialEq)]
pub struct VickreyAuctionResult {
    pub winner_id: String,
    pub winning_bid: f64,
    pub clearing_price: f64,
    pub total_bids: usize,
}

/// Evaluates a second-price sealed-bid Vickrey auction with optional reserve price.
pub fn evaluate_vickrey_auction(bids: &[AuctionBid], reserve_price: f64) -> Option<VickreyAuctionResult> {
    if bids.is_empty() {
        return None;
    }

    let mut valid_bids: Vec<&AuctionBid> = bids.iter().filter(|b| b.amount >= reserve_price).collect();
    if valid_bids.is_empty() {
        return None;
    }

    valid_bids.sort_by(|a, b| safe_f64_cmp(b.amount, a.amount));

    let winner = valid_bids[0];
    let clearing_price = if valid_bids.len() > 1 {
        valid_bids[1].amount.max(reserve_price)
    } else {
        reserve_price
    };

    Some(VickreyAuctionResult {
        winner_id: winner.bidder_id.clone(),
        winning_bid: winner.amount,
        clearing_price,
        total_bids: bids.len(),
    })
}

/// 2-Player Normal-Form Game with arbitrary strategy dimensions (M x N).
#[derive(Debug, Clone)]
pub struct NormalFormGame {
    pub p1_strategies: usize,
    pub p2_strategies: usize,
    /// Payoff matrix for Player 1: [s1 * p2_strategies + s2]
    pub p1_payoffs: Vec<f64>,
    /// Payoff matrix for Player 2: [s1 * p2_strategies + s2]
    pub p2_payoffs: Vec<f64>,
}

impl NormalFormGame {
    pub fn new(p1_strategies: usize, p2_strategies: usize, p1_payoffs: Vec<f64>, p2_payoffs: Vec<f64>) -> Result<Self> {
        let expected_size = p1_strategies * p2_strategies;
        if p1_payoffs.len() != expected_size || p2_payoffs.len() != expected_size {
            bail!("Payoff matrix size mismatch: expected {}", expected_size);
        }
        Ok(Self {
            p1_strategies,
            p2_strategies,
            p1_payoffs,
            p2_payoffs,
        })
    }

    /// Finds all Pure Strategy Nash Equilibria: pairs (s1, s2) where neither player has incentive to deviate.
    pub fn pure_nash_equilibria(&self) -> Vec<(usize, usize)> {
        let mut equilibria = Vec::new();

        for s1 in 0..self.p1_strategies {
            for s2 in 0..self.p2_strategies {
                let idx = s1 * self.p2_strategies + s2;
                let current_p1 = self.p1_payoffs[idx];
                let current_p2 = self.p2_payoffs[idx];

                // Check Player 1 deviation
                let mut p1_is_best = true;
                for alt_s1 in 0..self.p1_strategies {
                    let alt_idx = alt_s1 * self.p2_strategies + s2;
                    if self.p1_payoffs[alt_idx] > current_p1 {
                        p1_is_best = false;
                        break;
                    }
                }

                // Check Player 2 deviation
                let mut p2_is_best = true;
                for alt_s2 in 0..self.p2_strategies {
                    let alt_idx = s1 * self.p2_strategies + alt_s2;
                    if self.p2_payoffs[alt_idx] > current_p2 {
                        p2_is_best = false;
                        break;
                    }
                }

                if p1_is_best && p2_is_best {
                    equilibria.push((s1, s2));
                }
            }
        }
        equilibria
    }
}

/// Computes Shapley values for an N-player cooperative game given characteristic function `v(coalition_bitmask)`.
pub fn compute_shapley_values(num_players: usize, v: impl Fn(u32) -> f64) -> Vec<f64> {
    if num_players == 0 || num_players > 16 {
        return vec![];
    }

    let mut factorials = vec![1.0; num_players + 1];
    for i in 1..=num_players {
        factorials[i] = factorials[i - 1] * i as f64;
    }
    let n_fact = factorials[num_players];

    let mut shapley = vec![0.0; num_players];
    let total_coalitions = 1u32 << num_players;

    for (i, val_slot) in shapley.iter_mut().enumerate().take(num_players) {
        let player_mask = 1u32 << i;
        let mut val = 0.0;

        for s in 0..total_coalitions {
            if (s & player_mask) == 0 {
                let size_s = s.count_ones() as usize;
                let weight = (factorials[size_s] * factorials[num_players - size_s - 1]) / n_fact;
                let marginal = v(s | player_mask) - v(s);
                val += weight * marginal;
            }
        }
        *val_slot = val;
    }

    shapley
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vickrey_auction_structured() {
        let bids = vec![
            AuctionBid { bidder_id: "NodeA".to_string(), amount: 15.0 },
            AuctionBid { bidder_id: "NodeB".to_string(), amount: 25.0 },
            AuctionBid { bidder_id: "NodeC".to_string(), amount: 10.0 },
        ];
        let res = evaluate_vickrey_auction(&bids, 5.0).unwrap();
        assert_eq!(res.winner_id, "NodeB");
        assert_eq!(res.winning_bid, 25.0);
        assert_eq!(res.clearing_price, 15.0);
    }

    #[test]
    fn test_pure_nash_prisoners_dilemma() {
        // Cooperate (0), Defect (1)
        // Payoffs: [CC=(-1, -1), CD=(-3, 0), DC=(0, -3), DD=(-2, -2)]
        let p1 = vec![-1.0, -3.0, 0.0, -2.0];
        let p2 = vec![-1.0, 0.0, -3.0, -2.0];
        let game = NormalFormGame::new(2, 2, p1, p2).unwrap();
        let eq = game.pure_nash_equilibria();
        // Unique pure Nash equilibrium is (Defect, Defect) -> (1, 1)
        assert_eq!(eq, vec![(1, 1)]);
    }

    #[test]
    fn test_shapley_fairness() {
        // 2 players: v(empty)=0, v(1)=10, v(2)=20, v(1,2)=50
        let v = |mask: u32| match mask {
            1 => 10.0,
            2 => 20.0,
            3 => 50.0,
            _ => 0.0,
        };
        let shapley = compute_shapley_values(2, v);
        // Player 1: 0.5 * (10 - 0) + 0.5 * (50 - 20) = 5 + 15 = 20
        // Player 2: 0.5 * (20 - 0) + 0.5 * (50 - 10) = 10 + 20 = 30
        assert_eq!(shapley[0], 20.0);
        assert_eq!(shapley[1], 30.0);
    }
}
