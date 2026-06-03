/// Genome Trait Loader — loads trait presets from JSON files in `registry/genome/traits/`.

use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tracing::{info, warn};
use crate::unified_registry::{Registry, RegistryConfig, EntryMeta};

/// A genome trait preset with persona, cognitive, and domain modifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomeTrait {
    pub trait_id: String,
    pub description: String,
    #[serde(default)]
    pub persona_modifiers: PersonaModifiers,
    #[serde(default)]
    pub cognitive_modifiers: CognitiveModifiers,
    #[serde(default)]
    pub domain_modifiers: DomainModifiers,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonaModifiers {
    #[serde(default)]
    pub primary_archetype: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
    #[serde(default)]
    pub formality: Option<f32>,
    #[serde(default)]
    pub verbosity: Option<f32>,
    #[serde(default)]
    pub directive_authority: Option<f32>,
    #[serde(default)]
    pub empathy_level: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitiveModifiers {
    #[serde(default)]
    pub risk_tolerance: Option<f32>,
    #[serde(default)]
    pub exploration_vs_stability: Option<f32>,
    #[serde(default)]
    pub audit_strictness: Option<f32>,
    #[serde(default)]
    pub analytical_depth: Option<f32>,
    #[serde(default)]
    pub creative_variance: Option<f32>,
    #[serde(default)]
    pub delegation_bias: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainModifiers {
    #[serde(default)]
    pub thought: Option<f32>,
    #[serde(default)]
    pub knowledge: Option<f32>,
    #[serde(default)]
    pub leadership: Option<f32>,
    #[serde(default)]
    pub creation: Option<f32>,
    #[serde(default)]
    pub security: Option<f32>,
    #[serde(default)]
    pub intelligence: Option<f32>,
}

/// Load all genome traits from a directory.
pub fn load_traits_from_dir(dir: &Path) -> Result<Vec<GenomeTrait>> {
    let mut traits = Vec::new();

    if !dir.exists() {
        warn!("Genome traits directory does not exist: {}", dir.display());
        return Ok(traits);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            match load_trait_file(&path) {
                Ok(trait_data) => {
                    info!("Loaded genome trait: {} from {}", trait_data.trait_id, path.display());
                    traits.push(trait_data);
                }
                Err(e) => {
                    warn!("Failed to load trait from {}: {}", path.display(), e);
                }
            }
        }
    }

    info!("Loaded {} genome traits from {}", traits.len(), dir.display());
    Ok(traits)
}

/// Load a single trait file.
fn load_trait_file(path: &Path) -> Result<GenomeTrait> {
    let json = std::fs::read_to_string(path)?;
    let trait_data: GenomeTrait = serde_json::from_str(&json)?;
    Ok(trait_data)
}

/// Register loaded traits into a unified registry.
pub fn register_traits(
    registry: &mut Registry<GenomeTrait>,
    traits_dir: &Path,
) -> Result<usize> {
    let traits = load_traits_from_dir(traits_dir)?;
    let mut count = 0;

    for trait_data in traits {
        let id = trait_data.trait_id.clone();
        let meta = EntryMeta::new("1.0.0")
            .with_tags(vec!["genome-trait".into()]);

        if let Err(e) = registry.register(id, trait_data, meta) {
            warn!("Failed to register trait: {}", e);
        } else {
            count += 1;
        }
    }

    info!("Registered {} genome traits", count);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_load_trait_file() {
        let dir = std::env::temp_dir().join("test_genome_traits");
        std::fs::create_dir_all(&dir).ok();

        let json = r#"{
            "trait_id": "test_trait",
            "description": "A test trait",
            "persona_modifiers": {
                "primary_archetype": "Tester",
                "formality": 50.0
            },
            "cognitive_modifiers": {
                "risk_tolerance": 0.5
            }
        }"#;

        let path = dir.join("test.json");
        let mut f = File::create(&path).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        let traits = load_traits_from_dir(&dir).unwrap();
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].trait_id, "test_trait");
        assert_eq!(traits[0].persona_modifiers.primary_archetype.as_deref(), Some("Tester"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_register_traits() {
        let dir = std::env::temp_dir().join("test_genome_traits2");
        std::fs::create_dir_all(&dir).ok();

        let json = r#"{"trait_id": "alpha", "description": "Alpha trait"}"#;
        std::fs::write(dir.join("alpha.json"), json).unwrap();

        let mut registry = Registry::<GenomeTrait>::new(RegistryConfig::default());
        let count = register_traits(&mut registry, &dir).unwrap();
        assert_eq!(count, 1);
        assert!(registry.get("alpha").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }
}
