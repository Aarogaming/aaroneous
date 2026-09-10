use anyhow::Result;
use crate::mcp_bridge::types::{MpcCapability, MpcMessage};
use std::collections::HashMap;

pub struct McpServer {
    capabilities: HashMap<String, MpcCapability>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub async fn start(&self) -> Result<()> {
        println!("[McpServer] Starting bridge server...");
        // In production, this would bind to NATS or HTTP
        Ok(())
    }

    pub fn register_capability(&mut self, cap: MpcCapability) {
        println!("[McpServer] Registering capability: {}.{}", cap.domain, cap.method_name);
        let key = format!("{}.{}", cap.domain, cap.method_name);
        self.capabilities.insert(key, cap);
    }

    pub async fn handle_request(&self, msg: MpcMessage) -> Result<serde_json::Value> {
        println!("[McpServer] Handling request for method: {}", msg.method);
        
        if self.capabilities.contains_key(&msg.method) {
            // Route to appropriate module
            Ok(serde_json::json!({"status": "ok", "message": "Method executed"}))
        } else {
            Err(anyhow::anyhow!("Capability not found: {}", msg.method))
        }
    }
}
