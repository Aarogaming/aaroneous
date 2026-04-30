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
/// use aaroneous::mcp_service::{McpService, ServiceConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // Create service
///     let config = ServiceConfig::new()
///         .with_name("Aaroneous MCP")
///         .with_port(8080);
///     
///     let mut service = McpService::new(config).await?;
///     
///     // Register capabilities from various domains
///     service.register_domain("federation", federation_capabilities()).await?;
///     service.register_domain("intelligence", intelligence_capabilities()).await?;
///     service.register_domain("consensus", consensus_capabilities()).await?;
///     
///     // Start service on HTTP, WebSocket, and MCP
///     service.start().await?;
///     
///     Ok(())
/// }
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
pub mod config;
pub mod capability;
pub mod transport;
pub mod auth;
pub mod http_api;

pub use service::McpService;
pub use config::{ServiceConfig, TransportConfig};
pub use capability::{Capability, CapabilityDomain, CapabilityHandler, CapabilityResult};
pub use transport::{Transport, HttpTransport, WebSocketTransport, NatsTransport};
pub use auth::{AuthProvider, ApiKeyAuth, OAuth2Auth};
pub use http_api::{HttpServer, RestApi};
