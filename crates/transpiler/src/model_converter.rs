//! model_converter.rs
//! GGUF, LoRA, and Safetensors model weight metadata translation and quantization utilities.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Model Quantization Type
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantizationType {
    FP32,
    FP16,
    Q8_0,
    Q6_K,
    Q5_K_M,
    Q4_K_M,
}

impl QuantizationType {
    /// Nominal bits per weight for memory estimation
    pub fn bits_per_weight(&self) -> f32 {
        match self {
            Self::FP32 => 32.0,
            Self::FP16 => 16.0,
            Self::Q8_0 => 8.5,
            Self::Q6_K => 6.6,
            Self::Q5_K_M => 5.5,
            Self::Q4_K_M => 4.5,
        }
    }
}

/// Metadata describing a quantized model file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub model_id: String,
    pub architecture: String,
    pub quantization: QuantizationType,
    pub parameter_count_billions: f32,
    pub context_length: usize,
    pub lora_compatible: bool,
}

impl ModelManifest {
    /// Estimates host/device VRAM requirement in megabytes
    pub fn estimate_memory_mb(&self) -> usize {
        let weight_bytes = (self.parameter_count_billions * 1e9 * (self.quantization.bits_per_weight() / 8.0)) as usize;
        // Include KV-cache overhead buffer based on context length
        let kv_cache_mb = (self.context_length * 2) / 1024;
        (weight_bytes / (1024 * 1024)) + kv_cache_mb
    }
}

/// Model Conversion & Quantization Engine
pub struct ModelConverter;

impl ModelConverter {
    /// Ingests model file metadata and plans an optimal quantization profile
    pub fn plan_quantization(
        _model_name: &str,
        available_vram_gb: f32,
    ) -> Result<QuantizationType> {
        let quant = if available_vram_gb >= 16.0 {
            QuantizationType::Q8_0
        } else if available_vram_gb >= 8.0 {
            QuantizationType::Q6_K // Primary default for 7B foundation models
        } else if available_vram_gb >= 6.0 {
            QuantizationType::Q5_K_M
        } else {
            QuantizationType::Q4_K_M
        };

        Ok(quant)
    }

    /// Deduces model family architecture from model identifier string
    pub fn deduce_architecture(model_id: &str) -> &'static str {
        let lower = model_id.to_lowercase();
        if lower.contains("llama-3") || lower.contains("llama3") {
            "llama3"
        } else if lower.contains("llama") {
            "llama"
        } else if lower.contains("mistral") || lower.contains("mixtral") {
            "mistral"
        } else if lower.contains("phi-3") || lower.contains("phi3") || lower.contains("phi-4") {
            "phi3"
        } else if lower.contains("gemma") {
            "gemma"
        } else if lower.contains("deepseek") {
            "deepseek"
        } else {
            "qwen2" // Foundation baseline default
        }
    }

    /// Deduces parameter count (in billions) from model identifier
    pub fn deduce_parameter_count(model_id: &str) -> f32 {
        let lower = model_id.to_lowercase();
        if lower.contains("0.5b") {
            0.5
        } else if lower.contains("1.5b") {
            1.5
        } else if lower.contains("1b") {
            1.0
        } else if lower.contains("3.8b") {
            3.8
        } else if lower.contains("3b") {
            3.0
        } else if lower.contains("7b") {
            7.0
        } else if lower.contains("8b") {
            8.0
        } else if lower.contains("9b") {
            9.0
        } else if lower.contains("14b") {
            14.0
        } else if lower.contains("27b") {
            27.0
        } else if lower.contains("32b") {
            32.0
        } else if lower.contains("70b") || lower.contains("72b") {
            70.0
        } else {
            7.0
        }
    }

    /// Creates a model manifest for a specialist foundation model
    pub fn generate_manifest(
        model_id: &str,
        quantization: QuantizationType,
    ) -> ModelManifest {
        let arch = Self::deduce_architecture(model_id);
        let params = Self::deduce_parameter_count(model_id);
        let context = if arch == "llama3" || arch == "qwen2" || arch == "mistral" {
            32768
        } else {
            8192
        };

        ModelManifest {
            model_id: model_id.to_string(),
            architecture: arch.to_string(),
            quantization,
            parameter_count_billions: params,
            context_length: context,
            lora_compatible: true,
        }
    }
}

/// Builder for explicit ModelManifest configuration
#[derive(Debug, Clone)]
pub struct ModelManifestBuilder {
    model_id: String,
    architecture: String,
    quantization: QuantizationType,
    parameter_count_billions: f32,
    context_length: usize,
    lora_compatible: bool,
}

impl ModelManifestBuilder {
    pub fn new(model_id: &str, quantization: QuantizationType) -> Self {
        let base = ModelConverter::generate_manifest(model_id, quantization);
        Self {
            model_id: base.model_id,
            architecture: base.architecture,
            quantization: base.quantization,
            parameter_count_billions: base.parameter_count_billions,
            context_length: base.context_length,
            lora_compatible: base.lora_compatible,
        }
    }

    pub fn architecture(mut self, arch: &str) -> Self {
        self.architecture = arch.to_string();
        self
    }

    pub fn parameter_count_billions(mut self, count: f32) -> Self {
        self.parameter_count_billions = count;
        self
    }

    pub fn context_length(mut self, len: usize) -> Self {
        self.context_length = len;
        self
    }

    pub fn lora_compatible(mut self, compatible: bool) -> Self {
        self.lora_compatible = compatible;
        self
    }

    pub fn build(self) -> ModelManifest {
        ModelManifest {
            model_id: self.model_id,
            architecture: self.architecture,
            quantization: self.quantization,
            parameter_count_billions: self.parameter_count_billions,
            context_length: self.context_length,
            lora_compatible: self.lora_compatible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_planning() {
        let quant = ModelConverter::plan_quantization("qwen2.5-7b", 10.0).unwrap();
        assert_eq!(quant, QuantizationType::Q6_K);

        let low_vram = ModelConverter::plan_quantization("qwen2.5-7b", 4.0).unwrap();
        assert_eq!(low_vram, QuantizationType::Q4_K_M);
    }

    #[test]
    fn test_manifest_generation_and_deduction() {
        let qwen = ModelConverter::generate_manifest("qwen2.5-7b-instruct", QuantizationType::Q6_K);
        assert_eq!(qwen.architecture, "qwen2");
        assert_eq!(qwen.parameter_count_billions, 7.0);

        let llama = ModelConverter::generate_manifest("Meta-Llama-3-8B-Instruct", QuantizationType::Q8_0);
        assert_eq!(llama.architecture, "llama3");
        assert_eq!(llama.parameter_count_billions, 8.0);
    }

    #[test]
    fn test_manifest_builder() {
        let manifest = ModelManifestBuilder::new("custom-model", QuantizationType::FP16)
            .architecture("deepseek")
            .parameter_count_billions(67.0)
            .context_length(65536)
            .build();

        assert_eq!(manifest.architecture, "deepseek");
        assert_eq!(manifest.parameter_count_billions, 67.0);
        assert_eq!(manifest.context_length, 65536);
    }

    #[test]
    fn test_model_manifest_memory_estimation() {
        let manifest = ModelConverter::generate_manifest("qwen2.5-7b-instruct", QuantizationType::Q4_K_M);
        let mb = manifest.estimate_memory_mb();
        // 7B at 4.5 bits/weight ≈ 3937 MB + KV overhead (64 MB)
        assert!(mb > 3500 && mb < 4500);
    }
}
