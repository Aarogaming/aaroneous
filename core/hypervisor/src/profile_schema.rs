use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePermissions {
    pub max_sovereignty_tier: u8,
    pub allow_network: bool,
    pub whitelisted_domains: Vec<String>,
    pub requires_hitl: bool,
}

#[deprecated(since = "0.3.3", note = "Use NodePermissions instead")]
pub type HoxPermissions = NodePermissions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema_json: String, // Standard MCP JSON schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDefinition {
    pub category: String,
    pub expression_level: f32,
    pub permissions: NodePermissions,
    pub mcp_tools: Vec<McpToolDefinition>, // Tools exposed by this module/node
}

#[deprecated(since = "0.3.3", note = "Use ModuleDefinition instead")]
pub type EnzymeGenetics = ModuleDefinition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMap {
    pub schema_version: String,
    pub enzymes: HashMap<String, ModuleDefinition>,
}

#[deprecated(since = "0.3.3", note = "Use NodeMap instead")]
pub type HoxMap = NodeMap;

impl Default for HoxMap {
    fn default() -> Self {
        let mut enzymes = HashMap::new();

        // Strategic Specialist (Tier 2 - Remote)
        enzymes.insert(
            "orchestrator".to_string(),
            EnzymeGenetics {
                category: "strategic_planning".to_string(),
                expression_level: 0.95,
                permissions: HoxPermissions {
                    max_sovereignty_tier: 2,
                    allow_network: true,
                    whitelisted_domains: vec![
                        "api.openai.com".to_string(),
                        "api.anthropic.com".to_string(),
                    ],
                    requires_hitl: true,
                },
                mcp_tools: vec![McpToolDefinition {
                    name: "request_strategic_overview".to_string(),
                    description: "Generates a high-level DAG for a complex goal".to_string(),
                    input_schema_json: "{}".to_string(),
                }],
            },
        );

        // Diplomatic Specialist (Tier 2 - Hybrid)
        enzymes.insert(
            "solon".to_string(),
            EnzymeGenetics {
                category: "diplomatic_negotiation".to_string(),
                expression_level: 0.98,
                permissions: HoxPermissions {
                    max_sovereignty_tier: 2,
                    allow_network: true,
                    whitelisted_domains: vec![
                        "api.openai.com".to_string(),
                        "api.anthropic.com".to_string(),
                    ],
                    requires_hitl: true,
                },
                mcp_tools: vec![McpToolDefinition {
                    name: "negotiate_consensus".to_string(),
                    description: "Mediates between conflicting specialist intents".to_string(),
                    input_schema_json: "{}".to_string(),
                }],
            },
        );

        // Local Execution Specialist (Tier 0 - Local)
        enzymes.insert(
            "fabricator".to_string(),
            EnzymeGenetics {
                category: "execution".to_string(),
                expression_level: 0.99,
                permissions: HoxPermissions {
                    max_sovereignty_tier: 0,
                    allow_network: false,
                    whitelisted_domains: vec![],
                    requires_hitl: false,
                },
                mcp_tools: vec![McpToolDefinition {
                    name: "execute_rust_kernel".to_string(),
                    description: "Compiles and runs a sandboxed Rust function".to_string(),
                    input_schema_json: "{\"code\": {\"type\": \"string\"}}".to_string(),
                }],
            },
        );

        NodeMap {
            schema_version: "3.0".to_string(),
            enzymes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    #[allow(deprecated)]
    fn test_nodemap_hoxmap_equivalence() {
        assert_eq!(TypeId::of::<NodeMap>(), TypeId::of::<HoxMap>());
        assert_eq!(
            TypeId::of::<NodePermissions>(),
            TypeId::of::<HoxPermissions>()
        );
        assert_eq!(
            TypeId::of::<ModuleDefinition>(),
            TypeId::of::<EnzymeGenetics>()
        );

        let node_map = NodeMap::default();
        let hox_map: HoxMap = node_map.clone();
        assert_eq!(node_map.schema_version, hox_map.schema_version);
        assert_eq!(node_map.enzymes.len(), 3);
    }
}
