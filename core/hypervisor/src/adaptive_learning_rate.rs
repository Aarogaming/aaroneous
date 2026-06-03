// Adaptive Learning Rate Optimization
// Dynamically adjusts learning rates based on convergence signals

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Learning rate adjustment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningStrategy {
    /// Increase learning rate (convergence fast, not optimal)
    Accelerate,
    /// Maintain current learning rate (optimal convergence)
    Maintain,
    /// Decrease learning rate (oscillating, approaching optimum)
    Decelerate,
    /// Dramatically reduce (diverging, prevent instability)
    Emergency,
}

/// Loss trend analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossTrend {
    /// Loss consistently decreasing (good convergence)
    Improving,
    /// Loss oscillating around value (near optimum)
    Oscillating,
    /// Loss stagnant (plateau, consider strategy change)
    Stagnant,
    /// Loss increasing (diverging, need correction)
    Diverging,
}

/// Adaptive learning rate optimizer
pub struct AdaptiveLearningOptimizer {
    pub current_learning_rate: f64,
    pub min_learning_rate: f64,
    pub max_learning_rate: f64,
    pub loss_history: VecDeque<f64>,
    pub learning_rate_history: VecDeque<f64>,
    pub strategy: LearningStrategy,
    pub window_size: usize,
    pub adjustment_factor: f64,  // 1.1-1.2 typical (10-20% changes)
}

impl AdaptiveLearningOptimizer {
    /// Create a new adaptive learning rate optimizer
    pub fn new(initial_lr: f64, min_lr: f64, max_lr: f64) -> Self {
        println!("[LearningRateOptimizer] Initialized with LR={:.6}, min={:.6}, max={:.6}", 
            initial_lr, min_lr, max_lr);
        
        Self {
            current_learning_rate: initial_lr,
            min_learning_rate: min_lr,
            max_learning_rate: max_lr,
            loss_history: VecDeque::with_capacity(100),
            learning_rate_history: VecDeque::with_capacity(100),
            strategy: LearningStrategy::Maintain,
            window_size: 20,
            adjustment_factor: 1.15,  // 15% changes
        }
    }

    /// Record a loss measurement
    pub fn record_loss(&mut self, loss: f64) {
        self.loss_history.push_back(loss);
        self.learning_rate_history.push_back(self.current_learning_rate);
        
        // Keep histories bounded
        if self.loss_history.len() > 200 {
            self.loss_history.pop_front();
        }
        if self.learning_rate_history.len() > 200 {
            self.learning_rate_history.pop_front();
        }
    }

    /// Analyze loss trend and recommend learning rate adjustment
    pub fn optimize(&mut self) -> (LearningStrategy, f64) {
        if self.loss_history.len() < self.window_size {
            return (LearningStrategy::Maintain, self.current_learning_rate);
        }

        // Analyze recent loss trend
        let trend = self.analyze_trend();
        
        // Determine strategy based on trend
        let new_strategy = match trend {
            LossTrend::Improving => LearningStrategy::Accelerate,
            LossTrend::Oscillating => LearningStrategy::Maintain,
            LossTrend::Stagnant => LearningStrategy::Decelerate,
            LossTrend::Diverging => LearningStrategy::Emergency,
        };

        // Only change strategy if different
        if new_strategy != self.strategy {
            println!("[LearningRateOptimizer] Strategy change: {:?} → {:?}", 
                self.strategy, new_strategy);
            self.strategy = new_strategy;
        }

        // Adjust learning rate based on strategy
        let new_lr = self.apply_strategy(new_strategy);
        
        let old_lr = self.current_learning_rate;
        self.current_learning_rate = new_lr;
        
        if (new_lr - old_lr).abs() > 1e-8 {
            println!("[LearningRateOptimizer] LR adjusted: {:.6} → {:.6} ({:?})", 
                old_lr, new_lr, trend);
        }

        (new_strategy, new_lr)
    }

    /// Analyze loss trend
    fn analyze_trend(&self) -> LossTrend {
        let recent_losses: Vec<f64> = self.loss_history
            .iter()
            .rev()
            .take(self.window_size)
            .copied()
            .collect();
        
        if recent_losses.len() < 2 {
            return LossTrend::Stagnant;
        }

        // Calculate trend components
        let improving_count = recent_losses
            .windows(2)
            .filter(|w| w[1] < w[0])  // Loss decreased
            .count();
        
        let oscillating_count = recent_losses
            .windows(2)
            .filter(|w| (w[1] - w[0]).abs() < 0.01 * w[0].max(0.001))  // Small change
            .count();

        let diverging_count = recent_losses
            .windows(2)
            .filter(|w| w[1] > w[0] * 1.05)  // Loss increased >5%
            .count();

        // Determine dominant trend
        if improving_count as f32 / recent_losses.len() as f32 > 0.7 {
            LossTrend::Improving
        } else if oscillating_count as f32 / recent_losses.len() as f32 > 0.6 {
            LossTrend::Oscillating
        } else if diverging_count as f32 / recent_losses.len() as f32 > 0.5 {
            LossTrend::Diverging
        } else {
            LossTrend::Stagnant
        }
    }

    /// Apply learning rate strategy
    fn apply_strategy(&self, strategy: LearningStrategy) -> f64 {
        match strategy {
            LearningStrategy::Accelerate => {
                (self.current_learning_rate * self.adjustment_factor)
                    .min(self.max_learning_rate)
            }
            LearningStrategy::Maintain => {
                self.current_learning_rate
            }
            LearningStrategy::Decelerate => {
                (self.current_learning_rate / self.adjustment_factor)
                    .max(self.min_learning_rate)
            }
            LearningStrategy::Emergency => {
                (self.current_learning_rate / 2.0).max(self.min_learning_rate)
            }
        }
    }

    /// Get convergence metrics
    pub fn get_convergence_metrics(&self) -> ConvergenceMetrics {
        if self.loss_history.len() < 2 {
            return ConvergenceMetrics::default();
        }

        let losses: Vec<f64> = self.loss_history.iter().copied().collect();
        let initial_loss = losses[0];
        let final_loss = losses[losses.len() - 1];
        let min_loss = losses.iter().copied().fold(f64::INFINITY, f64::min);

        let improvement = ((initial_loss - final_loss) / initial_loss * 100.0).max(0.0);
        let distance_to_best = (final_loss - min_loss) / min_loss * 100.0;

        // Estimate convergence rate (improvements per 100 steps)
        let convergence_rate = if losses.len() > 100 {
            ((losses[0] - losses[100]) / losses[0] * 100.0) / 100.0
        } else {
            0.0
        };

        ConvergenceMetrics {
            initial_loss,
            final_loss,
            min_loss,
            improvement_percent: improvement,
            distance_to_best_percent: distance_to_best,
            convergence_rate_per_step: convergence_rate,
            total_steps: losses.len(),
            current_learning_rate: self.current_learning_rate,
        }
    }

    /// Estimate steps to convergence
    pub fn estimate_convergence_steps(&self) -> usize {
        let metrics = self.get_convergence_metrics();
        
        if metrics.convergence_rate_per_step <= 0.0 {
            return 0;
        }

        let remaining_improvement = metrics.distance_to_best_percent;
        let estimated_steps = (remaining_improvement / metrics.convergence_rate_per_step) as usize;
        
        estimated_steps.min(100000)  // Cap at 100k steps
    }
}

/// Convergence metrics
#[derive(Debug, Clone)]
pub struct ConvergenceMetrics {
    pub initial_loss: f64,
    pub final_loss: f64,
    pub min_loss: f64,
    pub improvement_percent: f64,
    pub distance_to_best_percent: f64,
    pub convergence_rate_per_step: f64,
    pub total_steps: usize,
    pub current_learning_rate: f64,
}

impl Default for ConvergenceMetrics {
    fn default() -> Self {
        Self {
            initial_loss: 0.0,
            final_loss: 0.0,
            min_loss: 0.0,
            improvement_percent: 0.0,
            distance_to_best_percent: 0.0,
            convergence_rate_per_step: 0.0,
            total_steps: 0,
            current_learning_rate: 0.001,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.1);
        assert_eq!(optimizer.current_learning_rate, 0.001);
        assert_eq!(optimizer.strategy, LearningStrategy::Maintain);
    }

    #[test]
    fn test_record_loss() {
        let mut optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.1);
        
        optimizer.record_loss(0.5);
        optimizer.record_loss(0.45);
        optimizer.record_loss(0.40);
        
        assert_eq!(optimizer.loss_history.len(), 3);
    }

    #[test]
    fn test_improving_trend() {
        let mut optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.1);
        
        // Record improving loss trend
        for i in 0..30 {
            optimizer.record_loss(1.0 - (i as f64 * 0.01));
        }
        
        let (strategy, _) = optimizer.optimize();
        assert_eq!(strategy, LearningStrategy::Accelerate);
    }

    #[test]
    fn test_oscillating_trend() {
        let mut optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.1);
        
        // Record oscillating loss trend
        let mut loss = 0.5;
        for _ in 0..30 {
            optimizer.record_loss(loss);
            loss = 0.5 + (loss - 0.5) * -0.95;  // Small oscillation
        }
        
        let (strategy, _) = optimizer.optimize();
        // Should suggest maintain or decelerate
        assert!(strategy == LearningStrategy::Maintain || strategy == LearningStrategy::Decelerate);
    }

    #[test]
    fn test_diverging_trend() {
        let mut optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.1);
        
        // Record diverging loss trend
        for i in 0..30 {
            optimizer.record_loss(0.5 + (i as f64 * 0.05));
        }
        
        let (strategy, _) = optimizer.optimize();
        assert_eq!(strategy, LearningStrategy::Emergency);
    }

    #[test]
    fn test_learning_rate_bounds() {
        let mut optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.01);
        
        // Test upper bound
        optimizer.record_loss(0.5);
        for i in 1..30 {
            optimizer.record_loss(0.5 - i as f64 * 0.001);
        }
        
        let (_, new_lr) = optimizer.optimize();
        assert!(new_lr <= 0.01);  // Should not exceed max
    }

    #[test]
    fn test_convergence_metrics() {
        let mut optimizer = AdaptiveLearningOptimizer::new(0.001, 0.0001, 0.1);
        
        for i in 0..50 {
            optimizer.record_loss(1.0 / (1.0 + i as f64 * 0.1));
        }
        
        let metrics = optimizer.get_convergence_metrics();
        assert!(metrics.improvement_percent > 0.0);
        assert!(metrics.convergence_rate_per_step > 0.0);
    }
}

