// Smoke benchmarks for the hot paths added or repaired in
// Phase X. These are not full micro-benchmarks; they exist so
// that `cargo bench` produces a baseline number, and so that
// regressions show up as a non-zero delta in CI.
//
// Run with: `cargo bench -p a_run`
// Or one suite: `cargo bench -p a_run --bench phase_x_smoke`

use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;

use a_run::input_validation::{
    ValidationError, validate_bytes, validate_enum, validate_identifier, validate_range,
    validate_string,
};
use a_run::rate_limit::{TokenBucketConfig, TokenBucketLimiter, key_from_request};
use a_run::resilience::{CircuitBreaker, CircuitBreakerConfig, RetryPolicy};

fn bench_rate_limit_check(c: &mut Criterion) {
    let rl = TokenBucketLimiter::new(TokenBucketConfig {
        burst: 100.0,
        refill_per_second: 50.0,
        idle_eviction: None,
    });
    c.bench_function("rate_limit/check_single_key", |b| {
        b.iter(|| rl.check("bench-key"))
    });
}

fn bench_rate_limit_check_1000_keys(c: &mut Criterion) {
    let rl = TokenBucketLimiter::new(TokenBucketConfig {
        burst: 10.0,
        refill_per_second: 1.0,
        idle_eviction: None,
    });
    for i in 0..1000 {
        rl.check(&format!("k-{}", i));
    }
    c.bench_function("rate_limit/check_1000_distinct_keys", |b| {
        let mut i = 0u32;
        b.iter(|| {
            i = i.wrapping_add(1);
            rl.check(&format!("k-{}", i % 1000))
        })
    });
}

fn bench_key_from_request(c: &mut Criterion) {
    c.bench_function("rate_limit/key_from_request_auth", |b| {
        b.iter(|| key_from_request(Some("tenant-abc"), "10.0.0.1"))
    });
    c.bench_function("rate_limit/key_from_request_ip", |b| {
        b.iter(|| key_from_request(None, "10.0.0.1"))
    });
}

fn bench_validate_string(c: &mut Criterion) {
    c.bench_function("input_validation/validate_string_short", |b| {
        b.iter(|| validate_string("name", "hello", 100))
    });
    c.bench_function("input_validation/validate_string_long", |b| {
        let s = "x".repeat(500);
        b.iter(|| validate_string("name", &s, 1024))
    });
}

fn bench_validate_identifier(c: &mut Criterion) {
    c.bench_function("input_validation/validate_identifier_typical", |b| {
        b.iter(|| validate_identifier("model", "merlin-v2.1"))
    });
}

fn bench_validate_range(c: &mut Criterion) {
    c.bench_function("input_validation/validate_range_f64", |b| {
        b.iter(|| validate_range::<f64>("p", 0.5, 0.0, 1.0))
    });
}

fn bench_validate_bytes(c: &mut Criterion) {
    let payload = vec![0u8; 1024];
    c.bench_function("input_validation/validate_bytes_1kib", |b| {
        b.iter(|| validate_bytes("payload", &payload, 4096))
    });
}

fn bench_validate_enum(c: &mut Criterion) {
    let allowed = ["genome", "tensor", "model", "wasm", "link"];
    c.bench_function("input_validation/validate_enum_hit", |b| {
        b.iter(|| validate_enum("kind", "genome", &allowed))
    });
    c.bench_function("input_validation/validate_enum_miss", |b| {
        b.iter(|| validate_enum("kind", "unknown", &allowed))
    });
}

fn bench_circuit_breaker_call(c: &mut Criterion) {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig::default()
            .with_name("bench")
            .with_failure_threshold(3)
            .with_open_duration(Duration::from_millis(100)),
    );
    c.bench_function("resilience/circuit_breaker/call_ok", |b| {
        b.iter(|| cb.call(|| Ok::<_, String>(42)))
    });
}

fn bench_circuit_breaker_state_read(c: &mut Criterion) {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
    c.bench_function("resilience/circuit_breaker/state", |b| {
        b.iter(|| cb.state())
    });
}

fn bench_retry_policy_delay(c: &mut Criterion) {
    let p = RetryPolicy::default();
    c.bench_function("resilience/retry_policy/delay", |b| {
        b.iter(|| p.delay_for(3))
    });
    let p_jitter_off = RetryPolicy {
        jitter: false,
        ..RetryPolicy::default()
    };
    c.bench_function("resilience/retry_policy/delay_no_jitter", |b| {
        b.iter(|| p_jitter_off.delay_for(3))
    });
}

fn bench_validation_error_display(c: &mut Criterion) {
    let e: ValidationError = "name: must not be empty".to_string().into();
    c.bench_function("input_validation/error_display", |b| {
        b.iter(|| format!("{}", e))
    });
}

criterion_group!(
    benches,
    bench_rate_limit_check,
    bench_rate_limit_check_1000_keys,
    bench_key_from_request,
    bench_validate_string,
    bench_validate_identifier,
    bench_validate_range,
    bench_validate_bytes,
    bench_validate_enum,
    bench_circuit_breaker_call,
    bench_circuit_breaker_state_read,
    bench_retry_policy_delay,
    bench_validation_error_display,
);
criterion_main!(benches);
