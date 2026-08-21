// Phase 15 graceful-degradation tests.
//
// These tests exercise the new modules in combination to
// verify that the system degrades gracefully under the
// failure modes the rate limiter, circuit breaker, and
// retry policy are designed to handle.
//
// The tests live in their own file (rather than appended
// to `resilience.rs` or `rate_limit.rs`) so that the
// per-module test counts stay clean. They are
// lib-internal tests; they do not exercise the HTTP
// stack.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::rate_limit::{TokenBucketConfig, TokenBucketDecision, TokenBucketLimiter};
use crate::resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState, RetryPolicy,
    with_circuit_breaker, with_retry,
};

/// Scenario 1: a rate-limited client is denied; the
/// underlying specialist is *not* invoked.
#[test]
fn rate_limit_denial_does_not_invoke_specialist() {
    let rl = TokenBucketLimiter::new(TokenBucketConfig {
        burst: 2.0,
        refill_per_second: 0.0,
        idle_eviction: None,
    });
    let calls = AtomicU32::new(0);

    // Simulate the request-handler pattern: check the
    // bucket first, and only invoke the inner call on
    // Allow.
    for _ in 0..5 {
        match rl.check("client-a") {
            TokenBucketDecision::Allow { .. } => {
                calls.fetch_add(1, Ordering::SeqCst);
            }
            TokenBucketDecision::Deny { .. } => {
                // Simulated: the specialist is not called.
            }
        }
    }
    // 2 allowed (burst), 3 denied. The specialist saw 2 calls.
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

/// Scenario 2: per-key isolation. One hostile client
/// cannot starve another.
#[test]
fn rate_limit_isolates_keys() {
    let rl = TokenBucketLimiter::new(TokenBucketConfig {
        burst: 1.0,
        refill_per_second: 0.0,
        idle_eviction: None,
    });
    // Hostile client A exhausts its bucket.
    assert!(matches!(rl.check("A"), TokenBucketDecision::Allow { .. }));
    assert!(matches!(rl.check("A"), TokenBucketDecision::Deny { .. }));
    // Innocent client B is unaffected.
    assert!(matches!(rl.check("B"), TokenBucketDecision::Allow { .. }));
}

/// Scenario 3: circuit breaker + retry policy: the retry
/// policy does not retry while the breaker is open.
#[test]
fn breaker_open_skips_retries() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            open_duration: Duration::from_secs(60),
            ..CircuitBreakerConfig::default()
        }
        .with_failure_threshold(1),
    );
    // Trip the breaker.
    let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("a") });
    assert_eq!(cb.state(), CircuitState::Open);

    let inner_calls = AtomicU32::new(0);
    let result: Result<i32, CircuitBreakerError> = with_circuit_breaker(
        &cb,
        &RetryPolicy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(1),
            jitter: false,
            ..RetryPolicy::default()
        },
        || -> Result<i32, &str> {
            inner_calls.fetch_add(1, Ordering::SeqCst);
            Ok(1)
        },
    );
    assert_eq!(result.unwrap_err(), CircuitBreakerError::Open);
    // Inner function was never called: the breaker
    // short-circuited before the retry loop even started.
    assert_eq!(inner_calls.load(Ordering::SeqCst), 0);
}

/// Scenario 4: the breaker recovers via the half-open
/// probe and the system resumes serving. This is the
/// "graceful recovery" path.
#[test]
fn breaker_recovers_via_half_open_probe() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            open_duration: Duration::from_millis(40),
            ..CircuitBreakerConfig::default()
        }
        .with_failure_threshold(2),
    );
    // Trip the breaker.
    for _ in 0..2 {
        let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("x") });
    }
    assert_eq!(cb.state(), CircuitState::Open);

    // While Open, every call short-circuits.
    let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(1) });
    // Wait for cool-down.
    std::thread::sleep(Duration::from_millis(60));
    // First call after cool-down is a probe; on success
    // the breaker closes.
    let result: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(42) });
    assert_eq!(result.unwrap(), 42);
    assert_eq!(cb.state(), CircuitState::Closed);

    // Subsequent calls succeed without fail.
    for _ in 0..5 {
        let r: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(1) });
        assert!(r.is_ok());
    }
}

/// Scenario 5: under sustained failure the breaker
/// half-opens, the probe fails, and the breaker
/// re-opens. This prevents the system from hammering
/// a degraded dependency.
#[test]
fn breaker_reopens_after_failed_probe() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            open_duration: Duration::from_millis(20),
            ..CircuitBreakerConfig::default()
        }
        .with_failure_threshold(1),
    );
    let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("a") });
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("b") });
    assert_eq!(cb.state(), CircuitState::Open);
    // Subsequent calls short-circuit.
    let r: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(1) });
    assert_eq!(r.unwrap_err(), CircuitBreakerError::Open);
}

/// Scenario 6: retry policy gives up after the configured
/// number of attempts and returns the last error. This is
/// the "graceful failure" path for transient-but-persistent
/// errors.
#[test]
fn retry_policy_gives_up_with_last_error() {
    let policy = RetryPolicy {
        max_attempts: 3,
        initial_delay: Duration::from_millis(1),
        jitter: false,
        ..RetryPolicy::default()
    };
    let attempts = AtomicU32::new(0);
    let result: Result<i32, _> = with_retry(&policy, || -> Result<i32, String> {
        attempts.fetch_add(1, Ordering::SeqCst);
        Err("transient".to_string())
    });
    let err = result.unwrap_err();
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(err.attempts, 3);
    assert_eq!(err.last_error, "transient");
}

/// Scenario 7: rate limiter is thread-safe under concurrent
/// load. Two threads hammering the same key see exactly
/// `burst` allowed, the rest denied.
#[test]
fn rate_limit_thread_safety() {
    let rl = Arc::new(TokenBucketLimiter::new(TokenBucketConfig {
        burst: 100.0,
        refill_per_second: 0.0,
        idle_eviction: None,
    }));
    let allowed = Arc::new(AtomicUsize::new(0));
    let denied = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..8 {
        let rl = Arc::clone(&rl);
        let allowed = Arc::clone(&allowed);
        let denied = Arc::clone(&denied);
        handles.push(std::thread::spawn(move || {
            for _ in 0..100 {
                match rl.check("shared-key") {
                    TokenBucketDecision::Allow { .. } => {
                        allowed.fetch_add(1, Ordering::SeqCst);
                    }
                    TokenBucketDecision::Deny { .. } => {
                        denied.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let a = allowed.load(Ordering::SeqCst);
    let d = denied.load(Ordering::SeqCst);
    // 8 threads × 100 calls = 800 calls total. With burst=100,
    // the first 100 are allowed, the remaining 700 are denied.
    // We allow a tiny fudge because of refill elapsed during
    // the test loop, but it should be small (< 5%).
    assert_eq!(a + d, 800);
    assert!(a <= 110, "allowed={} (expected ~100)", a);
    assert!(d >= 690, "denied={} (expected ~700)", d);
}

/// Property-ish check: over a deterministic stream of mixed keys,
/// per-key allowance never exceeds the configured burst.
#[test]
fn rate_limit_respects_per_key_burst_under_randomized_inputs() {
    let rl = TokenBucketLimiter::new(TokenBucketConfig {
        burst: 3.0,
        refill_per_second: 0.0,
        idle_eviction: None,
    });
    let mut allowed_per_key = std::collections::HashMap::<String, usize>::new();
    let mut seed = 0x1234_5678_9abc_def0u64;
    for _ in 0..100 {
        // Deterministic LCG so the test is stable but exercises
        // a broad mix of keys.
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let key = format!("key-{}", seed % 8);
        if matches!(rl.check(&key), TokenBucketDecision::Allow { .. }) {
            *allowed_per_key.entry(key).or_insert(0) += 1;
        }
    }
    assert!(allowed_per_key.values().all(|&n| n <= 3));
}

/// Property-ish check: breaker counters remain monotonic while
/// the breaker transitions through a randomized success/failure stream.
#[test]
fn circuit_breaker_counters_are_monotonic_under_mixed_outcomes() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 4,
        success_threshold: 1,
        open_duration: Duration::from_millis(5),
        name: "prop-breaker".to_string(),
        ..CircuitBreakerConfig::default()
    });
    let mut seed = 0xfeed_beefu64;
    let mut last = cb.counters();
    for _ in 0..64 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let fail = seed & 1 == 0;
        let _ = if fail {
            cb.call(|| -> Result<(), &str> { Err("x") })
        } else {
            cb.call(|| -> Result<(), &str> { Ok(()) })
        };
        let now = cb.counters();
        assert!(now.calls_total >= last.calls_total);
        assert!(now.calls_rejected_total >= last.calls_rejected_total);
        assert!(now.trips_total >= last.trips_total);
        assert!(now.recoveries_total >= last.recoveries_total);
        last = now;
    }
}

/// Chaos check: a worker panic is caught by the runtime and
/// surfaced as a join error without poisoning the rest of the test.
#[test]
fn panic_recovery_catches_worker_panic() {
    let handle = std::thread::spawn(|| {
        panic!("intentional panic for chaos test");
    });
    assert!(handle.join().is_err());
}

/// Chaos check: an aborted async task is cancelled cleanly.
#[tokio::test]
async fn join_handle_abort_cancels_task_cleanly() {
    let handle = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(60)).await;
        42u32
    });
    handle.abort();
    let result = handle.await;
    assert!(result.is_err());
}

/// Scenario 8: end-to-end timing sanity check. A closed
/// breaker serving traffic should add < 1µs per call.
/// This is a soft assertion (fails only on gross regression).
#[test]
fn breaker_closed_call_is_fast() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
    let n = 10_000;
    let start = Instant::now();
    for _ in 0..n {
        let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(1) });
    }
    let elapsed = start.elapsed();
    let per_call = elapsed / n;
    // Coarse bound: 100µs/call. This is a guardrail only; the
    // actual benchmark lives in the Criterion suite.
    assert!(
        per_call < Duration::from_micros(100),
        "per-call time {} ns exceeded 100µs budget",
        per_call.as_nanos()
    );
}
