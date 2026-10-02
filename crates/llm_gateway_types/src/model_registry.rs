use serde::{Deserialize, Serialize};

/// Information about a GGUF model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub path: std::path::PathBuf,
    pub size_bytes: u64,
    pub model_type: ModelType,
    pub recommended_score: f32, // 0.0-1.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelType {
    QwenTiny,     // 0.5B - ultra-lightweight
    QwenSmall,    // 1.8B - recommended default
    QwenBase,     // 7B - good balance
    QwenLarge,    // 14B+ - high capability
    LlamaSmall,   // Llama 2 7B
    LlamaBase,    // Llama 2 13B
    MistralSmall, // Mistral 7B
    Other,        // Unknown
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
            _ if lower.contains("qwen") && (lower.contains("1.8") || lower.contains("1_8")) => {
                ModelType::QwenSmall
            }
            _ if lower.contains("qwen") && (lower.contains("7b") || lower.contains("7_b")) => {
                ModelType::QwenBase
            }
            _ if lower.contains("qwen") => ModelType::QwenLarge,
            _ if lower.contains("llama") && lower.contains("7") => ModelType::LlamaSmall,
            _ if lower.contains("llama") && (lower.contains("13") || lower.contains("13b")) => {
                ModelType::LlamaBase
            }
            _ if lower.contains("mistral") => ModelType::MistralSmall,
            _ => ModelType::Other,
        }
    }

    pub fn recommended_score(&self) -> f32 {
        match self {
            ModelType::QwenSmall => 0.95,
            ModelType::QwenBase => 0.85,
            ModelType::QwenTiny => 0.70,
            ModelType::QwenLarge => 0.75,
            ModelType::MistralSmall => 0.80,
            ModelType::LlamaSmall => 0.75,
            ModelType::LlamaBase => 0.70,
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
