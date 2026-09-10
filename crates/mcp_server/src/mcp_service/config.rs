use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Service configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Listen address for HTTP
    pub http_addr: SocketAddr,
    /// WebSocket port (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket_port: Option<u16>,
    /// NATS server URL (optional, for federation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_url: Option<String>,
    /// Enable MCP protocol support
    pub mcp_enabled: bool,
    /// Enable HTTP API
    pub http_api_enabled: bool,
    /// Enable WebSocket
    pub websocket_enabled: bool,
    /// API key required
    pub api_key_required: bool,
    /// OAuth2 enabled
    pub oauth2_enabled: bool,
    /// Rate limiting (requests per second)
    pub rate_limit_rps: u32,
    /// Enable audit logging
    pub audit_logging: bool,
    /// Federation enabled
    pub federation_enabled: bool,
    /// Federation sync interval (seconds)
    pub sync_interval_secs: u64,
    /// Peer repositories (for federation)
    pub peers: Vec<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "Aaroneous MCP Service".to_string(),
            version: "3.0.0".to_string(),
            http_addr: ([0, 0, 0, 0], 8080).into(),
            websocket_port: Some(8443),
            nats_url: Some("nats://localhost:4222".to_string()),
            mcp_enabled: true,
            http_api_enabled: true,
            websocket_enabled: true,
            api_key_required: true,
            oauth2_enabled: false,
            rate_limit_rps: 100,
            audit_logging: true,
            federation_enabled: true,
            sync_interval_secs: 60,
            peers: Vec::new(),
        }
    }
}

impl ServiceConfig {
    /// Create new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set service name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set service version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set HTTP listen address
    pub fn with_http_addr(mut self, addr: SocketAddr) -> Self {
        self.http_addr = addr;
        self
    }

    /// Set HTTP port
    pub fn with_http_port(mut self, port: u16) -> Self {
        self.http_addr.set_port(port);
        self
    }

    /// Enable/disable MCP protocol
    pub fn with_mcp_enabled(mut self, enabled: bool) -> Self {
        self.mcp_enabled = enabled;
        self
    }

    /// Enable/disable HTTP API
    pub fn with_http_api_enabled(mut self, enabled: bool) -> Self {
        self.http_api_enabled = enabled;
        self
    }

    /// Enable/disable WebSocket
    pub fn with_websocket_enabled(mut self, enabled: bool) -> Self {
        self.websocket_enabled = enabled;
        self
    }

    /// Set API key requirement
    pub fn with_api_key_required(mut self, required: bool) -> Self {
        self.api_key_required = required;
        self
    }

    /// Enable/disable OAuth2
    pub fn with_oauth2_enabled(mut self, enabled: bool) -> Self {
        self.oauth2_enabled = enabled;
        self
    }

    /// Set rate limit
    pub fn with_rate_limit_rps(mut self, rps: u32) -> Self {
        self.rate_limit_rps = rps;
        self
    }

    /// Set NATS URL
    pub fn with_nats_url(mut self, url: impl Into<String>) -> Self {
        self.nats_url = Some(url.into());
        self
    }

    /// Set federation enabled
    pub fn with_federation_enabled(mut self, enabled: bool) -> Self {
        self.federation_enabled = enabled;
        self
    }

    /// Add peer
    pub fn with_peer(mut self, peer: impl Into<String>) -> Self {
        self.peers.push(peer.into());
        self
    }

    /// Get HTTP port
    pub fn http_port(&self) -> u16 {
        self.http_addr.port()
    }

    /// Check if service is configured for this transport
    pub fn is_transport_enabled(&self, transport: &str) -> bool {
        match transport {
            "mcp" => self.mcp_enabled,
            "http" => self.http_api_enabled,
            "websocket" => self.websocket_enabled,
            "nats" => self.nats_url.is_some(),
            _ => false,
        }
    }
}

/// Transport-specific configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type (mcp, http, websocket, nats)
    pub transport_type: String,
    /// Enable this transport
    pub enabled: bool,
    /// Transport-specific settings
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServiceConfig::default();
        assert_eq!(config.name, "Aaroneous MCP Service");
        assert!(config.http_api_enabled);
        assert!(config.mcp_enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = ServiceConfig::new()
            .with_name("Test Service")
            .with_http_port(9090)
            .with_api_key_required(false);

        assert_eq!(config.name, "Test Service");
        assert_eq!(config.http_port(), 9090);
        assert!(!config.api_key_required);
    }

    #[test]
    fn test_transport_enabled() {
        let config = ServiceConfig::new()
            .with_http_api_enabled(true)
            .with_mcp_enabled(false);

        assert!(config.is_transport_enabled("http"));
        assert!(!config.is_transport_enabled("mcp"));
    }
}
