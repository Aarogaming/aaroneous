use crate::mcp_bridge::types::{McpRequest, McpResponse, error_codes};
use crate::mcp_bridge::protocol::{McpMessage, subjects};
use nats::asynk::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// MCP Client - calls Aaroneous services from AAS
pub struct McpClient {
    nats_url: String,
    conn: Option<Connection>,
    pending_responses: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,
    repo_id: String,
    domain: String,
}

impl McpClient {
    /// Create new MCP client
    pub fn new(
        nats_url: impl Into<String>,
        repo_id: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            nats_url: nats_url.into(),
            conn: None,
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
            repo_id: repo_id.into(),
            domain: domain.into(),
        }
    }

    /// Connect to NATS
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = nats::asynk::connect(&self.nats_url).await?;
        self.conn = Some(conn);
        Ok(())
    }

    /// Call a capability and get response
    pub async fn call(
        &self,
        capability: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.call_with_timeout(capability, params, None).await
    }

    /// Call a capability with timeout
    pub async fn call_with_timeout(
        &self,
        capability: impl Into<String>,
        params: serde_json::Value,
        timeout_ms: Option<u64>,
    ) -> Result<serde_json::Value, String> {
        let conn = self.conn.as_ref().ok_or("Not connected to NATS")?;
        let capability_str = capability.into();

        // Generate request ID and trace ID
        let request_id = Uuid::new_v4().to_string();
        let trace_id = Uuid::new_v4().to_string();

        // Create request
        let req = McpRequest::new(
            &request_id,
            &trace_id,
            &capability_str,
            params,
            &self.repo_id,
            &self.domain,
        );

        let mut req_with_timeout = req.clone();
        if let Some(ms) = timeout_ms {
            req_with_timeout.timeout_ms = Some(ms);
        }

        // Create response channel
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_responses.write().await.insert(request_id.clone(), tx);

        // Serialize and send request
        let msg = McpMessage::Request(req_with_timeout);
        let payload = msg.to_json().map_err(|e| e.to_string())?;
        let subject = subjects::request(&capability_str);

        // Send with reply subject for this specific request
        let reply_subject = subjects::response(&capability_str, &request_id);
        conn.publish_request(&subject, &reply_subject, payload.as_bytes())
            .await
            .map_err(|e| format!("Publish error: {}", e))?;

        // Wait for response with timeout
        let response_timeout = timeout_ms
            .unwrap_or(5000) + 1000; // Add 1s buffer

        let response = tokio::time::timeout(
            std::time::Duration::from_millis(response_timeout),
            rx,
        )
        .await
        .map_err(|_| {
            self.pending_responses.blocking_write().remove(&request_id);
            "Response timeout".to_string()
        })?
        .map_err(|_| "Request cancelled".to_string())?;

        // Extract data or return error
        match response.result {
            crate::mcp_bridge::types::McpResult::Success { data, .. } => Ok(data),
            crate::mcp_bridge::types::McpResult::Error { code, message, .. } => {
                Err(format!("{}: {}", code, message))
            }
        }
    }

    /// Discover available capabilities
    pub async fn discover_capabilities(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.as_ref().ok_or("Not connected to NATS")?;

        let request_id = Uuid::new_v4().to_string();

        // Create discovery request
        let discovery_req = serde_json::json!({
            "request_id": request_id,
        });

        // Send and wait for response
        let response = conn
            .request(subjects::CAPABILITIES, discovery_req.to_string().as_bytes())
            .await
            .map_err(|e| format!("Discovery error: {}", e))?;

        let capabilities: serde_json::Value =
            serde_json::from_slice(&response.data).map_err(|e| e.to_string())?;

        capabilities
            .get("capabilities")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .ok_or("Invalid capability response".to_string())
    }

    /// Handle incoming response
    pub async fn handle_response(&self, response: McpResponse) {
        if let Some(tx) = self.pending_responses.write().await.remove(&response.request_id) {
            let _ = tx.send(response);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = McpClient::new("nats://localhost:4222", "AAS", "leadership");
        assert_eq!(client.repo_id, "AAS");
        assert_eq!(client.domain, "leadership");
    }

    #[tokio::test]
    async fn test_call_without_connection() {
        let client = McpClient::new("nats://localhost:4222", "AAS", "leadership");
        let result = client
            .call("federation.healthcheck", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }
}
