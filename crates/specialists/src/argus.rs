//! argus.rs
//! Argus (The All-Seeing Guardian) & Sentinel (Cryptographic Vault & Safety Gatekeeper).
//! Domain Opcode: 0x0500 (SECURITY_GOVERNANCE)

use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// Security audit verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    pub target: String,
    pub is_safe: bool,
    pub violations_detected: Vec<String>,
    pub rate_limit_passed: bool,
}

/// Sentinel Relic Engine: Cryptographic secrets vault & boundary firewall
pub struct SentinelRelic {
    pub threats_blocked: usize,
    pub audits_performed: usize,
    pub sentinel_engine: compute::ArgusSafetySentinel,
}

impl Default for SentinelRelic {
    fn default() -> Self {
        Self {
            threats_blocked: 0,
            audits_performed: 0,
            sentinel_engine: compute::ArgusSafetySentinel::new(),
        }
    }
}

impl RelicEngine for SentinelRelic {
    fn relic_name(&self) -> &'static str {
        "Sentinel"
    }

    fn supervisor_name(&self) -> &'static str {
        "Argus"
    }

    fn relic_status(&self) -> String {
        format!(
            "Sentinel Firewall: {} audits completed, {} malicious/unsafe operations blocked",
            self.audits_performed, self.threats_blocked
        )
    }
}

/// Argus Sovereign Specialist
pub struct ArgusSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub sentinel: SentinelRelic,
}

impl Default for ArgusSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl ArgusSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            sentinel: SentinelRelic::default(),
        }
    }

    /// Audits a proposed operation or payload for host safety
    pub fn audit_operation(&mut self, operation_name: &str, payload_bytes: &[u8]) -> Result<SecurityAuditReport> {
        self.sentinel.audits_performed += 1;
        info!(target: "specialist::argus", %operation_name, "Auditing operation for host safety compliance");

        let mut violations = Vec::new();
        let payload_str = String::from_utf8_lossy(payload_bytes);

        // Check for unconstrained OS hijacking patterns
        if payload_str.contains("SendInput") && !payload_str.contains("AARONEOUS_ALLOW_HOST_INPUT") {
            violations.push("Unguarded host OS SendInput call detected".to_string());
        }
        if payload_str.contains("format C:") || payload_str.contains("rm -rf /") {
            violations.push("Destructive filesystem operation detected".to_string());
        }

        let is_safe = violations.is_empty();
        if !is_safe {
            self.sentinel.threats_blocked += 1;
            warn!(target: "specialist::argus", %operation_name, count = violations.len(), "Security violations detected by Sentinel");
        }

        Ok(SecurityAuditReport {
            target: operation_name.to_string(),
            is_safe,
            violations_detected: violations,
            rate_limit_passed: true,
        })
    }

    /// Audits a 256-dim continuous latent state tensor using the Deep SVDD safe hypersphere manifold
    pub fn audit_latent_state(&mut self, state: &[f32; 256]) -> compute::LatentAuditVerdict {
        self.sentinel.audits_performed += 1;
        let verdict = self.sentinel.sentinel_engine.audit_synapse_tensor(state);
        if !verdict.is_safe {
            self.sentinel.threats_blocked += 1;
        }
        verdict
    }
}

#[async_trait]
impl SovereignSpecialist for ArgusSpecialist {
    fn name(&self) -> &'static str {
        "Argus"
    }

    fn domain_opcode(&self) -> u16 {
        0x0500
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let report = self.audit_operation(&packet.source, &packet.payload)?;
        let payload = serde_json::to_vec(&report)?;

        if !report.is_safe {
            bail!("Argus rejected packet due to safety violations: {:?}", report.violations_detected);
        }

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: "Argus verified operation as safe and compliant".to_string(),
            payload,
        })
    }

    fn recharge_metabolism(&mut self, tokens: f32) {
        self.tokens = (self.tokens + tokens).min(self.max_tokens);
    }

    fn health_report(&self) -> SpecialistHealth {
        SpecialistHealth {
            name: self.name().to_string(),
            domain_opcode: self.domain_opcode(),
            tokens: self.tokens,
            max_tokens: self.max_tokens,
            backlog_count: 0,
            is_dormant: self.tokens < 1.0,
            last_active: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argus_safety_audit() {
        let mut argus = ArgusSpecialist::new();
        let safe_report = argus.audit_operation("compute", b"let x = 1 + 2;").unwrap();
        assert!(safe_report.is_safe);

        let unsafe_report = argus.audit_operation("hid", b"SendInput(&input);").unwrap();
        assert!(!unsafe_report.is_safe);
        assert_eq!(argus.sentinel.threats_blocked, 1);
    }

    #[test]
    fn test_argus_latent_manifold_guardrail() {
        let mut argus = ArgusSpecialist::new();
        let safe_tensor = [0.1f32; 256];
        let verdict = argus.audit_latent_state(&safe_tensor);
        assert!(verdict.is_safe);

        let rogue_tensor = [50.0f32; 256];
        let rogue_verdict = argus.audit_latent_state(&rogue_tensor);
        assert!(!rogue_verdict.is_safe);
        assert!(rogue_verdict.was_projected);
        assert!(argus.sentinel.threats_blocked >= 1);
    }
}
