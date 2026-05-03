// LLM Rate Limiter
// Sliding-window rate enforcement + cost tracking for LLM calls.
//
// Design:
// - For local GGUF (no API cost): default limit is very high (10,000/hr)
//   so inference is effectively unlimited unless you explicitly configure it
// - For cloud providers (future): set a lower limit matching API tier
// - The sliding window is 1 hour; calls are counted per-hour atomically
// - When limit is hit, check_limit() returns Err immediately (no blocking)
//
// Cost tracking is always active regardless of rate limit state.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use tracing::{warn, debug};
use anyhow::{anyhow, Result};

pub struct RateLimiter {
    /// Maximum calls allowed per hour (0 = unlimited)
    max_calls_per_hour: u32,
    /// Sliding window: calls in the current hour
    window_calls: AtomicU64,
    /// When the current 1-hour window started
    window_start: Mutex<Instant>,
    /// Total calls across all windows (for stats)
    total_calls: AtomicU64,
    /// Total tokens used
    tokens_used: AtomicU64,
    /// Total cost (for cloud providers)
    total_cost: Mutex<f64>,
    /// Calls rejected by rate limiting
    rejected_calls: AtomicU64,
}

impl RateLimiter {
    /// Create a rate limiter.
    ///
    /// `max_per_hour = 0` means unlimited (appropriate for local GGUF).
    /// Set a value for cloud API providers to enforce their rate limits.
    pub fn new(max_per_hour: u32) -> Self {
        Self {
            max_calls_per_hour: max_per_hour,
            window_calls: AtomicU64::new(0),
            window_start: Mutex::new(Instant::now()),
            total_calls: AtomicU64::new(0),
            tokens_used: AtomicU64::new(0),
            total_cost: Mutex::new(0.0),
            rejected_calls: AtomicU64::new(0),
        }
    }

    /// Check if a call is allowed under the current rate limit.
    ///
    /// - Returns `Ok(())` if allowed (or if limit is 0 = unlimited)
    /// - Returns `Err` if the per-hour limit has been exceeded
    ///
    /// The sliding window resets every hour. When a window expires, the
    /// counter resets and the call is allowed.
    pub async fn check_limit(&self) -> Result<()> {
        // 0 = unlimited (default for local GGUF)
        if self.max_calls_per_hour == 0 {
            return Ok(());
        }

        // Slide the window if an hour has elapsed
        {
            let mut start = self.window_start.lock();
            if start.elapsed() >= Duration::from_secs(3600) {
                let prev = self.window_calls.swap(0, Ordering::SeqCst);
                *start = Instant::now();
                debug!("Rate limiter: window reset (had {} calls)", prev);
            }
        }

        let current = self.window_calls.load(Ordering::SeqCst);
        if current >= self.max_calls_per_hour as u64 {
            self.rejected_calls.fetch_add(1, Ordering::Relaxed);
            let window_start = *self.window_start.lock();
            let elapsed = window_start.elapsed();
            let remaining_secs = 3600u64.saturating_sub(elapsed.as_secs());
            warn!(
                "Rate limit hit: {} calls in current window (max {}). \
                 Window resets in {}s.",
                current, self.max_calls_per_hour, remaining_secs
            );
            return Err(anyhow!(
                "Rate limit exceeded: {} calls/hr (max {}). \
                 Retry in {}s or increase max_calls_per_hour.",
                current, self.max_calls_per_hour, remaining_secs
            ));
        }

        // Increment AFTER the check (optimistic — slightly over-counts under contention)
        self.window_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Record a completed API call with token usage and cost.
    pub fn record_call(&self, tokens: u64, cost: f64) {
        self.tokens_used.fetch_add(tokens, Ordering::SeqCst);
        self.total_calls.fetch_add(1, Ordering::SeqCst);

        let mut total = self.total_cost.lock();
        *total += cost;

        debug!(
            "Recorded LLM call: {} tokens, ${:.4} (total: ${:.2})",
            tokens, cost, *total
        );
    }

    /// Get current rate limit and cost statistics.
    pub fn get_cost_info(&self) -> super::CostInfo {
        super::CostInfo {
            tokens_used: self.tokens_used.load(Ordering::SeqCst),
            total_cost: *self.total_cost.lock(),
            calls_made: self.total_calls.load(Ordering::SeqCst),
        }
    }

    /// Current calls in the active window and window progress.
    pub fn window_status(&self) -> (u64, u32, u64) {
        let current = self.window_calls.load(Ordering::SeqCst);
        let remaining = {
            let start = self.window_start.lock();
            3600u64.saturating_sub(start.elapsed().as_secs())
        };
        (current, self.max_calls_per_hour, remaining)
    }

    /// Calls rejected by rate limiting.
    pub fn rejected_count(&self) -> u64 {
        self.rejected_calls.load(Ordering::Relaxed)
    }

    /// Reset all counters (for testing / manual override).
    pub fn reset(&self) {
        self.window_calls.store(0, Ordering::SeqCst);
        self.total_calls.store(0, Ordering::SeqCst);
        self.tokens_used.store(0, Ordering::SeqCst);
        self.rejected_calls.store(0, Ordering::Relaxed);
        *self.total_cost.lock() = 0.0;
        *self.window_start.lock() = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_calls() {
        let limiter = RateLimiter::new(10);
        assert!(limiter.check_limit().await.is_ok());
        limiter.record_call(100, 0.001);
    }

    #[tokio::test]
    async fn test_rate_limit_enforced() {
        let limiter = RateLimiter::new(3);
        // First 3 calls allowed
        assert!(limiter.check_limit().await.is_ok());
        assert!(limiter.check_limit().await.is_ok());
        assert!(limiter.check_limit().await.is_ok());
        // 4th call rejected
        assert!(limiter.check_limit().await.is_err());
        assert_eq!(limiter.rejected_count(), 1);
    }

    #[tokio::test]
    async fn test_unlimited_mode() {
        let limiter = RateLimiter::new(0);  // unlimited
        for _ in 0..1000 {
            assert!(limiter.check_limit().await.is_ok());
        }
    }

    #[test]
    fn test_cost_tracking() {
        let limiter = RateLimiter::new(100);
        limiter.record_call(100, 0.001);
        limiter.record_call(150, 0.0015);
        let info = limiter.get_cost_info();
        assert_eq!(info.tokens_used, 250);
        assert_eq!(info.calls_made, 2);
        assert!((info.total_cost - 0.0025).abs() < 0.0001);
    }

    #[test]
    fn test_reset() {
        let limiter = RateLimiter::new(100);
        limiter.record_call(100, 0.001);
        limiter.reset();
        let info = limiter.get_cost_info();
        assert_eq!(info.calls_made, 0);
    }

    #[test]
    fn test_window_status() {
        let limiter = RateLimiter::new(100);
        let (current, max, remaining) = limiter.window_status();
        assert_eq!(current, 0);
        assert_eq!(max, 100);
        assert!(remaining > 3590);
    }
}
