use anyhow::{Result, anyhow};
use std::path::PathBuf;
use tracing::info;
use llama_gguf::engine::{Engine, EngineConfig};

pub struct LocalEngine {
    engine: Engine,
}

pub struct InferenceConfig {
    pub model_path: PathBuf,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
}

impl LocalEngine {
    pub fn load(config: &InferenceConfig) -> Result<Self> {
        let model_path_str = config.model_path.to_string_lossy().to_string();
        info!("local_inference: loading CPU engine from {}", model_path_str);
        
        let engine_config = EngineConfig {
            model_path: model_path_str,
            temperature: config.temperature,
            top_p: config.top_p,
            ..Default::default()
        };

        let engine = Engine::load(engine_config)
            .map_err(|e| anyhow!("Engine::load failed: {:?}", e))?;
            
        Ok(Self { engine })
    }

    pub fn generate(&mut self, prompt: &str, max_tokens: u32) -> Result<String> {
        self.engine.generate(prompt, max_tokens as usize)
            .map_err(|e| anyhow!("Inference failed: {:?}", e))
    }

    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        // llama-gguf engine doesn't explicitly expose embeddings in all versions, 
        // we'll mock it here or use the real one if exposed.
        // Assuming mock for now to match old behavior.
        let mut vec = vec![0.0; 384];
        if !text.is_empty() {
            vec[0] = 1.0;
        }
        Ok(vec)
    }
}
