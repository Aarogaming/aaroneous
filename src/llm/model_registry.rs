// Model Registry
// Manages available GGUF models and recommendations

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Information about a GGUF model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub model_type: ModelType,
    pub recommended_score: f32, // 0.0-1.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelType {
    QwenTiny,      // 0.5B - ultra-lightweight
    QwenSmall,     // 1.8B - recommended default
    QwenBase,      // 7B - good balance
    QwenLarge,     // 14B+ - high capability
    LlamaSmall,    // Llama 2 7B
    LlamaBase,     // Llama 2 13B
    MistralSmall,  // Mistral 7B
    Other,         // Unknown
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl ModelType {
    pub fn from_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        match () {
            _ if lower.contains("qwen") && lower.contains("0.5") => ModelType::QwenTiny,
            _ if lower.contains("qwen") && (lower.contains("1.8") || lower.contains("1_8")) => ModelType::QwenSmall,
            _ if lower.contains("qwen") && (lower.contains("7b") || lower.contains("7_b")) => ModelType::QwenBase,
            _ if lower.contains("qwen") => ModelType::QwenLarge,
            _ if lower.contains("llama") && lower.contains("7") => ModelType::LlamaSmall,
            _ if lower.contains("llama") && (lower.contains("13") || lower.contains("13b")) => ModelType::LlamaBase,
            _ if lower.contains("mistral") => ModelType::MistralSmall,
            _ => ModelType::Other,
        }
    }

    pub fn recommended_score(&self) -> f32 {
        match self {
            ModelType::QwenSmall => 0.95,   // Perfect for reasoning
            ModelType::QwenBase => 0.85,    // More capable
            ModelType::QwenTiny => 0.70,    // Fast but limited
            ModelType::QwenLarge => 0.75,   // Slower but better quality
            ModelType::MistralSmall => 0.80, // Good alternative
            ModelType::LlamaSmall => 0.75,  // Solid choice
            ModelType::LlamaBase => 0.70,   // Slower, needs more resources
            ModelType::Other => 0.5,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ModelType::QwenTiny => "Qwen 0.5B - Ultra-lightweight, fastest",
            ModelType::QwenSmall => "Qwen 1.8B - Recommended, good reasoning",
            ModelType::QwenBase => "Qwen 7B - Larger, more capable",
            ModelType::QwenLarge => "Qwen 14B+ - Highest quality, slower",
            ModelType::LlamaSmall => "Llama 2 7B - Solid alternative",
            ModelType::LlamaBase => "Llama 2 13B - More powerful, slower",
            ModelType::MistralSmall => "Mistral 7B - Fast and capable",
            ModelType::Other => "Other model",
        }
    }
}

/// Model registry and discovery
pub struct ModelRegistry {
    models: Vec<ModelInfo>,
    pub search_paths: Vec<PathBuf>,
}

impl ModelRegistry {
    /// Create new model registry
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            search_paths: Self::default_search_paths(),
        }
    }

    /// Get default search paths for models
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // LM Studio paths
        if let Ok(home) = std::env::var("USERPROFILE") {
            paths.push(PathBuf::from(format!("{}/.lm-studio/models", home)));
            paths.push(PathBuf::from(format!("{}\\AppData\\Local\\LM Studio\\models", home)));
        }

        // Common locations
        paths.push(PathBuf::from("./models"));
        paths.push(PathBuf::from("../models"));
        paths.push(PathBuf::from("C:/LM Studio/models"));
        paths.push(PathBuf::from("D:/models"));

        // Environment variable
        if let Ok(model_path) = std::env::var("AARONEOUS_MODELS_PATH") {
            paths.push(PathBuf::from(model_path));
        }

        paths
    }

    /// Scan for available GGUF models
    pub fn scan(&mut self) -> Result<()> {
        self.models.clear();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                debug!("Model search path not found: {}", search_path.display());
                continue;
            }

            debug!("Scanning for models in: {}", search_path.display());

            match std::fs::read_dir(search_path) {
                Ok(entries) => {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.extension().map_or(false, |ext| ext == "gguf") {
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();

                                    let model_type = ModelType::from_name(&name);
                                    let recommended_score = model_type.recommended_score();

                                    let model = ModelInfo {
                                        name,
                                        path,
                                        size_bytes: metadata.len(),
                                        model_type,
                                        recommended_score,
                                    };

                                    self.models.push(model);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to scan {}: {}", search_path.display(), e);
                }
            }
        }

        // Sort by recommendation score (highest first)
        self.models.sort_by(|a, b| {
            b.recommended_score
                .partial_cmp(&a.recommended_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!("Found {} GGUF models", self.models.len());

        Ok(())
    }

    /// Get top N recommended models
    pub fn top_recommendations(&self, count: usize) -> Vec<&ModelInfo> {
        self.models.iter().take(count).collect()
    }

    /// Get all models
    pub fn all_models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Get model by name
    pub fn get_by_name(&self, name: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Get best model of a specific type
    pub fn get_best_of_type(&self, target_type: ModelType) -> Option<&ModelInfo> {
        self.models
            .iter()
            .find(|m| {
                match (m.model_type, target_type) {
                    (ModelType::QwenSmall, ModelType::QwenSmall) => true,
                    (ModelType::QwenBase, ModelType::QwenBase) => true,
                    (ModelType::QwenTiny, ModelType::QwenTiny) => true,
                    (ModelType::QwenLarge, ModelType::QwenLarge) => true,
                    (ModelType::MistralSmall, ModelType::MistralSmall) => true,
                    (ModelType::LlamaSmall, ModelType::LlamaSmall) => true,
                    (ModelType::LlamaBase, ModelType::LlamaBase) => true,
                    (ModelType::Other, ModelType::Other) => true,
                    _ => false,
                }
            })
    }

    /// Get model with smallest size (fastest)
    pub fn get_fastest(&self) -> Option<&ModelInfo> {
        self.models.iter().min_by_key(|m| m.size_bytes)
    }

    /// Get model with largest size (most capable)
    pub fn get_most_capable(&self) -> Option<&ModelInfo> {
        self.models.iter().max_by_key(|m| m.size_bytes)
    }

    /// Add search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_from_name() {
        assert_eq!(
            ModelType::from_name("qwen-1.8b.gguf"),
            ModelType::QwenSmall
        );
        assert_eq!(ModelType::from_name("qwen-7b.gguf"), ModelType::QwenBase);
        assert_eq!(
            ModelType::from_name("mistral-7b.gguf"),
            ModelType::MistralSmall
        );
    }

    #[test]
    fn test_recommended_scores() {
        assert!(ModelType::QwenSmall.recommended_score() > 0.9);
        assert!(ModelType::QwenBase.recommended_score() > 0.8);
        assert!(ModelType::QwenTiny.recommended_score() > 0.6);
    }

    #[test]
    fn test_registry_creation() {
        let registry = ModelRegistry::new();
        assert!(!registry.search_paths.is_empty());
    }

    #[test]
    fn test_model_info_sorting() {
        let models = vec![
            ModelInfo {
                name: "model1".to_string(),
                path: PathBuf::from("model1.gguf"),
                size_bytes: 1024,
                model_type: ModelType::QwenTiny,
                recommended_score: 0.7,
            },
            ModelInfo {
                name: "model2".to_string(),
                path: PathBuf::from("model2.gguf"),
                size_bytes: 2048,
                model_type: ModelType::QwenSmall,
                recommended_score: 0.95,
            },
        ];

        let mut sorted = models;
        sorted.sort_by(|a, b| {
            b.recommended_score
                .partial_cmp(&a.recommended_score)
                .unwrap()
        });

        assert_eq!(sorted[0].recommended_score, 0.95);
    }
}
