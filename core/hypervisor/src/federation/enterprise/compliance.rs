/// Compliance Monitoring Framework
///
/// Tracks compliance with regulations and internal policies
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    Warning,
    Violated,
    Unknown,
}

/// Compliance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub category: String, // GDPR, HIPAA, SOC2, etc.
    pub check_interval_ms: u64,
    pub enabled: bool,
}

impl ComplianceRule {
    pub fn new(rule_id: String, name: String, category: String) -> Self {
        Self {
            rule_id,
            name,
            description: String::new(),
            category,
            check_interval_ms: 3600000, // 1 hour
            enabled: true,
        }
    }
}

/// Compliance violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub violation_id: String,
    pub rule_id: String,
    pub timestamp_ms: u64,
    pub severity: u32,
    pub description: String,
    pub remediation: Option<String>,
}

/// Compliance monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMonitor {
    pub rules: HashMap<String, ComplianceRule>,
    pub violations: Vec<ComplianceViolation>,
    pub status_map: HashMap<String, ComplianceStatus>,
    pub last_check_ms: u64,
}

impl ComplianceMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            rules: HashMap::new(),
            violations: Vec::new(),
            status_map: HashMap::new(),
            last_check_ms: 0,
        };

        // Add default compliance rules
        monitor.add_default_rules();
        monitor
    }

    /// Add default compliance rules
    fn add_default_rules(&mut self) {
        let rules = vec![
            ("gdpr_retention", "Data retention limits", "GDPR"),
            ("hipaa_audit", "Audit logging enabled", "HIPAA"),
            ("soc2_encryption", "Data encryption", "SOC2"),
            ("access_control", "Access control in place", "General"),
            ("rate_limiting", "Rate limiting enforced", "General"),
        ];

        for (id, name, category) in rules {
            let rule = ComplianceRule::new(id.to_string(), name.to_string(), category.to_string());
            self.rules.insert(id.to_string(), rule);
            self.status_map
                .insert(id.to_string(), ComplianceStatus::Unknown);
        }
    }

    /// Add custom rule
    pub fn add_rule(&mut self, rule: ComplianceRule) {
        let rule_id = rule.rule_id.clone();
        self.rules.insert(rule_id.clone(), rule);
        self.status_map.insert(rule_id, ComplianceStatus::Unknown);
    }

    /// Record a violation
    pub fn record_violation(&mut self, rule_id: String, description: String) {
        let violation = ComplianceViolation {
            violation_id: uuid::Uuid::new_v4().to_string(),
            rule_id: rule_id.clone(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            severity: 5,
            description,
            remediation: None,
        };

        self.violations.push(violation);
        self.status_map.insert(rule_id, ComplianceStatus::Violated);
    }

    /// Update rule status
    pub fn update_status(&mut self, rule_id: String, status: ComplianceStatus) {
        self.status_map.insert(rule_id, status);
    }

    /// Get compliance status
    pub fn get_status(&self) -> Vec<(String, ComplianceStatus)> {
        self.status_map
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Check overall compliance
    pub fn is_compliant(&self) -> bool {
        self.status_map
            .values()
            .all(|status| !matches!(status, ComplianceStatus::Violated))
    }

    /// Get compliance report
    pub fn generate_report(&self) -> ComplianceReport {
        let total_rules = self.rules.len();
        let compliant = self
            .status_map
            .values()
            .filter(|s| matches!(s, ComplianceStatus::Compliant))
            .count();
        let warnings = self
            .status_map
            .values()
            .filter(|s| matches!(s, ComplianceStatus::Warning))
            .count();
        let violations = self
            .status_map
            .values()
            .filter(|s| matches!(s, ComplianceStatus::Violated))
            .count();

        ComplianceReport {
            total_rules,
            compliant_rules: compliant,
            warning_rules: warnings,
            violated_rules: violations,
            overall_compliant: self.is_compliant(),
            recent_violations: self.violations.iter().rev().take(10).cloned().collect(),
        }
    }
}

impl Default for ComplianceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub total_rules: usize,
    pub compliant_rules: usize,
    pub warning_rules: usize,
    pub violated_rules: usize,
    pub overall_compliant: bool,
    pub recent_violations: Vec<ComplianceViolation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_rule_creation() {
        let rule = ComplianceRule::new(
            "rule1".to_string(),
            "Test Rule".to_string(),
            "GDPR".to_string(),
        );
        assert_eq!(rule.name, "Test Rule");
        assert!(rule.enabled);
    }

    #[test]
    fn test_compliance_monitor_creation() {
        let monitor = ComplianceMonitor::new();
        assert!(!monitor.rules.is_empty());
    }

    #[test]
    fn test_compliance_monitor_violation() {
        let mut monitor = ComplianceMonitor::new();
        monitor.record_violation("gdpr_retention".to_string(), "Too many days".to_string());
        assert!(!monitor.is_compliant());
    }

    #[test]
    fn test_compliance_report() {
        let monitor = ComplianceMonitor::new();
        let report = monitor.generate_report();
        assert!(report.total_rules > 0);
    }
}
