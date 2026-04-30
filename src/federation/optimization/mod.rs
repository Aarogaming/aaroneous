/// Phase H: Optimization Module
/// 
/// Provides performance optimization capabilities for the Aaroneous Federation:
/// - Model quantization (INT4, INT8, FP16)
/// - GPU acceleration (CUDA, Metal, Intel Arc)
/// - Cache warming strategies
/// - Batch processing for proposal aggregation

pub mod quantization;
pub mod gpu_acceleration;
pub mod cache_warming;
pub mod batch_processing;
pub mod kernel_fusion;
pub mod memory_pooling;
pub mod sparse_optimization;

pub use quantization::{
    QuantizationType, QuantizationStrategy, QuantizationConfig, QuantizedModel,
};
pub use gpu_acceleration::{
    GPUType, GPUInfo, GPUMemoryManager, GPUInferenceContext, GPUAccelerationStrategy,
};
pub use cache_warming::{
    CacheWarmingStrategy, AccessPattern, CacheWarmingTracker, WarmingSchedule,
};
pub use batch_processing::{
    BatchConfig, ProposalBatch, BatchManager,
};

/// Optimization profile for different deployment scenarios
#[derive(Debug, Clone)]
pub enum OptimizationProfile {
    /// Mobile: Aggressive quantization (INT8), cache warming, batching
    Mobile,
    /// Tablet: Balanced quantization (FP16), GPU if available
    Tablet,
    /// Desktop: FP16 quantization, full GPU support, aggressive batching
    Desktop,
    /// Server: Minimal optimization, focus on throughput
    Server,
    /// Custom: User-specified settings
    Custom {
        quantization: QuantizationConfig,
        gpu_strategy: gpu_acceleration::GPUAccelerationStrategy,
        cache_warming: CacheWarmingStrategy,
        batch_config: BatchConfig,
    },
}

impl OptimizationProfile {
    /// Get quantization config for profile
    pub fn quantization_config(&self) -> QuantizationConfig {
        match self {
            OptimizationProfile::Mobile => QuantizationConfig::mobile(),
            OptimizationProfile::Tablet => QuantizationConfig::desktop(),
            OptimizationProfile::Desktop => QuantizationConfig::desktop(),
            OptimizationProfile::Server => QuantizationConfig::server(),
            OptimizationProfile::Custom { quantization, .. } => quantization.clone(),
        }
    }

    /// Get GPU strategy for profile
    pub fn gpu_strategy(&self) -> gpu_acceleration::GPUAccelerationStrategy {
        match self {
            OptimizationProfile::Mobile => gpu_acceleration::GPUAccelerationStrategy::conservative(),
            OptimizationProfile::Tablet => gpu_acceleration::GPUAccelerationStrategy::balanced(),
            OptimizationProfile::Desktop => gpu_acceleration::GPUAccelerationStrategy::aggressive(),
            OptimizationProfile::Server => gpu_acceleration::GPUAccelerationStrategy::disabled(),
            OptimizationProfile::Custom { gpu_strategy, .. } => gpu_strategy.clone(),
        }
    }

    /// Get cache warming strategy for profile
    pub fn cache_warming_strategy(&self) -> CacheWarmingStrategy {
        match self {
            OptimizationProfile::Mobile => CacheWarmingStrategy::minimal(),
            OptimizationProfile::Tablet => CacheWarmingStrategy::balanced(),
            OptimizationProfile::Desktop => CacheWarmingStrategy::aggressive(),
            OptimizationProfile::Server => CacheWarmingStrategy::minimal(),
            OptimizationProfile::Custom { cache_warming, .. } => cache_warming.clone(),
        }
    }

    /// Get batch config for profile
    pub fn batch_config(&self) -> BatchConfig {
        match self {
            OptimizationProfile::Mobile => BatchConfig::conservative(),
            OptimizationProfile::Tablet => BatchConfig::balanced(),
            OptimizationProfile::Desktop => BatchConfig::aggressive(),
            OptimizationProfile::Server => BatchConfig::aggressive(),
            OptimizationProfile::Custom { batch_config, .. } => batch_config.clone(),
        }
    }
}

/// Optimization summary and statistics
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    pub quantization_memory_saved_mb: u32,
    pub quantization_speedup: f32,
    pub gpu_accelerated: bool,
    pub gpu_memory_usage_percent: f32,
    pub cache_hit_rate: f32,
    pub avg_batch_size: f32,
    pub throughput_proposals_per_sec: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_profile_mobile() {
        let profile = OptimizationProfile::Mobile;
        let quant = profile.quantization_config();
        assert!(!quant.strategies.is_empty());
    }

    #[test]
    fn test_optimization_profile_desktop() {
        let profile = OptimizationProfile::Desktop;
        let gpu = profile.gpu_strategy();
        assert!(gpu.enabled);
    }

    #[test]
    fn test_optimization_profile_server() {
        let profile = OptimizationProfile::Server;
        let gpu = profile.gpu_strategy();
        assert!(!gpu.enabled);
    }
}
