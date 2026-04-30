/// MCP Bridge Module
///
/// Provides bidirectional communication between AAS (Python asyncio) and Aaroneous (Rust async).
/// Uses NATS as the transport layer, with MCP (Model Context Protocol) semantics for method calls.
///
/// # Architecture
///
/// ```text
/// AAS (Python)                    NATS JetStream           Aaroneous (Rust)
/// ├─ MCP Client                   ├─ Subject routing       ├─ MCP Server
/// │  (asyncio)        ────────→   ├─ Request/Reply        │  (tokio)
/// └─ Plugin System                └─ Event streaming      └─ Capability Registry
///
/// Message Flow:
/// 1. AAS plugin calls MCP method with trace_id
/// 2. Message sent to NATS: "mcp.aaroneous.{capability}.request"
/// 3. Aaroneous MCP server receives
/// 4. Routes to handler based on capability_id
/// 5. Returns result on NATS: "mcp.aaroneous.{capability}.response"
/// 6. AAS client receives and deserializes
/// ```
///
/// # Supported Capabilities
///
/// - `federation.healthcheck` - Cluster health status
/// - `event_log.append` - Add event to distributed log
/// - `tracing.emit_span` - Record distributed trace
/// - `consensus.propose_mutation` - Submit Raft mutation
/// - `critic.validate` - Run validation suite
/// - `recovery.checkpoint` - Create federation checkpoint
///
/// # Example Usage
///
/// ```rust,no_run
/// use aaroneous::mcp_bridge::{McpServer, McpCapability};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let mut server = McpServer::new("nats://localhost:4222").await?;
///
///     // Register capabilities
///     server.register_capability(McpCapability {
///         domain: "federation".to_string(),
///         method_name: "healthcheck".to_string(),
///         version: "1.0.0".to_string(),
///     }).await?;
///
///     // Start listening for requests
///     server.start().await?;
///     Ok(())
/// }
/// ```

pub mod server;
pub mod client;
pub mod protocol;
pub mod types;

pub use server::McpServer;
pub use client::McpClient;
pub use protocol::McpMessage;
pub use types::{McpCapability, McpRequest, McpResponse};
