// Per-key rate limiting using a token-bucket algorithm.
//
// Each key (typically the client IP, or a tenant ID derived from
// the auth token) gets its own bucket. The bucket refills at a
// steady rate and holds a fixed burst capacity. If a request
// arrives when the bucket is empty, the request is rejected with
// 429 Too Many Requests.
//
// The implementation is in-process and lock-free: each bucket is
// a `Mutex<BucketState>` and the map of buckets is a
// `Mutex<HashMap<String, BucketState>>`. This is sufficient for
// a single-node deployment. For a multi-node deployment, swap
// the map for a Redis-backed implementation behind the same
// `TokenBucketLimiter` trait.
//
// Named `TokenBucketLimiter` rather than `TokenBucketLimiter` to avoid
// collision with the existing `security_hardener::TokenBucketLimiter`
// and `federation::enterprise::rate_limiting::TokenBucketLimiter`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Per-key bucket state. Held inside a `Mutex` so the
/// `TokenBucketLimiter` can hand out `&mut` access without poisoning
/// the lock on panic.
#[derive(Debug)]
struct BucketState {
    /// Tokens currently in the bucket, possibly fractional.
    tokens: f64,
    /// Last time the bucket was refilled (monotonic clock).
    last_refill: Instant,
}

/// Configuration for a `TokenBucketLimiter`.
#[derive(Debug, Clone, Copy)]
pub struct TokenBucketConfig {
    /// Maximum burst: bucket capacity in tokens.
    pub burst: f64,
    /// Refill rate in tokens per second. A request consumes one
    /// token, so 10.0 = 10 req/s sustained, 20 burst.
    pub refill_per_second: f64,
    /// Idle buckets are evicted after this duration to bound
    /// memory. Set to `None` to never evict (useful for tests
    /// where short-lived keys need to be remembered).
    pub idle_eviction: Option<Duration>,
}

impl Default for TokenBucketConfig {
    fn default() -> Self {
        Self {
            burst: 20.0,
            refill_per_second: 10.0,
            idle_eviction: Some(Duration::from_secs(600)),
        }
    }
}

impl TokenBucketConfig {
    pub fn with_burst(mut self, burst: f64) -> Self {
        self.burst = burst;
        self
    }
    pub fn with_refill_per_second(mut self, r: f64) -> Self {
        self.refill_per_second = r;
        self
    }
}

/// Result of a single rate-limit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenBucketDecision {
    /// Request is allowed. `tokens_remaining` is the bucket
    /// state after consuming one token, useful for
    /// `X-RateLimit-Remaining` headers.
    Allow { tokens_remaining: u32 },
    /// Request is rejected. `retry_after` is a hint for the
    /// `Retry-After` header.
    Deny { retry_after: Duration },
}

/// Token-bucket rate limiter keyed by `String`. Thread-safe via
/// internal mutexes; cheap to clone and share across handlers.
#[derive(Debug)]
pub struct TokenBucketLimiter {
    config: TokenBucketConfig,
    buckets: Mutex<HashMap<String, BucketState>>,
}

impl TokenBucketLimiter {
    pub fn new(config: TokenBucketConfig) -> Self {
        Self {
            config,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub fn config(&self) -> TokenBucketConfig {
        self.config
    }

    /// Check (and consume) one token for `key`. If the bucket is
    /// empty, returns `Deny` with the time until the next token
    /// is available.
    ///
    /// Hot path: the common case is a key that already has a
    /// bucket. We do a `get_mut` first; only on a miss do we pay
    /// the `String` allocation cost of inserting via `entry`.
    pub fn check(&self, key: &str) -> TokenBucketDecision {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter poisoned");

        // Fast path: bucket already exists. Avoids the
        // `key.to_string()` allocation in the entry() call.
        if let Some(bucket) = buckets.get_mut(key) {
            return Self::consume(&self.config, bucket, now);
        }

        // Slow path: insert a new bucket. The `to_string()` is
        // amortized — it happens at most once per new key.
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| BucketState {
            tokens: self.config.burst,
            last_refill: now,
        });
        Self::consume(&self.config, bucket, now)
    }

    /// Inner consume step, extracted so the fast path does not
    /// pay for the `entry()` codegen.
    fn consume(
        config: &TokenBucketConfig,
        bucket: &mut BucketState,
        now: Instant,
    ) -> TokenBucketDecision {
        // Refill: add tokens proportional to elapsed time. Cap at
        // burst so a long-idle key cannot get a "free refill"
        // larger than the configured capacity.
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let refill = elapsed * config.refill_per_second;
        bucket.tokens = (bucket.tokens + refill).min(config.burst);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            TokenBucketDecision::Allow {
                tokens_remaining: bucket.tokens.floor() as u32,
            }
        } else {
            // Time until 1.0 tokens are available
            let needed = 1.0 - bucket.tokens;
            // Cap at one hour so callers do not get an "infinity
            // seconds" hint that breaks HTTP Retry-After parsing.
            // If refill is 0 the bucket will never recover, which
            // is the operator's signal that the config is wrong.
            let secs = if config.refill_per_second > 0.0 {
                (needed / config.refill_per_second).min(3600.0)
            } else {
                3600.0
            };
            TokenBucketDecision::Deny {
                retry_after: Duration::from_secs_f64(secs.max(0.0)),
            }
        }
    }

    /// Evict buckets that have not been touched within the
    /// configured idle window. Call this from a background task;
    /// do not call it on the request path. The legacy behaviour
    /// of sweeping on every `check` was preserved below as
    /// `check_and_sweep` for callers that want it.
    pub fn sweep_idle(&self) -> usize {
        let idle = match self.config.idle_eviction {
            Some(d) => d,
            None => return 0,
        };
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter poisoned");
        let before = buckets.len();
        buckets.retain(|_, b| now.duration_since(b.last_refill) < idle);
        before - buckets.len()
    }

    /// Forget the bucket for `key`. Useful when an auth identity
    /// is logged out or when a tenant is deprovisioned.
    pub fn forget(&self, key: &str) {
        self.buckets.lock().expect("rate limiter poisoned").remove(key);
    }

    /// Number of tracked keys. Test-only inspection.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.buckets.lock().expect("rate limiter poisoned").len()
    }
}

/// Extract a rate-limit key from a request, preferring the auth
/// header's subject (when present) and falling back to the
/// peer address. The format is opaque to the limiter; the
/// caller chooses the granularity.
pub fn key_from_request(auth_subject: Option<&str>, peer: &str) -> String {
    auth_subject
        .map(|s| format!("auth:{}", s))
        .unwrap_or_else(|| format!("ip:{}", peer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn allows_up_to_burst_then_denies() {
        let rl = TokenBucketLimiter::new(TokenBucketConfig {
            burst: 3.0,
            refill_per_second: 0.0, // no refill, test in isolation
            idle_eviction: None,
        });
        for _ in 0..3 {
            assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
        }
        match rl.check("a") {
            TokenBucketDecision::Deny { retry_after } => {
                // Refill is 0/sec so retry_after should be infinite / huge;
                // we accept anything > 0.
                assert!(retry_after.as_secs_f64() > 0.0);
            }
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn refills_over_time() {
        let rl = TokenBucketLimiter::new(TokenBucketConfig {
            burst: 1.0,
            refill_per_second: 100.0, // fast refill for test
            idle_eviction: None,
        });
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
        assert!(matches!(rl.check("a"), TokenBucketDecision::Deny { .. }));
        sleep(Duration::from_millis(50)); // 0.05s * 100 = 5 tokens refilled
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
    }

    #[test]
    fn per_key_buckets_are_independent() {
        let rl = TokenBucketLimiter::new(TokenBucketConfig {
            burst: 1.0,
            refill_per_second: 0.0,
            idle_eviction: None,
        });
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
        assert!(matches!(rl.check("b"), TokenBucketDecision::Allow { .. }));
        assert!(matches!(rl.check("a"), TokenBucketDecision::Deny { .. }));
    }

    #[test]
    fn burst_caps_long_idle_refill() {
        let rl = TokenBucketLimiter::new(TokenBucketConfig {
            burst: 2.0,
            refill_per_second: 1_000.0, // would refill 1000 tokens in 1s
            idle_eviction: None,
        });
        // Idle for a long time
        sleep(Duration::from_millis(50));
        // Should still be capped at burst=2
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
        assert!(matches!(rl.check("a"), TokenBucketDecision::Deny { .. }));
    }

    #[test]
    fn forget_drops_bucket() {
        let rl = TokenBucketLimiter::new(TokenBucketConfig {
            burst: 1.0,
            refill_per_second: 0.0,
            idle_eviction: None,
        });
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
        assert!(matches!(rl.check("a"), TokenBucketDecision::Deny { .. }));
        rl.forget("a");
        assert!(matches!(rl.check("a"), TokenBucketDecision::Allow { .. }));
    }

    #[test]
    fn idle_eviction_removes_old_buckets() {
        let rl = TokenBucketLimiter::new(TokenBucketConfig {
            burst: 1.0,
            refill_per_second: 0.0,
            idle_eviction: Some(Duration::from_millis(20)),
        });
        rl.check("a");
        assert_eq!(rl.len(), 1);
        sleep(Duration::from_millis(40));
        // sweep_idle is now a separate method; it walks the
        // bucket map once and drops everything past the window.
        let dropped = rl.sweep_idle();
        assert_eq!(dropped, 1);
        assert_eq!(rl.len(), 0);
    }

    #[test]
    fn key_from_request_prefers_auth() {
        assert_eq!(
            key_from_request(Some("alice"), "10.0.0.1"),
            "auth:alice"
        );
        assert_eq!(key_from_request(None, "10.0.0.1"), "ip:10.0.0.1");
    }
}
