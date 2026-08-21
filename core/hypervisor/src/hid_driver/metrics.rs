/// HID Driver Performance Metrics
///
/// Tracks latency, throughput, and error rates for HID operations
use std::collections::VecDeque;

/// HID driver performance metrics
#[derive(Debug, Clone)]
pub struct HidMetrics {
    /// Total commands executed
    pub total_commands: u32,

    /// Sum of all latencies (for average calculation)
    pub sum_latency_us: u64,

    /// Minimum latency observed
    pub min_latency_us: u32,

    /// Maximum latency observed
    pub max_latency_us: u32,

    /// Recent latencies (rolling window)
    pub latencies: VecDeque<u32>,

    /// Error count
    pub error_count: u32,
}

/// Latency percentiles (p50, p95, p99)
#[derive(Debug, Clone)]
pub struct LatencyPercentiles {
    pub p50: u32, // Median
    pub p95: u32, // 95th percentile
    pub p99: u32, // 99th percentile
}

impl HidMetrics {
    /// Calculate average latency
    pub fn average_latency_us(&self) -> u32 {
        if self.total_commands == 0 {
            return 0;
        }
        (self.sum_latency_us / self.total_commands as u64) as u32
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "HID Metrics: {} commands, avg={}μs, min={}μs, max={}μs",
            self.total_commands,
            self.average_latency_us(),
            self.min_latency_us,
            self.max_latency_us
        )
    }

    /// Check if latency meets target (<1ms p99)
    pub fn meets_latency_target(&self, percentiles: &LatencyPercentiles) -> bool {
        percentiles.p99 < 1000 // <1000 microseconds = <1ms
    }
}

impl Default for HidMetrics {
    fn default() -> Self {
        Self {
            total_commands: 0,
            sum_latency_us: 0,
            min_latency_us: u32::MAX,
            max_latency_us: 0,
            latencies: VecDeque::new(),
            error_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_default() {
        let metrics = HidMetrics::default();
        assert_eq!(metrics.total_commands, 0);
        assert_eq!(metrics.error_count, 0);
    }

    #[test]
    fn test_average_latency_calculation() {
        let mut metrics = HidMetrics::default();
        metrics.total_commands = 10;
        metrics.sum_latency_us = 500; // Total 500us for 10 commands

        assert_eq!(metrics.average_latency_us(), 50); // 50us average
    }

    #[test]
    fn test_latency_percentiles() {
        let mut metrics = HidMetrics::default();

        // Add latencies: 10, 20, 30, ..., 100
        for i in 1..=10 {
            metrics.latencies.push_back(i * 10);
        }

        let mut sorted: Vec<u32> = metrics.latencies.iter().copied().collect();
        sorted.sort();

        let p50 = sorted[5]; // 60
        let p95 = sorted[9]; // 100 (or close)

        assert!(p50 > 50 && p50 < 70);
        assert!(p95 >= p50);
    }

    #[test]
    fn test_latency_target_met() {
        let metrics = HidMetrics::default();
        let percentiles = LatencyPercentiles {
            p50: 50,
            p95: 200,
            p99: 500,
        };

        assert!(metrics.meets_latency_target(&percentiles));
    }

    #[test]
    fn test_latency_target_not_met() {
        let metrics = HidMetrics::default();
        let percentiles = LatencyPercentiles {
            p50: 500,
            p95: 2000,
            p99: 5000,
        };

        assert!(!metrics.meets_latency_target(&percentiles));
    }
}
