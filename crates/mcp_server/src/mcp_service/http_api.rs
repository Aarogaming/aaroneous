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
///     "hypervisor": {
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
///     "hypervisor": { "url": "http://localhost:8766/mcp", "transport": "http" }
///   }
/// }
/// ```
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info};

/// MCP HTTP+SSE transport configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServiceConfig {
    pub port: u16,
    pub bind_addr: [u8; 4],
    pub auth_key: Option<String>,
}

impl Default for McpServiceConfig {
    fn default() -> Self {
        Self {
            port: 0,
            bind_addr: [127, 0, 0, 1],
            auth_key: None,
        }
    }
}

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
    pub auth: Arc<tokio::sync::RwLock<crate::mcp_service::ApiKeyAuth>>,
    /// Active SSE session channels (session_id → broadcast sender)
    pub sse_sessions: SseSessions,
    /// Bind address for deriving SSE endpoint URLs dynamically.
    pub bind_addr: SocketAddr,
}

// ── Server ────────────────────────────────────────────────────────────────────

pub struct HttpServer {
    addr: SocketAddr,
    auth: Option<Arc<tokio::sync::RwLock<crate::mcp_service::ApiKeyAuth>>>,
}

impl HttpServer {
    pub fn new(addr: SocketAddr, _cfg: McpServiceConfig) -> Self {
        Self { addr, auth: None }
    }

    pub fn with_auth(
        addr: SocketAddr,
        _cfg: McpServiceConfig,
        auth: Arc<tokio::sync::RwLock<crate::mcp_service::ApiKeyAuth>>,
    ) -> Self {
        Self {
            addr,
            auth: Some(auth),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Build and start the MCP HTTP+SSE server.
    pub async fn run(
        self,
        service: Arc<McpService>,
        cfg: McpServiceConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let auth = match self.auth {
            Some(a) => a,
            None => {
                let mut api_auth =
                    crate::mcp_service::ApiKeyAuth::with_clock(service.clock.clone());
                if let Some(ref key) = cfg.auth_key {
                    api_auth.register_key_with_class(
                        key,
                        "operator",
                        crate::mcp_service::CapabilityClass::Mutation,
                        vec!["*".to_string()],
                        crate::capability_broker::SandboxPolicy::FullPrivilege,
                    );
                }
                Arc::new(tokio::sync::RwLock::new(api_auth))
            }
        };

        let has_api_keys = !auth.read().await.is_empty();
        if !self.addr.ip().is_loopback() && !has_api_keys {
            return Err(format!(
                "refusing non-loopback MCP bind {} without auth keys configured",
                self.addr
            )
            .into());
        }

        let state = McpAppState {
            service,
            auth,
            sse_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            bind_addr: self.addr,
        };

        let app = build_app(state);

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

/// Construct the Axum application router from McpAppState.
pub fn build_app(state: McpAppState) -> Router {
    let state_for_auth = state.clone();
    let protected = Router::new()
        // MCP JSON-RPC 2.0 transport (primary)
        .route("/mcp", post(handle_mcp_post))
        // SSE transport (for Claude Desktop / streaming clients)
        .route("/sse", get(handle_sse))
        .layer(axum::middleware::from_fn(
            move |headers: HeaderMap, req: axum::extract::Request<Body>, next: Next| {
                let st = state_for_auth.clone();
                mcp_api_key_auth_inner(headers, req, next, st)
            },
        ));

    let public = Router::new()
        // Health probe (unauthenticated)
        .route("/health", get(handle_health))
        // MCP discovery endpoint (returns server info)
        .route("/", get(handle_root));

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

// ── Auth Middleware ───────────────────────────────────────────────────────────

/// API-key auth guard for MCP routes (derives trusted CallerContext).
async fn mcp_api_key_auth_inner(
    headers: HeaderMap,
    mut req: axum::extract::Request<Body>,
    next: Next,
    state: McpAppState,
) -> Response {
    let now_ms = state.service.clock.now_ms();
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.strip_prefix("Bearer ")
                .or_else(|| s.strip_prefix("bearer "))
        })
        .map(|s| s.trim());

    let auth_guard = state.auth.read().await;
    if let Some(token) = provided {
        if token.is_empty() {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Bearer token must not be empty"
                })),
            )
                .into_response();
        }

        match auth_guard.authenticate_caller(token, now_ms) {
            Ok(caller) => {
                drop(auth_guard);
                req.extensions_mut().insert(caller);
                next.run(req).await
            }
            Err(err) => (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": err.to_string()
                })),
            )
                .into_response(),
        }
    } else {
        // No credentials provided
        if auth_guard.is_empty() && state.bind_addr.ip().is_loopback() {
            // Local loopback compatibility adapter with zero keys configured
            drop(auth_guard);
            let caller = crate::mcp_service::CallerContext::local_development(now_ms);
            req.extensions_mut().insert(caller);
            next.run(req).await
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Valid Authorization: Bearer <AARONEOUS_API_KEY> required"
                })),
            )
                .into_response()
        }
    }
}

/// Accepts both single requests and batched arrays.
/// Returns single response or array of responses.
async fn handle_mcp_post(
    State(state): State<McpAppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    axum::extract::Extension(caller): axum::extract::Extension<crate::mcp_service::CallerContext>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let session_id = params.get("session").cloned();
    let now_ms = state.service.clock.now_ms();

    // Parse request body as JSON
    let raw: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            let err = JsonRpcResponse::err(None, -32700, &format!("Parse error: {}", e));
            return Json(serde_json::to_value(err).unwrap_or_default()).into_response();
        }
    };

    debug!(
        "MCP POST: {}",
        raw.get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("batch")
    );

    // Handle batch or single
    if let Some(requests) = raw.as_array() {
        let mut responses = Vec::new();
        for req in requests {
            // Skip notifications (no id field)
            if req.get("id").is_none() {
                state
                    .service
                    .handle_jsonrpc_with_context(req.clone(), &caller, now_ms)
                    .await;
                continue;
            }
            let resp = state
                .service
                .handle_jsonrpc_with_context(req.clone(), &caller, now_ms)
                .await;
            responses.push(serde_json::to_value(resp).unwrap_or_default());
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

    let resp = state
        .service
        .handle_jsonrpc_with_context(raw, &caller, now_ms)
        .await;

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

    Json(serde_json::to_value(resp).unwrap_or_default()).into_response()
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
            "params": { "level": "info", "logger": "hypervisor_mcp",
                "data": format!("Hypervisor MCP ready — {} tools | {}", tool_count, endpoint_url) }
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
        "name": "Hypervisor MCP Server",
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
        "name": "Hypervisor",
        "description": "Sovereign runtime — 9 specialized agents powered by abliterated non-coding base models",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP/2024-11-05",
        "transport": {
            "http": format!("POST http://{}/mcp", addr),
            "sse": format!("GET http://{}/sse", addr),
        },
        "tools": tool_names,
        "claude_desktop_config": {
            "mcpServers": {
                "hypervisor": {
                    "url": format!("http://{}/sse", addr),
                    "transport": "sse"
                }
            }
        },
        "cursor_config": {
            "cursor.mcp.servers": {
                "hypervisor": {
                    "url": format!("http://{}/mcp", addr),
                    "transport": "http"
                }
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_service::auth::{ApiKeyAuth, MockClock};
    use crate::mcp_service::capability::CapabilityClass;
    use crate::mcp_service::service::{McpService, ServiceConfig};
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn setup_test_app(
        clock: Arc<dyn crate::mcp_service::Clock>,
        bind_addr: SocketAddr,
    ) -> (Router, Arc<tokio::sync::RwLock<ApiKeyAuth>>) {
        let service = Arc::new(McpService::with_workspace_root_and_clock(
            ServiceConfig::default(),
            std::path::PathBuf::from("."),
            clock.clone(),
        ));
        let auth = Arc::new(tokio::sync::RwLock::new(ApiKeyAuth::with_clock(clock)));
        let state = McpAppState {
            service,
            auth: auth.clone(),
            sse_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            bind_addr,
        };
        (build_app(state), auth)
    }

    #[tokio::test]
    async fn test_http_readonly_key_refused_mutation_tool() {
        let clock = Arc::new(MockClock::new(1000));
        let (app, auth) = setup_test_app(clock, "127.0.0.1:8765".parse().unwrap());

        {
            let mut guard = auth.write().await;
            guard.register_key_with_class(
                "ro-key-123",
                "auditor",
                CapabilityClass::ReadOnlyInspection,
                vec!["*".to_string()],
                crate::capability_broker::SandboxPolicy::ReadConstrained,
            );
        }

        // Call a mutating tool
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "mutate_system_state",
                "arguments": {}
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("authorization", "Bearer ro-key-123")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&payload).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert!(json_resp.get("error").is_some());
        let err_msg = json_resp["error"]["message"].as_str().unwrap();
        assert!(
            err_msg.contains("Insufficient capability class"),
            "Expected capability class error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_http_scope_boundary_enforcement() {
        let clock = Arc::new(MockClock::new(1000));
        let (app, auth) = setup_test_app(clock, "127.0.0.1:8765".parse().unwrap());

        {
            let mut guard = auth.write().await;
            guard.register_key_with_class(
                "sec-key-456",
                "sec-team",
                CapabilityClass::Mutation,
                vec!["security.*".to_string()],
                crate::capability_broker::SandboxPolicy::FullPrivilege,
            );
        }

        // 1. Tool in adjacent namespace without '.' separator must be rejected
        let payload_evil = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "securityevil.audit",
                "arguments": {}
            }
        });

        let req_evil = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("authorization", "Bearer sec-key-456")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&payload_evil).unwrap()))
            .unwrap();

        let resp = app.clone().oneshot(req_evil).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json_resp.get("error").is_some());
        assert!(
            json_resp["error"]["message"]
                .as_str()
                .unwrap()
                .contains("not granted by scopes")
        );

        // 2. Tool in exact matching prefix must not be rejected by scopes
        let payload_valid = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "security.audit",
                "arguments": {}
            }
        });

        let req_valid = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("authorization", "Bearer sec-key-456")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&payload_valid).unwrap()))
            .unwrap();

        let resp_valid = app.oneshot(req_valid).await.unwrap();
        let bytes_valid = axum::body::to_bytes(resp_valid.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json_valid: serde_json::Value = serde_json::from_slice(&bytes_valid).unwrap();
        if let Some(err) = json_valid.get("error") {
            let msg = err["message"].as_str().unwrap_or_default();
            assert!(
                !msg.contains("not granted by scopes"),
                "Got unexpected scope disallowed error: {}",
                msg
            );
        }
    }

    #[tokio::test]
    async fn test_http_expired_and_revoked_keys() {
        let mock_clock = Arc::new(MockClock::new(1000));
        let (app, auth) = setup_test_app(mock_clock.clone(), "127.0.0.1:8765".parse().unwrap());

        {
            let mut guard = auth.write().await;
            guard.register_key_with_expiry(
                "exp-key",
                "alice",
                CapabilityClass::Mutation,
                vec!["*".to_string()],
                crate::capability_broker::SandboxPolicy::FullPrivilege,
                Some(5000),
            );
            guard.register_key_with_class(
                "revoked-principal-key",
                "bob",
                CapabilityClass::Mutation,
                vec!["*".to_string()],
                crate::capability_broker::SandboxPolicy::FullPrivilege,
            );
            guard.revoke_principal("bob");
        }

        // 1. Expired key request at now=6000 returns 401
        mock_clock.advance(5000);
        let req_exp = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("authorization", "Bearer exp-key")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
            ))
            .unwrap();

        let resp_exp = app.clone().oneshot(req_exp).await.unwrap();
        assert_eq!(resp_exp.status(), StatusCode::UNAUTHORIZED);

        // 2. Revoked principal key returns 401
        let req_rev = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("authorization", "Bearer revoked-principal-key")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            ))
            .unwrap();

        let resp_rev = app.oneshot(req_rev).await.unwrap();
        assert_eq!(resp_rev.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_http_unauthenticated_non_loopback_fails_closed() {
        let clock = Arc::new(MockClock::new(1000));
        // Non-loopback bind
        let (app, _) = setup_test_app(clock, "192.168.1.100:8765".parse().unwrap());

        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
