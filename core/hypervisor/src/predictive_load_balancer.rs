// Predictive Load Balancing for Specialists
// Forecasts specialist workload and pre-distributes tasks intelligently

use std::collections::HashMap;
use std::time::Instant;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Historical load measurement for a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMeasurement {
    pub specialist_id: String,
    pub timestamp: DateTime<Utc>,
    pub queue_depth: usize,
    pub tokens_available: f32,
    pub execution_latency_us: u64,
    pub success_rate: f32,
}

/// Predicted load for upcoming period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPrediction {
    pub specialist_id: String,
    pub predicted_queue_depth: f32,
    pub predicted_latency_us: f32,
    pub confidence: f32,  // 0.0-1.0
    pub recommendation: LoadRecommendation,
}

/// Recommendation for task allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadRecommendation {
    /// Accept more tasks (load low)
    AcceptMore,
    /// Current load appropriate
    Balanced,
    /// Reduce incoming tasks (load high)
    Reduce,
    /// Reject new tasks (overloaded)
    Reject,
}

/// Predictive load balancer
pub struct PredictiveLoadBalancer {
    pub specialists: Vec<String>,
    pub measurements: HashMap<String, Vec<LoadMeasurement>>,
    pub predictions: HashMap<String, LoadPrediction>,
    pub max_history: usize,
    pub prediction_window_secs: u64,  // Predict N seconds ahead
    pub last_prediction: Option<DateTime<Utc>>,
}

impl PredictiveLoadBalancer {
    /// Create a new predictive load balancer
    pub fn new(specialists: Vec<String>, prediction_window_secs: u64) -> Self {
        println!("[LoadBalancer] Initialized with {} specialists, {}s prediction window",
            specialists.len(), prediction_window_secs);
        
        let mut measurements = HashMap::new();
        for specialist in &specialists {
            measurements.insert(specialist.clone(), Vec::new());
        }
        
        Self {
            specialists,
            measurements,
            predictions: HashMap::new(),
            max_history: 300,  // 5 minutes at 1-second intervals
            prediction_window_secs,
            last_prediction: None,
        }
    }

    /// Record a load measurement for a specialist
    pub fn record_measurement(
        &mut self,
        specialist_id: &str,
        queue_depth: usize,
        tokens_available: f32,
        execution_latency_us: u64,
        success_rate: f32,
    ) {
        let measurement = LoadMeasurement {
            specialist_id: specialist_id.to_string(),
            timestamp: Utc::now(),
            queue_depth,
            tokens_available,
            execution_latency_us,
            success_rate,
        };

        if let Some(history) = self.measurements.get_mut(specialist_id) {
            history.push(measurement);
            
            // Keep history bounded
            if history.len() > self.max_history {
                history.remove(0);
            }
        }
    }

    /// Predict load for all specialists
    pub fn predict_loads(&mut self) -> HashMap<String, LoadPrediction> {
        self.last_prediction = Some(Utc::now());
        
        for specialist_id in &self.specialists.clone() {
            if let Some(history) = self.measurements.get(specialist_id) {
                if history.len() >= 2 {
                    let prediction = self.predict_specialist_load(specialist_id, history);
                    self.predictions.insert(specialist_id.clone(), prediction);
                }
            }
        }
        
        println!("[LoadBalancer] Predictions updated for {} specialists", 
            self.predictions.len());
        
        self.predictions.clone()
    }

    /// Predict load for a specific specialist using trend analysis
    fn predict_specialist_load(
        &self,
        specialist_id: &str,
        history: &[LoadMeasurement],
    ) -> LoadPrediction {
        if history.is_empty() {
            return LoadPrediction {
                specialist_id: specialist_id.to_string(),
                predicted_queue_depth: 0.0,
                predicted_latency_us: 0.0,
                confidence: 0.0,
                recommendation: LoadRecommendation::Balanced,
            };
        }

        // Calculate trends
        let recent = &history[history.len().saturating_sub(10)..];
        
        let avg_queue: f32 = recent.iter()
            .map(|m| m.queue_depth as f32)
            .sum::<f32>() / recent.len().max(1) as f32;
        
        let avg_latency: f32 = recent.iter()
            .map(|m| m.execution_latency_us as f32)
            .sum::<f32>() / recent.len().max(1) as f32;
        
        let avg_success: f32 = recent.iter()
            .map(|m| m.success_rate)
            .sum::<f32>() / recent.len().max(1) as f32;

        // Calculate trend (simple linear extrapolation)
        let queue_trend = if recent.len() >= 2 {
            (recent[recent.len()-1].queue_depth as f32 - 
             recent[0].queue_depth as f32) / recent.len() as f32
        } else {
            0.0
        };

        // Project forward
        let predicted_queue = (avg_queue + queue_trend * 5.0).max(0.0);
        
        // Confidence based on consistency
        let variance = recent.iter()
            .map(|m| (m.queue_depth as f32 - avg_queue).powi(2))
            .sum::<f32>() / recent.len().max(1) as f32;
        
        let confidence = (1.0 - (variance / (avg_queue.max(1.0) * avg_queue.max(1.0))).min(1.0)).max(0.3);

        // Generate recommendation
        let recommendation = if predicted_queue < 2.0 && avg_success > 0.95 {
            LoadRecommendation::AcceptMore
        } else if predicted_queue < 5.0 && avg_success > 0.90 {
            LoadRecommendation::Balanced
        } else if predicted_queue < 10.0 {
            LoadRecommendation::Reduce
        } else {
            LoadRecommendation::Reject
        };

        LoadPrediction {
            specialist_id: specialist_id.to_string(),
            predicted_queue_depth: predicted_queue,
            predicted_latency_us: avg_latency * (1.0 + predicted_queue / avg_queue.max(1.0)),
            confidence,
            recommendation,
        }
    }

    /// Get best specialist for a new task
    pub fn select_best_specialist(&self, task_type: &str) -> Option<String> {
        let mut best_specialist: Option<String> = None;
        let mut best_score: f32 = -1.0;

        for (specialist_id, prediction) in &self.predictions {
            // Score based on recommendation and confidence
            let score = match prediction.recommendation {
                LoadRecommendation::AcceptMore => 1.0,
                LoadRecommendation::Balanced => 0.8,
                LoadRecommendation::Reduce => 0.3,
                LoadRecommendation::Reject => -1.0,
            } * prediction.confidence;

            if score > best_score {
                best_score = score;
                best_specialist = Some(specialist_id.clone());
            }
        }

        if let Some(specialist) = &best_specialist {
            println!("[LoadBalancer] Selected {} for {} task (score: {:.2})", 
                specialist, task_type, best_score);
        }

        best_specialist
    }

    /// Get load statistics
    pub fn get_statistics(&self) -> LoadBalancerStatistics {
        let mut total_queue_depth = 0usize;
        let mut total_tokens = 0.0f32;
        let mut count = 0;

        for history in self.measurements.values() {
            if let Some(latest) = history.last() {
                total_queue_depth += latest.queue_depth;
                total_tokens += latest.tokens_available;
                count += 1;
            }
        }

        let avg_queue = if count > 0 {
            total_queue_depth as f32 / count as f32
        } else {
            0.0
        };

        let avg_tokens = if count > 0 {
            total_tokens / count as f32
        } else {
            0.0
        };

        let overloaded = self.predictions.values()
            .filter(|p| p.recommendation == LoadRecommendation::Reject)
            .count();

        LoadBalancerStatistics {
            avg_queue_depth: avg_queue,
            avg_tokens_available: avg_tokens,
            specialists_overloaded: overloaded,
            total_specialists: self.specialists.len(),
            prediction_confidence: self.predictions.values()
                .map(|p| p.confidence)
                .sum::<f32>() / self.predictions.len().max(1) as f32,
        }
    }

    /// Recommend task distribution strategy
    pub fn recommend_distribution(&self) -> DistributionStrategy {
        let stats = self.get_statistics();
        
        let accept_more = self.predictions.values()
            .filter(|p| p.recommendation == LoadRecommendation::AcceptMore)
            .count();
        
        let balanced = self.predictions.values()
            .filter(|p| p.recommendation == LoadRecommendation::Balanced)
            .count();

        if accept_more > 0 {
            DistributionStrategy::Aggressive
        } else if balanced >= self.specialists.len() / 2 {
            DistributionStrategy::Normal
        } else if stats.specialists_overloaded > 0 {
            DistributionStrategy::Conservative
        } else {
            DistributionStrategy::Emergency
        }
    }
}

/// Statistics about current load balancing state
#[derive(Debug, Clone)]
pub struct LoadBalancerStatistics {
    pub avg_queue_depth: f32,
    pub avg_tokens_available: f32,
    pub specialists_overloaded: usize,
    pub total_specialists: usize,
    pub prediction_confidence: f32,
}

/// Task distribution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionStrategy {
    /// Accept all tasks, pre-allocate aggressively
    Aggressive,
    /// Normal balanced distribution
    Normal,
    /// Conservative distribution, defer lower-priority tasks
    Conservative,
    /// Emergency mode, only critical tasks
    Emergency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer_creation() {
        let specialists = vec!["spec_a".to_string(), "spec_b".to_string()];
        let balancer = PredictiveLoadBalancer::new(specialists, 10);
        
        assert_eq!(balancer.specialists.len(), 2);
        assert_eq!(balancer.prediction_window_secs, 10);
    }

    #[test]
    fn test_record_measurement() {
        let specialists = vec!["spec_a".to_string()];
        let mut balancer = PredictiveLoadBalancer::new(specialists, 10);
        
        balancer.record_measurement("spec_a", 3, 50.0, 100, 0.95);
        
        assert_eq!(balancer.measurements["spec_a"].len(), 1);
        let measurement = &balancer.measurements["spec_a"][0];
        assert_eq!(measurement.queue_depth, 3);
        assert_eq!(measurement.tokens_available, 50.0);
    }

    #[test]
    fn test_predict_loads() {
        let specialists = vec!["spec_a".to_string()];
        let mut balancer = PredictiveLoadBalancer::new(specialists, 10);
        
        // Record multiple measurements to establish trend
        for i in 0..5 {
            balancer.record_measurement("spec_a", i, 100.0 - i as f32 * 10.0, 100, 0.95);
        }
        
        balancer.predict_loads();
        
        assert!(balancer.predictions.contains_key("spec_a"));
        let pred = &balancer.predictions["spec_a"];
        assert!(pred.confidence > 0.0);
    }

    #[test]
    fn test_select_best_specialist() {
        let specialists = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut balancer = PredictiveLoadBalancer::new(specialists, 10);
        
        balancer.record_measurement("spec_a", 2, 100.0, 100, 0.99);
        balancer.record_measurement("spec_b", 10, 10.0, 500, 0.80);
        
        balancer.predict_loads();
        
        let selected = balancer.select_best_specialist("test_task");
        assert_eq!(selected, Some("spec_a".to_string()));
    }

    #[test]
    fn test_distribution_strategy() {
        let specialists = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut balancer = PredictiveLoadBalancer::new(specialists, 10);
        
        balancer.record_measurement("spec_a", 1, 100.0, 50, 0.99);
        balancer.record_measurement("spec_b", 2, 95.0, 60, 0.98);
        
        balancer.predict_loads();
        
        let strategy = balancer.recommend_distribution();
        assert!(strategy != DistributionStrategy::Emergency);
    }

    #[test]
    fn test_load_balancer_statistics() {
        let specialists = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut balancer = PredictiveLoadBalancer::new(specialists, 10);
        
        balancer.record_measurement("spec_a", 3, 80.0, 100, 0.95);
        balancer.record_measurement("spec_b", 2, 90.0, 100, 0.98);
        
        let stats = balancer.get_statistics();
        assert_eq!(stats.total_specialists, 2);
        assert!(stats.avg_queue_depth > 0.0);
    }
}

