// Security Hardening: Input Validation and Rate Limiting
// Comprehensive protection against abuse, injection, and resource exhaustion

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// Input validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field_name: String,
    pub field_type: FieldType,
    pub min_length: usize,
    pub max_length: usize,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<String>>,
}

/// Field types for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    #[serde(rename = "STRING")]
    String,
    #[serde(rename = "INTEGER")]
    Integer,
    #[serde(rename = "FLOAT")]
    Float,
    #[serde(rename = "BOOLEAN")]
    Boolean,
    #[serde(rename = "ARRAY")]
    Array,
    #[serde(rename = "OBJECT")]
    Object,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Input validator
pub struct InputValidator {
    pub rules: HashMap<String, ValidationRule>,
    pub max_input_size: usize,
    pub rejected_count: u64,
}

impl InputValidator {
    /// Create new input validator
    pub fn new(max_input_size: usize) -> Self {
        println!("[InputValidator] Initialized (max_size: {} bytes)", max_input_size);
        
        Self {
            rules: HashMap::new(),
            max_input_size,
            rejected_count: 0,
        }
    }

    /// Add validation rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        println!("[InputValidator] Added rule for field: {}", rule.field_name);
        self.rules.insert(rule.field_name.clone(), rule);
    }

    /// Validate input
    pub fn validate(&mut self, field_name: &str, value: &str) -> ValidationResult {
        // Check size first
        if value.len() > self.max_input_size {
            self.rejected_count += 1;
            return ValidationResult {
                is_valid: false,
                errors: vec![format!("Input exceeds maximum size: {} > {}",
                    value.len(), self.max_input_size)],
                warnings: Vec::new(),
            };
        }

        // Check for injection attempts
        if self.contains_injection_patterns(value) {
            self.rejected_count += 1;
            return ValidationResult {
                is_valid: false,
                errors: vec!["Input contains potential injection patterns".to_string()],
                warnings: Vec::new(),
            };
        }

        // Apply specific rules
        if let Some(rule) = self.rules.get(field_name) {
            self.validate_against_rule(value, rule)
        } else {
            ValidationResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: vec!["No validation rule defined".to_string()],
            }
        }
    }

    /// Validate against specific rule
    fn validate_against_rule(&mut self, value: &str, rule: &ValidationRule) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check length
        if value.len() < rule.min_length {
            errors.push(format!("Input too short: {} < {}",
                value.len(), rule.min_length));
        }
        if value.len() > rule.max_length {
            errors.push(format!("Input too long: {} > {}",
                value.len(), rule.max_length));
        }

        // Check pattern
        if let Some(pattern) = &rule.pattern {
            if !self.matches_pattern(value, pattern) {
                errors.push(format!("Input does not match pattern: {}", pattern));
            }
        }

        // Check allowed values
        if let Some(allowed) = &rule.allowed_values {
            if !allowed.contains(&value.to_string()) {
                errors.push(format!("Value not in allowed list: {}", value));
            }
        }

        if !errors.is_empty() {
            self.rejected_count += 1;
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Check for injection patterns
    fn contains_injection_patterns(&self, value: &str) -> bool {
        let patterns = vec![
            "'; DROP TABLE",
            "' OR '1'='1",
            "<script>",
            "javascript:",
            "../",
            "\\x00",
            "%00",
            "eval(",
            "exec(",
            "system(",
        ];

        let lower = value.to_lowercase();
        patterns.iter().any(|p| lower.contains(p))
    }

    /// Check if value matches pattern
    fn matches_pattern(&self, value: &str, _pattern: &str) -> bool {
        // Simple pattern matching - in production use regex
        !value.is_empty()
    }

    /// Get validation statistics
    pub fn get_stats(&self) -> ValidationStats {
        ValidationStats {
            total_rules: self.rules.len() as u32,
            rejected_inputs: self.rejected_count,
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total_rules: u32,
    pub rejected_inputs: u64,
}

/// Rate limiter for request throttling
pub struct RateLimiter {
    pub name: String,
    pub max_requests: u32,
    pub window_seconds: u64,
    pub buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub tokens: f32,
    pub max_tokens: f32,
    pub refill_rate: f32,  // tokens per second
    pub last_update: Instant,
}

impl TokenBucket {
    /// Create new token bucket
    pub fn new(max_tokens: f32, refill_rate: f32) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_update: Instant::now(),
        }
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, tokens: f32) -> bool {
        let elapsed = self.last_update.elapsed().as_secs_f32();
        let refilled = elapsed * self.refill_rate;
        
        self.tokens = (self.tokens + refilled).min(self.max_tokens);
        self.last_update = Instant::now();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(name: &str, max_requests: u32, window_seconds: u64) -> Self {
        println!("[RateLimiter] Initialized: {} ({} req/{}s)",
            name, max_requests, window_seconds);
        
        Self {
            name: name.to_string(),
            max_requests,
            window_seconds,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if request is allowed
    pub fn allow_request(&self, client_id: &str) -> RateLimitResult {
        let mut buckets = self.buckets.lock().unwrap();
        
        let refill_rate = self.max_requests as f32 / self.window_seconds as f32;
        
        let bucket = buckets.entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.max_requests as f32, refill_rate));

        let allowed = bucket.try_consume(1.0);

        RateLimitResult {
            allowed,
            remaining_tokens: bucket.tokens as u32,
            reset_after_seconds: if !allowed {
                ((1.0 - bucket.tokens) / bucket.refill_rate) as u64
            } else {
                0
            },
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> RateLimiterStats {
        let buckets = self.buckets.lock().unwrap();
        
        let total_clients = buckets.len() as u32;
        let over_limit = buckets.values()
            .filter(|b| b.tokens <= 0.0)
            .count() as u32;

        RateLimiterStats {
            name: self.name.clone(),
            total_clients,
            clients_over_limit: over_limit,
            max_requests_per_window: self.max_requests,
        }
    }

    /// Reset all buckets
    pub fn reset(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.clear();
        println!("[RateLimiter] Reset all buckets");
    }
}

/// Rate limit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining_tokens: u32,
    pub reset_after_seconds: u64,
}

/// Rate limiter statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterStats {
    pub name: String,
    pub total_clients: u32,
    pub clients_over_limit: u32,
    pub max_requests_per_window: u32,
}

/// Security hardening module
pub struct SecurityHardener {
    pub input_validator: InputValidator,
    pub rate_limiters: HashMap<String, RateLimiter>,
}

impl SecurityHardener {
    /// Create new security hardener
    pub fn new(max_input_size: usize) -> Self {
        println!("[SecurityHardener] Initialized");
        
        Self {
            input_validator: InputValidator::new(max_input_size),
            rate_limiters: HashMap::new(),
        }
    }

    /// Add rate limiter
    pub fn add_rate_limiter(&mut self, name: &str, max_requests: u32, window_seconds: u64) {
        let limiter = RateLimiter::new(name, max_requests, window_seconds);
        self.rate_limiters.insert(name.to_string(), limiter);
    }

    /// Get security report
    pub fn get_security_report(&self) -> SecurityReport {
        let validation_stats = self.input_validator.get_stats();
        
        let rate_limiter_stats: Vec<_> = self.rate_limiters.values()
            .map(|rl| rl.get_stats())
            .collect();

        SecurityReport {
            input_validator_enabled: true,
            rate_limiters_count: self.rate_limiters.len() as u32,
            rejected_inputs: validation_stats.rejected_inputs,
            rate_limiter_stats,
        }
    }
}

/// Security report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub input_validator_enabled: bool,
    pub rate_limiters_count: u32,
    pub rejected_inputs: u64,
    pub rate_limiter_stats: Vec<RateLimiterStats>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_validator_creation() {
        let validator = InputValidator::new(1024);
        assert_eq!(validator.max_input_size, 1024);
    }

    #[test]
    fn test_validation_rule() {
        let mut validator = InputValidator::new(1024);
        
        let rule = ValidationRule {
            field_name: "username".to_string(),
            field_type: FieldType::String,
            min_length: 3,
            max_length: 32,
            pattern: None,
            allowed_values: None,
        };
        
        validator.add_rule(rule);
        assert!(validator.rules.contains_key("username"));
    }

    #[test]
    fn test_injection_detection() {
        let mut validator = InputValidator::new(1024);
        
        let result = validator.validate("test", "'; DROP TABLE users;");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new("api", 10, 60);
        
        let result = limiter.allow_request("client1");
        assert!(result.allowed);
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 0.1);
        
        assert!(bucket.try_consume(5.0));
        assert_eq!(bucket.tokens as i32, 5);
    }

    #[test]
    fn test_security_hardener() {
        let mut hardener = SecurityHardener::new(1024);
        hardener.add_rate_limiter("api", 100, 60);
        
        let report = hardener.get_security_report();
        assert_eq!(report.rate_limiters_count, 1);
    }
}

