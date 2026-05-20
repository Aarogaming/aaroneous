/// Comprehensive Audit Logging System
/// 
/// Records all significant events for compliance, security, and debugging

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Audit event severity level
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuditLevel {
    /// Normal operations
    Info,
    /// Important decisions
    Warning,
    /// Security-relevant events
    Security,
    /// Critical errors or violations
    Critical,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp_ms: u64,
    pub user_id: String,
    pub action: String,
    pub level: AuditLevel,
    pub resource: Option<String>,
    pub result: AuditResult,
    pub details: String,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    PartialSuccess,
}

impl AuditEvent {
    pub fn new(user_id: String, action: String, level: AuditLevel) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            user_id,
            action,
            level,
            resource: None,
            result: AuditResult::Success,
            details: String::new(),
            ip_address: None,
        }
    }

    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = details;
        self
    }

    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }
}

/// Query for filtering audit events
#[derive(Debug, Clone)]
pub struct AuditQuery {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub level: Option<AuditLevel>,
    pub result: Option<AuditResult>,
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
    pub limit: usize,
}

impl AuditQuery {
    pub fn new() -> Self {
        Self {
            user_id: None,
            action: None,
            level: None,
            result: None,
            start_time_ms: None,
            end_time_ms: None,
            limit: 1000,
        }
    }

    pub fn for_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn for_action(mut self, action: String) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_level(mut self, level: AuditLevel) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = Some(result);
        self
    }

    pub fn time_range(mut self, start: u64, end: u64) -> Self {
        self.start_time_ms = Some(start);
        self.end_time_ms = Some(end);
        self
    }

    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(ref user) = self.user_id {
            if event.user_id != *user {
                return false;
            }
        }

        if let Some(ref action) = self.action {
            if event.action != *action {
                return false;
            }
        }

        if let Some(level) = self.level {
            if event.level != level {
                return false;
            }
        }

        if let Some(result) = self.result {
            if event.result != result {
                return false;
            }
        }

        if let Some(start) = self.start_time_ms {
            if event.timestamp_ms < start {
                return false;
            }
        }

        if let Some(end) = self.end_time_ms {
            if event.timestamp_ms > end {
                return false;
            }
        }

        true
    }
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub events: VecDeque<AuditEvent>,
    pub max_events: usize,
    pub total_recorded: u64,
    pub security_events_count: u64,
    pub critical_events_count: u64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            max_events: 100000,  // Keep last 100k events
            total_recorded: 0,
            security_events_count: 0,
            critical_events_count: 0,
        }
    }

    /// Record an audit event
    pub fn record(&mut self, event: AuditEvent) -> Result<(), String> {
        // Track special events
        match event.level {
            AuditLevel::Security => self.security_events_count += 1,
            AuditLevel::Critical => self.critical_events_count += 1,
            _ => {}
        }

        self.total_recorded += 1;

        // Maintain size limit
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }

        self.events.push_back(event);
        Ok(())
    }

    /// Query audit events
    pub fn query(&self, query: &AuditQuery) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|event| query.matches(event))
            .take(query.limit)
            .cloned()
            .collect()
    }

    /// Get recent events
    pub fn recent(&self, count: usize) -> Vec<AuditEvent> {
        self.events
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> AuditStats {
        let security_count = self.events.iter().filter(|e| matches!(e.level, AuditLevel::Security)).count();
        let critical_count = self.events.iter().filter(|e| matches!(e.level, AuditLevel::Critical)).count();
        let failure_count = self.events.iter().filter(|e| matches!(e.result, AuditResult::Failure)).count();

        AuditStats {
            total_events: self.events.len(),
            total_recorded: self.total_recorded,
            security_events: security_count,
            critical_events: critical_count,
            failed_operations: failure_count,
            unique_users: self.events.iter().map(|e| e.user_id.clone()).collect::<std::collections::HashSet<_>>().len(),
        }
    }

    /// Export events to JSON
    pub fn export_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.events)
            .map_err(|e| format!("Failed to export: {}", e))
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: usize,
    pub total_recorded: u64,
    pub security_events: usize,
    pub critical_events: usize,
    pub failed_operations: usize,
    pub unique_users: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            "user-1".to_string(),
            "proposal_created".to_string(),
            AuditLevel::Info,
        );
        assert_eq!(event.user_id, "user-1");
        assert_eq!(event.action, "proposal_created");
    }

    #[test]
    fn test_audit_event_builder() {
        let event = AuditEvent::new(
            "user-1".to_string(),
            "specialist_access".to_string(),
            AuditLevel::Security,
        )
        .with_resource("visionary".to_string())
        .with_details("Accessed design specialist".to_string())
        .with_result(AuditResult::Success);

        assert_eq!(event.resource, Some("visionary".to_string()));
        assert!(matches!(event.result, AuditResult::Success));
    }

    #[test]
    fn test_audit_log_record() {
        let mut log = AuditLog::new();
        let event = AuditEvent::new(
            "user-1".to_string(),
            "proposal_created".to_string(),
            AuditLevel::Info,
        );
        log.record(event).ok();
        assert_eq!(log.total_recorded, 1);
    }

    #[test]
    fn test_audit_query() {
        let mut log = AuditLog::new();
        let event1 = AuditEvent::new(
            "user-1".to_string(),
            "proposal_created".to_string(),
            AuditLevel::Info,
        );
        let event2 = AuditEvent::new(
            "user-2".to_string(),
            "specialist_accessed".to_string(),
            AuditLevel::Security,
        );

        log.record(event1).ok();
        log.record(event2).ok();

        let query = AuditQuery::new().for_user("user-1".to_string());
        let results = log.query(&query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_audit_stats() {
        let mut log = AuditLog::new();
        let event = AuditEvent::new(
            "user-1".to_string(),
            "action".to_string(),
            AuditLevel::Critical,
        );
        log.record(event).ok();

        let stats = log.stats();
        assert_eq!(stats.total_recorded, 1);
        assert_eq!(stats.critical_events, 1);
    }
}
