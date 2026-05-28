// Model Loader
// Manages model discovery, verification, and recommendation

use crate::llm::model_registry::{ModelInfo, ModelRegistry, ModelType};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn};

/// Top recommended models for Aaroneous
/// Ordered by recommendation score
pub const TOP_RECOMMENDED_MODELS: &[(&str, &str)] = &[
    ("Qwen 1.8B", "qwen-1.8b-gguf"),      // Best balance
    ("Qwen 0.5B", "qwen-0.5b-gguf"),      // Fastest
    ("Mistral 7B", "mistral-7b-gguf"),    // Good alternative
    ("Qwen 7B", "qwen-7b-gguf"),          // More capable
    ("Llama 2 7B", "llama-2-7b-gguf"),    // Solid choice
];

/// Model loader and manager
pub struct ModelLoader {
    registry: ModelRegistry,
}

impl ModelLoader {
    /// Create new model loader
    pub fn new() -> Self {
        Self {
            registry: ModelRegistry::new(),
        }
    }

    /// Initialize: scan for available models
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing model loader...");
        self.registry.scan()?;
        self.print_available_models();
        Ok(())
    }

    /// Get top 5 recommended models
    pub fn get_top_5_recommendations(&self) -> Vec<&ModelInfo> {
        let recommendations = self.registry.top_recommendations(5);
        
        if recommendations.is_empty() {
            warn!("No GGUF models found. Please download models to:");
            for path in &self.registry.search_paths {
                warn!("  {}", path.display());
            }
        }
        
        recommendations
    }

    /// Get best model for reasoning
    pub fn get_recommended_model(&self) -> Option<&ModelInfo> {
        // Prefer Qwen 1.8B or fallback to first recommendation
        self.registry
            .get_best_of_type(ModelType::QwenSmall)
            .or_else(|| self.registry.top_recommendations(1).first().copied())
    }

    /// Get fastest model (smallest)
    pub fn get_fastest_model(&self) -> Option<&ModelInfo> {
        self.registry.get_fastest()
    }

    /// Get most capable model (largest)
    pub fn get_most_capable_model(&self) -> Option<&ModelInfo> {
        self.registry.get_most_capable()
    }

    /// Print available models in friendly format
    pub fn print_available_models(&self) {
        let models = self.registry.all_models();
        
        if models.is_empty() {
            println!("\n⚠️  No GGUF models found!");
            println!("\nTo use Aaroneous with local LLM inference:");
            println!("1. Download a Qwen GGUF model from HuggingFace");
            println!("2. Place it in one of these directories:");
            for path in &self.registry.search_paths {
                println!("   - {}", path.display());
            }
            println!("\nRecommended models:");
        for (name, _hf_id) in TOP_RECOMMENDED_MODELS {
                println!("  • {}", name);
            }
            return;
        }

        println!("\n✓ Found {} GGUF models:\n", models.len());

        for (idx, model) in models.iter().enumerate() {
            let size_mb = model.size_bytes as f64 / 1_000_000.0;
            let score = (model.recommended_score * 100.0) as u32;
            println!(
                "  {}. {} ({:.1} MB) [{}% recommended]",
                idx + 1,
                model.name,
                size_mb,
                score
            );
            println!("     Type: {}", model.model_type.description());
            println!("     Path: {}\n", model.path.display());
        }
    }

    /// Print recommendations
    pub fn print_recommendations(&self) {
        println!("\n🎯 Top 5 Recommended Models:\n");

        let recommendations = self.get_top_5_recommendations();
        
        for (idx, model) in recommendations.iter().enumerate() {
            let size_mb = model.size_bytes as f64 / 1_000_000.0;
            println!("  {}. {}", idx + 1, model.name);
            println!("     Size: {:.1} MB", size_mb);
            println!("     Type: {}", model.model_type.description());
            println!("     Score: {:.0}%", model.recommended_score * 100.0);
            println!("     Path: {}\n", model.path.display());
        }

        if let Some(best) = self.get_recommended_model() {
            println!("📌 Default Model: {}\n", best.name);
        }
    }

    /// Get model by name
    pub fn get_model(&self, name: &str) -> Option<&ModelInfo> {
        self.registry.get_by_name(name)
    }

    /// Get all available models
    pub fn all_models(&self) -> &[ModelInfo] {
        self.registry.all_models()
    }

    /// Add custom search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.registry.add_search_path(path);
    }

    /// Rescan for models (after adding new paths)
    pub fn rescan(&mut self) -> Result<()> {
        self.registry.scan()
    }
}

impl Default for ModelLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = ModelLoader::new();
        assert!(!loader.registry.search_paths.is_empty());
    }

    #[tokio::test]
    async fn test_loader_initialization() {
        let mut loader = ModelLoader::new();
        let result = loader.initialize().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_recommended_models_list() {
        assert!(!TOP_RECOMMENDED_MODELS.is_empty());
        assert_eq!(TOP_RECOMMENDED_MODELS.len(), 5);
    }
}
