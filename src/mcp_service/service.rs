use crate::mcp_service::{
    ServiceConfig, Capability, CapabilityDomain, CapabilityHandler, CapabilityResult,
    Transport, AuthProvider, ApiKeyAuth,
};
use crate::mcp_service::capability::ExecutionStatus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Universal MCP Service
pub struct McpService {
    config: ServiceConfig,
    domains: Arc<RwLock<HashMap<String, CapabilityDomain>>>,
    auth: Arc<Box<dyn AuthProvider>>,
    transports: Arc<RwLock<Vec<Box<dyn Transport>>>>,
    running: Arc<tokio::sync::Mutex<bool>>,
}

impl McpService {
    /// Create new MCP service
    pub fn new(config: ServiceConfig) -> Self {
        // Default to API key auth
        let auth: Box<dyn AuthProvider> = Box::new(ApiKeyAuth::new());

        Self {
            config,
            domains: Arc::new(RwLock::new(HashMap::new())),
            auth: Arc::new(auth),
            transports: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(tokio::sync::Mutex::new(false)),
        }
    }

    /// Register a capability domain
    pub async fn register_domain(
        &self,
        name: &str,
        domain: CapabilityDomain,
    ) -> Result<(), String> {
        let mut domains = self.domains.write().await;
        domains.insert(name.to_string(), domain);
        Ok(())
    }

    /// Register a capability in a domain
    pub async fn register_capability(
        &self,
        domain_name: &str,
        capability: Capability,
    ) -> Result<(), String> {
        let mut domains = self.domains.write().await;

        let domain = domains
            .entry(domain_name.to_string())
            .or_insert_with(|| CapabilityDomain::new(domain_name, ""));

        domain.register(capability);
        Ok(())
    }

    /// Get capability
    pub async fn get_capability(&self, id: &str) -> Option<Capability> {
        let domains = self.domains.read().await;

        for domain in domains.values() {
            if let Some(cap) = domain.get(id) {
                return Some(cap.clone());
            }
        }

        None
    }

    /// List all capabilities
    pub async fn list_capabilities(&self) -> Vec<Capability> {
        let domains = self.domains.read().await;
        let mut caps = Vec::new();

        for domain in domains.values() {
            for cap in domain.list() {
                caps.push(cap.clone());
            }
        }

        caps
    }

    /// List capabilities in a domain
    pub async fn list_domain_capabilities(&self, domain: &str) -> Option<Vec<Capability>> {
        let domains = self.domains.read().await;
        domains.get(domain).map(|d| {
            d.list().iter().map(|c| (*c).clone()).collect()
        })
    }

    /// Call a capability
    pub async fn call_capability(
        &self,
        id: &str,
        params: serde_json::Value,
    ) -> CapabilityResult {
        let request_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        // Get capability
        let cap = match self.get_capability(id).await {
            Some(c) => c,
            None => {
                return CapabilityResult::error(
                    request_id,
                    format!("Capability not found: {}", id),
                    start.elapsed().as_millis() as u32,
                );
            }
        };

        // In production, would call registered handler
        // For now, return mock result
        CapabilityResult::success(
            request_id,
            serde_json::json!({
                "capability": id,
                "status": "executed",
                "message": "Capability execution placeholder"
            }),
            start.elapsed().as_millis() as u32,
        )
    }

    /// Start the service
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.running.lock().await;
        if *running {
            return Err("Service already running".to_string());
        }

        tracing::info!(
            "Starting {} v{}",
            self.config.name,
            self.config.version
        );

        // Start enabled transports
        let mut transports = self.transports.write().await;

        if self.config.http_api_enabled {
            tracing::info!("HTTP API listening on {}", self.config.http_addr);
        }

        if self.config.websocket_enabled {
            if let Some(ws_port) = self.config.websocket_port {
                tracing::info!("WebSocket listening on port {}", ws_port);
            }
        }

        if self.config.mcp_enabled {
            tracing::info!("MCP protocol enabled");
        }

        if let Some(nats_url) = &self.config.nats_url {
            tracing::info!("NATS federation enabled at {}", nats_url);
        }

        *running = true;
        Ok(())
    }

    /// Stop the service
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.running.lock().await;
        if !*running {
            return Err("Service not running".to_string());
        }

        tracing::info!("Stopping {}", self.config.name);

        *running = false;
        Ok(())
    }

    /// Check if service is running
    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }

    /// Get service stats
    pub async fn get_stats(&self) -> ServiceStats {
        let domains = self.domains.read().await;
        let total_capabilities = domains.values().map(|d| d.capabilities.len()).sum();

        ServiceStats {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
            running: *self.running.lock().await,
            domains_count: domains.len(),
            capabilities_count: total_capabilities,
            enabled_transports: self.enabled_transports(),
        }
    }

    /// Get enabled transports
    fn enabled_transports(&self) -> Vec<String> {
        let mut transports = Vec::new();
        
        if self.config.http_api_enabled {
            transports.push("http".to_string());
        }
        if self.config.websocket_enabled {
            transports.push("websocket".to_string());
        }
        if self.config.mcp_enabled {
            transports.push("mcp".to_string());
        }
        if self.config.nats_url.is_some() {
            transports.push("nats".to_string());
        }

        transports
    }

    /// Get service configuration
    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }
}

/// Service statistics
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ServiceStats {
    pub name: String,
    pub version: String,
    pub running: bool,
    pub domains_count: usize,
    pub capabilities_count: usize,
    pub enabled_transports: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let config = ServiceConfig::new();
        let service = McpService::new(config);

        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_service_start_stop() {
        let config = ServiceConfig::new();
        let service = McpService::new(config);

        service.start().await.unwrap();
        assert!(service.is_running().await);

        service.stop().await.unwrap();
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_capability_registration() {
        let service = McpService::new(ServiceConfig::new());

        let cap = Capability::new("federation", "healthcheck", "Check federation health");
        let mut domain = CapabilityDomain::new("federation", "Federation operations");
        domain.register(cap);

        service.register_domain("federation", domain).await.unwrap();

        let caps = service.list_domain_capabilities("federation").await;
        assert!(caps.is_some());
        assert_eq!(caps.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_list_capabilities() {
        let service = McpService::new(ServiceConfig::new());

        let cap = Capability::new("federation", "healthcheck", "Check health");
        let mut domain = CapabilityDomain::new("federation", "");
        domain.register(cap);

        service.register_domain("federation", domain).await.unwrap();

        let caps = service.list_capabilities().await;
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].id, "federation.healthcheck");
    }

    #[tokio::test]
    async fn test_call_capability() {
        let service = McpService::new(ServiceConfig::new());

        let result = service
            .call_capability("unknown.capability", serde_json::json!({}))
            .await;

        assert_eq!(result.status, ExecutionStatus::Failed);
    }

    #[tokio::test]
    async fn test_service_stats() {
        let service = McpService::new(ServiceConfig::new());

        let stats = service.get_stats().await;
        assert_eq!(stats.capabilities_count, 0);
        assert!(!stats.running);
    }
}
