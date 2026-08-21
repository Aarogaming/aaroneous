// Automatic Model Discovery
// Automatically detects installed model loading software and loads models on startup

use crate::llm::model_environment::{ModelEnvironment, ModelEnvironmentDetector};
use crate::llm::model_loader::ModelLoader;
use crate::llm::model_registry::ModelInfo;
use anyhow::Result;
use std::sync::OnceLock;
use tracing::{debug, info};

/// Cached auto-discovered models (singleton)
static AUTO_DISCOVERED_MODELS: OnceLock<AutoDiscoveryResult> = OnceLock::new();

/// Result of auto-discovery process
#[derive(Debug, Clone)]
pub struct AutoDiscoveryResult {
    pub detected_environment: Option<ModelEnvironment>,
    pub models_found: Vec<ModelInfo>,
    pub recommended_model: Option<ModelInfo>,
    pub fastest_model: Option<ModelInfo>,
}

impl AutoDiscoveryResult {
    pub fn has_models(&self) -> bool {
        !self.models_found.is_empty()
    }

    pub fn model_count(&self) -> usize {
        self.models_found.len()
    }
}

/// Perform auto-discovery (should be called once at startup)
pub async fn auto_discover_models() -> Result<AutoDiscoveryResult> {
    info!("🔍 Auto-discovering model environment and GGUF models...");

    // Detect environment
    let mut detector = ModelEnvironmentDetector::new();
    detector.scan()?;

    let detected_env = detector.get_best_environment().map(|e| e.environment);

    if let Some(env) = detected_env {
        info!("Found model environment: {}", env.name());

        // Create loader and add paths from detected environment
        let mut loader = ModelLoader::new();
        let search_paths = env.get_search_paths();

        for path in search_paths {
            loader.add_search_path(path);
        }

        // Scan for models
        loader.initialize().await?;

        let all_models = loader.all_models().to_vec();
        let recommended = loader.get_recommended_model().cloned();
        let fastest = loader.get_fastest_model().cloned();

        if !all_models.is_empty() {
            info!(
                "✅ Auto-discovery complete: found {} models",
                all_models.len()
            );
            if let Some(ref model) = recommended {
                info!("📌 Recommended model: {}", model.name);
            }
        } else {
            info!(
                "⚠️  Model environment detected ({}) but no GGUF models found",
                env.name()
            );
        }

        let result = AutoDiscoveryResult {
            detected_environment: Some(env),
            models_found: all_models,
            recommended_model: recommended,
            fastest_model: fastest,
        };

        Ok(result)
    } else {
        info!("⚠️  No model loading software detected (LM Studio, Ollama, LocalAI)");
        debug!("Users can still provide models via AARONEOUS_MODELS_PATH environment variable");

        let result = AutoDiscoveryResult {
            detected_environment: None,
            models_found: Vec::new(),
            recommended_model: None,
            fastest_model: None,
        };

        Ok(result)
    }
}

/// Get auto-discovered models (initializes on first call)
pub async fn get_auto_discovered_models() -> Result<&'static AutoDiscoveryResult> {
    if AUTO_DISCOVERED_MODELS.get().is_none() {
        let result = auto_discover_models().await?;
        let _ = AUTO_DISCOVERED_MODELS.set(result);
    }

    Ok(AUTO_DISCOVERED_MODELS.get().unwrap())
}

/// Initialize auto-discovery (call once at application startup)
pub async fn initialize_auto_discovery() -> Result<()> {
    let _ = get_auto_discovered_models().await?;
    Ok(())
}

/// Get recommended model for use in LLMClient
pub async fn get_recommended_model_for_llm() -> Result<Option<ModelInfo>> {
    let discovered = get_auto_discovered_models().await?;
    Ok(discovered.recommended_model.clone())
}

/// Check if models are available
pub async fn models_available() -> Result<bool> {
    let discovered = get_auto_discovered_models().await?;
    Ok(discovered.has_models())
}

/// Print auto-discovery summary
pub async fn print_auto_discovery_summary() -> Result<()> {
    let discovered = get_auto_discovered_models().await?;

    println!("\n📊 Auto-Discovery Summary:");
    println!(
        "  Environment: {}",
        discovered
            .detected_environment
            .map(|e| e.name())
            .unwrap_or("None detected")
    );
    println!("  Models found: {}", discovered.model_count());

    if let Some(recommended) = &discovered.recommended_model {
        println!("  Recommended: {}", recommended.name);
    }

    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_discovery_result_has_models() {
        let result = AutoDiscoveryResult {
            detected_environment: None,
            models_found: vec![],
            recommended_model: None,
            fastest_model: None,
        };
        assert!(!result.has_models());
        assert_eq!(result.model_count(), 0);
    }

    #[test]
    fn test_auto_discovery_result_with_models() {
        use crate::llm::model_registry::ModelType;
        use std::path::PathBuf;

        let model = ModelInfo {
            name: "test-model".to_string(),
            path: PathBuf::from("test.gguf"),
            size_bytes: 1024,
            model_type: ModelType::QwenSmall,
            recommended_score: 0.95,
        };

        let result = AutoDiscoveryResult {
            detected_environment: Some(ModelEnvironment::LMStudio),
            models_found: vec![model],
            recommended_model: None,
            fastest_model: None,
        };

        assert!(result.has_models());
        assert_eq!(result.model_count(), 1);
    }
}
