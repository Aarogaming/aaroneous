/// Model Quantization System for Memory Efficiency
/// 
/// Reduces model sizes by 2-8x through bit-width reduction:
/// - INT4: 4-bit integers (8x compression)
/// - INT8: 8-bit integers (4x compression)
/// - FP16: 16-bit floats (2x compression)
/// - Original: 32-bit floats

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantizationType {
    /// No quantization - full precision FP32
    None,
    /// 16-bit floating point (2x compression)
    FP16,
    /// 8-bit signed integers (4x compression)
    INT8,
    /// 4-bit integers (8x compression)
    INT4,
}

impl QuantizationType {
    /// Compression ratio compared to FP32
    pub fn compression_ratio(&self) -> f32 {
        match self {
            QuantizationType::None => 1.0,
            QuantizationType::FP16 => 2.0,
            QuantizationType::INT8 => 4.0,
            QuantizationType::INT4 => 8.0,
        }
    }

    /// Inference speed relative to FP32 (on quantization-aware hardware)
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            QuantizationType::None => 1.0,
            QuantizationType::FP16 => 1.2,    // Faster on modern GPUs
            QuantizationType::INT8 => 3.0,    // Integer ops are fast
            QuantizationType::INT4 => 6.0,    // Very fast but less accurate
        }
    }

    /// Memory size in MB for a specialist model (e.g., 1GB base)
    pub fn model_size_mb(&self, base_mb: u32) -> u32 {
        (base_mb as f32 / self.compression_ratio()) as u32
    }

    /// Name for logging/display
    pub fn name(&self) -> &'static str {
        match self {
            QuantizationType::None => "FP32",
            QuantizationType::FP16 => "FP16",
            QuantizationType::INT8 => "INT8",
            QuantizationType::INT4 => "INT4",
        }
    }
}

/// Quantization strategy for a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationStrategy {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub quantization_type: QuantizationType,
    /// Accuracy loss estimate (0.0 = no loss, 0.2 = 20% accuracy drop)
    pub accuracy_loss: f32,
    /// Performance gain estimate (1.2 = 20% faster)
    pub performance_gain: f32,
    /// Is enabled for this deployment
    pub enabled: bool,
    /// Fallback if quantized model fails
    pub fallback: Option<Box<QuantizationStrategy>>,
}

impl QuantizationStrategy {
    /// Create a new strategy
    pub fn new(specialist_id: crate::federation::specialist::SpecialistId, qtype: QuantizationType) -> Self {
        let (accuracy_loss, performance_gain) = match qtype {
            QuantizationType::None => (0.0, 1.0),
            QuantizationType::FP16 => (0.01, 1.2),    // Minimal loss
            QuantizationType::INT8 => (0.05, 3.0),    // Small loss
            QuantizationType::INT4 => (0.15, 6.0),    // Noticeable loss
        };

        Self {
            specialist_id,
            quantization_type: qtype,
            accuracy_loss,
            performance_gain,
            enabled: true,
            fallback: None,
        }
    }

    /// Set fallback strategy
    pub fn with_fallback(mut self, fallback: QuantizationStrategy) -> Self {
        self.fallback = Some(Box::new(fallback));
        self
    }

    /// Score this strategy (higher = better overall)
    pub fn score(&self) -> f32 {
        // Prefer faster inference with minimal accuracy loss
        let speed_score = self.performance_gain;
        let accuracy_penalty = 1.0 - (self.accuracy_loss * 10.0); // Accuracy loss is weighted
        speed_score * accuracy_penalty
    }
}

/// Quantization configuration for entire deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationConfig {
    pub strategies: HashMap<crate::federation::specialist::SpecialistId, QuantizationStrategy>,
    pub default_strategy: QuantizationStrategy,
}

impl QuantizationConfig {
    /// Create config for mobile deployment (aggressive quantization)
    pub fn mobile() -> Self {
        let mut strategies = HashMap::new();

        // Mobile specialization: INT8 for most, FP16 for latency-critical
        strategies.insert(
            crate::federation::specialist::SpecialistId::Sentinel,
            QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Sentinel,
                QuantizationType::FP16,
            )
            .with_fallback(QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Sentinel,
                QuantizationType::INT8,
            )),
        );

        strategies.insert(
            crate::federation::specialist::SpecialistId::Omnipresent,
            QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Omnipresent,
                QuantizationType::INT8,
            ),
        );

        strategies.insert(
            crate::federation::specialist::SpecialistId::Symbiotic,
            QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Symbiotic,
                QuantizationType::INT8,
            ),
        );

        Self {
            strategies,
            default_strategy: QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Sentinel,
                QuantizationType::INT8,
            ),
        }
    }

    /// Create config for desktop (balanced)
    pub fn desktop() -> Self {
        let mut strategies = HashMap::new();

        strategies.insert(
            crate::federation::specialist::SpecialistId::Sentinel,
            QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Sentinel,
                QuantizationType::FP16,
            ),
        );

        strategies.insert(
            crate::federation::specialist::SpecialistId::Visionary,
            QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Visionary,
                QuantizationType::FP16,
            ),
        );

        Self {
            strategies,
            default_strategy: QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Sentinel,
                QuantizationType::FP16,
            ),
        }
    }

    /// Create config for server (minimal quantization)
    pub fn server() -> Self {
        let strategies = HashMap::new();

        Self {
            strategies,
            default_strategy: QuantizationStrategy::new(
                crate::federation::specialist::SpecialistId::Sentinel,
                QuantizationType::None,
            ),
        }
    }

    /// Get strategy for a specialist
    pub fn strategy_for(
        &self,
        specialist_id: crate::federation::specialist::SpecialistId,
    ) -> &QuantizationStrategy {
        self.strategies.get(&specialist_id).unwrap_or(&self.default_strategy)
    }

    /// Calculate total memory savings
    pub fn total_memory_saved_mb(&self, base_models_mb: u32) -> u32 {
        let full_size = base_models_mb;
        let quantized_size: u32 = (0..6)
            .map(|i| {
                let id = match i {
                    0 => crate::federation::specialist::SpecialistId::Sentinel,
                    1 => crate::federation::specialist::SpecialistId::Visionary,
                    2 => crate::federation::specialist::SpecialistId::Omnipresent,
                    3 => crate::federation::specialist::SpecialistId::Symbiotic,
                    4 => crate::federation::specialist::SpecialistId::Phygital,
                    _ => crate::federation::specialist::SpecialistId::Archivist,
                };
                let strategy = self.strategy_for(id);
                let model_base = id.model_size_mb();
                strategy.quantization_type.model_size_mb(model_base)
            })
            .sum();

        full_size - quantized_size
    }
}

/// Quantized model representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedModel {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub quantization_type: QuantizationType,
    pub original_size_mb: u32,
    pub quantized_size_mb: u32,
    pub accuracy_loss_percent: f32,
    pub inference_speedup: f32,
    pub checksum: String,
}

impl QuantizedModel {
    /// Create a new quantized model
    pub fn new(
        specialist_id: crate::federation::specialist::SpecialistId,
        quantization_type: QuantizationType,
        original_size_mb: u32,
    ) -> Self {
        let quantized_size_mb = quantization_type.model_size_mb(original_size_mb);
        let accuracy_loss_percent = match quantization_type {
            QuantizationType::None => 0.0,
            QuantizationType::FP16 => 1.0,
            QuantizationType::INT8 => 5.0,
            QuantizationType::INT4 => 15.0,
        };

        Self {
            specialist_id,
            quantization_type,
            original_size_mb,
            quantized_size_mb,
            accuracy_loss_percent,
            inference_speedup: quantization_type.speed_multiplier(),
            checksum: format!("qm_{:?}_{}", specialist_id, quantization_type as u32),
        }
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f32 {
        self.original_size_mb as f32 / self.quantized_size_mb as f32
    }

    /// Check if this quantization is acceptable
    pub fn is_acceptable(&self, max_accuracy_loss: f32) -> bool {
        self.accuracy_loss_percent <= max_accuracy_loss * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_type_compression() {
        assert_eq!(QuantizationType::None.compression_ratio(), 1.0);
        assert_eq!(QuantizationType::FP16.compression_ratio(), 2.0);
        assert_eq!(QuantizationType::INT8.compression_ratio(), 4.0);
        assert_eq!(QuantizationType::INT4.compression_ratio(), 8.0);
    }

    #[test]
    fn test_quantization_type_speed() {
        assert!(QuantizationType::INT8.speed_multiplier() > QuantizationType::FP16.speed_multiplier());
        assert!(QuantizationType::INT4.speed_multiplier() > QuantizationType::INT8.speed_multiplier());
    }

    #[test]
    fn test_model_size_calculation() {
        let base = 1000u32;
        assert_eq!(QuantizationType::FP16.model_size_mb(base), 500);
        assert_eq!(QuantizationType::INT8.model_size_mb(base), 250);
        assert_eq!(QuantizationType::INT4.model_size_mb(base), 125);
    }

    #[test]
    fn test_quantization_strategy_score() {
        let strategy_fp16 = QuantizationStrategy::new(
            crate::federation::specialist::SpecialistId::Visionary,
            QuantizationType::FP16,
        );
        let strategy_int8 = QuantizationStrategy::new(
            crate::federation::specialist::SpecialistId::Visionary,
            QuantizationType::INT8,
        );

        // Both should have positive scores
        assert!(strategy_fp16.score() > 0.0);
        assert!(strategy_int8.score() > 0.0);

        // INT8 should score higher (better speed for small accuracy loss)
        assert!(strategy_int8.score() > strategy_fp16.score());
    }

    #[test]
    fn test_quantization_config_mobile() {
        let config = QuantizationConfig::mobile();
        assert!(!config.strategies.is_empty());

        // Mobile should have strategies for multiple specialists
        assert!(config.strategies.contains_key(&crate::federation::specialist::SpecialistId::Sentinel));
    }

    #[test]
    fn test_quantization_config_desktop() {
        let config = QuantizationConfig::desktop();
        assert!(!config.strategies.is_empty());
    }

    #[test]
    fn test_quantized_model_compression() {
        let model = QuantizedModel::new(
            crate::federation::specialist::SpecialistId::Visionary,
            QuantizationType::INT8,
            1000,
        );

        assert_eq!(model.original_size_mb, 1000);
        assert_eq!(model.quantized_size_mb, 250);
        assert_eq!(model.compression_ratio(), 4.0);
    }

    #[test]
    fn test_quantized_model_acceptable() {
        let model = QuantizedModel::new(
            crate::federation::specialist::SpecialistId::Visionary,
            QuantizationType::INT8,
            1000,
        );

        // 5% accuracy loss should be acceptable at 10% threshold
        assert!(model.is_acceptable(0.10));
        // But not at 1% threshold
        assert!(!model.is_acceptable(0.01));
    }
}
