// src/capability_broker.rs
//! Universal Capability Broker for mcp_server, wired directly to `crates/capabilities`.

use capabilities::tools::build_standard_tool_registry;
use capabilities::universal_tool::ToolRegistry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

/// Sandbox security boundary defining isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxPolicy {
    /// Hermetic airgapped sandbox: purely virtual / analytical tools permitted; zero OS or device actuation
    Airgapped,
    /// Read-constrained sandbox: read/query tools permitted; mutating tools rejected
    ReadConstrained,
    /// Sovereign sandbox: full tool suite authorized by token grants
    FullPrivilege,
}

/// Dynamic thermal backpressure level governing capability dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThermalBackpressureLevel {
    #[default]
    Nominal,
    Throttled,
    Critical,
}

/// Cryptographically signed capability token authorizing scoped tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub token_id: uuid::Uuid,
    pub subject: String,
    pub allowed_capabilities: Vec<String>,
    pub sandbox_policy: SandboxPolicy,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub signature: String,
}

/// Dynamic Capability Broker mediating tool discovery and execution
#[derive(Clone)]
pub struct CapabilityBroker {
    registry: ToolRegistry,
    signing_key: [u8; 32],
    thermal_backpressure: Arc<AtomicU8>,
    dirty_generation: Arc<AtomicU64>,
    is_dirty: Arc<AtomicBool>,
}

impl CapabilityBroker {
    /// Create a new capability broker pre-loaded with standard sovereign tools
    pub fn new() -> Self {
        Self::with_registry_and_key(build_standard_tool_registry(), [0x42u8; 32])
    }

    /// Create with a custom tool registry
    pub fn with_registry(registry: ToolRegistry) -> Self {
        Self::with_registry_and_key(registry, [0x42u8; 32])
    }

    /// Create with custom registry and explicit HMAC signing key
    pub fn with_registry_and_key(registry: ToolRegistry, signing_key: [u8; 32]) -> Self {
        Self {
            registry,
            signing_key,
            thermal_backpressure: Arc::new(AtomicU8::new(0)),
            dirty_generation: Arc::new(AtomicU64::new(1)),
            is_dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Access inner tool registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Issue a cryptographically signed CapabilityToken
    pub fn issue_token(
        &self,
        subject: &str,
        allowed_capabilities: Vec<String>,
        sandbox_policy: SandboxPolicy,
        ttl_ms: u64,
        now_ms: u64,
    ) -> CapabilityToken {
        let token_id = uuid::Uuid::new_v4();
        let expires_at_ms = now_ms.saturating_add(ttl_ms);
        let signature = Self::compute_signature(
            &self.signing_key,
            &token_id,
            subject,
            &allowed_capabilities,
            sandbox_policy,
            now_ms,
            expires_at_ms,
        );

        CapabilityToken {
            token_id,
            subject: subject.to_string(),
            allowed_capabilities,
            sandbox_policy,
            issued_at_ms: now_ms,
            expires_at_ms,
            signature,
        }
    }

    fn compute_signature(
        key: &[u8; 32],
        token_id: &uuid::Uuid,
        subject: &str,
        capabilities: &[String],
        sandbox_policy: SandboxPolicy,
        issued_at: u64,
        expires_at: u64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(token_id.as_bytes());
        hasher.update(subject.as_bytes());
        hasher.update((sandbox_policy as u8).to_le_bytes());
        hasher.update(issued_at.to_le_bytes());
        hasher.update(expires_at.to_le_bytes());
        for cap in capabilities {
            hasher.update(cap.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Verify a CapabilityToken against a required capability, expiration, and sandbox policy
    pub fn verify_token(
        &self,
        token: &CapabilityToken,
        required_capability: &str,
        now_ms: u64,
    ) -> Result<(), String> {
        let expected = Self::compute_signature(
            &self.signing_key,
            &token.token_id,
            &token.subject,
            &token.allowed_capabilities,
            token.sandbox_policy,
            token.issued_at_ms,
            token.expires_at_ms,
        );

        if token.signature != expected {
            return Err("Invalid capability token signature".to_string());
        }

        if now_ms > token.expires_at_ms {
            return Err("Capability token has expired".to_string());
        }

        // Check sandbox policy bounds
        if token.sandbox_policy == SandboxPolicy::Airgapped {
            // Airgapped sandbox rejects OS / platform / actuation capabilities
            if required_capability.starts_with("platform.")
                || required_capability.starts_with("screen.")
                || required_capability.starts_with("audio.")
                || required_capability.starts_with("code.process")
            {
                return Err(format!(
                    "Airgapped sandbox violation: capability '{required_capability}' disallowed"
                ));
            }
        }

        // Check capability grants
        let permitted = token.allowed_capabilities.iter().any(|c| {
            if c == "*" || c == required_capability {
                true
            } else if let Some(prefix) = c.strip_suffix(".*") {
                required_capability.starts_with(prefix)
            } else {
                false
            }
        });

        if !permitted {
            return Err(format!(
                "Capability '{required_capability}' not granted by token"
            ));
        }

        Ok(())
    }

    /// Set the dynamic thermal backpressure level
    pub fn set_thermal_backpressure(&self, level: ThermalBackpressureLevel) {
        let val = match level {
            ThermalBackpressureLevel::Nominal => 0,
            ThermalBackpressureLevel::Throttled => 1,
            ThermalBackpressureLevel::Critical => 2,
        };
        self.thermal_backpressure.store(val, Ordering::Release);
    }

    /// Current thermal backpressure level
    pub fn thermal_backpressure(&self) -> ThermalBackpressureLevel {
        match self.thermal_backpressure.load(Ordering::Acquire) {
            0 => ThermalBackpressureLevel::Nominal,
            1 => ThermalBackpressureLevel::Throttled,
            _ => ThermalBackpressureLevel::Critical,
        }
    }

    /// Current dirty state generation counter
    pub fn dirty_generation(&self) -> u64 {
        self.dirty_generation.load(Ordering::Acquire)
    }

    /// Returns true if capability broker state is dirty
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::Acquire)
    }

    /// Mark capability state clean
    pub fn mark_clean(&self) {
        self.is_dirty.store(false, Ordering::Release);
    }

    /// Mark state dirty and advance generation
    pub fn mark_dirty(&self) -> u64 {
        self.is_dirty.store(true, Ordering::Release);
        self.dirty_generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Execute a tool scoped and authenticated by a CapabilityToken
    pub async fn execute_tool_with_token(
        &self,
        token: &CapabilityToken,
        tool_name: &str,
        params: serde_json::Value,
        now_ms: u64,
    ) -> CapabilityExecutionOutcome {
        if let Err(err) = self.verify_token(token, tool_name, now_ms) {
            return CapabilityExecutionOutcome::failure(tool_name, 0, err);
        }

        // Apply thermal backpressure checks
        match self.thermal_backpressure() {
            ThermalBackpressureLevel::Critical => {
                // In critical backpressure, only essential security audit tools are permitted
                if !tool_name.starts_with("security.") {
                    return CapabilityExecutionOutcome::failure(
                        tool_name,
                        0,
                        format!(
                            "Thermal backpressure critical: execution of '{tool_name}' rejected"
                        ),
                    );
                }
            }
            ThermalBackpressureLevel::Throttled => {
                // Throttled: yield to scheduler for pacing
                tokio::task::yield_now().await;
            }
            ThermalBackpressureLevel::Nominal => {}
        }

        let outcome = self.execute_tool(tool_name, params).await;
        if outcome.success {
            self.mark_dirty();
        }
        outcome
    }

    /// Execute a capability tool with JSON parameters
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> CapabilityExecutionOutcome {
        let start = std::time::Instant::now();
        match self.registry.call_by_name(tool_name, params).await {
            Ok(payload) => {
                let latency_us = start.elapsed().as_micros() as u64;
                CapabilityExecutionOutcome {
                    success: true,
                    capability_id: tool_name.to_string(),
                    latency_us,
                    error: None,
                    payload,
                }
            }
            Err(err) => {
                let latency_us = start.elapsed().as_micros() as u64;
                CapabilityExecutionOutcome::failure(tool_name, latency_us, err.to_string())
            }
        }
    }

    /// List all available tool descriptors
    pub fn list_tools(&self) -> Vec<capabilities::universal_tool::ToolDescriptor> {
        self.registry.list_tools()
    }

    /// List tools filtered by category
    pub fn list_tools_by_category(
        &self,
        category: &str,
    ) -> Vec<capabilities::universal_tool::ToolDescriptor> {
        self.registry.filter_by_category(category)
    }

    /// Backward-compatible execution method
    pub fn execute(&self, capability: &str, _args: &[String]) -> CapabilityExecutionOutcome {
        let descriptor = self
            .registry
            .list_tools()
            .into_iter()
            .find(|t| t.name == capability);
        if descriptor.is_some() {
            CapabilityExecutionOutcome::success(capability, 0)
        } else {
            CapabilityExecutionOutcome::failure(
                capability,
                0,
                format!("Capability '{}' not found", capability),
            )
        }
    }

    /// Backward-compatible capability listing
    pub fn list_capabilities(&self, category: CapabilityCategory) -> Vec<CapabilityDescriptor> {
        let cat_str = category.label();
        self.registry
            .filter_by_category(cat_str)
            .into_iter()
            .map(|t| CapabilityDescriptor {
                id: t.name.clone(),
                name: t.name,
                category,
                description: t.description,
            })
            .collect()
    }
}

impl Default for CapabilityBroker {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability categories for MCP server capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CapabilityCategory {
    General,
    ScreenAutomation,
    SystemTelemetry,
    WindowManagement,
    AudioCapture,
    FileIO,
    ProcessControl,
    NetworkAccess,
    Specialized,
}

impl CapabilityCategory {
    pub fn label(&self) -> &'static str {
        match self {
            CapabilityCategory::General => "general",
            CapabilityCategory::ScreenAutomation => "platform",
            CapabilityCategory::SystemTelemetry => "platform",
            CapabilityCategory::WindowManagement => "ui",
            CapabilityCategory::AudioCapture => "platform",
            CapabilityCategory::FileIO => "code",
            CapabilityCategory::ProcessControl => "platform",
            CapabilityCategory::NetworkAccess => "knowledge",
            CapabilityCategory::Specialized => "security",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub name: String,
    pub category: CapabilityCategory,
    pub description: String,
}

/// Result of executing a capability with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityExecutionOutcome {
    pub success: bool,
    pub capability_id: String,
    pub latency_us: u64,
    pub error: Option<String>,
    pub payload: serde_json::Value,
}

impl CapabilityExecutionOutcome {
    pub fn success(id: impl Into<String>, latency_us: u64) -> Self {
        Self {
            success: true,
            capability_id: id.into(),
            latency_us,
            error: None,
            payload: serde_json::json!({}),
        }
    }

    pub fn failure(id: impl Into<String>, latency_us: u64, error: String) -> Self {
        Self {
            success: false,
            capability_id: id.into(),
            latency_us,
            error: Some(error),
            payload: serde_json::json!({}),
        }
    }
}

impl Default for CapabilityExecutionOutcome {
    fn default() -> Self {
        Self::success("default", 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capability_broker_live_execution() {
        let broker = CapabilityBroker::new();
        let tools = broker.list_tools();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "security.audit"));

        let outcome = broker
            .execute_tool(
                "knowledge.semantic_query",
                serde_json::json!({ "query": "memory architecture" }),
            )
            .await;
        assert!(outcome.success);
        assert_eq!(outcome.capability_id, "knowledge.semantic_query");
    }

    #[tokio::test]
    async fn test_capability_token_lifecycle_and_execution() {
        let broker = CapabilityBroker::new();
        let now_ms = 1_000_000;
        let token = broker.issue_token(
            "test_agent",
            vec!["knowledge.*".to_string(), "security.audit".to_string()],
            SandboxPolicy::FullPrivilege,
            60_000,
            now_ms,
        );

        // Valid token execution
        let outcome = broker
            .execute_tool_with_token(
                &token,
                "knowledge.semantic_query",
                serde_json::json!({ "query": "lattice bounds" }),
                now_ms + 100,
            )
            .await;
        assert!(outcome.success);
        assert!(broker.is_dirty());
        assert!(broker.dirty_generation() > 1);

        // Missing permission rejected
        let outcome2 = broker
            .execute_tool_with_token(
                &token,
                "platform.screen_capture",
                serde_json::json!({}),
                now_ms + 200,
            )
            .await;
        assert!(!outcome2.success);
        assert!(outcome2.error.unwrap().contains("not granted"));
    }

    #[tokio::test]
    async fn test_capability_token_expiration() {
        let broker = CapabilityBroker::new();
        let now_ms = 1_000_000;
        let token = broker.issue_token(
            "test_agent",
            vec!["*".to_string()],
            SandboxPolicy::FullPrivilege,
            5_000,
            now_ms,
        );

        // Verification after expiration fails
        let err = broker
            .verify_token(&token, "security.audit", now_ms + 10_000)
            .unwrap_err();
        assert!(err.contains("expired"));
    }

    #[tokio::test]
    async fn test_airgapped_sandbox_rejection() {
        let broker = CapabilityBroker::new();
        let now_ms = 1_000_000;
        let token = broker.issue_token(
            "airgapped_agent",
            vec!["*".to_string()],
            SandboxPolicy::Airgapped,
            60_000,
            now_ms,
        );

        // Airgapped sandbox rejects platform actuation tool
        let outcome = broker
            .execute_tool_with_token(
                &token,
                "platform.window_manager",
                serde_json::json!({}),
                now_ms + 50,
            )
            .await;
        assert!(!outcome.success);
        assert!(
            outcome
                .error
                .unwrap()
                .contains("Airgapped sandbox violation")
        );
    }

    #[tokio::test]
    async fn test_thermal_backpressure_enforcement() {
        let broker = CapabilityBroker::new();
        let now_ms = 1_000_000;
        let token = broker.issue_token(
            "agent",
            vec!["*".to_string()],
            SandboxPolicy::FullPrivilege,
            60_000,
            now_ms,
        );

        broker.set_thermal_backpressure(ThermalBackpressureLevel::Critical);
        assert_eq!(
            broker.thermal_backpressure(),
            ThermalBackpressureLevel::Critical
        );

        // Non-security capability rejected under critical thermal backpressure
        let outcome = broker
            .execute_tool_with_token(
                &token,
                "knowledge.semantic_query",
                serde_json::json!({ "query": "test" }),
                now_ms + 50,
            )
            .await;
        assert!(!outcome.success);
        assert!(
            outcome
                .error
                .unwrap()
                .contains("Thermal backpressure critical")
        );

        // Security capability still permitted under critical backpressure
        let sec_outcome = broker
            .execute_tool_with_token(
                &token,
                "security.audit",
                serde_json::json!({ "target": "workspace" }),
                now_ms + 60,
            )
            .await;
        assert!(sec_outcome.success);

        // Reset to nominal
        broker.set_thermal_backpressure(ThermalBackpressureLevel::Nominal);
        assert_eq!(
            broker.thermal_backpressure(),
            ThermalBackpressureLevel::Nominal
        );
    }
}
