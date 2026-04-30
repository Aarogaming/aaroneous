use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;

/// Result of capability execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityResult {
    /// Unique request ID
    pub request_id: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Result data
    pub data: serde_json::Value,
    /// Execution latency (ms)
    pub latency_ms: u32,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Execution status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionStatus {
    /// Successfully executed
    Success,
    /// Partial success
    PartialSuccess,
    /// Failed execution
    Failed,
    /// Timed out
    Timeout,
    /// Invalid request
    InvalidRequest,
}

impl CapabilityResult {
    /// Create successful result
    pub fn success(request_id: impl Into<String>, data: serde_json::Value, latency_ms: u32) -> Self {
        Self {
            request_id: request_id.into(),
            status: ExecutionStatus::Success,
            data,
            latency_ms,
            error: None,
        }
    }

    /// Create error result
    pub fn error(request_id: impl Into<String>, error: impl Into<String>, latency_ms: u32) -> Self {
        Self {
            request_id: request_id.into(),
            status: ExecutionStatus::Failed,
            data: serde_json::json!({}),
            latency_ms,
            error: Some(error.into()),
        }
    }

    /// Create timeout result
    pub fn timeout(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            status: ExecutionStatus::Timeout,
            data: serde_json::json!({}),
            latency_ms: 0,
            error: Some("Request timeout".to_string()),
        }
    }
}

/// Capability handler function type
pub type CapabilityHandler = Box<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;

/// Capability definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    /// Fully qualified capability ID (domain.method)
    pub id: String,
    /// Capability domain (federation, intelligence, consensus, etc.)
    pub domain: String,
    /// Capability method name
    pub method: String,
    /// Human-readable description
    pub description: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// Input JSON schema (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// Output JSON schema (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Supported by transports (mcp, http, websocket, nats)
    pub supported_transports: Vec<String>,
    /// Whether this capability requires authentication
    pub requires_auth: bool,
    /// Whether this capability mutates state
    pub is_mutating: bool,
    /// Example request/response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<HashMap<String, serde_json::Value>>,
}

impl Capability {
    /// Create new capability
    pub fn new(domain: impl Into<String>, method: impl Into<String>, description: impl Into<String>) -> Self {
        let domain = domain.into();
        let method = method.into();
        let id = format!("{}.{}", domain, method);

        Self {
            id,
            domain,
            method,
            description: description.into(),
            version: "1.0.0".to_string(),
            input_schema: None,
            output_schema: None,
            required_permissions: Vec::new(),
            supported_transports: vec![
                "mcp".to_string(),
                "http".to_string(),
                "websocket".to_string(),
            ],
            requires_auth: true,
            is_mutating: false,
            examples: None,
        }
    }

    /// Set version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set input schema
    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set output schema
    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Mark as mutating
    pub fn mutating(mut self) -> Self {
        self.is_mutating = true;
        self
    }

    /// Add required permission
    pub fn with_permission(mut self, perm: impl Into<String>) -> Self {
        self.required_permissions.push(perm.into());
        self
    }

    /// Allow unauthenticated access
    pub fn public(mut self) -> Self {
        self.requires_auth = false;
        self
    }
}

/// Capability domain (namespace for related capabilities)
#[derive(Clone, Debug)]
pub struct CapabilityDomain {
    /// Domain name (federation, intelligence, consensus, etc.)
    pub name: String,
    /// Domain description
    pub description: String,
    /// Capabilities in this domain
    pub capabilities: HashMap<String, Capability>,
}

impl CapabilityDomain {
    /// Create new domain
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            capabilities: HashMap::new(),
        }
    }

    /// Register capability in domain
    pub fn register(&mut self, capability: Capability) {
        self.capabilities.insert(capability.id.clone(), capability);
    }

    /// Get capability
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.get(id)
    }

    /// List all capabilities
    pub fn list(&self) -> Vec<&Capability> {
        self.capabilities.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new("federation", "healthcheck", "Check federation health");
        assert_eq!(cap.id, "federation.healthcheck");
        assert_eq!(cap.domain, "federation");
        assert_eq!(cap.method, "healthcheck");
    }

    #[test]
    fn test_capability_builder() {
        let cap = Capability::new("intelligence", "forecast", "Predict metrics")
            .with_version("2.0.0")
            .mutating()
            .with_permission("write:metrics");

        assert_eq!(cap.version, "2.0.0");
        assert!(cap.is_mutating);
        assert!(cap.required_permissions.contains(&"write:metrics".to_string()));
    }

    #[test]
    fn test_capability_domain() {
        let mut domain = CapabilityDomain::new("federation", "Federation operations");
        let cap = Capability::new("federation", "healthcheck", "Check health");
        
        domain.register(cap);
        assert_eq!(domain.list().len(), 1);
        assert!(domain.get("federation.healthcheck").is_some());
    }

    #[test]
    fn test_capability_result() {
        let result = CapabilityResult::success("req-1", serde_json::json!({"ok": true}), 100);
        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_capability_result_error() {
        let result = CapabilityResult::error("req-1", "Service unavailable", 50);
        assert_eq!(result.status, ExecutionStatus::Failed);
        assert!(result.error.is_some());
    }
}
