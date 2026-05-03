/// MCP HTTP+SSE transport.
///
/// Implements the Anthropic Model Context Protocol 2024-11-05 specification:
///   - POST /mcp  — JSON-RPC 2.0 request/response (main transport)
///   - GET  /sse  — Server-Sent Events for server-initiated notifications
///   - GET  /health — Liveness probe
///
/// # Client configuration
///
/// Claude Desktop (~/Library/Application Support/Claude/claude_desktop_config.json):
/// ```json
/// {
///   "mcpServers": {
///     "aaroneous": {
///       "url": "http://localhost:8766/sse",
///       "transport": "sse"
///     }
///   }
/// }
/// ```
///
/// Cursor (settings.json):
/// ```json
/// {
///   "cursor.mcp.servers": {
///     "aaroneous": { "url": "http://localhost:8766/mcp", "transport": "http" }
///   }
/// }
/// ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::Value;
use futures_util::StreamExt;
use tracing::{info, debug, warn};

use crate::mcp_service::service::{McpService, JsonRpcRequest, JsonRpcResponse};

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct McpAppState {
    pub service: Arc<McpService>,
}

// ── Server ────────────────────────────────────────────────────────────────────

pub struct HttpServer {
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Build and start the MCP HTTP+SSE server.
    pub async fn run(
        self,
        service: Arc<McpService>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = McpAppState { service };

        let app = Router::new()
            // MCP JSON-RPC 2.0 transport (primary)
            .route("/mcp",    post(handle_mcp_post))
            // SSE transport (for Claude Desktop / streaming clients)
            .route("/sse",    get(handle_sse))
            // Health probe (unauthenticated)
            .route("/health", get(handle_health))
            // MCP discovery endpoint (returns server info)
            .route("/",       get(handle_root))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        info!("MCP server listening on {} (JSON-RPC 2.0 + SSE)", self.addr);
        info!("  Claude Desktop: add url='http://{}' to claude_desktop_config.json", self.addr);
        info!("  Cursor: add url='http://{}/mcp' to settings.json", self.addr);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /mcp — JSON-RPC 2.0 request handler.
///
/// Accepts both single requests and batched arrays.
/// Returns single response or array of responses.
async fn handle_mcp_post(
    State(state): State<McpAppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Parse request body as JSON
    let raw: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            let err = JsonRpcResponse::err(
                None, -32700,
                &format!("Parse error: {}", e),
            );
            return Json(serde_json::to_value(err).unwrap()).into_response();
        }
    };

    debug!("MCP POST: {}", raw.get("method").and_then(|m| m.as_str()).unwrap_or("batch"));

    // Handle batch or single
    if raw.is_array() {
        let requests = raw.as_array().unwrap();
        let mut responses = Vec::new();
        for req in requests {
            // Skip notifications (no id field)
            if req.get("id").is_none() {
                state.service.handle_jsonrpc(req.clone()).await;
                continue;
            }
            let resp = state.service.handle_jsonrpc(req.clone()).await;
            responses.push(serde_json::to_value(resp).unwrap());
        }
        return Json(Value::Array(responses)).into_response();
    }

    // Single request — check if notification (no id)
    let is_notification = raw.get("id").is_none() &&
        raw.get("method").and_then(|m| m.as_str())
           .map(|m| m.starts_with("notifications/"))
           .unwrap_or(false);

    let resp = state.service.handle_jsonrpc(raw).await;

    if is_notification {
        // Notifications get 204 No Content
        return StatusCode::NO_CONTENT.into_response();
    }

    Json(serde_json::to_value(resp).unwrap()).into_response()
}

/// GET /sse — Server-Sent Events transport for Claude Desktop.
///
/// The SSE transport works as follows:
/// 1. Client connects to /sse
/// 2. Server sends `endpoint` event with the POST URL
/// 3. Client sends JSON-RPC requests to that URL
/// 4. Server sends responses back as `message` SSE events
///
/// This is the transport Claude Desktop uses (it requires SSE, not HTTP POST).
async fn handle_sse(
    State(state): State<McpAppState>,
) -> impl IntoResponse {
    // The SSE endpoint first advertises the POST endpoint, then keeps alive
    // Real SSE MCP would need a session-keyed response channel; for now we
    // send the endpoint advertisement and keep the connection open.
    // Read tool count before entering the stream (can't hold async guard across yield)
    let tool_count = state.service.tools.read().await.len();

    let stream = async_stream::stream! {
        // Required first event: tell the client where to POST requests
        yield Ok::<Event, Infallible>(
            Event::default()
                .event("endpoint")
                .data("http://localhost:8766/mcp")
        );

        // Server capabilities announcement
        let init_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/message",
            "params": {
                "level": "info",
                "logger": "aaroneous",
                "data": format!(
                    "Aaroneous MCP server ready — {} tools available. \
                     Submit intents to the sovereign hive via tools/call.",
                    tool_count
                )
            }
        });
        yield Ok::<Event, Infallible>(
            Event::default()
                .event("message")
                .data(serde_json::to_string(&init_notification).unwrap_or_default())
        );

        // Keep-alive loop — ping every 30s
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            yield Ok::<Event, Infallible>(Event::default().comment("keepalive"));
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// GET /health — Liveness probe.
async fn handle_health(State(state): State<McpAppState>) -> impl IntoResponse {
    let tool_count = state.service.tools.read().await.len();
    Json(serde_json::json!({
        "status": "healthy",
        "name": "Aaroneous MCP Server",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP 2024-11-05",
        "transport": ["http", "sse"],
        "tools_registered": tool_count,
        "uptime_secs": state.service.uptime_secs(),
        "requests_total": state.service.request_count(),
    }))
}

/// GET / — MCP server discovery metadata.
async fn handle_root(State(state): State<McpAppState>) -> impl IntoResponse {
    let tool_names: Vec<String> = state.service.tools.read().await
        .iter().map(|t| t.name.clone()).collect();

    Json(serde_json::json!({
        "name": "Aaroneous",
        "description": "Sovereign AI hive — 9 specialized agents powered by abliterated non-coding base models",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP/2024-11-05",
        "transport": {
            "http": "POST http://localhost:8766/mcp",
            "sse": "GET http://localhost:8766/sse",
        },
        "tools": tool_names,
        "claude_desktop_config": {
            "mcpServers": {
                "aaroneous": {
                    "url": "http://localhost:8766/sse",
                    "transport": "sse"
                }
            }
        },
        "cursor_config": {
            "cursor.mcp.servers": {
                "aaroneous": {
                    "url": "http://localhost:8766/mcp",
                    "transport": "http"
                }
            }
        }
    }))
}
