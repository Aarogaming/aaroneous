// Metrics Aggregation and Performance Monitoring
// Real-time collection, aggregation, and performance tracking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A single metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: u64,
    pub value: f64,
    pub tags: HashMap<String, String>,
}

/// Aggregated metric statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    pub name: String,
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub percentile_50: f64,
    pub percentile_95: f64,
    pub percentile_99: f64,
}

/// Performance counter for specific operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCounter {
    pub name: String,
    pub call_count: u64,
    pub total_time_us: u64,
    pub min_time_us: u64,
    pub max_time_us: u64,
    pub average_time_us: u64,
    pub last_call_time_us: u64,
}

/// System-wide metrics aggregator
pub struct MetricsAggregator {
    pub metrics: Arc<Mutex<HashMap<String, Vec<MetricPoint>>>>,
    pub counters: Arc<Mutex<HashMap<String, PerformanceCounter>>>,
    pub sampling_rate: f32, // 0.0 to 1.0
    pub max_points_per_metric: usize,
    pub retention_seconds: u64,
    pub start_time: Instant,
}

impl MetricsAggregator {
    /// Create new metrics aggregator
    pub fn new(sampling_rate: f32, max_points: usize, retention_sec: u64) -> Self {
        println!(
            "[MetricsAggregator] Initialized (sampling: {:.1}%, retention: {}s)",
            sampling_rate * 100.0,
            retention_sec
        );

        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            counters: Arc::new(Mutex::new(HashMap::new())),
            sampling_rate,
            max_points_per_metric: max_points,
            retention_seconds: retention_sec,
            start_time: Instant::now(),
        }
    }

    /// Record a metric value
    pub fn record_metric(&self, name: &str, value: f64, tags: HashMap<String, String>) {
        // Apply sampling
        if rand::random::<f32>() > self.sampling_rate {
            return;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let point = MetricPoint {
            timestamp: now,
            value,
            tags,
        };

        let mut metrics = self.metrics.lock().unwrap();
        metrics.entry(name.to_string()).or_default().push(point);

        // Enforce max points
        if let Some(points) = metrics.get_mut(name)
            && points.len() > self.max_points_per_metric
        {
            points.remove(0);
        }
    }

    /// Record operation timing
    pub fn record_operation(&self, name: &str, time_us: u64) {
        let mut counters = self.counters.lock().unwrap();

        counters
            .entry(name.to_string())
            .and_modify(|c| {
                c.call_count += 1;
                c.total_time_us += time_us;
                c.min_time_us = c.min_time_us.min(time_us);
                c.max_time_us = c.max_time_us.max(time_us);
                c.average_time_us = c.total_time_us / c.call_count;
                c.last_call_time_us = time_us;
            })
            .or_insert_with(|| PerformanceCounter {
                name: name.to_string(),
                call_count: 1,
                total_time_us: time_us,
                min_time_us: time_us,
                max_time_us: time_us,
                average_time_us: time_us,
                last_call_time_us: time_us,
            });
    }

    /// Get aggregated stats for a metric
    pub fn get_metric_stats(&self, name: &str) -> Option<MetricStats> {
        let metrics = self.metrics.lock().unwrap();

        if let Some(points) = metrics.get(name) {
            if points.is_empty() {
                return None;
            }

            let values: Vec<f64> = points.iter().map(|p| p.value).collect();
            let count = values.len() as u64;
            let sum: f64 = values.iter().sum();
            let mean = sum / count as f64;

            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let percentile_50 = sorted[sorted.len() / 2];
            let percentile_95 = sorted[std::cmp::min(95 * sorted.len() / 100, sorted.len() - 1)];
            let percentile_99 = sorted[std::cmp::min(99 * sorted.len() / 100, sorted.len() - 1)];

            Some(MetricStats {
                name: name.to_string(),
                count,
                sum,
                min: sorted[0],
                max: sorted[sorted.len() - 1],
                mean,
                percentile_50,
                percentile_95,
                percentile_99,
            })
        } else {
            None
        }
    }

    /// Get performance counter
    pub fn get_counter(&self, name: &str) -> Option<PerformanceCounter> {
        let counters = self.counters.lock().unwrap();
        counters.get(name).cloned()
    }

    /// Get all counters
    pub fn get_all_counters(&self) -> Vec<PerformanceCounter> {
        let counters = self.counters.lock().unwrap();
        counters.values().cloned().collect()
    }

    /// Generate performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let metrics = self.metrics.lock().unwrap();
        let counters = self.counters.lock().unwrap();

        let metric_stats: Vec<_> = metrics
            .keys()
            .filter_map(|name| self.get_metric_stats(name))
            .collect();

        let counter_stats: Vec<_> = counters.values().cloned().collect();

        let uptime_sec = self.start_time.elapsed().as_secs();

        PerformanceReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            uptime_seconds: uptime_sec,
            metric_count: metrics.len(),
            counter_count: counters.len(),
            metric_stats,
            counter_stats,
            total_data_points: metrics.values().map(|v| v.len()).sum(),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        let mut counters = self.counters.lock().unwrap();

        metrics.clear();
        counters.clear();

        println!("[MetricsAggregator] Metrics reset");
    }

    /// Clean old data points
    pub fn cleanup_old_data(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff = now - self.retention_seconds;

        let mut metrics = self.metrics.lock().unwrap();

        for points in metrics.values_mut() {
            points.retain(|p| p.timestamp > cutoff);
        }

        // Remove empty metric entries
        metrics.retain(|_, points| !points.is_empty());
    }

    /// Get system health summary
    pub fn get_health_summary(&self) -> SystemHealthSummary {
        let counters = self.counters.lock().unwrap();

        let counters_list: Vec<_> = counters.values().collect();

        if counters_list.is_empty() {
            return SystemHealthSummary::default();
        }

        let avg_operation_time: u64 = counters_list.iter().map(|c| c.average_time_us).sum::<u64>()
            / counters_list.len() as u64;

        let max_operation_time = counters_list
            .iter()
            .map(|c| c.max_time_us)
            .max()
            .unwrap_or(0);

        let total_operations: u64 = counters_list.iter().map(|c| c.call_count).sum();

        SystemHealthSummary {
            total_operations,
            average_operation_us: avg_operation_time,
            max_operation_us: max_operation_time,
            tracked_metrics: counters.len(),
            health_status: if avg_operation_time < 1000 {
                HealthStatus::Excellent
            } else if avg_operation_time < 5000 {
                HealthStatus::Good
            } else if avg_operation_time < 10000 {
                HealthStatus::Fair
            } else {
                HealthStatus::Poor
            },
        }
    }
}

/// Complete performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub metric_count: usize,
    pub counter_count: usize,
    pub total_data_points: usize,
    pub metric_stats: Vec<MetricStats>,
    pub counter_stats: Vec<PerformanceCounter>,
}

/// System health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthSummary {
    pub total_operations: u64,
    pub average_operation_us: u64,
    pub max_operation_us: u64,
    pub tracked_metrics: usize,
    pub health_status: HealthStatus,
}

impl Default for SystemHealthSummary {
    fn default() -> Self {
        Self {
            total_operations: 0,
            average_operation_us: 0,
            max_operation_us: 0,
            tracked_metrics: 0,
            health_status: HealthStatus::Unknown,
        }
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HealthStatus {
    #[serde(rename = "EXCELLENT")]
    Excellent,
    #[serde(rename = "GOOD")]
    Good,
    #[serde(rename = "FAIR")]
    Fair,
    #[serde(rename = "POOR")]
    Poor,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

// Stub for rand - in production use rand crate
mod rand {
    pub fn random<T: Default>() -> T {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_aggregator_creation() {
        let agg = MetricsAggregator::new(1.0, 1000, 3600);
        assert_eq!(agg.sampling_rate, 1.0);
    }

    #[test]
    fn test_record_metric() {
        let agg = MetricsAggregator::new(1.0, 1000, 3600);
        let mut tags = HashMap::new();
        tags.insert("region".to_string(), "us-east".to_string());

        agg.record_metric("latency_ms", 45.3, tags);

        let stats = agg.get_metric_stats("latency_ms");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().count, 1);
    }

    #[test]
    fn test_record_operation() {
        let agg = MetricsAggregator::new(1.0, 1000, 3600);

        agg.record_operation("process_task", 1500);
        agg.record_operation("process_task", 2000);
        agg.record_operation("process_task", 1000);

        let counter = agg.get_counter("process_task").unwrap();
        assert_eq!(counter.call_count, 3);
        assert_eq!(counter.min_time_us, 1000);
        assert_eq!(counter.max_time_us, 2000);
    }

    #[test]
    fn test_health_summary() {
        let agg = MetricsAggregator::new(1.0, 1000, 3600);

        agg.record_operation("op1", 500);
        agg.record_operation("op2", 800);

        let health = agg.get_health_summary();
        assert!(health.total_operations > 0);
        assert_eq!(health.health_status, HealthStatus::Excellent);
    }

    #[test]
    fn test_cleanup_old_data() {
        let agg = MetricsAggregator::new(1.0, 1000, 0); // 0 second retention
        let mut tags = HashMap::new();

        agg.record_metric("metric1", 10.0, tags);
        agg.cleanup_old_data();

        // Old data should be cleaned
        let metrics = agg.metrics.lock().unwrap();
        assert!(metrics.is_empty() || metrics.get("metric1").unwrap().is_empty());
    }

    #[test]
    fn test_performance_report() {
        let agg = MetricsAggregator::new(1.0, 1000, 3600);

        agg.record_operation("task1", 1000);
        agg.record_operation("task2", 2000);

        let report = agg.generate_report();
        assert!(report.counter_count > 0);
        assert!(report.uptime_seconds <= 3600);
    }
}
