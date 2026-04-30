// LLM Rate Limiter
// Manages API rate limiting and cost tracking

use governor::Quota;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{warn, debug};
use anyhow::{anyhow, Result};

pub struct RateLimiter {
    // Simple token bucket counter
    tokens_used: AtomicU64,
    calls_made: AtomicU64,
    total_cost: parking_lot::Mutex<f64>,
    max_calls_per_hour: u32,
}

impl RateLimiter {
    /// Create rate limiter with max calls per hour
    pub fn new(max_per_hour: u32) -> Self {
        Self {
            tokens_used: AtomicU64::new(0),
            calls_made: AtomicU64::new(0),
            total_cost: parking_lot::Mutex::new(0.0),
            max_calls_per_hour: max_per_hour,
        }
    }

    /// Check if rate limit allows next call (always allows for local GGUF)
    pub async fn check_limit(&self) -> Result<()> {
        // Local GGUF has no rate limits
        Ok(())
    }

    /// Record API call with token count
    pub fn record_call(&self, tokens: u64, cost: f64) {
        self.tokens_used.fetch_add(tokens, Ordering::SeqCst);
        self.calls_made.fetch_add(1, Ordering::SeqCst);

        let mut total = self.total_cost.lock();
        *total += cost;

        debug!(
            "Recorded LLM call: {} tokens, ${:.4} (total: ${:.2})",
            tokens, cost, *total
        );
    }

    /// Get current cost info
    pub fn get_cost_info(&self) -> super::CostInfo {
        super::CostInfo {
            tokens_used: self.tokens_used.load(Ordering::SeqCst),
            total_cost: *self.total_cost.lock(),
            calls_made: self.calls_made.load(Ordering::SeqCst),
        }
    }

    /// Reset counters (for testing)
    pub fn reset(&self) {
        self.tokens_used.store(0, Ordering::SeqCst);
        self.calls_made.store(0, Ordering::SeqCst);
        *self.total_cost.lock() = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_calls() {
        let limiter = RateLimiter::new(10);
        
        // Should allow first call
        assert!(limiter.check_limit().await.is_ok());
        limiter.record_call(100, 0.001);
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
        let info = limiter.get_cost_info();
        assert_eq!(info.calls_made, 1);
        
        limiter.reset();
        let info = limiter.get_cost_info();
        assert_eq!(info.calls_made, 0);
        assert_eq!(info.tokens_used, 0);
    }
}
