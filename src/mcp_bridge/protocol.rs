use serde::{Deserialize, Serialize};
use crate::mcp_bridge::types::{McpRequest, McpResponse};

/// MCP Message envelope
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum McpMessage {
    /// Request from AAS to Aaroneous
    Request(McpRequest),
    /// Response from Aaroneous to AAS
    Response(McpResponse),
    /// Heartbeat to establish connectivity
    Heartbeat {
        source_repo: String,
        timestamp: i64,
        version: String,
    },
    /// Capability discovery request
    DiscoverCapabilities {
        request_id: String,
    },
    /// Capability discovery response
    CapabilitiesDiscovered {
        request_id: String,
        capabilities: Vec<String>,  // List of "domain.method_name" strings
        timestamp: i64,
    },
}

impl McpMessage {
    /// Get trace ID from message (if applicable)
    pub fn trace_id(&self) -> Option<String> {
        match self {
            McpMessage::Request(req) => Some(req.trace_id.clone()),
            McpMessage::Response(res) => Some(res.trace_id.clone()),
            _ => None,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Protocol version
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// NATS subject patterns for MCP
pub mod subjects {
    /// Request subject: mcp.aaroneous.{capability}.request
    pub fn request(capability: &str) -> String {
        format!("mcp.aaroneous.{}.request", capability)
    }

    /// Response subject: mcp.aaroneous.{capability}.response.{request_id}
    pub fn response(capability: &str, request_id: &str) -> String {
        format!("mcp.aaroneous.{}.response.{}", capability, request_id)
    }

    /// Heartbeat subject
    pub const HEARTBEAT: &str = "mcp.aaroneous.heartbeat";

    /// Capability discovery
    pub const CAPABILITIES: &str = "mcp.aaroneous.capabilities";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let req = McpRequest::new(
            "req-123",
            "trace-abc",
            "federation.healthcheck",
            serde_json::json!({}),
            "AaroneousAutomationSuite",
            "leadership",
        );

        let msg = McpMessage::Request(req);
        let json = msg.to_json().unwrap();
        let decoded = McpMessage::from_json(&json).unwrap();

        assert!(matches!(decoded, McpMessage::Request(_)));
    }

    #[test]
    fn test_subject_generation() {
        assert_eq!(
            subjects::request("federation.healthcheck"),
            "mcp.aaroneous.federation.healthcheck.request"
        );
        assert_eq!(
            subjects::response("federation.healthcheck", "req-123"),
            "mcp.aaroneous.federation.healthcheck.response.req-123"
        );
    }
}
