use crate::chromosome_registry::EpigeneticSwitches;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Live LoRA adapter record holding epigenetic switches and active weight deltas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLoraAdapter {
    pub id: String,
    pub switches: EpigeneticSwitches,
    pub weight_deltas: Vec<f32>,
    pub gradient_variance: f32,
    pub rank: usize,
    pub alpha: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraAdapterVault {
    pub adapters: HashMap<String, EpigeneticSwitches>,
    pub live_adapters: HashMap<String, LiveLoraAdapter>,
}

impl Default for LoraAdapterVault {
    fn default() -> Self {
        Self::new()
    }
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

        Self {
            adapters,
            live_adapters: HashMap::new(),
        }
    }

    pub fn get_switches(&self, id: &str) -> Option<&EpigeneticSwitches> {
        self.adapters.get(id).or_else(|| self.live_adapters.get(id).map(|la| &la.switches))
    }

    /// Registers or updates a live dynamic LoRA adapter with active weight deltas
    pub fn register_live_adapter(
        &mut self,
        id: String,
        switches: EpigeneticSwitches,
        weight_deltas: Vec<f32>,
        gradient_variance: f32,
        rank: usize,
        alpha: f32,
    ) {
        let live = LiveLoraAdapter {
            id: id.clone(),
            switches: switches.clone(),
            weight_deltas,
            gradient_variance,
            rank,
            alpha,
        };
        self.adapters.insert(id.clone(), switches);
        self.live_adapters.insert(id, live);
    }

    /// Persists live LoRA adapter records to a JSON manifest file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path).with_context(|| format!("Failed to create {}", path.display()))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    /// Loads LoRA adapter vault from a JSON manifest file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
        let reader = BufReader::new(file);
        let vault = serde_json::from_reader(reader)?;
        Ok(vault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_vault_roundtrip() {
        let mut vault = LoraAdapterVault::new();
        vault.register_live_adapter(
            "test_live_adapter".to_string(),
            EpigeneticSwitches {
                active_loras: vec!["test.lora".to_string()],
                temperature_bias: 0.15,
                top_p: 0.90,
            },
            vec![0.01, -0.02, 0.03, 0.005],
            0.00042,
            16,
            32.0,
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("lora_vault.json");
        vault.save_to_file(&path).unwrap();

        let loaded = LoraAdapterVault::load_from_file(&path).unwrap();
        assert!(loaded.adapters.contains_key("test_live_adapter"));
        assert_eq!(loaded.live_adapters.get("test_live_adapter").unwrap().rank, 16);
    }
}
