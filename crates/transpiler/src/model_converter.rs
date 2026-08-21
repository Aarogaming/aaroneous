//! model_converter.rs
//! GGUF, LoRA, and Safetensors model weight metadata translation and quantization utilities.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Model Quantization Type
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantizationType {
    FP32,
    FP16,
    Q8_0,
    Q6_K,
    Q5_K_M,
    Q4_K_M,
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

    /// Creates a model manifest for a specialist foundation model
    pub fn generate_manifest(
        model_id: &str,
        quantization: QuantizationType,
    ) -> ModelManifest {
        ModelManifest {
            model_id: model_id.to_string(),
            architecture: "qwen2".to_string(),
            quantization,
            parameter_count_billions: 7.0,
            context_length: 32768,
            lora_compatible: true,
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
}
