//! crates/specialists/src/universal_tool.rs
//! Universal Tool Interface and Dual-Face Execution Adapter for Aaroneous.
//!
//! Bridges three distinct execution models into a single, clean capability:
//! 1. Cloud / External API: HTTP JSON payload.
//! 2. AI LLM (OpenCode, Claude, Cursor): Anthropic/OpenAI JSON-RPC 2.0 MCP Tool Calls.
//! 3. Native .si Models: Direct zero-copy R^256 latent vector transformations in VRAM.

use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Standard metadata descriptor for an exposed capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub opcode: u16,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub category: String, // "security", "code", "memory", "knowledge", "platform", "review"
}

/// The Universal Dual-Face Tool Contract
#[async_trait]
pub trait UniversalTool: Send + Sync {
    /// Canonical tool name (e.g., "security.audit", "code.rewrite_pattern")
    fn name(&self) -> &'static str;

    /// Primary machine-native opcode for native .si model dispatch (e.g., 0x0500)
    fn opcode(&self) -> u16;

    /// Category of the capability
    fn category(&self) -> &'static str;

    /// Human- and LLM-readable description of what this capability does
    fn description(&self) -> &'static str;

    /// JSON Schema definition of acceptable parameters
    fn parameters_schema(&self) -> serde_json::Value;

    /// Face 1: Invocation via JSON parameters (for Cloud REST, LLM MCP, and CLI)
    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value>;

    /// Face 2: Invocation via native latent tensors (for local .si neural models in VRAM)
    /// Performs sub-microsecond in-place or projected latent transformation
    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()>;

    /// Generates tool metadata for registry discovery
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: self.name().to_string(),
            opcode: self.opcode(),
            description: self.description().to_string(),
            parameters_schema: self.parameters_schema(),
            category: self.category().to_string(),
        }
    }
}

/// Unified Registry managing all plug-and-play Universal Tools
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools_by_name: HashMap<String, Arc<dyn UniversalTool>>,
    tools_by_opcode: HashMap<u16, Arc<dyn UniversalTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a universal tool into the registry
    pub fn register(&mut self, tool: Arc<dyn UniversalTool>) {
        self.tools_by_name.insert(tool.name().to_string(), tool.clone());
        self.tools_by_opcode.insert(tool.opcode(), tool);
    }

    /// Dispatches a JSON call by tool name (Cloud / LLM MCP path)
    pub async fn call_by_name(&self, name: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match self.tools_by_name.get(name) {
            Some(tool) => tool.call_json(params).await,
            None => bail!("Tool not found in registry: {}", name),
        }
    }

    /// Dispatches a zero-copy latent transformation by opcode (.si model path)
    pub fn call_by_opcode(&self, opcode: u16, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        match self.tools_by_opcode.get(&opcode) {
            Some(tool) => tool.call_latent(input, output),
            None => bail!("Tool opcode 0x{:04X} not found in registry", opcode),
        }
    }

    /// Exports all tool descriptors for MCP discovery (tools/list) and OpenAPI docs
    pub fn list_tools(&self) -> Vec<ToolDescriptor> {
        self.tools_by_name.values().map(|t| t.descriptor()).collect()
    }

    /// Number of registered tools
    pub fn len(&self) -> usize {
        self.tools_by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools_by_name.is_empty()
    }
}
