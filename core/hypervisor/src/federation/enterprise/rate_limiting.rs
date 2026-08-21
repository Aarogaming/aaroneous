/// Rate Limiting and Quota Management
///
/// Prevent abuse through rate limiting and quota enforcement
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaLimit {
    pub resource: String,
    pub limit: u32,
    pub window_seconds: u32,
    pub hard_limit: bool,
}

impl QuotaLimit {
    pub fn new(resource: String, limit: u32) -> Self {
        Self {
            resource,
            limit,
            window_seconds: 3600, // 1 hour
            hard_limit: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    pub limits: HashMap<String, QuotaLimit>,
    pub usage: HashMap<String, Vec<u64>>, // user_id -> timestamps
    pub blocked_users: HashMap<String, u64>, // user_id -> unblock_time
}

impl RateLimiter {
    pub fn new() -> Self {
        let mut limiter = Self {
            limits: HashMap::new(),
            usage: HashMap::new(),
            blocked_users: HashMap::new(),
        };

        // Add default limits
        limiter.add_limit(QuotaLimit::new("proposals_per_hour".to_string(), 1000));
        limiter.add_limit(QuotaLimit::new("api_calls_per_minute".to_string(), 100));
        limiter.add_limit(QuotaLimit::new("model_loads_per_hour".to_string(), 100));
        limiter
    }

    pub fn add_limit(&mut self, limit: QuotaLimit) {
        self.limits.insert(limit.resource.clone(), limit);
    }

    /// Check if user can proceed
    pub fn check_limit(&mut self, user_id: &str) -> Result<bool, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if user is blocked
        if let Some(unblock_time) = self.blocked_users.get(user_id) {
            if now < *unblock_time {
                return Err(format!("User blocked until {}", unblock_time));
            } else {
                self.blocked_users.remove(user_id);
            }
        }

        // Check quotas
        let usage = self.usage.entry(user_id.to_string()).or_default();

        // Clean old entries
        usage.retain(|&timestamp| now - timestamp < 3600);

        // Check limits
        if usage.len() > 1000 {
            // Default limit
            // Block user
            let block_until = now + (30 * 60); // 30 minutes
            self.blocked_users.insert(user_id.to_string(), block_until);
            return Err("Rate limit exceeded".to_string());
        }

        // Record usage
        usage.push(now);
        Ok(true)
    }

    /// Get usage for user
    pub fn get_usage(&self, user_id: &str) -> usize {
        self.usage.get(user_id).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_limit_creation() {
        let limit = QuotaLimit::new("test".to_string(), 100);
        assert_eq!(limit.limit, 100);
    }

    #[test]
    fn test_rate_limiter_check() {
        let mut limiter = RateLimiter::new();
        let result = limiter.check_limit("user-1");
        assert!(result.is_ok());
    }
}
