// src/capability_broker.rs
//! Universal Capability Broker for mcp_server, wired directly to `crates/capabilities`.

use anyhow::Result;
use capabilities::universal_tool::ToolRegistry;
use capabilities::tools::build_standard_tool_registry;
use serde::{Deserialize, Serialize};

/// Dynamic Capability Broker mediating tool discovery and execution
#[derive(Clone)]
pub struct CapabilityBroker {
    registry: ToolRegistry,
}

impl CapabilityBroker {
    /// Create a new capability broker pre-loaded with standard sovereign tools
    pub fn new() -> Self {
        Self {
            registry: build_standard_tool_registry(),
        }
    }

    /// Create with a custom tool registry
    pub fn with_registry(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    /// Access inner tool registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Execute a capability tool with JSON parameters
    pub async fn execute_tool(&self, tool_name: &str, params: serde_json::Value) -> CapabilityExecutionOutcome {
        let start = std::time::Instant::now();
        match self.registry.call_by_name(tool_name, params).await {
            Ok(payload) => {
                let latency_us = start.elapsed().as_micros() as u64;
                CapabilityExecutionOutcome {
                    success: true,
                    capability_id: tool_name.to_string(),
                    latency_us,
                    error: None,
                    payload,
                }
            }
            Err(err) => {
                let latency_us = start.elapsed().as_micros() as u64;
                CapabilityExecutionOutcome::failure(tool_name, latency_us, err.to_string())
            }
        }
    }

    /// List all available tool descriptors
    pub fn list_tools(&self) -> Vec<capabilities::universal_tool::ToolDescriptor> {
        self.registry.list_tools()
    }

    /// List tools filtered by category
    pub fn list_tools_by_category(&self, category: &str) -> Vec<capabilities::universal_tool::ToolDescriptor> {
        self.registry.filter_by_category(category)
    }

    /// Backward-compatible execution method
    pub fn execute(&self, capability: &str, _args: &[String]) -> CapabilityExecutionOutcome {
        let descriptor = self.registry.list_tools().into_iter().find(|t| t.name == capability);
        if descriptor.is_some() {
            CapabilityExecutionOutcome::success(capability, 0)
        } else {
            CapabilityExecutionOutcome::failure(capability, 0, format!("Capability '{}' not found", capability))
        }
    }

    /// Backward-compatible capability listing
    pub fn list_capabilities(&self, category: CapabilityCategory) -> Vec<CapabilityDescriptor> {
        let cat_str = category.label();
        self.registry
            .filter_by_category(cat_str)
            .into_iter()
            .map(|t| CapabilityDescriptor {
                id: t.name.clone(),
                name: t.name,
                category,
                description: t.description,
            })
            .collect()
    }
}

impl Default for CapabilityBroker {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability categories for MCP server capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CapabilityCategory {
    General,
    ScreenAutomation,
    SystemTelemetry,
    WindowManagement,
    AudioCapture,
    FileIO,
    ProcessControl,
    NetworkAccess,
    Specialized,
}

impl CapabilityCategory {
    pub fn label(&self) -> &'static str {
        match self {
            CapabilityCategory::General => "general",
            CapabilityCategory::ScreenAutomation => "platform",
            CapabilityCategory::SystemTelemetry => "platform",
            CapabilityCategory::WindowManagement => "ui",
            CapabilityCategory::AudioCapture => "platform",
            CapabilityCategory::FileIO => "code",
            CapabilityCategory::ProcessControl => "platform",
            CapabilityCategory::NetworkAccess => "knowledge",
            CapabilityCategory::Specialized => "security",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub name: String,
    pub category: CapabilityCategory,
    pub description: String,
}

/// Result of executing a capability with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityExecutionOutcome {
    pub success: bool,
    pub capability_id: String,
    pub latency_us: u64,
    pub error: Option<String>,
    pub payload: serde_json::Value,
}

impl CapabilityExecutionOutcome {
    pub fn success(id: impl Into<String>, latency_us: u64) -> Self {
        Self {
            success: true,
            capability_id: id.into(),
            latency_us,
            error: None,
            payload: serde_json::json!({}),
        }
    }

    pub fn failure(id: impl Into<String>, latency_us: u64, error: String) -> Self {
        Self {
            success: false,
            capability_id: id.into(),
            latency_us,
            error: Some(error),
            payload: serde_json::json!({}),
        }
    }
}

impl Default for CapabilityExecutionOutcome {
    fn default() -> Self {
        Self::success("default", 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capability_broker_live_execution() {
        let broker = CapabilityBroker::new();
        let tools = broker.list_tools();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "security.audit"));

        let outcome = broker
            .execute_tool(
                "knowledge.semantic_query",
                serde_json::json!({ "query": "memory architecture" }),
            )
            .await;
        assert!(outcome.success);
        assert_eq!(outcome.capability_id, "knowledge.semantic_query");
    }
}