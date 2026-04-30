use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Capability definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpCapability {
    /// Capability domain (e.g., "federation", "event_log", "consensus")
    pub domain: String,
    /// Method name within domain (e.g., "healthcheck", "append", "propose_mutation")
    pub method_name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// JSON Schema for input parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// JSON Schema for output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl McpCapability {
    /// Create a new capability
    pub fn new(domain: impl Into<String>, method_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            method_name: method_name.into(),
            version: version.into(),
            input_schema: None,
            output_schema: None,
            description: None,
        }
    }

    /// Get full capability ID
    pub fn id(&self) -> String {
        format!("{}.{}", self.domain, self.method_name)
    }

    /// Get NATS subject for this capability
    pub fn request_subject(&self) -> String {
        format!("mcp.aaroneous.{}.request", self.id())
    }

    /// Get NATS subject for responses
    pub fn response_subject(&self, request_id: &str) -> String {
        format!("mcp.aaroneous.{}.response.{}", self.id(), request_id)
    }
}

/// MCP Request sent to Aaroneous
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpRequest {
    /// Unique request ID (used for reply routing)
    pub request_id: String,
    /// Distributed trace ID (links to tracing system)
    pub trace_id: String,
    /// Capability domain and method
    pub capability: String,  // "domain.method_name"
    /// Request parameters
    pub params: serde_json::Value,
    /// Source repository (e.g., "AaroneousAutomationSuite")
    pub source_repo: String,
    /// Source domain (e.g., "leadership", "intelligence")
    pub source_domain: String,
    /// Timestamp (Unix milliseconds)
    pub timestamp: i64,
    /// Optional timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl McpRequest {
    /// Create a new request
    pub fn new(
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        capability: impl Into<String>,
        params: serde_json::Value,
        source_repo: impl Into<String>,
        source_domain: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            capability: capability.into(),
            params,
            source_repo: source_repo.into(),
            source_domain: source_domain.into(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            timeout_ms: None,
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// MCP Response from Aaroneous
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpResponse {
    /// Echoed from request
    pub request_id: String,
    /// Echoed from request
    pub trace_id: String,
    /// "federation.healthcheck", etc.
    pub capability: String,
    /// Success or error
    pub result: McpResult,
    /// Timestamp when response created
    pub timestamp: i64,
}

/// MCP Result - either success or error
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpResult {
    /// Successful response
    Success {
        /// Response data
        data: serde_json::Value,
        /// Execution latency in milliseconds
        latency_ms: u32,
    },
    /// Error response
    Error {
        /// Error code (e.g., "TIMEOUT", "INVALID_PARAM", "INTERNAL")
        code: String,
        /// Error message
        message: String,
        /// Optional details
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<HashMap<String, serde_json::Value>>,
    },
}

impl McpResponse {
    /// Create successful response
    pub fn success(request_id: impl Into<String>, trace_id: impl Into<String>, capability: impl Into<String>, data: serde_json::Value, latency_ms: u32) -> Self {
        Self {
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            capability: capability.into(),
            result: McpResult::Success { data, latency_ms },
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create error response
    pub fn error(request_id: impl Into<String>, trace_id: impl Into<String>, capability: impl Into<String>, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            capability: capability.into(),
            result: McpResult::Error {
                code: code.into(),
                message: message.into(),
                details: None,
            },
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Check if response was successful
    pub fn is_success(&self) -> bool {
        matches!(self.result, McpResult::Success { .. })
    }
}

/// Capability handler function
pub type CapabilityHandler = Box<dyn Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> + Send + Sync>;

/// Standard error codes
pub mod error_codes {
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const INVALID_PARAM: &str = "INVALID_PARAM";
    pub const INTERNAL: &str = "INTERNAL";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const UNAVAILABLE: &str = "UNAVAILABLE";
}
