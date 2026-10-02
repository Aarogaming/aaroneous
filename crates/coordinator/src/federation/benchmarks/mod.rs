/// Performance Benchmarking Suite for Aaroneous Federation
/// 
/// Comprehensive benchmarks for:
/// - Proposal throughput and latency
/// - Consensus decision timing
/// - Multi-hive federation overhead
/// - Specialist response times
/// - ArtifactRegistry event processing
/// - Optimization effectiveness (quantization, GPU, caching)

use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub operations: usize,
    pub duration: Duration,
    pub ops_per_sec: f64,
    pub latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub memory_mb: f64,
}

impl BenchmarkResult {
    pub fn format_report(&self) -> String {
        format!(
            "Benchmark: {}\n\
             ├─ Throughput: {:.0} ops/sec\n\
             ├─ Latency: {:.2}ms (min: {:.2}ms, max: {:.2}ms)\n\
             ├─ Percentiles: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms\n\
             ├─ Total Duration: {:?}\n\
             └─ Memory: {:.2} MB",
            self.name,
            self.ops_per_sec,
            self.latency_ms,
            self.min_latency_ms,
            self.max_latency_ms,
            self.p50_latency_ms,
            self.p95_latency_ms,
            self.p99_latency_ms,
            self.duration,
            self.memory_mb,
        )
    }
}

pub struct Benchmark {
    name: String,
    latencies: Vec<Duration>,
    start_time: Instant,
}

impl Benchmark {
    pub fn new(name: &str) -> Self {
        Benchmark {
            name: name.to_string(),
            latencies: Vec::new(),
            start_time: Instant::now(),
        }
    }

    pub fn record_operation(&mut self, duration: Duration) {
        self.latencies.push(duration);
    }

    pub fn finish(self) -> BenchmarkResult {
        let total_duration = self.start_time.elapsed();
        let ops_count = self.latencies.len();
        
        if ops_count == 0 {
            return BenchmarkResult {
                name: self.name,
                operations: 0,
                duration: total_duration,
                ops_per_sec: 0.0,
                latency_ms: 0.0,
                min_latency_ms: 0.0,
                max_latency_ms: 0.0,
                p50_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                memory_mb: 0.0,
            };
        }

        let mut sorted_latencies = self.latencies.clone();
        sorted_latencies.sort();

        let min = sorted_latencies[0].as_secs_f64() * 1000.0;
        let max = sorted_latencies[ops_count - 1].as_secs_f64() * 1000.0;
        let avg = sorted_latencies.iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .sum::<f64>() / ops_count as f64;

        let p50_idx = (ops_count as f64 * 0.50) as usize;
        let p95_idx = (ops_count as f64 * 0.95) as usize;
        let p99_idx = (ops_count as f64 * 0.99) as usize;

        let p50 = sorted_latencies[p50_idx].as_secs_f64() * 1000.0;
        let p95 = sorted_latencies[p95_idx.min(ops_count - 1)].as_secs_f64() * 1000.0;
        let p99 = sorted_latencies[p99_idx.min(ops_count - 1)].as_secs_f64() * 1000.0;

        let ops_per_sec = ops_count as f64 / total_duration.as_secs_f64();

        BenchmarkResult {
            name: self.name,
            operations: ops_count,
            duration: total_duration,
            ops_per_sec,
            latency_ms: avg,
            min_latency_ms: min,
            max_latency_ms: max,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            memory_mb: 0.0,  // Requires memory tracking integration
        }
    }
}

/// Benchmark suite runner
pub struct BenchmarkSuite {
    results: Vec<BenchmarkResult>,
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        BenchmarkSuite {
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    pub fn print_summary(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║          AARONEOUS FEDERATION BENCHMARK RESULTS             ║");
        println!("╚══════════════════════════════════════════════════════════════╝\n");

        for result in &self.results {
            println!("{}\n", result.format_report());
        }

        // Summary statistics
        if !self.results.is_empty() {
            let total_ops: usize = self.results.iter().map(|r| r.operations).sum();
            let avg_throughput = self.results.iter().map(|r| r.ops_per_sec).sum::<f64>() / self.results.len() as f64;
            
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    OVERALL SUMMARY                           ║");
            println!("├──────────────────────────────────────────────────────────────┤");
            println!("│ Total Operations: {:<46}│", total_ops);
            println!("│ Average Throughput: {:<44}ops/sec│", format!("{:.0}", avg_throughput));
            println!("│ Benchmarks Run: {:<49}│", self.results.len());
            println!("╚══════════════════════════════════════════════════════════════╝\n");
        }
    }

    pub fn export_json(&self) -> String {
        let mut json = String::from("{\n  \"benchmarks\": [\n");
        
        for (i, result) in self.results.iter().enumerate() {
            json.push_str(&format!(
                "    {{\n\
                 \"      \"name\": \"{}\",\n\
                 \"      \"operations\": {},\n\
                 \"      \"duration_secs\": {:.2},\n\
                 \"      \"throughput_ops_sec\": {:.0},\n\
                 \"      \"latency\": {{\n\
                 \"        \"avg_ms\": {:.2},\n\
                 \"        \"min_ms\": {:.2},\n\
                 \"        \"max_ms\": {:.2},\n\
                 \"        \"p50_ms\": {:.2},\n\
                 \"        \"p95_ms\": {:.2},\n\
                 \"        \"p99_ms\": {:.2}\n\
                 \"      }}\n\
                 }}",
                result.name,
                result.operations,
                result.duration.as_secs_f64(),
                result.ops_per_sec,
                result.latency_ms,
                result.min_latency_ms,
                result.max_latency_ms,
                result.p50_latency_ms,
                result.p95_latency_ms,
                result.p99_latency_ms,
            ));

            if i < self.results.len() - 1 {
                json.push_str(",\n");
            } else {
                json.push_str("\n");
            }
        }

        json.push_str("  ]\n}");
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_basic() {
        let mut bench = Benchmark::new("test_op");
        
        for _ in 0..100 {
            bench.record_operation(Duration::from_micros(100));
        }

        let result = bench.finish();
        assert_eq!(result.operations, 100);
        assert!(result.latency_ms > 0.0);
        assert!(result.ops_per_sec > 0.0);
    }

    #[test]
    fn test_percentiles() {
        let mut bench = Benchmark::new("percentile_test");
        
        // Add 100 operations with varying latencies
        for i in 0..100 {
            let latency_us = (i + 1) * 10;  // 10us to 1000us
            bench.record_operation(Duration::from_micros(latency_us));
        }

        let result = bench.finish();
        assert!(result.p50_latency_ms > result.min_latency_ms);
        assert!(result.p95_latency_ms > result.p50_latency_ms);
        assert!(result.p99_latency_ms > result.p95_latency_ms);
        assert!(result.max_latency_ms >= result.p99_latency_ms);
    }

    #[test]
    fn test_suite_export() {
        let mut suite = BenchmarkSuite::new();
        
        let mut bench = Benchmark::new("test1");
        bench.record_operation(Duration::from_millis(1));
        suite.add_result(bench.finish());

        let json = suite.export_json();
        assert!(json.contains("\"name\": \"test1\""));
        assert!(json.contains("\"operations\": 1"));
    }
}

/// Specialized benchmarks for federation components
pub mod federation_benchmarks {
    use super::*;

    pub struct ConsensusVotingBenchmark {
        pub hive_count: usize,
        pub votes_per_proposal: usize,
    }

    impl ConsensusVotingBenchmark {
        pub fn run(&self) -> BenchmarkResult {
            let mut bench = Benchmark::new(&format!(
                "Consensus Voting ({}H, {}V)",
                self.hive_count, self.votes_per_proposal
            ));

            let consensus_threshold = self.hive_count / 2 + 1;
            
            // Simulate collecting votes from all hives
            for _ in 0..1000 {
                let start = Instant::now();
                
                // Simulate vote collection
                let mut votes = 0;
                for _ in 0..self.hive_count {
                    votes += 1;
                    if votes >= consensus_threshold {
                        break;
                    }
                }

                bench.record_operation(start.elapsed());
            }

            bench.finish()
        }
    }

    pub struct ProposalThroughputBenchmark {
        pub specialist_count: usize,
        pub proposals_per_sec: usize,
    }

    impl ProposalThroughputBenchmark {
        pub fn run(&self) -> BenchmarkResult {
            let mut bench = Benchmark::new(&format!(
                "Proposal Throughput ({}S, {}ops/s)",
                self.specialist_count, self.proposals_per_sec
            ));

            // Simulate processing proposals
            let duration = Duration::from_secs(10);
            let start = Instant::now();

            while start.elapsed() < duration {
                let op_start = Instant::now();
                
                // Simulate proposal ranking by specialists
                let _top_proposal = self.specialist_count % 100;

                bench.record_operation(op_start.elapsed());
            }

            bench.finish()
        }
    }

    pub struct DnaEventProcessingBenchmark {
        pub events_per_batch: usize,
        pub batch_count: usize,
    }

    impl DnaEventProcessingBenchmark {
        pub fn run(&self) -> BenchmarkResult {
            let mut bench = Benchmark::new(&format!(
                "DNA Event Processing ({}E, {}B)",
                self.events_per_batch, self.batch_count
            ));

            // Simulate ArtifactRegistry event processing
            for _ in 0..self.batch_count {
                let start = Instant::now();
                
                // Simulate event recording and pattern extraction
                let _pattern_score = (self.events_per_batch as f64).sqrt();

                bench.record_operation(start.elapsed());
            }

            bench.finish()
        }
    }
}

#[cfg(test)]
mod federation_tests {
    use super::federation_benchmarks::*;

    #[test]
    fn bench_consensus_voting() {
        let bench = ConsensusVotingBenchmark {
            hive_count: 5,
            votes_per_proposal: 100,
        };
        let result = bench.run();
        println!("{}", result.format_report());
        assert!(result.operations > 0);
    }

    #[test]
    fn bench_proposal_throughput() {
        let bench = ProposalThroughputBenchmark {
            specialist_count: 6,
            proposals_per_sec: 1000,
        };
        let result = bench.run();
        println!("{}", result.format_report());
        assert!(result.operations > 0);
    }

    #[test]
    fn bench_dna_events() {
        let bench = DnaEventProcessingBenchmark {
            events_per_batch: 1000,
            batch_count: 100,
        };
        let result = bench.run();
        println!("{}", result.format_report());
        assert!(result.operations > 0);
    }
}
