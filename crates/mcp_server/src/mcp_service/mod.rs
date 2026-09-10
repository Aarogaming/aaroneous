pub mod auth;
pub mod capability;
pub mod config;
pub mod http_api;
/// Universal MCP Service Module
///
/// Provides a vendor-agnostic MCP (Model Context Protocol) service that can be used by any client:
/// - OpenCode (AI coding assistant)
/// - VS Code Extensions
/// - Claude Web UI  
/// - Anthropic Claude
/// - Custom tools and applications
///
/// # Architecture
///
/// The MCP Service exposes capabilities through multiple transports:
/// - **MCP Protocol** - Standard Anthropic MCP (JSON-RPC 2.0)
/// - **HTTP/REST API** - RESTful interface with OpenAPI docs
/// - **WebSocket** - Real-time updates and streaming
/// - **NATS** - Internal federation transport
///
/// # Example Usage
///
/// ```rust,no_run
/// # use std::error::Error;
/// # use a_run::mcp_service::{McpService, ServiceConfig};
/// # async fn example() -> Result<(), Box<dyn Error>> {
///     // Create service
///     let config = ServiceConfig::new()
///         .with_name("Aaroneous MCP")
///         .with_http_port(8080);
///     
///     let service = McpService::new(config);
///     
///     // Register capabilities from various domains
///     service.register_sovereign_tools().await;
///     
///     // Handle requests (example)
///     let req = serde_json::json!({
///         "jsonrpc": "2.0",
///         "method": "tools/list",
///         "id": 1
///     });
///     let resp = service.handle_jsonrpc(req).await;
///     
/// # Ok(())
/// # }
/// ```
///
/// # Clients
///
/// ## OpenCode Integration
/// ```bash
/// # Discover capabilities
/// curl http://localhost:8080/api/v1/capabilities
///
/// # Call a capability
/// curl -X POST http://localhost:8080/api/v1/call \
///   -H "Authorization: Bearer <key>" \
///   -d '{"capability":"federation.healthcheck"}'
/// ```
///
/// ## VS Code Extension
/// The extension can use either MCP protocol or HTTP API for compatibility.
///
/// ## Claude Web UI
/// Uses OAuth2 authentication and REST API for seamless integration.
pub mod service;
pub mod transport;

pub use auth::{ApiKeyAuth, AuthProvider, OAuth2Auth};
pub use capability::{Capability, CapabilityDomain, CapabilityHandler, CapabilityResult};
pub use config::{ServiceConfig, TransportConfig};
pub use http_api::HttpServer;
pub use service::{JsonRpcRequest, JsonRpcResponse, McpService, McpTool, ServiceStats};
pub use transport::{HttpTransport, NatsTransport, Transport, WebSocketTransport};
