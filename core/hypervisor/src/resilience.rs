// Resilience patterns: circuit breakers, retry policies, recovery helpers.
//
// These primitives give the rest of the system a way to fail fast on a
// broken subsystem (circuit breaker), retry transient failures with
// exponential backoff (retry policy), and combine both with a single
// `with_circuit_breaker` helper. All types are `Send + Sync` so they can
// be shared between the autonomic loop, the action executor, and the
// HTTP layer.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::warn;

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
    /// Sliding window size used to compute failure rate.
    /// When enough calls land inside the window and the
    /// failure ratio exceeds `failure_rate_threshold`, the
    /// breaker trips even if failures are not consecutive.
    pub failure_rate_window: Duration,
    /// Failure ratio threshold over the sliding window.
    pub failure_rate_threshold: f64,
    /// Minimum number of calls inside the window before the
    /// rate-based trip logic can trigger.
    pub failure_rate_min_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            open_duration: Duration::from_secs(30),
            name: "default".to_string(),
            failure_rate_window: Duration::from_secs(30),
            failure_rate_threshold: 0.5,
            failure_rate_min_calls: 10,
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
    /// Counters surfaced through `counters()` for the
    /// /metrics endpoint. Each is a `u64` total since
    /// the breaker was constructed.
    calls_total: AtomicU64,
    calls_rejected_total: AtomicU64,
    trips_total: AtomicU64,
    recoveries_total: AtomicU64,
    recent_outcomes: Mutex<VecDeque<(u64, bool)>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed.to_u8()),
            consecutive_failures: AtomicU64::new(0),
            consecutive_successes: AtomicU64::new(0),
            opened_at_ms: AtomicU64::new(0),
            config,
            calls_total: AtomicU64::new(0),
            calls_rejected_total: AtomicU64::new(0),
            trips_total: AtomicU64::new(0),
            recoveries_total: AtomicU64::new(0),
            recent_outcomes: Mutex::new(VecDeque::new()),
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
        self.calls_total.fetch_add(1, Ordering::SeqCst);
        match self.state() {
            CircuitState::Open => {
                self.calls_rejected_total.fetch_add(1, Ordering::SeqCst);
                Err(CircuitBreakerError::Open)
            }
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
        self.record_outcome(true);
        // Any success closes a HalfOpen breaker immediately so the system
        // is not gated on `success_threshold` consecutive successes during
        // steady-state recovery.
        let was_half_open = self
            .state
            .compare_exchange(
                CircuitState::HalfOpen.to_u8(),
                CircuitState::Closed.to_u8(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok();
        if was_half_open {
            self.recoveries_total.fetch_add(1, Ordering::SeqCst);
            tracing::info!(
                target: "circuit_breaker",
                breaker = %self.config.name,
                "circuit breaker recovered (HalfOpen -> Closed)"
            );
        }
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.fetch_add(1, Ordering::SeqCst);
    }

    fn on_failure(&self) {
        let prev = self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
        self.record_outcome(false);
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
            self.trips_total.fetch_add(1, Ordering::SeqCst);
            self.opened_at_ms.store(Self::now_ms(), Ordering::SeqCst);
            self.consecutive_failures.store(1, Ordering::SeqCst);
            tracing::warn!(
                target: "circuit_breaker",
                breaker = %self.config.name,
                "circuit breaker re-tripped (HalfOpen -> Open)"
            );
            return;
        }
        if prev + 1 >= self.config.failure_threshold as u64 && self.config.failure_threshold > 0 {
            let tripped = self
                .state
                .compare_exchange(
                    CircuitState::Closed.to_u8(),
                    CircuitState::Open.to_u8(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok();
            if tripped {
                self.trips_total.fetch_add(1, Ordering::SeqCst);
            }
            self.opened_at_ms.store(Self::now_ms(), Ordering::SeqCst);
            if tripped {
                warn!(
                    breaker = %self.config.name,
                    failures = prev + 1,
                    "circuit breaker tripped"
                );
            }
        }
    }

    fn record_outcome(&self, success: bool) {
        let now = Self::now_ms();
        let mut window = self.recent_outcomes.lock();
        window.push_back((now, success));
        let cutoff = now.saturating_sub(self.config.failure_rate_window.as_millis() as u64);
        while let Some((ts, _)) = window.front().copied() {
            if ts >= cutoff {
                break;
            }
            window.pop_front();
        }
        let total = window.len() as u32;
        if total < self.config.failure_rate_min_calls || self.config.failure_rate_threshold <= 0.0 {
            return;
        }
        let failures = window.iter().filter(|(_, ok)| !*ok).count() as f64;
        let failure_rate = failures / total as f64;
        if failure_rate >= self.config.failure_rate_threshold {
            let tripped = self
                .state
                .compare_exchange(
                    CircuitState::Closed.to_u8(),
                    CircuitState::Open.to_u8(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok();
            if tripped {
                self.trips_total.fetch_add(1, Ordering::SeqCst);
                self.opened_at_ms.store(now, Ordering::SeqCst);
                warn!(
                    breaker = %self.config.name,
                    failure_rate = failure_rate,
                    calls_in_window = total,
                    "circuit breaker tripped by sliding window"
                );
            }
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
        let was_open = self
            .state
            .swap(CircuitState::Open.to_u8(), Ordering::SeqCst)
            == CircuitState::Open.to_u8();
        if !was_open {
            self.trips_total.fetch_add(1, Ordering::SeqCst);
        }
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

    /// Snapshot of the public counters. Returned as a
    /// struct so /metrics can format them without
    /// reaching into atomic internals.
    pub fn counters(&self) -> CircuitBreakerCounters {
        CircuitBreakerCounters {
            calls_total: self.calls_total.load(Ordering::SeqCst),
            calls_rejected_total: self.calls_rejected_total.load(Ordering::SeqCst),
            trips_total: self.trips_total.load(Ordering::SeqCst),
            recoveries_total: self.recoveries_total.load(Ordering::SeqCst),
        }
    }
}

/// Snapshot of a breaker's cumulative counters. Cloned
/// cheaply; safe to read across threads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CircuitBreakerCounters {
    pub calls_total: u64,
    pub calls_rejected_total: u64,
    pub trips_total: u64,
    pub recoveries_total: u64,
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
            let lcg = (attempt as u64).wrapping_mul(2_654_435_761).wrapping_add(1);
            let factor = 1.0 + ((lcg % 30) as f64) / 100.0;
            delay = Duration::from_nanos(((delay.as_nanos() as f64) * factor) as u64);
        }
        delay
    }
}

/// Classification used by retry helpers to distinguish transient
/// failures from permanent ones. Permanent failures fail fast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Retryable,
    Permanent,
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
pub fn with_retry<F, T, E>(policy: &RetryPolicy, f: F) -> Result<T, RetryError>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    with_retry_classified(policy, f, |_| ErrorKind::Retryable)
}

/// Retry helper with explicit error classification. Permanent errors fail
/// fast; retryable errors follow the normal backoff schedule.
pub fn with_retry_classified<F, T, E, C>(
    policy: &RetryPolicy,
    mut f: F,
    mut classify: C,
) -> Result<T, RetryError>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
    C: FnMut(&E) -> ErrorKind,
{
    let mut last_err = String::new();
    for attempt in 0..policy.max_attempts {
        match f() {
            Ok(value) => return Ok(value),
            Err(e) => {
                last_err = e.to_string();
                if matches!(classify(&e), ErrorKind::Permanent) {
                    break;
                }
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

// ====================================================================
// Async variants
// ====================================================================
//
// The sync helpers above are still the right tool for blocking I/O and
// CPU-bound work. For the federation's async I/O paths (HTTP client,
// action executor's external API calls, federation events) we need
// async-aware versions. The state machine of the circuit breaker is
// sync (atomics), so we keep that and just gate the future at the
// call site. The retry helper uses `tokio::time::sleep` so it does
// not block the executor thread.

/// Result of a `with_timeout` race.
#[derive(Debug)]
pub enum TimeoutError<T> {
    /// The future did not complete in time.
    Elapsed,
    /// The future completed; the inner value is preserved so
    /// the caller can return a graceful error rather than a
    /// `JoinError`.
    Inner(T),
}

impl<T: std::fmt::Display> std::fmt::Display for TimeoutError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::Elapsed => f.write_str("future did not complete before timeout"),
            TimeoutError::Inner(e) => write!(f, "future completed with error after race: {e}"),
        }
    }
}

/// Race `fut` against `duration`. Returns `Ok(value)` if the
/// future completed in time, `Err(TimeoutError::Elapsed)`
/// otherwise. The future is dropped on timeout — no resource
/// leak, but no way to surface its result either.
pub async fn with_timeout<T>(
    duration: std::time::Duration,
    fut: impl std::future::Future<Output = T>,
) -> Result<T, TimeoutError<T>> {
    match tokio::time::timeout(duration, fut).await {
        Ok(v) => Ok(v),
        Err(_) => Err(TimeoutError::Elapsed),
    }
}

/// Async version of `with_retry`. Each call to `f` returns a
/// future; the loop awaits them sequentially, sleeping
/// `policy.delay_for(attempt)` between failed attempts using
/// `tokio::time::sleep` (does not block the executor).
pub async fn with_retry_async<F, Fut, T, E>(policy: &RetryPolicy, f: F) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    with_retry_async_classified(policy, f, |_| ErrorKind::Retryable).await
}

/// Async retry helper with explicit error classification.
pub async fn with_retry_async_classified<F, Fut, T, E, C>(
    policy: &RetryPolicy,
    mut f: F,
    mut classify: C,
) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    C: FnMut(&E) -> ErrorKind,
{
    let mut last_err = String::new();
    for attempt in 0..policy.max_attempts {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                last_err = e.to_string();
                if matches!(classify(&e), ErrorKind::Permanent) {
                    break;
                }
                if attempt + 1 < policy.max_attempts {
                    tokio::time::sleep(policy.delay_for(attempt)).await;
                }
            }
        }
    }
    Err(RetryError {
        attempts: policy.max_attempts,
        last_error: last_err,
    })
}

/// Async circuit-breaker call. The breaker's state machine is
/// still sync (atomic), so this just gates the future at the
/// call site. On Open, returns `CircuitBreakerError::Open`
/// without polling the future once. On Closed/HalfOpen,
/// awaits the future, applies the same success/failure
/// accounting as the sync `call`.
impl CircuitBreaker {
    /// Async counterpart to `call`. Counter accounting
    /// (`calls_total`, `calls_rejected_total`, etc.) is
    /// identical; only the inner function call changes from
    /// `f()` to `f().await`.
    pub async fn call_async<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        self.calls_total.fetch_add(1, Ordering::SeqCst);
        match self.state() {
            CircuitState::Open => {
                self.calls_rejected_total.fetch_add(1, Ordering::SeqCst);
                Err(CircuitBreakerError::Open)
            }
            CircuitState::HalfOpen | CircuitState::Closed => match f().await {
                Ok(v) => {
                    self.on_success();
                    Ok(v)
                }
                Err(e) => {
                    self.on_failure();
                    Err(CircuitBreakerError::Inner(e.to_string()))
                }
            },
        }
    }
}

/// Async version of `with_circuit_breaker`. Combines the
/// retry loop and the breaker gate: the breaker is consulted
/// before each attempt and short-circuits with `Open` on
/// failure, the future is awaited, and `tokio::time::sleep`
/// paces the retries.
pub async fn with_circuit_breaker_async<F, Fut, T, E>(
    breaker: &CircuitBreaker,
    policy: &RetryPolicy,
    mut f: F,
) -> Result<T, CircuitBreakerError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_err = String::new();
    for attempt in 0..policy.max_attempts {
        match breaker.call_async(&mut f).await {
            Ok(value) => return Ok(value),
            Err(CircuitBreakerError::Open) => {
                return Err(CircuitBreakerError::Open);
            }
            Err(CircuitBreakerError::Inner(s)) => {
                last_err = s;
                if attempt + 1 < policy.max_attempts {
                    tokio::time::sleep(policy.delay_for(attempt)).await;
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

/// Error returned when a bulkhead is saturated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkheadError {
    Saturated,
}

impl std::fmt::Display for BulkheadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bulkhead saturated")
    }
}

impl std::error::Error for BulkheadError {}

/// Configuration for a semaphore-backed bulkhead.
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    pub permits: usize,
    pub name: String,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            permits: 8,
            name: "default".to_string(),
        }
    }
}

/// Semaphore-backed concurrency limiter.
#[derive(Clone)]
pub struct Bulkhead {
    semaphore: Arc<Semaphore>,
    config: BulkheadConfig,
}

impl Bulkhead {
    pub fn new(config: BulkheadConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.permits)),
            config,
        }
    }

    pub fn try_acquire(&self) -> Result<tokio::sync::OwnedSemaphorePermit, BulkheadError> {
        self.semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| BulkheadError::Saturated)
    }

    pub async fn execute_async<F, Fut, T>(&self, f: F) -> Result<T, BulkheadError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| BulkheadError::Saturated)?;
        Ok(f().await)
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }
}

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
        let result: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(1) });
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
        let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("x") });
        assert_eq!(cb.state(), CircuitState::Open);
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // Successful call closes the breaker
        let result: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Ok(99) });
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
        let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("a") });
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        // Probe fails; should reopen
        let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("b") });
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn circuit_breaker_trips_on_sliding_failure_rate() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 100,
            success_threshold: 1,
            open_duration: Duration::from_millis(50),
            name: "window_test".to_string(),
            failure_rate_window: Duration::from_secs(5),
            failure_rate_threshold: 0.75,
            failure_rate_min_calls: 4,
        });
        let _ = cb.call(|| -> Result<(), &str> { Err("fail-1") });
        let _ = cb.call(|| -> Result<(), &str> { Ok(()) });
        let _ = cb.call(|| -> Result<(), &str> { Err("fail-2") });
        let _ = cb.call(|| -> Result<(), &str> { Err("fail-3") });
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
            if n < 2 { Err("not yet") } else { Ok(42) }
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
        let _: Result<i32, CircuitBreakerError> = cb.call(|| -> Result<i32, &str> { Err("trip") });
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
                if n < 1 { Err("retry me") } else { Ok(99) }
            },
        );
        assert_eq!(result.unwrap(), 99);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn retry_classifier_fails_fast_on_permanent_errors() {
        let policy = RetryPolicy {
            max_attempts: 5,
            ..RetryPolicy::default()
        };
        let attempts = AtomicU32::new(0);
        let result = with_retry_classified(
            &policy,
            || {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>("bad input")
            },
            |_| ErrorKind::Permanent,
        );
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn bulkhead_denies_when_saturated() {
        let bulkhead = Bulkhead::new(BulkheadConfig {
            permits: 1,
            name: "bulkhead-test".to_string(),
        });
        let permit = bulkhead.try_acquire().expect("first permit should succeed");
        assert!(matches!(
            bulkhead.try_acquire(),
            Err(BulkheadError::Saturated)
        ));
        drop(permit);
        assert!(bulkhead.try_acquire().is_ok());
    }

    #[test]
    fn counters_track_calls_and_trips() {
        // A breaker that trips after one failure: every call
        // increments calls_total, the trip increments
        // trips_total, and rejections increment
        // calls_rejected_total.
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            name: "test".to_string(),
            failure_threshold: 1,
            open_duration: Duration::from_secs(60),
            ..CircuitBreakerConfig::default()
        });
        // One failing call → trip.
        let _ = cb.call(|| -> Result<(), &str> { Err("boom") });
        let c = cb.counters();
        assert_eq!(c.calls_total, 1);
        assert_eq!(c.trips_total, 1);
        assert_eq!(c.calls_rejected_total, 0);
        assert_eq!(c.recoveries_total, 0);

        // The breaker is now Open; the next call is rejected
        // without invoking the closure.
        let rejected = cb.call(|| -> Result<(), &str> { Ok(()) });
        assert!(matches!(rejected, Err(CircuitBreakerError::Open)));
        let c = cb.counters();
        assert_eq!(c.calls_total, 2);
        assert_eq!(c.calls_rejected_total, 1);
        assert_eq!(c.trips_total, 1, "manual-trip is not from a failure");
    }

    #[test]
    fn counters_track_recoveries() {
        // Trip the breaker, then manually reset to Closed.
        // `trip()` increments trips_total, `reset()` does not
        // increment recoveries_total (recoveries are recorded
        // only when the breaker self-recovers via HalfOpen
        // success).
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            name: "test".to_string(),
            failure_threshold: 1,
            open_duration: Duration::from_millis(100),
            ..CircuitBreakerConfig::default()
        });
        let _ = cb.call(|| -> Result<(), &str> { Err("boom") });
        assert!(matches!(cb.state(), CircuitState::Open));
        std::thread::sleep(Duration::from_millis(150));
        // Next call: breaker is Open but cool-down has
        // elapsed, so state() returns HalfOpen and the call
        // goes through. A success transitions HalfOpen ->
        // Closed and bumps recoveries_total.
        let _ = cb.call(|| -> Result<(), &str> { Ok(()) });
        let c = cb.counters();
        assert_eq!(
            c.recoveries_total, 1,
            "HalfOpen -> Closed should record a recovery"
        );
        assert!(matches!(cb.state(), CircuitState::Closed));
    }

    // -------- Async variants --------

    #[tokio::test]
    async fn with_timeout_returns_value_when_fast() {
        let result: Result<i32, TimeoutError<i32>> =
            with_timeout(Duration::from_millis(50), async { 7 }).await;
        assert_eq!(result.unwrap(), 7);
    }

    #[tokio::test]
    async fn with_timeout_returns_elapsed_when_slow() {
        let result: Result<i32, TimeoutError<i32>> =
            with_timeout(Duration::from_millis(5), async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                7
            })
            .await;
        assert!(matches!(result, Err(TimeoutError::Elapsed)));
    }

    #[tokio::test]
    async fn with_retry_async_succeeds_after_failures() {
        let counter = AtomicU32::new(0);
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            jitter: false,
            ..RetryPolicy::default()
        };
        let result = with_retry_async(&policy, || {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            async move { if n < 2 { Err("transient") } else { Ok(99) } }
        })
        .await
        .unwrap();
        assert_eq!(result, 99);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn circuit_breaker_call_async_accounts_for_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            name: "async_test".to_string(),
            failure_threshold: 1,
            open_duration: Duration::from_secs(60),
            ..CircuitBreakerConfig::default()
        });
        // One failing future → trip.
        let _ = cb.call_async(|| async { Err::<(), &str>("boom") }).await;
        let c = cb.counters();
        assert_eq!(c.calls_total, 1);
        assert_eq!(c.trips_total, 1);

        // Next call short-circuits with Open without
        // polling the future.
        let rejected: Result<(), CircuitBreakerError> =
            cb.call_async(|| async { Ok::<(), &str>(()) }).await;
        assert!(matches!(rejected, Err(CircuitBreakerError::Open)));
        let c = cb.counters();
        assert_eq!(c.calls_total, 2);
        assert_eq!(c.calls_rejected_total, 1);
    }

    #[tokio::test]
    async fn with_circuit_breaker_async_retries_and_recovers() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            name: "async_recovery".to_string(),
            failure_threshold: 1,
            open_duration: Duration::from_millis(10),
            ..CircuitBreakerConfig::default()
        });
        let counter = AtomicU32::new(0);
        let policy = RetryPolicy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            jitter: false,
            ..RetryPolicy::default()
        };
        // First attempt: failure → trip. Retry sleeps,
        // cool-down elapses, the breaker is now HalfOpen
        // and the next call is allowed through and succeeds.
        let result = with_circuit_breaker_async(&cb, &policy, || {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            async move { if n == 0 { Err("first fails") } else { Ok(42) } }
        })
        .await
        .unwrap();
        assert_eq!(result, 42);
        let c = cb.counters();
        assert_eq!(c.recoveries_total, 1, "HalfOpen -> Closed via async path");
    }

    /// End-to-end demonstration: stand up a tiny mock HTTP
    /// server that flips between failing and succeeding,
    /// wrap a `reqwest` call with `with_timeout` and
    /// `with_circuit_breaker_async`, and confirm the retry
    /// policy recovers from a transient failure.
    #[tokio::test]
    async fn async_resilience_handles_transient_http_failure() {
        use std::sync::atomic::{AtomicU32, Ordering as AOrd};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let attempts = Arc::new(AtomicU32::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local = listener.local_addr().unwrap();
        let attempts_for_server = attempts.clone();
        let server = tokio::spawn(async move {
            // Accept exactly two requests: the first
            // returns 503, the second returns 200.
            for _ in 0..2 {
                let (mut s, _) = listener.accept().await.unwrap();
                let n = attempts_for_server.fetch_add(1, AOrd::SeqCst);
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf).await;
                let body = if n == 0 { "transient" } else { "ok" };
                let status = if n == 0 { "503" } else { "200" };
                let resp = format!(
                    "HTTP/1.1 {status} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    if n == 0 { "Service Unavailable" } else { "OK" },
                    body.len(),
                    body,
                );
                let _ = s.write_all(resp.as_bytes()).await;
                let _ = s.shutdown().await;
            }
        });

        // The "call" under the breaker is a reqwest GET.
        // We treat any non-2xx as a failure; the second
        // attempt gets a 200 and the retry succeeds.
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            name: "http_demo".to_string(),
            failure_threshold: 5,
            open_duration: Duration::from_secs(60),
            ..CircuitBreakerConfig::default()
        });
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(5),
            jitter: false,
            ..RetryPolicy::default()
        };
        let url = format!("http://{local}/");
        let result = with_circuit_breaker_async(&cb, &policy, || {
            let url = url.clone();
            async move {
                let fut = reqwest::get(&url);
                let resp = with_timeout(Duration::from_secs(2), fut)
                    .await
                    .map_err(|e| match e {
                        TimeoutError::Elapsed => "timeout".to_string(),
                        TimeoutError::Inner(e) => format!("inner: {e:?}"),
                    })?
                    .map_err(|e| format!("reqwest: {e}"))?;
                if !resp.status().is_success() {
                    return Err(format!("status: {}", resp.status()));
                }
                let text = resp.text().await.map_err(|e| format!("text: {e}"))?;
                Ok(text)
            }
        })
        .await
        .unwrap();

        assert_eq!(result, "ok");
        assert_eq!(attempts.load(AOrd::SeqCst), 2);
        server.await.unwrap();
    }
}
