use crate::mcp_service::service::{JsonRpcResponse, McpService};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{
        IntoResponse, Json, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use serde_json::Value;
use std::convert::Infallible;
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
use tracing::{debug, info};

// ── State ─────────────────────────────────────────────────────────────────────

/// Session-keyed SSE response channels for Claude Desktop.
type SseSessions = Arc<
    tokio::sync::RwLock<
        std::collections::HashMap<String, Arc<tokio::sync::broadcast::Sender<String>>>,
    >,
>;

#[derive(Clone)]
pub struct McpAppState {
    pub service: Arc<McpService>,
    /// Active SSE session channels (session_id → broadcast sender)
    pub sse_sessions: SseSessions,
    /// Bind address for deriving SSE endpoint URLs dynamically.
    pub bind_addr: SocketAddr,
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
    pub async fn run(self, service: Arc<McpService>) -> Result<(), Box<dyn std::error::Error>> {
        let has_api_key = std::env::var("AARONEOUS_API_KEY")
            .map(|key| !key.is_empty())
            .unwrap_or(false);
        if !self.addr.ip().is_loopback() && !has_api_key {
            return Err(format!(
                "refusing non-loopback MCP bind {} without AARONEOUS_API_KEY",
                self.addr
            )
            .into());
        }

        let state = McpAppState {
            service,
            sse_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            bind_addr: self.addr,
        };

        // Build separate routers: protected routes get auth middleware,
        // health/discovery routes remain unauthenticated.
        let protected = Router::new()
            // MCP JSON-RPC 2.0 transport (primary)
            .route("/mcp", post(handle_mcp_post))
            // SSE transport (for Claude Desktop / streaming clients)
            .route("/sse", get(handle_sse))
            .layer(axum::middleware::from_fn(mcp_api_key_auth));

        let public = Router::new()
            // Health probe (unauthenticated)
            .route("/health", get(handle_health))
            // MCP discovery endpoint (returns server info)
            .route("/", get(handle_root));

        let app = Router::new()
            .merge(protected)
            .merge(public)
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        info!("MCP server listening on {} (JSON-RPC 2.0 + SSE)", self.addr);
        info!(
            "  Claude Desktop: add url='http://{}' to claude_desktop_config.json",
            self.addr
        );
        info!(
            "  Cursor: add url='http://{}/mcp' to settings.json",
            self.addr
        );

        axum::serve(listener, app).await?;
        Ok(())
    }
}

// ── Auth Middleware ───────────────────────────────────────────────────────────

/// API-key auth guard for MCP routes.
///
/// Mirrors the federation router's `api_key_auth` middleware:
/// - If `AARONEOUS_API_KEY` env var is **not** set → pass through for loopback
///   development servers; non-loopback startup is rejected by `HttpServer`.
/// - If set → require `Authorization: Bearer <key>` header on every request.
async fn mcp_api_key_auth(
    headers: HeaderMap,
    req: axum::extract::Request<Body>,
    next: Next,
) -> Response {
    let Some(required_key) = std::env::var("AARONEOUS_API_KEY").ok() else {
        // Auth not configured — pass through
        return next.run(req).await;
    };

    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.strip_prefix("Bearer ")
                .or_else(|| s.strip_prefix("bearer "))
        });

    match provided {
        Some(key) if key == required_key => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized",
                "message": "Set Authorization: Bearer <AARONEOUS_API_KEY>"
            })),
        )
            .into_response(),
    }
}

///
/// Accepts both single requests and batched arrays.
/// Returns single response or array of responses.
async fn handle_mcp_post(
    State(state): State<McpAppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let session_id = params.get("session").cloned();
    // Parse request body as JSON
    let raw: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            let err = JsonRpcResponse::err(None, -32700, &format!("Parse error: {}", e));
            return Json(serde_json::to_value(err).unwrap()).into_response();
        }
    };

    debug!(
        "MCP POST: {}",
        raw.get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("batch")
    );

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
    let is_notification = raw.get("id").is_none()
        && raw
            .get("method")
            .and_then(|m| m.as_str())
            .map(|m| m.starts_with("notifications/"))
            .unwrap_or(false);

    let resp = state.service.handle_jsonrpc(raw).await;

    if is_notification {
        return StatusCode::NO_CONTENT.into_response();
    }

    // Push response to session's SSE channel if session_id was provided (Claude Desktop)
    if let Some(ref sid) = session_id {
        let sessions = state.sse_sessions.read().await;
        if let Some(tx) = sessions.get(sid) {
            let resp_str = serde_json::to_string(&serde_json::to_value(&resp).unwrap_or_default())
                .unwrap_or_default();
            let _ = tx.send(resp_str);
        }
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
async fn handle_sse(State(state): State<McpAppState>) -> impl IntoResponse {
    let tool_count = state.service.tools.read().await.len();

    // Create session ID and register a broadcast channel for this SSE connection.
    // Claude Desktop will POST to /mcp?session=<id>, and responses are pushed here.
    let session_id = format!("sse-{}", uuid::Uuid::new_v4().simple());
    let (tx, mut rx) = tokio::sync::broadcast::channel::<String>(64);
    let tx_arc = Arc::new(tx);
    {
        let mut sessions = state.sse_sessions.write().await;
        sessions.insert(session_id.clone(), tx_arc.clone());
    }

    let sessions_clone = state.sse_sessions.clone();
    let sid_clone = session_id.clone();
    let endpoint_url = format!("http://{}/mcp?session={}", state.bind_addr, session_id);

    let stream = async_stream::stream! {
        // MCP spec: first event must be "endpoint" with the POST URL (includes session)
        yield Ok::<Event, Infallible>(
            Event::default().event("endpoint").data(endpoint_url.clone())
        );

        // Announce readiness
        let init = serde_json::json!({
            "jsonrpc": "2.0", "method": "notifications/message",
            "params": { "level": "info", "logger": "aaroneous",
                "data": format!("Aaroneous MCP ready — {} tools | {}", tool_count, endpoint_url) }
        });
        yield Ok::<Event, Infallible>(
            Event::default().event("message")
                .data(serde_json::to_string(&init).unwrap_or_default())
        );

        // Stream: push POST /mcp responses as SSE message events + keepalives
        let mut keepalive = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(resp_str) => {
                            yield Ok::<Event, Infallible>(
                                Event::default().event("message").data(resp_str)
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::debug!("SSE session {} lagged {}", sid_clone, n);
                        }
                        Err(_) => break,
                    }
                }
                _ = keepalive.tick() => {
                    yield Ok::<Event, Infallible>(Event::default().comment("keepalive"));
                }
            }
        }
        // Cleanup on disconnect
        let mut sessions = sessions_clone.write().await;
        sessions.remove(&sid_clone);
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
        "llm": state.service.llm_provider_status(),
    }))
}

/// GET / — MCP server discovery metadata.
async fn handle_root(State(state): State<McpAppState>) -> impl IntoResponse {
    let tool_names: Vec<String> = state
        .service
        .tools
        .read()
        .await
        .iter()
        .map(|t| t.name.clone())
        .collect();

    let addr = state.bind_addr;
    Json(serde_json::json!({
        "name": "Aaroneous",
        "description": "Sovereign AI hive — 9 specialized agents powered by abliterated non-coding base models",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP/2024-11-05",
        "transport": {
            "http": format!("POST http://{}/mcp", addr),
            "sse": format!("GET http://{}/sse", addr),
        },
        "tools": tool_names,
        "claude_desktop_config": {
            "mcpServers": {
                "aaroneous": {
                    "url": format!("http://{}/sse", addr),
                    "transport": "sse"
                }
            }
        },
        "cursor_config": {
            "cursor.mcp.servers": {
                "aaroneous": {
                    "url": format!("http://{}/mcp", addr),
                    "transport": "http"
                }
            }
        }
    }))
}
