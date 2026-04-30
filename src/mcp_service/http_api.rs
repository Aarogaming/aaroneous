use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    extract::{State, Path, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use uuid::Uuid;

/// Shared application state for HTTP handlers
#[derive(Clone)]
pub struct AppState {
    pub service: Arc<crate::mcp_service::McpService>,
}

/// HTTP Server for REST API
pub struct HttpServer {
    addr: SocketAddr,
    state: Option<AppState>,
}

impl HttpServer {
    /// Create new HTTP server
    pub fn new(addr: SocketAddr) -> Self {
        Self { 
            addr,
            state: None,
        }
    }

    /// Set the MCP service state
    pub fn with_service(mut self, service: Arc<crate::mcp_service::McpService>) -> Self {
        self.state = Some(AppState {
            service,
        });
        self
    }

    /// Get server address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Build and start the HTTP server (async)
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.ok_or("No MCP service provided")?;

        // Build router with all endpoints
        let app = Router::new()
            // Health check
            .route("/health", get(health_check))
            // Status endpoint
            .route("/status", get(service_status))
            // Capabilities
            .route("/api/v1/capabilities", get(list_capabilities))
            .route("/api/v1/capabilities/:id", get(get_capability))
            // Call capability
            .route("/api/v1/call", post(call_capability))
            // OpenAPI spec
            .route("/api/v1/openapi.json", get(openapi_spec))
            .with_state(state);

        // Create TCP listener
        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        tracing::info!("HTTP server listening on {}", self.addr);

        // Run server
        axum::serve(listener, app).await?;
        Ok(())
    }
}

/// REST API interface
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RestApi {
    /// API version
    pub version: String,
    /// Base path (e.g., "/api/v1")
    pub base_path: String,
}

impl RestApi {
    /// Create new REST API
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            base_path: "/api/v1".to_string(),
        }
    }

    /// List capabilities endpoint
    pub fn list_capabilities_path(&self) -> String {
        format!("{}/capabilities", self.base_path)
    }

    /// Get capability details endpoint
    pub fn get_capability_path(&self, id: &str) -> String {
        format!("{}/capabilities/{}", self.base_path, id)
    }

    /// Call capability endpoint
    pub fn call_path(&self) -> String {
        format!("{}/call", self.base_path)
    }

    /// Health check endpoint
    pub fn health_path(&self) -> String {
        format!("{}/health", self.base_path)
    }

    /// Status endpoint
    pub fn status_path(&self) -> String {
        format!("{}/status", self.base_path)
    }
}

/// HTTP request for calling a capability
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallRequest {
    /// Capability ID (e.g., "federation.healthcheck")
    pub capability: String,
    /// Optional trace ID for distributed tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Request parameters
    pub params: serde_json::Value,
    /// Optional timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

/// HTTP response from capability call
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallResponse {
    /// Request ID (for tracing)
    pub request_id: String,
    /// Execution status
    pub status: String,
    /// Result data
    pub result: serde_json::Value,
    /// Execution time in milliseconds
    pub latency_ms: u32,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Service health response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status (healthy, degraded, unhealthy)
    pub status: String,
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Active endpoints
    pub endpoints: Vec<String>,
    /// Total requests processed
    pub requests_total: u64,
    /// Total errors
    pub errors_total: u64,
    /// Timestamp
    pub timestamp: String,
}

/// Service status response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Federation status
    pub federation_status: String,
    /// Active nodes
    pub active_nodes: u32,
    /// Total events
    pub total_events: u64,
    /// Event log size (bytes)
    pub log_size_bytes: u64,
    /// Active transports
    pub active_transports: Vec<String>,
    /// Rate limit info
    pub rate_limit_info: RateLimitInfo,
}

/// Rate limit information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Requests per second limit
    pub limit_rps: u32,
    /// Current requests in window
    pub current_rps: u32,
    /// Seconds remaining in window
    pub window_remaining_secs: u32,
}

/// HTTP Handler functions

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let now = chrono::Utc::now();
    Json(HealthResponse {
        status: "healthy".to_string(),
        name: "Aaroneous MCP Service".to_string(),
        version: "2.0.0".to_string(),
        uptime_secs: 0, // Would track actual uptime
        endpoints: vec![
            "/health".to_string(),
            "/status".to_string(),
            "/api/v1/capabilities".to_string(),
            "/api/v1/call".to_string(),
            "/api/v1/openapi.json".to_string(),
        ],
        requests_total: 0, // Would track actual requests
        errors_total: 0,
        timestamp: now.to_rfc3339(),
    })
}

/// Service status endpoint
async fn service_status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        federation_status: "active".to_string(),
        active_nodes: 1,
        total_events: 0,
        log_size_bytes: 0,
        active_transports: vec!["http".to_string()],
        rate_limit_info: RateLimitInfo {
            limit_rps: 1000,
            current_rps: 0,
            window_remaining_secs: 60,
        },
    })
}

/// List all capabilities
async fn list_capabilities(State(state): State<AppState>) -> Json<Vec<crate::mcp_service::Capability>> {
    let caps = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            state.service.list_capabilities().await
        })
    });
    Json(caps)
}

/// Get specific capability
async fn get_capability(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::mcp_service::Capability>, (StatusCode, String)> {
    let cap = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            state.service.get_capability(&id).await
        })
    });
    
    cap.ok_or_else(|| (
        StatusCode::NOT_FOUND,
        format!("Capability '{}' not found", id),
    ))
    .map(Json)
}

/// Call a capability
async fn call_capability(
    State(state): State<AppState>,
    Json(req): Json<CallRequest>,
) -> Json<CallResponse> {
    let request_id = Uuid::new_v4().to_string();
    let start = std::time::Instant::now();
    
    // In a real implementation, we would:
    // 1. Look up the capability by ID
    // 2. Execute it with the provided params
    // 3. Return the result
    
    let latency_ms = start.elapsed().as_millis() as u32;
    
    Json(CallResponse {
        request_id,
        status: "success".to_string(),
        result: serde_json::json!({"message": "Capability execution placeholder"}),
        latency_ms,
        error: None,
    })
}

/// Get OpenAPI spec
async fn openapi_spec(State(state): State<AppState>) -> Json<serde_json::Value> {
    let caps = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            state.service.list_capabilities().await
        })
    });
    
    Json(OpenApiGenerator::generate_spec(&caps))
}

/// OpenAPI documentation generator
pub struct OpenApiGenerator;

impl OpenApiGenerator {
    /// Generate OpenAPI spec for capabilities
    pub fn generate_spec(capabilities: &[crate::mcp_service::Capability]) -> serde_json::Value {
        serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Aaroneous MCP Service API",
                "version": "3.0.0",
                "description": "Universal Model Context Protocol service"
            },
            "servers": [
                {"url": "http://localhost:8080/api/v1"}
            ],
            "paths": {
                "/capabilities": {
                    "get": {
                        "summary": "List all capabilities",
                        "responses": {
                            "200": {
                                "description": "List of capabilities",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {
                                                "type": "object"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "/call": {
                    "post": {
                        "summary": "Call a capability",
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "capability": {"type": "string"},
                                            "params": {"type": "object"}
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "Capability execution result"
                            }
                        }
                    }
                },
                "/health": {
                    "get": {
                        "summary": "Health check",
                        "responses": {
                            "200": {
                                "description": "Service health status"
                            }
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_server_creation() {
        let server = HttpServer::new(([127, 0, 0, 1], 8080).into());
        assert_eq!(server.addr().port(), 8080);
    }

    #[test]
    fn test_rest_api_paths() {
        let api = RestApi::new("1.0.0");
        assert_eq!(api.list_capabilities_path(), "/api/v1/capabilities");
        assert_eq!(api.call_path(), "/api/v1/call");
        assert_eq!(api.health_path(), "/api/v1/health");
    }

    #[test]
    fn test_call_request() {
        let req = CallRequest {
            capability: "federation.healthcheck".to_string(),
            trace_id: Some("trace-1".to_string()),
            params: serde_json::json!({}),
            timeout_ms: Some(5000),
        };

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CallRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.capability, "federation.healthcheck");
    }

    #[test]
    fn test_call_response() {
        let resp = CallResponse {
            request_id: "req-1".to_string(),
            status: "success".to_string(),
            result: serde_json::json!({"ok": true}),
            latency_ms: 100,
            error: None,
        };

        assert_eq!(resp.status, "success");
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_health_response() {
        let resp = HealthResponse {
            status: "healthy".to_string(),
            name: "Aaroneous MCP".to_string(),
            version: "3.0.0".to_string(),
            uptime_secs: 3600,
            endpoints: vec!["http".to_string(), "websocket".to_string()],
            requests_total: 10000,
            errors_total: 5,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        assert_eq!(resp.status, "healthy");
        assert_eq!(resp.requests_total, 10000);
    }
}
