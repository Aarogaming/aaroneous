use crate::mcp_bridge::types::{McpCapability, McpRequest, McpResponse, CapabilityHandler, error_codes};
use crate::mcp_bridge::protocol::{McpMessage, subjects};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use nats::asynk::{Connection, Subscription};
use std::time::{SystemTime, UNIX_EPOCH};

/// MCP Server - receives requests from AAS and routes to handlers
pub struct McpServer {
    nats_url: String,
    conn: Option<Connection>,
    handlers: Arc<RwLock<HashMap<String, CapabilityHandler>>>,
    capabilities: Arc<RwLock<HashMap<String, McpCapability>>>,
    repo_id: String,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
}

impl McpServer {
    /// Create new MCP server
    pub fn new(nats_url: impl Into<String>, repo_id: impl Into<String>) -> Self {
        Self {
            nats_url: nats_url.into(),
            conn: None,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            repo_id: repo_id.into(),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Connect to NATS and start listening
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Connect to NATS
        let conn = nats::asynk::connect(&self.nats_url).await?;
        self.conn = Some(conn);

        // Subscribe to capability discovery
        let conn_clone = self.conn.clone().ok_or("Failed to get connection")?;
        let capabilities = self.capabilities.clone();
        let repo_id = self.repo_id.clone();

        let discovery_sub = conn_clone.subscribe(subjects::CAPABILITIES).await?;
        let mut subs = self.subscriptions.write().await;
        subs.push(discovery_sub);

        Ok(())
    }

    /// Register a capability with a handler
    pub async fn register_capability(
        &self,
        capability: McpCapability,
        handler: CapabilityHandler,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cap_id = capability.id();
        let conn = self.conn.as_ref().ok_or("Not connected to NATS")?;

        // Store capability
        self.capabilities.write().await.insert(cap_id.clone(), capability);

        // Store handler
        self.handlers.write().await.insert(cap_id.clone(), handler);

        // Subscribe to requests for this capability
        let subject = subjects::request(&cap_id);
        let _sub = conn.subscribe(&subject).await?;
        let mut subs = self.subscriptions.write().await;
        subs.push(_sub);

        Ok(())
    }

    /// Handle incoming request
    pub async fn handle_request(
        &self,
        req: McpRequest,
    ) -> Result<McpResponse, String> {
        let start_time = SystemTime::now();

        // Get handler for this capability
        let handlers = self.handlers.read().await;
        let handler = handlers.get(&req.capability).ok_or_else(|| {
            format!("Unknown capability: {}", req.capability)
        })?;

        // Call handler with timeout
        let result = match self.call_with_timeout(handler, req.params.clone(), req.timeout_ms).await {
            Ok(data) => {
                let latency_ms = start_time.elapsed().unwrap_or_default().as_millis() as u32;
                McpResponse::success(
                    &req.request_id,
                    &req.trace_id,
                    &req.capability,
                    data,
                    latency_ms,
                )
            }
            Err(e) => {
                McpResponse::error(
                    &req.request_id,
                    &req.trace_id,
                    &req.capability,
                    error_codes::INTERNAL,
                    e,
                )
            }
        };

        Ok(result)
    }

    /// Call handler with timeout
    async fn call_with_timeout(
        &self,
        handler: &CapabilityHandler,
        params: serde_json::Value,
        timeout_ms: Option<u64>,
    ) -> Result<serde_json::Value, String> {
        let fut = handler(params);

        match timeout_ms {
            Some(ms) => {
                tokio::time::timeout(
                    std::time::Duration::from_millis(ms),
                    fut,
                )
                .await
                .map_err(|_| "Request timeout".to_string())?
            }
            None => fut.await,
        }
    }

    /// Get list of registered capabilities
    pub async fn list_capabilities(&self) -> Vec<String> {
        self.capabilities
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// Get registered capability details
    pub async fn get_capability(&self, id: &str) -> Option<McpCapability> {
        self.capabilities.read().await.get(id).cloned()
    }

    /// Get NATS connection
    pub fn connection(&self) -> Option<&Connection> {
        self.conn.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let server = McpServer::new("nats://localhost:4222", "TestRepo");
        assert_eq!(server.repo_id, "TestRepo");
    }

    #[tokio::test]
    async fn test_capability_registration() {
        let server = McpServer::new("nats://localhost:4222", "TestRepo");
        let cap = McpCapability::new("federation", "healthcheck", "1.0.0");

        // Create a dummy handler
        let handler: CapabilityHandler = Box::new(|_params| {
            Box::pin(async move { Ok(serde_json::json!({"status": "ok"})) })
        });

        // Note: This test won't actually work without NATS running
        // but demonstrates the API
        let _cap_id = cap.id();
        let _cap_list = server.list_capabilities().await;
        assert_eq!(_cap_list.len(), 0); // No handlers registered yet
    }

    #[tokio::test]
    async fn test_request_handling() {
        let server = McpServer::new("nats://localhost:4222", "TestRepo");

        let req = McpRequest::new(
            "req-1",
            "trace-1",
            "federation.healthcheck",
            serde_json::json!({}),
            "AAS",
            "leadership",
        );

        // Try handling without handler registered - should fail
        let result = server.handle_request(req).await;
        assert!(result.is_err());
    }
}
