use crate::chromosome_registry::EpigeneticSwitches;
use std::collections::HashMap;

pub struct LoraAdapterVault {
    pub adapters: HashMap<String, EpigeneticSwitches>,
}

impl LoraAdapterVault {
    pub fn new() -> Self {
        let mut adapters = HashMap::new();

        // Academic Research Adapter: Optimizes for citation analysis and objective synthesis
        adapters.insert("academic_research_v1".to_string(), EpigeneticSwitches {
            active_loras: vec!["academic_prose.lora".to_string(), "citation_validator.lora".to_string()],
            temperature_bias: 0.2, // Lower temperature for precision
            top_p: 0.85,
        });

        // Code Optimization Adapter: Optimizes for Rust performance and safety patterns
        adapters.insert("code_optimizer_v1".to_string(), EpigeneticSwitches {
            active_loras: vec!["rust_efficiency.lora".to_string(), "memory_safety_checks.lora".to_string()],
            temperature_bias: 0.1, // Near-deterministic for code logic
            top_p: 0.95,
        });

        // Creative Synthesis Adapter: Optimizes for high-entropy ideation
        adapters.insert("creative_synthesis_v1".to_string(), EpigeneticSwitches {
            active_loras: vec!["metaphorical_mapping.lora".to_string()],
            temperature_bias: 0.9, // High creativity
            top_p: 1.0,
        });

        Self { adapters }
    }

    pub fn get_switches(&self, id: &str) -> Option<&EpigeneticSwitches> {
        self.adapters.get(id)
    }
}
