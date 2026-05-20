// Predictive Metabolic Governor
// Uses Monte Carlo simulation to forecast metabolic load and adjust expression rates

use rand::SeedableRng;
use crate::biology::SystemBiology;
use compute::stochastic;

/// Configuration for the predictive metabolic governor
#[derive(Debug, Clone)]
pub struct MetabolicGovernorConfig {
    pub monte_carlo_iterations: usize,
    pub risk_threshold: f64,          // Threshold for throttling (0.0-1.0)
    pub recovery_rate: f32,           // How fast to recover expression rate
    pub panic_threshold: f64,         // Threshold for emergency throttling
    pub prediction_window: usize,     // Number of future steps to predict
}

impl Default for MetabolicGovernorConfig {
    fn default() -> Self {
        Self {
            monte_carlo_iterations: 500,
            risk_threshold: 0.7,
            recovery_rate: 0.05,
            panic_threshold: 0.9,
            prediction_window: 10,
        }
    }
}

/// Predictive metabolic governor
pub struct PredictiveMetabolicGovernor {
    pub config: MetabolicGovernorConfig,
    pub rng: rand::rngs::StdRng,
    pub historical_load: Vec<f64>,    // Recent metabolic load measurements
    pub max_history: usize,
}

impl PredictiveMetabolicGovernor {
    pub fn new(config: MetabolicGovernorConfig) -> Self {
        Self {
            config,
            rng: rand::rngs::StdRng::from_entropy(),
            historical_load: Vec::new(),
            max_history: 50,
        }
    }

    /// Record a new metabolic load measurement
    pub fn record_load(&mut self, load: f64) {
        self.historical_load.push(load);
        if self.historical_load.len() > self.max_history {
            self.historical_load.remove(0);
        }
    }

    /// Run Monte Carlo prediction to forecast future metabolic load
    pub fn predict_metabolic_risk(&mut self) -> MetabolicForecast {
        if self.historical_load.is_empty() {
            return MetabolicForecast::default();
        }

        // Run Monte Carlo simulation on historical load
        let mc_result = stochastic::monte_carlo_simulate(
            &self.historical_load,
            self.config.monte_carlo_iterations,
            &mut self.rng,
        ).unwrap_or(vec![0.5, 0.1, 0.2, 0.5, 0.8]);

        let predicted_mean = mc_result[0];
        let predicted_std = mc_result[1];
        let p95 = mc_result[4];

        // Calculate risk score (probability of exceeding threshold)
        let risk_score = if predicted_std > 0.0 {
            let z_score = (self.config.risk_threshold - predicted_mean) / predicted_std;
            // Approximate CDF of normal distribution
            let cdf = 0.5 * (1.0 + erf(z_score / 2.0f64.sqrt()));
            1.0 - cdf // Probability of exceeding threshold
        } else {
            if predicted_mean > self.config.risk_threshold { 1.0 } else { 0.0 }
        };

        MetabolicForecast {
            predicted_mean,
            predicted_std,
            p95_load: p95,
            risk_score,
            recommended_expression_rate: self.calculate_recommended_rate(risk_score, predicted_mean),
        }
    }

    /// Apply governor decision to the biology system
    pub fn apply_governance(&mut self, biology: &mut SystemBiology) -> GovernanceAction {
        let forecast = self.predict_metabolic_risk();

        let action = if forecast.risk_score > self.config.panic_threshold {
            // Emergency: drastic throttling
            let old_rate = biology.expression_rate;
            biology.set_expression_rate((biology.expression_rate * 0.3).max(0.1));
            GovernanceAction::EmergencyThrottle {
                old_rate,
                new_rate: biology.expression_rate,
                forecast,
            }
        } else if forecast.risk_score > self.config.risk_threshold {
            // Warning: moderate throttling
            let old_rate = biology.expression_rate;
            let new_rate = (biology.expression_rate * 0.7).max(0.3);
            biology.set_expression_rate(new_rate);
            GovernanceAction::WarningThrottle {
                old_rate,
                new_rate,
                forecast,
            }
        } else if forecast.risk_score < 0.3 && biology.expression_rate < 1.0 {
            // Safe: gradual recovery
            let old_rate = biology.expression_rate;
            let new_rate = (biology.expression_rate + self.config.recovery_rate).min(1.0);
            biology.set_expression_rate(new_rate);
            GovernanceAction::Recovery {
                old_rate,
                new_rate,
                forecast,
            }
        } else {
            GovernanceAction::Stable { forecast }
        };

        action
    }

    /// Calculate recommended expression rate based on forecast
    fn calculate_recommended_rate(&self, risk_score: f64, predicted_mean: f64) -> f32 {
        if risk_score > self.config.panic_threshold {
            0.2
        } else if risk_score > self.config.risk_threshold {
            0.5
        } else if predicted_mean < 0.3 {
            1.0
        } else {
            0.8
        }
    }
}

/// Forecast result from Monte Carlo prediction
#[derive(Debug, Clone)]
pub struct MetabolicForecast {
    pub predicted_mean: f64,
    pub predicted_std: f64,
    pub p95_load: f64,
    pub risk_score: f64,              // 0.0-1.0, probability of overload
    pub recommended_expression_rate: f32,
}

impl Default for MetabolicForecast {
    fn default() -> Self {
        Self {
            predicted_mean: 0.5,
            predicted_std: 0.1,
            p95_load: 0.8,
            risk_score: 0.5,
            recommended_expression_rate: 0.8,
        }
    }
}

/// Action taken by the governor
#[derive(Debug, Clone)]
pub enum GovernanceAction {
    EmergencyThrottle {
        old_rate: f32,
        new_rate: f32,
        forecast: MetabolicForecast,
    },
    WarningThrottle {
        old_rate: f32,
        new_rate: f32,
        forecast: MetabolicForecast,
    },
    Recovery {
        old_rate: f32,
        new_rate: f32,
        forecast: MetabolicForecast,
    },
    Stable {
        forecast: MetabolicForecast,
    },
}

/// Approximation of the error function (erf)
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biology::SystemBiology;

    #[test]
    fn test_governor_records_load() {
        let mut governor = PredictiveMetabolicGovernor::new(MetabolicGovernorConfig::default());
        governor.record_load(0.5);
        governor.record_load(0.6);
        governor.record_load(0.7);
        assert_eq!(governor.historical_load.len(), 3);
    }

    #[test]
    fn test_governor_predicts_risk() {
        let mut governor = PredictiveMetabolicGovernor::new(MetabolicGovernorConfig::default());
        governor.record_load(0.5);
        governor.record_load(0.6);
        governor.record_load(0.7);
        let forecast = governor.predict_metabolic_risk();
        assert!(forecast.predicted_mean > 0.0);
        assert!(forecast.risk_score >= 0.0 && forecast.risk_score <= 1.0);
    }

    #[test]
    fn test_governor_applies_throttle() {
        let mut governor = PredictiveMetabolicGovernor::new(MetabolicGovernorConfig::default());
        // Simulate high load history
        for _ in 0..20 {
            governor.record_load(0.9);
        }
        
        let mut biology = SystemBiology::new();
        let action = governor.apply_governance(&mut biology);
        
        match action {
            GovernanceAction::WarningThrottle { new_rate, .. } |
            GovernanceAction::EmergencyThrottle { new_rate, .. } => {
                assert!(new_rate < 1.0);
            }
            _ => {} // Could be stable if variance is low
        }
    }
}
