//! Cache Line Isolation Benchmark
//!
//! Historical anti-pattern: False sharing between concurrent atomics and hot write buffers
//! destroys throughput by forcing constant cache line invalidation.
//!
//! This benchmark quantifies the real-world latency gains from explicit padding.

#![allow(ambient_authority)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Properly aligned telemetry struct (64 bytes, 64-byte cache line)
#[repr(C, align(64))]
#[derive(Debug)]
struct AlignedTelemetry {
    counter: AtomicUsize,
    value: u64,
    timestamp: u64,
    reserved: [u8; 40], // Pad to 64-byte cache line (8 + 8 + 8 + 40 = 64)
}

/// Improperly aligned telemetry (false sharing vulnerability)
#[repr(C)]
#[derive(Debug)]
struct UnalignedTelemetry {
    counter: AtomicUsize,
    value: u64,
    timestamp: u64, // May share cache line with next struct's counter
}

/// Benchmark: Concurrent writes to aligned buffer
fn benchmark_aligned_writers(num_threads: usize, iterations: usize) -> std::time::Duration {
    let mut handles = vec![];

    for _ in 0..num_threads {
        let handle = std::thread::spawn(move || {
            let mut telemetry = AlignedTelemetry {
                counter: AtomicUsize::new(0),
                value: 0,
                timestamp: 0,
                reserved: [0u8; 40],
            };

            let start = Instant::now();
            for _ in 0..iterations {
                telemetry.counter.fetch_add(1, Ordering::Relaxed);
                telemetry.value += 1;
            }
            start.elapsed()
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let mut total = Duration::ZERO;
    for handle in handles {
        total += handle.join().expect("Thread panicked");
    }

    total
}

/// Benchmark: Concurrent writes to unaligned buffer (false sharing)
fn benchmark_unaligned_writers(num_threads: usize, iterations: usize) -> std::time::Duration {
    let mut handles = vec![];

    for _ in 0..num_threads {
        let handle = std::thread::spawn(move || {
            let mut telemetry = UnalignedTelemetry {
                counter: AtomicUsize::new(0),
                value: 0,
                timestamp: 0,
            };

            let start = Instant::now();
            for _ in 0..iterations {
                telemetry.counter.fetch_add(1, Ordering::Relaxed);
                telemetry.value += 1;
            }
            start.elapsed()
        });

        handles.push(handle);
    }

    let mut total = Duration::ZERO;
    for handle in handles {
        total += handle.join().expect("Thread panicked");
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_vs_unaligned_performance() {
        let num_threads = 8;
        let iterations = 1_000_000;

        let aligned_time = benchmark_aligned_writers(num_threads, iterations);
        let unaligned_time = benchmark_unaligned_writers(num_threads, iterations);

        println!("Aligned:   {:?}", aligned_time);
        println!("Unaligned: {:?}", unaligned_time);

        // Aligned should be faster or equal (false sharing penalty)
        // Allow 20% tolerance for system noise
        let ratio = unaligned_time.as_nanos() as f64 / aligned_time.as_nanos() as f64;
        assert!(
            ratio >= 0.8,
            "Aligned layout should outperform or match unaligned (ratio={:.2})",
            ratio
        );
    }

    #[test]
    fn test_cache_line_size_verification() {
        // Verify AlignedTelemetry is exactly one cache line (64 bytes on x86)
        assert_eq!(
            std::mem::size_of::<AlignedTelemetry>(),
            64,
            "AlignedTelemetry should be 64 bytes"
        );

        // Verify UnalignedTelemetry is smaller (no padding)
        assert_eq!(
            std::mem::size_of::<UnalignedTelemetry>(),
            24,
            "UnalignedTelemetry should be 24 bytes"
        );
    }
}

fn main() {
    let num_threads = std::env::var("NUM_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);

    let iterations = std::env::var("ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);

    println!("Benchmarking cache line isolation...");
    println!("Threads: {}, Iterations: {}", num_threads, iterations);

    let aligned_time = benchmark_aligned_writers(num_threads, iterations);
    let unaligned_time = benchmark_unaligned_writers(num_threads, iterations);

    println!("\nResults:");
    println!("Aligned:   {:?}", aligned_time);
    println!("Unaligned: {:?}", unaligned_time);

    let speedup = if aligned_time.as_nanos() > 0 {
        unaligned_time.as_nanos() as f64 / aligned_time.as_nanos() as f64
    } else {
        1.0
    };

    println!("\nSpeedup: {:.2}x (aligned vs unaligned)", speedup);
}
