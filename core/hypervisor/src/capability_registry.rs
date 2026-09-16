use crate::workspace::WorkspacePaths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub agent_id: String,
    pub base_model_path: String, // The "Fixed Husk" (immutable GGUF)
    pub lora_switches: LoraAdapterSwitches,
    pub enzymatic_allowlist: Vec<String>, // Deterministic WASM phenotypes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraAdapterSwitches {
    pub active_loras: Vec<String>, // Tiny Rank-1 adapters for hot-swapping
    pub temperature_bias: f32,
    pub top_p: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistry {
    pub schema_version: String,
    pub profiles: HashMap<String, NodeDefinition>,
}

pub type ChromosomeRegistry = NodeRegistry;
pub type HoxChromosome = NodeDefinition;

impl Default for NodeRegistry {
    fn default() -> Self {
        let mut profiles = HashMap::new();

        profiles.insert(
            "researcher".to_string(),
            NodeDefinition {
                agent_id: "researcher_v3".to_string(),
                base_model_path: WorkspacePaths::data_dir()
                    .join("models/llama-3-8b-instruct.gguf")
                    .to_string_lossy()
                    .to_string(),
                lora_switches: LoraAdapterSwitches {
                    active_loras: vec![
                        "academic_writing.lora".to_string(),
                        "evidence_synthesis.lora".to_string(),
                    ],
                    temperature_bias: 0.4,
                    top_p: 0.9,
                },
                enzymatic_allowlist: vec![
                    "retina_browser.wasm".to_string(),
                    "compliance_gatekeeper.wasm".to_string(),
                ],
            },
        );

        NodeRegistry {
            schema_version: "4.0".to_string(),
            profiles,
        }
    }
}
