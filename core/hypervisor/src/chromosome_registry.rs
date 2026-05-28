use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::workspace::WorkspacePaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoxChromosome {
    pub agent_id: String,
    pub base_model_path: String, // The "Fixed Husk" (immutable GGUF)
    pub epigenetic_switches: EpigeneticSwitches,
    pub enzymatic_allowlist: Vec<String>, // Deterministic WASM phenotypes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpigeneticSwitches {
    pub active_loras: Vec<String>, // Tiny Rank-1 adapters for hot-swapping
    pub temperature_bias: f32,
    pub top_p: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromosomeRegistry {
    pub schema_version: String,
    pub profiles: HashMap<String, HoxChromosome>,
}

impl Default for ChromosomeRegistry {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        
        profiles.insert("researcher".to_string(), HoxChromosome {
            agent_id: "researcher_v3".to_string(),
            base_model_path: WorkspacePaths::data_dir().join("models/llama-3-8b-instruct.gguf").to_string_lossy().to_string(),
            epigenetic_switches: EpigeneticSwitches {
                active_loras: vec!["academic_writing.lora".to_string(), "evidence_synthesis.lora".to_string()],
                temperature_bias: 0.4,
                top_p: 0.9,
            },
            enzymatic_allowlist: vec!["retina_browser.wasm".to_string(), "compliance_gatekeeper.wasm".to_string()],
        });

        ChromosomeRegistry {
            schema_version: "4.0".to_string(),
            profiles,
        }
    }
}
