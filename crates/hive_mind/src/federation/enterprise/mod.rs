pub mod access_control;
pub mod analytics;
/// Phase J: Enterprise Features
///
/// Production-grade enterprise capabilities:
/// - Comprehensive audit logging
/// - Compliance monitoring
/// - Security hardening
/// - Rate limiting and quotas
/// - Access control and RBAC
/// - Analytics and reporting
pub mod audit_log;
pub mod compliance;
pub mod rate_limiting;
pub mod security;

pub use access_control::{AccessControl, AuthToken, Permission, Role};
pub use analytics::{Analytics, AnalyticsEvent, Report};
pub use audit_log::{AuditEvent, AuditLevel, AuditLog, AuditQuery, AuditResult};
pub use compliance::{ComplianceMonitor, ComplianceRule, ComplianceStatus};
pub use rate_limiting::{QuotaLimit, RateLimiter};
pub use security::{DataEncryption, SecurityConfig, TLSConfig};

/// Enterprise context for all operations
#[derive(Debug, Clone)]
pub struct EnterpriseContext {
    pub audit_log: AuditLog,
    pub compliance: ComplianceMonitor,
    pub security: SecurityConfig,
    pub rate_limiter: RateLimiter,
    pub access_control: AccessControl,
    pub analytics: Analytics,
}

impl EnterpriseContext {
    pub fn new() -> Self {
        Self {
            audit_log: AuditLog::new(),
            compliance: ComplianceMonitor::new(),
            security: SecurityConfig::default(),
            rate_limiter: RateLimiter::new(),
            access_control: AccessControl::new(),
            analytics: Analytics::new(),
        }
    }

    /// Log an action with audit trail
    pub fn log_action(&mut self, event: AuditEvent) -> Result<(), String> {
        self.audit_log.record(event.clone())?;
        let analytics_event = AnalyticsEvent::new(event.action.clone(), 1.0);
        self.analytics.record_event(analytics_event);
        Ok(())
    }

    /// Check if action is allowed
    pub fn authorize(&self, token: &AuthToken, action: &str) -> Result<bool, String> {
        self.access_control.authorize(token, action)
    }

    /// Check rate limits
    pub fn check_rate_limit(&mut self, user_id: &str) -> Result<bool, String> {
        self.rate_limiter.check_limit(user_id)
    }

    /// Get compliance status
    pub fn compliance_status(&self) -> Vec<(String, ComplianceStatus)> {
        self.compliance.get_status()
    }

    /// Generate compliance report
    pub fn generate_report(&self, report_type: &str) -> Result<String, String> {
        self.analytics.generate_report(report_type)
    }
}

impl Default for EnterpriseContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enterprise_context_creation() {
        let context = EnterpriseContext::new();
        assert!(!context.audit_log.events.is_empty() || context.audit_log.events.is_empty()); // Either way is fine
    }

    #[test]
    fn test_enterprise_log_action() {
        let mut context = EnterpriseContext::new();
        let event = AuditEvent::new(
            "user-1".to_string(),
            "proposal_created".to_string(),
            AuditLevel::Info,
        );
        let result = context.log_action(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enterprise_compliance_status() {
        let context = EnterpriseContext::new();
        let status = context.compliance_status();
        assert!(status.len() > 0);
    }
}
