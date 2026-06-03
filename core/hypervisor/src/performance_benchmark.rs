// Performance Benchmarking and Optimization Tracking
// Comprehensive performance metrics, baselines, and optimization analysis

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Benchmark operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkOperation {
    pub name: String,
    pub description: String,
    pub operation_type: OperationType,
    pub iterations: u32,
}

/// Operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    #[serde(rename = "TASK_EXECUTION")]
    TaskExecution,
    #[serde(rename = "LEARNING_UPDATE")]
    LearningUpdate,
    #[serde(rename = "CHECKPOINT")]
    Checkpoint,
    #[serde(rename = "CONSENSUS")]
    Consensus,
    #[serde(rename = "STATE_REPLICATION")]
    StateReplication,
    #[serde(rename = "ROUTE_DECISION")]
    RouteDecision,
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub operation_name: String,
    pub timestamp: u64,
    pub iterations: u32,
    pub total_time_us: u64,
    pub average_time_us: u64,
    pub min_time_us: u64,
    pub max_time_us: u64,
    pub stddev_time_us: u64,
    pub p50_time_us: u64,
    pub p95_time_us: u64,
    pub p99_time_us: u64,
    pub throughput_ops_per_sec: f32,
    pub memory_used_mb: u64,
}

/// Performance baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub operation_name: String,
    pub version: String,
    pub timestamp: u64,
    pub average_time_us: u64,
    pub p99_time_us: u64,
    pub throughput_ops_per_sec: f32,
}

/// Performance regression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRegression {
    pub operation_name: String,
    pub baseline_avg_us: u64,
    pub current_avg_us: u64,
    pub regression_percent: f32,
    pub severity: RegressionSeverity,
}

/// Regression severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionSeverity {
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "MINOR")]
    Minor,      // < 5%
    #[serde(rename = "MODERATE")]
    Moderate,   // 5-20%
    #[serde(rename = "SEVERE")]
    Severe,     // > 20%
}

/// Performance benchmark suite
pub struct PerformanceBenchmark {
    pub name: String,
    pub version: String,
    pub results: Vec<BenchmarkResult>,
    pub baselines: HashMap<String, PerformanceBaseline>,
    pub regressions: Vec<PerformanceRegression>,
    pub regression_threshold_percent: f32,
}

impl PerformanceBenchmark {
    /// Create new performance benchmark
    pub fn new(name: &str, version: &str, regression_threshold: f32) -> Self {
        println!("[PerformanceBenchmark] Initialized: {} v{}", name, version);
        
        Self {
            name: name.to_string(),
            version: version.to_string(),
            results: Vec::new(),
            baselines: HashMap::new(),
            regressions: Vec::new(),
            regression_threshold_percent: regression_threshold,
        }
    }

    /// Run benchmark
    pub fn run_benchmark(&mut self, operation: &BenchmarkOperation) -> BenchmarkResult {
        println!("[PerformanceBenchmark] Running: {} ({} iterations)",
            operation.name, operation.iterations);

        let start = Instant::now();
        let mut times = Vec::new();

        // Run iterations
        for _ in 0..operation.iterations {
            let iter_start = Instant::now();
            
            // Simulate operation
            self.execute_operation(&operation.operation_type);
            
            let iter_time = iter_start.elapsed().as_micros() as u64;
            times.push(iter_time);
        }

        let total_time = start.elapsed().as_micros() as u64;
        let avg_time = total_time / operation.iterations as u64;

        times.sort();
        let min_time = times[0];
        let max_time = times[times.len() - 1];
        let p50_idx = times.len() / 2;
        let p95_idx = (times.len() as f32 * 0.95) as usize;
        let p99_idx = (times.len() as f32 * 0.99) as usize;

        let p50 = times[p50_idx];
        let p95 = times[p95_idx.min(times.len() - 1)];
        let p99 = times[p99_idx.min(times.len() - 1)];

        // Calculate standard deviation
        let mean = avg_time as f64;
        let variance: f64 = times.iter()
            .map(|t| {
                let diff = *t as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / times.len() as f64;
        let stddev = variance.sqrt() as u64;

        let throughput = (operation.iterations as f32 * 1_000_000.0) / total_time as f32;

        let result = BenchmarkResult {
            operation_name: operation.name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            iterations: operation.iterations,
            total_time_us: total_time,
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            stddev_time_us: stddev,
            p50_time_us: p50,
            p95_time_us: p95,
            p99_time_us: p99,
            throughput_ops_per_sec: throughput,
            memory_used_mb: 0,  // Would collect from system
        };

        println!("[PerformanceBenchmark] Complete: {} - avg={}μs, p99={}μs, {:.0}ops/sec",
            operation.name, avg_time, p99, throughput);

        // Check for regressions
        self.check_regressions(&result);

        self.results.push(result.clone());
        result
    }

    /// Set baseline for operation
    pub fn set_baseline(&mut self, operation_name: &str, result: &BenchmarkResult) {
        let baseline = PerformanceBaseline {
            operation_name: operation_name.to_string(),
            version: self.version.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            average_time_us: result.average_time_us,
            p99_time_us: result.p99_time_us,
            throughput_ops_per_sec: result.throughput_ops_per_sec,
        };

        println!("[PerformanceBenchmark] Set baseline for: {}", operation_name);
        self.baselines.insert(operation_name.to_string(), baseline);
    }

    /// Check for regressions
    fn check_regressions(&mut self, result: &BenchmarkResult) {
        if let Some(baseline) = self.baselines.get(&result.operation_name) {
            let regression_percent = 
                ((result.average_time_us as f32 - baseline.average_time_us as f32) 
                / baseline.average_time_us as f32) * 100.0;

            if regression_percent > self.regression_threshold_percent {
                let severity = if regression_percent < 5.0 {
                    RegressionSeverity::Minor
                } else if regression_percent < 20.0 {
                    RegressionSeverity::Moderate
                } else {
                    RegressionSeverity::Severe
                };

                let regression = PerformanceRegression {
                    operation_name: result.operation_name.clone(),
                    baseline_avg_us: baseline.average_time_us,
                    current_avg_us: result.average_time_us,
                    regression_percent,
                    severity: severity.clone(),
                };

                println!("[PerformanceBenchmark] REGRESSION: {} ({:.1}%)",
                    result.operation_name, regression_percent);

                self.regressions.push(regression);
            }
        }
    }

    /// Execute operation simulation
    fn execute_operation(&self, _op_type: &OperationType) {
        // Simulate operation execution
        // In real implementation, would execute actual code
        std::thread::sleep(std::time::Duration::from_micros(100));
    }

    /// Get results summary
    pub fn get_summary(&self) -> BenchmarkSummary {
        let total_operations: u64 = self.results.iter()
            .map(|r| r.iterations as u64)
            .sum();

        let avg_latency = if !self.results.is_empty() {
            self.results.iter()
                .map(|r| r.average_time_us as f64)
                .sum::<f64>() / self.results.len() as f64
        } else {
            0.0
        };

        let avg_throughput = if !self.results.is_empty() {
            self.results.iter()
                .map(|r| r.throughput_ops_per_sec)
                .sum::<f32>() / self.results.len() as f32
        } else {
            0.0
        };

        let regression_count = self.regressions.len();
        let severe_regressions = self.regressions.iter()
            .filter(|r| matches!(r.severity, RegressionSeverity::Severe))
            .count();

        BenchmarkSummary {
            benchmark_name: self.name.clone(),
            version: self.version.clone(),
            total_operations,
            total_benchmarks: self.results.len() as u32,
            average_latency_us: avg_latency,
            average_throughput_ops_per_sec: avg_throughput,
            regressions_detected: regression_count as u32,
            severe_regressions: severe_regressions as u32,
            status: if severe_regressions > 0 {
                BenchmarkStatus::RegessionDetected
            } else if regression_count > 0 {
                BenchmarkStatus::MinorIssues
            } else {
                BenchmarkStatus::Healthy
            },
        }
    }

    /// Generate comparison report
    pub fn generate_comparison(&self) -> ComparisonReport {
        let mut operation_comparisons = Vec::new();

        for result in &self.results {
            if let Some(baseline) = self.baselines.get(&result.operation_name) {
                let improvement_percent = 
                    ((baseline.average_time_us as f32 - result.average_time_us as f32)
                    / baseline.average_time_us as f32) * 100.0;

                operation_comparisons.push(OperationComparison {
                    operation_name: result.operation_name.clone(),
                    baseline_avg_us: baseline.average_time_us,
                    current_avg_us: result.average_time_us,
                    improvement_percent,
                    baseline_throughput: baseline.throughput_ops_per_sec,
                    current_throughput: result.throughput_ops_per_sec,
                });
            }
        }

        ComparisonReport {
            operation_comparisons,
        }
    }

    /// Export results as JSON
    pub fn export_results_json(&self) -> String {
        serde_json::to_string_pretty(&self.results)
            .unwrap_or_else(|_| "[]".to_string())
    }
}

/// Benchmark summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub benchmark_name: String,
    pub version: String,
    pub total_operations: u64,
    pub total_benchmarks: u32,
    pub average_latency_us: f64,
    pub average_throughput_ops_per_sec: f32,
    pub regressions_detected: u32,
    pub severe_regressions: u32,
    pub status: BenchmarkStatus,
}

/// Benchmark status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkStatus {
    #[serde(rename = "HEALTHY")]
    Healthy,
    #[serde(rename = "MINOR_ISSUES")]
    MinorIssues,
    #[serde(rename = "REGRESSION_DETECTED")]
    RegessionDetected,
}

/// Operation comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationComparison {
    pub operation_name: String,
    pub baseline_avg_us: u64,
    pub current_avg_us: u64,
    pub improvement_percent: f32,
    pub baseline_throughput: f32,
    pub current_throughput: f32,
}

/// Comparison report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub operation_comparisons: Vec<OperationComparison>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_creation() {
        let benchmark = PerformanceBenchmark::new("test", "1.0.0", 5.0);
        assert_eq!(benchmark.name, "test");
    }

    #[test]
    fn test_run_benchmark() {
        let mut benchmark = PerformanceBenchmark::new("test", "1.0.0", 5.0);
        
        let operation = BenchmarkOperation {
            name: "test_op".to_string(),
            description: "Test operation".to_string(),
            operation_type: OperationType::TaskExecution,
            iterations: 10,
        };

        let result = benchmark.run_benchmark(&operation);
        assert_eq!(result.iterations, 10);
        assert!(result.average_time_us > 0);
    }

    #[test]
    fn test_baseline_and_regression() {
        let mut benchmark = PerformanceBenchmark::new("test", "1.0.0", 5.0);
        
        let operation = BenchmarkOperation {
            name: "test_op".to_string(),
            description: "Test operation".to_string(),
            operation_type: OperationType::TaskExecution,
            iterations: 10,
        };

        let result = benchmark.run_benchmark(&operation);
        benchmark.set_baseline("test_op", &result);

        assert!(benchmark.baselines.contains_key("test_op"));
    }

    #[test]
    fn test_summary() {
        let mut benchmark = PerformanceBenchmark::new("test", "1.0.0", 5.0);
        
        let operation = BenchmarkOperation {
            name: "test_op".to_string(),
            description: "Test".to_string(),
            operation_type: OperationType::TaskExecution,
            iterations: 10,
        };

        benchmark.run_benchmark(&operation);
        let summary = benchmark.get_summary();

        assert_eq!(summary.total_benchmarks, 1);
    }
}

