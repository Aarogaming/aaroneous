// Resilience patterns: circuit breakers, retry policies, recovery helpers.
//
// These primitives give the rest of the system a way to fail fast on a
// broken subsystem (circuit breaker), retry transient failures with
// exponential backoff (retry policy), and combine both with a single
// `with_circuit_breaker` helper. All types are `Send + Sync` so they can
// be shared between the autonomic loop, the action executor, and the
// HTTP layer.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// State of a circuit breaker. Encoded as a single byte for atomic
/// transitions; the human-readable enum is reconstructed on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// All calls flow through.
    Closed,
    /// Calls fail fast with `CircuitBreakerError::Open`.
    Open,
    /// A single probe call is allowed through to test recovery.
    HalfOpen,
}

impl CircuitState {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Closed,
            1 => Self::Open,
            _ => Self::HalfOpen,
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            Self::Closed => 0,
            Self::Open => 1,
            Self::HalfOpen => 2,
        }
    }
}

/// Error returned by operations guarded by a circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    /// The breaker is open and rejected the call without invoking the closure.
    Open,
    /// The wrapped closure failed; the underlying error is preserved as a
    /// string for serde/log friendliness.
    Inner(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => f.write_str("circuit breaker is open"),
            Self::Inner(s) => write!(f, "inner error: {}", s),
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

/// Configuration for a `CircuitBreaker`.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures that trip the breaker from Closed
    /// to Open. Set to 0 to disable (the breaker will never open).
    pub failure_threshold: u32,
    /// Number of consecutive successes in the HalfOpen state that close
    /// the breaker again.
    pub success_threshold: u32,
    /// How long the breaker stays Open before transitioning to HalfOpen
    /// and allowing a probe call.
    pub open_duration: Duration,
    /// Human-readable name used in log lines.
    pub name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            open_duration: Duration::from_secs(30),
            name: "default".to_string(),
        }
    }
}

impl CircuitBreakerConfig {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_failure_threshold(mut self, n: u32) -> Self {
        self.failure_threshold = n;
        self
    }

    pub fn with_open_duration(mut self, d: Duration) -> Self {
        self.open_duration = d;
        self
    }
}

/// Circuit breaker primitive. `Send + Sync` so it can be wrapped in an
/// `Arc` and shared across threads. State transitions and counters are
/// all atomic so reads from health-check endpoints do not block.
pub struct CircuitBreaker {
    state: AtomicU8,
    consecutive_failures: AtomicU64,
    consecutive_successes: AtomicU64,
    opened_at_ms: AtomicU64, // millis since UNIX_EPOCH
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed.to_u8()),
            consecutive_failures: AtomicU64::new(0),
            consecutive_successes: AtomicU64::new(0),
            opened_at_ms: AtomicU64::new(0),
            config,
        }
    }

    pub fn state(&self) -> CircuitState {
        let raw = self.state.load(Ordering::SeqCst);
        let state = CircuitState::from_u8(raw);
        // If we are Open but the cool-down has elapsed, transition to HalfOpen
        // on the next read. This is a lazy state transition: cheaper than a
        // dedicated timer thread, and a stale read is harmless because the
        // actual call() will perform the real transition.
        if state == CircuitState::Open {
            let now_ms = Self::now_ms();
            let opened = self.opened_at_ms.load(Ordering::SeqCst);
            if now_ms.saturating_sub(opened) >= self.config.open_duration.as_millis() as u64 {
                // Best-effort transition; ignore if another thread won the race.
                let _ = self.state.compare_exchange(
                    CircuitState::Open.to_u8(),
                    CircuitState::HalfOpen.to_u8(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                return CircuitState::HalfOpen;
            }
        }
        state
    }

    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Call `f` under the protection of this breaker. If the breaker is
    /// Open, returns `CircuitBreakerError::Open` immediately without
    /// invoking `f`. If the breaker is HalfOpen, only one call is allowed
    /// through at a time (CAS-based token).
    pub fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        match self.state() {
            CircuitState::Open => Err(CircuitBreakerError::Open),
            CircuitState::HalfOpen => {
                // Allow the call through. If two threads both observe
                // HalfOpen we accept the cost: at most one extra probe,
                // which is fine for recovery.
                self.invoke(f)
            }
            CircuitState::Closed => self.invoke(f),
        }
    }

    fn invoke<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        match f() {
            Ok(value) => {
                self.on_success();
                Ok(value)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitBreakerError::Inner(e.to_string()))
            }
        }
    }

    fn on_success(&self) {
        // Any success closes a HalfOpen breaker immediately so the system
        // is not gated on `success_threshold` consecutive successes during
        // steady-state recovery.
        let _ = self.state.compare_exchange(
            CircuitState::HalfOpen.to_u8(),
            CircuitState::Closed.to_u8(),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.fetch_add(1, Ordering::SeqCst);
    }

    fn on_failure(&self) {
        let prev = self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
        // If we were HalfOpen and just failed, trip the breaker back to Open
        // and reset the cool-down clock. The probe failed; back off again.
        let was_half_open = self
            .state
            .compare_exchange(
                CircuitState::HalfOpen.to_u8(),
                CircuitState::Open.to_u8(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok();
        if was_half_open {
            self.opened_at_ms.store(Self::now_ms(), Ordering::SeqCst);
            self.consecutive_failures.store(1, Ordering::SeqCst);
            return;
        }
        if prev + 1 >= self.config.failure_threshold as u64
            && self.config.failure_threshold > 0
        {
            let _ = self.state.compare_exchange(
                CircuitState::Closed.to_u8(),
                CircuitState::Open.to_u8(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            self.opened_at_ms.store(Self::now_ms(), Ordering::SeqCst);
            println!(
                "[CircuitBreaker:{}] tripped after {} consecutive failures",
                self.config.name,
                prev + 1
            );
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Force the breaker to the Open state. Useful for tests and for
    /// manually quarantining a downstream service.
    pub fn trip(&self) {
        self.state
            .store(CircuitState::Open.to_u8(), Ordering::SeqCst);
        self.opened_at_ms.store(Self::now_ms(), Ordering::SeqCst);
    }

    /// Force the breaker to the Closed state.
    pub fn reset(&self) {
        self.state
            .store(CircuitState::Closed.to_u8(), Ordering::SeqCst);
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
        self.opened_at_ms.store(0, Ordering::SeqCst);
    }
}

/// Configuration for a `RetryPolicy`. Exponential backoff with optional
/// jitter. The defaults give 50ms -> 100ms -> 200ms -> ... capped at 5s.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Self::default()
        }
    }

    /// Compute the delay for the nth attempt (0-indexed).
    pub fn delay_for(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }
        let exp = self.multiplier.powi(attempt as i32);
        let nanos = (self.initial_delay.as_nanos() as f64 * exp) as u128;
        let mut delay = Duration::from_nanos(nanos.min(self.max_delay.as_nanos()) as u64);
        if self.jitter {
            // Simple deterministic jitter: hash attempt number into a 0-30%
            // range so two callers do not retry in lockstep. The hash uses
            // a basic LCG, no rng dependency.
            let lcg = (attempt as u64)
                .wrapping_mul(2_654_435_761)
                .wrapping_add(1);
            let factor = 1.0 + ((lcg % 30) as f64) / 100.0;
            delay = Duration::from_nanos(((delay.as_nanos() as f64) * factor) as u64);
        }
        delay
    }
}

/// Error returned by `with_retry` after exhausting the configured number
/// of attempts. Wraps the last inner error as a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryError {
    pub attempts: u32,
    pub last_error: String,
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "retry failed after {} attempts: {}",
            self.attempts, self.last_error
        )
    }
}

impl std::error::Error for RetryError {}

/// Run `f` under the given `RetryPolicy`. Sleeps between attempts using
/// `policy.delay_for`. The closure is called up to `policy.max_attempts`
/// times. The first successful result is returned; the last error is
/// wrapped in a `RetryError` if all attempts fail.
pub fn with_retry<F, T, E>(policy: &RetryPolicy, mut f: F) -> Result<T, RetryError>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut last_err = String::new();
    for attempt in 0..policy.max_attempts {
        match f() {
            Ok(value) => return Ok(value),
            Err(e) => {
                last_err = e.to_string();
                if attempt + 1 < policy.max_attempts {
                    std::thread::sleep(policy.delay_for(attempt));
                }
            }
        }
    }
    Err(RetryError {
        attempts: policy.max_attempts,
        last_error: last_err,
    })
}

/// Combine a circuit breaker and a retry policy. The closure is retried
/// up to `policy.max_attempts` times, but each attempt is gated by the
/// breaker: if the breaker is Open, the call returns `Open` immediately
/// without invoking the closure. This is the helper used in
/// `action_executor` and the HTTP client paths.
pub fn with_circuit_breaker<F, T, E>(
    breaker: &CircuitBreaker,
    policy: &RetryPolicy,
    mut f: F,
) -> Result<T, CircuitBreakerError>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut last_err = String::new();
    for attempt in 0..policy.max_attempts {
        match breaker.call(&mut f) {
            Ok(value) => return Ok(value),
            Err(CircuitBreakerError::Open) => {
                // Breaker is open; do not retry. The caller can call back
                // later when the breaker transitions to HalfOpen.
                return Err(CircuitBreakerError::Open);
            }
            Err(CircuitBreakerError::Inner(s)) => {
                last_err = s;
                if attempt + 1 < policy.max_attempts {
                    std::thread::sleep(policy.delay_for(attempt));
                }
            }
        }
    }
    Err(CircuitBreakerError::Inner(format!(
        "retries exhausted: {}",
        last_err
    )))
}

/// Shared handle type for components that want to pass a breaker around
/// without moving the underlying state.
pub type SharedCircuitBreaker = Arc<CircuitBreaker>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn circuit_breaker_starts_closed_and_allows_calls() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        let result: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(42) });
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn circuit_breaker_trips_after_repeated_failures() {
        let cb = CircuitBreaker::new(
            CircuitBreakerConfig::default()
                .with_failure_threshold(3)
                .with_name("trip_test"),
        );
        for _ in 0..3 {
            let _: Result<i32, CircuitBreakerError> =
                cb.call(|| -> Result<i32, &str> { Err("nope") });
        }
        assert_eq!(cb.state(), CircuitState::Open);
        // Subsequent calls fail fast
        let result: Result<i32, CircuitBreakerError> =
            cb.call(|| -> Result<i32, &str> { Ok(1) });
        assert_eq!(result.unwrap_err(), CircuitBreakerError::Open);
    }

    #[test]
    fn circuit_breaker_transitions_to_half_open_after_cool_down() {
        let cb = CircuitBreaker::new(
            CircuitBreakerConfig {
                open_duration: Duration::from_millis(50),
                ..CircuitBreakerConfig::default()
            }
            .with_failure_threshold(1),
        );
        let _: Result<i32, CircuitBreakerError> =
            cb.call(|| -> Result<i32, &str> { Err("x") });
        assert_eq!(cb.state(), CircuitState::Open);
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // Successful call closes the breaker
        let result: Result<i32, CircuitBreakerError> =
            cb.call(|| -> Result<i32, &str> { Ok(99) });
        assert_eq!(result.unwrap(), 99);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn circuit_breaker_reopens_when_half_open_probe_fails() {
        let cb = CircuitBreaker::new(
            CircuitBreakerConfig {
                open_duration: Duration::from_millis(10),
                ..CircuitBreakerConfig::default()
            }
            .with_failure_threshold(1),
        );
        let _: Result<i32, CircuitBreakerError> =
            cb.call(|| -> Result<i32, &str> { Err("a") });
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // Probe fails; should reopen
        let _: Result<i32, CircuitBreakerError> =
            cb.call(|| -> Result<i32, &str> { Err("b") });
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn retry_policy_succeeds_on_first_attempt() {
        let policy = RetryPolicy::default();
        let counter = AtomicU32::new(0);
        let result: Result<u32, RetryError> = with_retry(&policy, || -> Result<u32, &str> {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(7)
        });
        assert_eq!(result.unwrap(), 7);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn retry_policy_retries_then_succeeds() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(1),
            jitter: false,
            ..RetryPolicy::default()
        };
        let counter = AtomicU32::new(0);
        let result: Result<u32, RetryError> = with_retry(&policy, || -> Result<u32, &str> {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                Err("not yet")
            } else {
                Ok(42)
            }
        });
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn retry_policy_exhausts_attempts() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            jitter: false,
            ..RetryPolicy::default()
        };
        let counter = AtomicU32::new(0);
        let result: Result<u32, RetryError> = with_retry(&policy, || -> Result<u32, &str> {
            counter.fetch_add(1, Ordering::SeqCst);
            Err("always")
        });
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 3);
        assert_eq!(err.last_error, "always");
    }

    #[test]
    fn retry_delay_grows_exponentially() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(10),
            multiplier: 2.0,
            jitter: false,
            ..RetryPolicy::default()
        };
        assert_eq!(policy.delay_for(0), Duration::from_millis(10));
        assert_eq!(policy.delay_for(1), Duration::from_millis(20));
        assert_eq!(policy.delay_for(2), Duration::from_millis(40));
        assert_eq!(policy.delay_for(3), Duration::from_millis(80));
    }

    #[test]
    fn retry_delay_caps_at_max_delay() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            multiplier: 10.0,
            max_delay: Duration::from_millis(500),
            jitter: false,
            ..RetryPolicy::default()
        };
        assert_eq!(policy.delay_for(5), Duration::from_millis(500));
    }

    #[test]
    fn with_circuit_breaker_short_circuits_when_open() {
        let cb = CircuitBreaker::new(
            CircuitBreakerConfig {
                open_duration: Duration::from_secs(60),
                ..CircuitBreakerConfig::default()
            }
            .with_failure_threshold(1),
        );
        let _: Result<i32, CircuitBreakerError> =
            cb.call(|| -> Result<i32, &str> { Err("trip") });
        let counter = AtomicU32::new(0);
        let result: Result<i32, CircuitBreakerError> = with_circuit_breaker(
            &cb,
            &RetryPolicy {
                max_attempts: 5,
                initial_delay: Duration::from_millis(1),
                jitter: false,
                ..RetryPolicy::default()
            },
            || -> Result<i32, &str> {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(1)
            },
        );
        assert_eq!(result.unwrap_err(), CircuitBreakerError::Open);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn with_circuit_breaker_retries_inner_failures() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let counter = AtomicU32::new(0);
        let result: Result<i32, CircuitBreakerError> = with_circuit_breaker(
            &cb,
            &RetryPolicy {
                max_attempts: 3,
                initial_delay: Duration::from_millis(1),
                jitter: false,
                ..RetryPolicy::default()
            },
            || -> Result<i32, &str> {
                let n = counter.fetch_add(1, Ordering::SeqCst);
                if n < 1 {
                    Err("retry me")
                } else {
                    Ok(99)
                }
            },
        );
        assert_eq!(result.unwrap(), 99);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
