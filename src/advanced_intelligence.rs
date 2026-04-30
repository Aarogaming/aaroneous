/// Advanced Intelligence System for Autonomous Optimization
///
/// Self-optimizing system that learns from operational patterns:
/// - Anomaly detection using statistical analysis
/// - Predictive analytics & forecasting
/// - Intelligent auto-scaling based on demand
/// - Self-healing & automatic recovery
/// - Cost optimization & resource allocation
/// - Performance optimization recommendations
///
/// Continuously improves system efficiency without manual intervention

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f32,
    pub metric_name: String,
}

/// Statistical analysis of metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStatistics {
    pub metric_name: String,
    pub mean: f32,
    pub standard_deviation: f32,
    pub min: f32,
    pub max: f32,
    pub p50: f32,  // Median
    pub p95: f32,
    pub p99: f32,
}

impl MetricStatistics {
    pub fn calculate(points: &[f32]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }

        let mean = points.iter().sum::<f32>() / points.len() as f32;
        let variance = points
            .iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f32>() / points.len() as f32;
        let std_dev = variance.sqrt();

        let mut sorted = points.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = sorted.len() / 2;
        let p95_idx = (sorted.len() as f32 * 0.95) as usize;
        let p99_idx = (sorted.len() as f32 * 0.99) as usize;

        Some(MetricStatistics {
            metric_name: "unknown".to_string(),
            mean,
            standard_deviation: std_dev,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            p50: sorted[p50_idx],
            p95: sorted[p95_idx.min(sorted.len() - 1)],
            p99: sorted[p99_idx.min(sorted.len() - 1)],
        })
    }
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetection {
    pub anomaly_detected: bool,
    pub severity: AnomalySeverity,
    pub metric_name: String,
    pub current_value: f32,
    pub expected_range: (f32, f32),
    pub confidence: f32, // 0-1
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Anomaly detector using statistical analysis
pub struct AnomalyDetector {
    history: Arc<RwLock<Vec<TimeSeriesPoint>>>,
    thresholds: Arc<RwLock<AnomalyThresholds>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyThresholds {
    pub std_dev_multiplier: f32,  // 2.0 = 2 standard deviations
    pub min_data_points: usize,   // Minimum points for analysis
    pub max_history_days: u32,    // How long to keep history
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            std_dev_multiplier: 2.5,
            min_data_points: 10,
            max_history_days: 30,
        }
    }
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            thresholds: Arc::new(RwLock::new(AnomalyThresholds::default())),
        }
    }

    pub fn record_metric(&self, metric_name: String, value: f32) {
        let point = TimeSeriesPoint {
            timestamp: Utc::now(),
            value,
            metric_name,
        };

        let mut history = self.history.write().unwrap();
        history.push(point);

        // Cleanup old data
        let cutoff = Utc::now() - Duration::days(30);
        history.retain(|p| p.timestamp > cutoff);
    }

    pub fn detect_anomaly(&self, metric_name: &str, current_value: f32) -> AnomalyDetection {
        let history = self.history.read().unwrap();
        let thresholds = self.thresholds.read().unwrap();

        let values: Vec<f32> = history
            .iter()
            .filter(|p| p.metric_name == metric_name)
            .map(|p| p.value)
            .collect();

        if values.len() < thresholds.min_data_points {
            return AnomalyDetection {
                anomaly_detected: false,
                severity: AnomalySeverity::Low,
                metric_name: metric_name.to_string(),
                current_value,
                expected_range: (0.0, 100.0),
                confidence: 0.0,
                recommendation: "Insufficient data".to_string(),
            };
        }

        let stats = MetricStatistics::calculate(&values).unwrap();
        let lower_bound = stats.mean - (stats.standard_deviation * thresholds.std_dev_multiplier);
        let upper_bound = stats.mean + (stats.standard_deviation * thresholds.std_dev_multiplier);

        let is_anomaly = current_value < lower_bound || current_value > upper_bound;
        let confidence = if is_anomaly {
            ((current_value - stats.mean).abs() / (stats.standard_deviation + 0.1)) / 10.0
        } else {
            0.0
        };

        let severity = if confidence > 0.9 {
            AnomalySeverity::Critical
        } else if confidence > 0.7 {
            AnomalySeverity::High
        } else if confidence > 0.5 {
            AnomalySeverity::Medium
        } else {
            AnomalySeverity::Low
        };

        let recommendation = if is_anomaly {
            format!(
                "Metric {} is {} of normal range [{:.1}, {:.1}]. Current: {:.1}",
                metric_name,
                if current_value > upper_bound { "above" } else { "below" },
                lower_bound,
                upper_bound,
                current_value
            )
        } else {
            "Metric within normal range".to_string()
        };

        AnomalyDetection {
            anomaly_detected: is_anomaly,
            severity,
            metric_name: metric_name.to_string(),
            current_value,
            expected_range: (lower_bound, upper_bound),
            confidence: confidence.min(1.0),
            recommendation,
        }
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Predictive forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    pub metric_name: String,
    pub forecast_time: DateTime<Utc>,
    pub predicted_value: f32,
    pub confidence_interval: (f32, f32),
    pub trend: ForecastTrend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForecastTrend {
    Increasing,
    Decreasing,
    Stable,
}

/// Simple linear regression forecaster
pub struct Forecaster {
    history: Arc<RwLock<VecDeque<TimeSeriesPoint>>>,
}

impl Forecaster {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        }
    }

    pub fn record_point(&self, point: TimeSeriesPoint) {
        let mut history = self.history.write().unwrap();
        history.push_back(point);
        
        // Keep last 100 points
        while history.len() > 100 {
            history.pop_front();
        }
    }

    pub fn forecast(&self, metric_name: &str, hours_ahead: u32) -> Option<Forecast> {
        let history = self.history.read().unwrap();
        
        let points: Vec<(f64, f64)> = history
            .iter()
            .filter(|p| p.metric_name == metric_name)
            .enumerate()
            .map(|(i, p)| (i as f64, p.value as f64))
            .collect();

        if points.len() < 3 {
            return None;
        }

        // Simple linear regression
        let n = points.len() as f64;
        let sum_x = points.iter().map(|(x, _)| x).sum::<f64>();
        let sum_y = points.iter().map(|(_, y)| y).sum::<f64>();
        let sum_xy = points.iter().map(|(x, y)| x * y).sum::<f64>();
        let sum_x2 = points.iter().map(|(x, _)| x * x).sum::<f64>();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        let predicted_x = points.len() as f64;
        let predicted_value = (slope * predicted_x + intercept) as f32;

        let trend = if slope > 0.1 {
            ForecastTrend::Increasing
        } else if slope < -0.1 {
            ForecastTrend::Decreasing
        } else {
            ForecastTrend::Stable
        };

        let confidence = (0.5 + (slope.abs() / 10.0)).min(1.0) as f32;
        let ci_range = 10.0 * (1.0 - confidence);

        Some(Forecast {
            metric_name: metric_name.to_string(),
            forecast_time: Utc::now() + Duration::hours(hours_ahead as i64),
            predicted_value,
            confidence_interval: (
                (predicted_value - ci_range).max(0.0),
                predicted_value + ci_range,
            ),
            trend,
        })
    }
}

impl Default for Forecaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Auto-scaling decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingDecision {
    pub action: ScalingAction,
    pub target_nodes: u32,
    pub reason: String,
    pub urgency: ScalingUrgency,
    pub estimated_cost_change: f32, // % change
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown,
    NoChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScalingUrgency {
    Immediate,
    Soon,
    Gradual,
    None,
}

/// Intelligent auto-scaler
pub struct AutoScaler {
    current_nodes: Arc<RwLock<u32>>,
    min_nodes: u32,
    max_nodes: u32,
    cpu_threshold_high: f32,
    cpu_threshold_low: f32,
}

impl AutoScaler {
    pub fn new(min_nodes: u32, max_nodes: u32) -> Self {
        Self {
            current_nodes: Arc::new(RwLock::new(min_nodes)),
            min_nodes,
            max_nodes,
            cpu_threshold_high: 80.0,
            cpu_threshold_low: 30.0,
        }
    }

    pub fn decide_scaling(
        &self,
        current_cpu: f32,
        current_memory: f32,
        forecast_cpu: f32,
    ) -> ScalingDecision {
        let current = *self.current_nodes.read().unwrap();

        // Check if we need to scale up
        if current_cpu > self.cpu_threshold_high || forecast_cpu > 90.0 {
            let target = (current + (current / 2).max(1)).min(self.max_nodes);
            return ScalingDecision {
                action: ScalingAction::ScaleUp,
                target_nodes: target,
                reason: format!(
                    "High CPU: {:.1}% (forecast: {:.1}%)",
                    current_cpu, forecast_cpu
                ),
                urgency: if current_cpu > 90.0 {
                    ScalingUrgency::Immediate
                } else {
                    ScalingUrgency::Soon
                },
                estimated_cost_change: ((target as f32 - current as f32) / current as f32) * 100.0,
            };
        }

        // Check if we can scale down
        if current_cpu < self.cpu_threshold_low && current_memory < 50.0 && current > self.min_nodes {
            let target = (current / 2).max(self.min_nodes);
            return ScalingDecision {
                action: ScalingAction::ScaleDown,
                target_nodes: target,
                reason: format!("Low utilization: CPU {:.1}%, Memory {:.1}%", current_cpu, current_memory),
                urgency: ScalingUrgency::Gradual,
                estimated_cost_change: ((target as f32 - current as f32) / current as f32) * 100.0,
            };
        }

        ScalingDecision {
            action: ScalingAction::NoChange,
            target_nodes: current,
            reason: "System in optimal state".to_string(),
            urgency: ScalingUrgency::None,
            estimated_cost_change: 0.0,
        }
    }

    pub fn apply_scaling(&self, target_nodes: u32) -> Result<(), String> {
        if target_nodes < self.min_nodes || target_nodes > self.max_nodes {
            return Err("Target outside node limits".to_string());
        }

        let mut current = self.current_nodes.write().unwrap();
        *current = target_nodes;
        info!("Auto-scaling: {} nodes", target_nodes);
        Ok(())
    }
}

/// Self-healing action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHealingAction {
    pub action_type: HealingActionType,
    pub target_resource: String,
    pub description: String,
    pub severity: HealingSeverity,
    pub auto_execute: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealingActionType {
    RestartService,
    ClearCache,
    RebalanceLoad,
    FailoverNode,
    RestoreBackup,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealingSeverity {
    Low,
    Medium,
    High,
}

/// Self-healing engine
pub struct SelfHealingEngine {
    recent_failures: Arc<RwLock<VecDeque<(DateTime<Utc>, String)>>>,
}

impl SelfHealingEngine {
    pub fn new() -> Self {
        Self {
            recent_failures: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        }
    }

    pub fn record_failure(&self, component: String) {
        let mut failures = self.recent_failures.write().unwrap();
        failures.push_back((Utc::now(), component));
        
        while failures.len() > 100 {
            failures.pop_front();
        }
    }

    pub fn diagnose_and_heal(&self, component: &str) -> Option<SelfHealingAction> {
        let failures = self.recent_failures.read().unwrap();
        let recent_count = failures
            .iter()
            .filter(|(t, c)| {
                c == component && Utc::now().signed_duration_since(*t).num_hours() < 1
            })
            .count();

        if recent_count == 0 {
            return None;
        }

        let action = if recent_count >= 3 {
            HealingActionType::RestartService
        } else if recent_count >= 2 {
            HealingActionType::ClearCache
        } else {
            HealingActionType::RebalanceLoad
        };

        let severity = match recent_count {
            3.. => HealingSeverity::High,
            2 => HealingSeverity::Medium,
            _ => HealingSeverity::Low,
        };

        Some(SelfHealingAction {
            action_type: action,
            target_resource: component.to_string(),
            description: format!(
                "{} failures in last hour. Attempting {}",
                recent_count,
                format!("{:?}", action)
            ),
            severity,
            auto_execute: severity == HealingSeverity::High,
        })
    }
}

impl Default for SelfHealingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub recommendation_id: String,
    pub category: OptimizationCategory,
    pub title: String,
    pub description: String,
    pub estimated_improvement: f32, // % improvement
    pub estimated_cost_savings: f32, // % savings
    pub implementation_difficulty: DifficultyLevel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationCategory {
    Performance,
    Cost,
    Reliability,
    Security,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
}

/// Optimization engine
pub struct OptimizationEngine;

impl OptimizationEngine {
    pub fn analyze_system(
        &self,
        avg_cpu: f32,
        avg_memory: f32,
        cache_hit_rate: f32,
        error_rate: f32,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // Cache optimization
        if cache_hit_rate < 0.7 {
            recommendations.push(OptimizationRecommendation {
                recommendation_id: uuid::Uuid::new_v4().to_string(),
                category: OptimizationCategory::Performance,
                title: "Increase cache sizes".to_string(),
                description: "L1/L2/L3 cache hit rates are below optimal. Consider increasing cache sizes or TTLs.".to_string(),
                estimated_improvement: 15.0,
                estimated_cost_savings: 10.0,
                implementation_difficulty: DifficultyLevel::Easy,
            });
        }

        // Memory optimization
        if avg_memory > 70.0 {
            recommendations.push(OptimizationRecommendation {
                recommendation_id: uuid::Uuid::new_v4().to_string(),
                category: OptimizationCategory::Cost,
                title: "Enable memory compression".to_string(),
                description: "High memory usage detected. Enable compression for cold data.".to_string(),
                estimated_improvement: 20.0,
                estimated_cost_savings: 30.0,
                implementation_difficulty: DifficultyLevel::Medium,
            });
        }

        // CPU optimization
        if avg_cpu > 75.0 {
            recommendations.push(OptimizationRecommendation {
                recommendation_id: uuid::Uuid::new_v4().to_string(),
                category: OptimizationCategory::Performance,
                title: "Scale up or optimize CPU-intensive operations".to_string(),
                description: "High CPU utilization. Consider horizontal scaling or query optimization.".to_string(),
                estimated_improvement: 25.0,
                estimated_cost_savings: 0.0,
                implementation_difficulty: DifficultyLevel::Hard,
            });
        }

        // Error rate optimization
        if error_rate > 0.5 {
            recommendations.push(OptimizationRecommendation {
                recommendation_id: uuid::Uuid::new_v4().to_string(),
                category: OptimizationCategory::Reliability,
                title: "Implement retry logic and circuit breakers".to_string(),
                description: "High error rate detected. Review error logs and implement resilience patterns.".to_string(),
                estimated_improvement: 30.0,
                estimated_cost_savings: 5.0,
                implementation_difficulty: DifficultyLevel::Medium,
            });
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_statistics() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = MetricStatistics::calculate(&values).unwrap();
        assert_eq!(stats.mean, 30.0);
        assert!(stats.standard_deviation > 0.0);
    }

    #[test]
    fn test_anomaly_detector_creation() {
        let detector = AnomalyDetector::new();
        detector.record_metric("cpu".to_string(), 50.0);
        
        let anomaly = detector.detect_anomaly("cpu", 500.0);
        assert!(!anomaly.anomaly_detected); // Not enough data
    }

    #[test]
    fn test_anomaly_detection() {
        let detector = AnomalyDetector::new();
        for i in 0..20 {
            detector.record_metric("cpu".to_string(), 50.0 + (i as f32 % 5.0));
        }
        
        let anomaly = detector.detect_anomaly("cpu", 150.0);
        assert!(anomaly.anomaly_detected);
    }

    #[test]
    fn test_forecaster_creation() {
        let forecaster = Forecaster::new();
        for i in 0..10 {
            forecaster.record_point(TimeSeriesPoint {
                timestamp: Utc::now(),
                value: (i as f32) * 10.0,
                metric_name: "test".to_string(),
            });
        }
        
        let forecast = forecaster.forecast("test", 1);
        assert!(forecast.is_some());
    }

    #[test]
    fn test_auto_scaler_scale_up() {
        let scaler = AutoScaler::new(2, 10);
        let decision = scaler.decide_scaling(85.0, 60.0, 95.0);
        assert_eq!(decision.action, ScalingAction::ScaleUp);
    }

    #[test]
    fn test_auto_scaler_scale_down() {
        let scaler = AutoScaler::new(2, 10);
        scaler.apply_scaling(8).ok(); // Start with 8 nodes
        let decision = scaler.decide_scaling(20.0, 40.0, 25.0);
        assert_eq!(decision.action, ScalingAction::ScaleDown);
    }

    #[test]
    fn test_auto_scaler_apply_scaling() {
        let scaler = AutoScaler::new(2, 10);
        assert!(scaler.apply_scaling(5).is_ok());
        assert!(scaler.apply_scaling(15).is_err());
    }

    #[test]
    fn test_self_healing_engine() {
        let engine = SelfHealingEngine::new();
        engine.record_failure("service-1".to_string());
        engine.record_failure("service-1".to_string());
        engine.record_failure("service-1".to_string());
        
        let action = engine.diagnose_and_heal("service-1");
        assert!(action.is_some());
        assert_eq!(action.unwrap().action_type, HealingActionType::RestartService);
    }

    #[test]
    fn test_optimization_recommendations() {
        let engine = OptimizationEngine;
        let recommendations = engine.analyze_system(85.0, 75.0, 0.6, 1.0);
        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_anomaly_severity() {
        let detector = AnomalyDetector::new();
        for _ in 0..20 {
            detector.record_metric("metric".to_string(), 50.0);
        }
        
        let anomaly = detector.detect_anomaly("metric", 200.0);
        assert!(anomaly.anomaly_detected);
        assert!(anomaly.severity != AnomalySeverity::Low);
    }

    #[test]
    fn test_forecast_trend() {
        let forecaster = Forecaster::new();
        for i in 0..20 {
            forecaster.record_point(TimeSeriesPoint {
                timestamp: Utc::now(),
                value: (i as f32) * 5.0,
                metric_name: "increasing".to_string(),
            });
        }
        
        let forecast = forecaster.forecast("increasing", 1);
        assert!(forecast.is_some());
        assert_eq!(forecast.unwrap().trend, ForecastTrend::Increasing);
    }
}
