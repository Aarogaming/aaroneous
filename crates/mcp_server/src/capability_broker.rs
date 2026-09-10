// src/capability_broker.rs
// Minimal stub for CapabilityBroker used by mcp_server.

use serde::{Deserialize, Serialize};

/// Minimal stub for CapabilityBroker used by mcp_server.
#[derive(Debug, Clone)]
pub struct CapabilityBroker;

impl CapabilityBroker {
    pub fn new() -> Self { Self }
    
    /// Execute a capability with given arguments
    pub fn execute(&self, _capability: &str, _args: &[String]) -> CapabilityExecutionOutcome {
        CapabilityExecutionOutcome::success("execute", 0)
    }
    
    /// List available capabilities in a category
    pub fn list_capabilities(&self, _category: CapabilityCategory) -> Vec<CapabilityDescriptor> {
        vec![]
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
    /// General-purpose capabilities (e.g., system utilities)
    General,
    /// Screen capture and automation capabilities
    ScreenAutomation,
    /// System monitoring and telemetry capabilities
    SystemTelemetry,
    /// Window management and control capabilities
    WindowManagement,
    /// Audio capture and processing capabilities
    AudioCapture,
    /// File I/O and storage capabilities
    FileIO,
    /// Process lifecycle management capabilities
    ProcessControl,
    /// Network operations and communication capabilities
    NetworkAccess,
    /// Specialized domain-specific capabilities
    Specialized,
}

impl CapabilityCategory {
    pub fn label(&self) -> &'static str {
        match self {
            CapabilityCategory::General => "general",
            CapabilityCategory::ScreenAutomation => "screen_automation",
            CapabilityCategory::SystemTelemetry => "system_telemetry",
            CapabilityCategory::WindowManagement => "window_management",
            CapabilityCategory::AudioCapture => "audio_capture",
            CapabilityCategory::FileIO => "file_io",
            CapabilityCategory::ProcessControl => "process_control",
            CapabilityCategory::NetworkAccess => "network_access",
            CapabilityCategory::Specialized => "specialized",
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
#[derive(Debug, Clone)]
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