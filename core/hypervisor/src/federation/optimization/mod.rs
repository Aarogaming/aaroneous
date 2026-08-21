// Optimization stub
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantizationType {
    Q4_0,
    Q5_0,
    Q8_0,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationConfig {
    pub quant_type: QuantizationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationStrategy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo;

pub struct GPUMemoryManager;
impl Default for GPUMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GPUMemoryManager {
    pub fn new() -> Self {
        Self
    }
}

pub struct GPUInferenceContext;
pub struct GPUAccelerationStrategy;
pub struct CacheWarmingStrategy;
pub struct AccessPattern;
pub struct CacheWarmingTracker;
pub struct WarmingSchedule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig;

pub struct ProposalBatch;
pub struct BatchManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationProfile;

impl OptimizationProfile {
    pub fn detect() -> Self {
        OptimizationProfile
    }
    pub fn resource_caps(&self) -> crate::federation::specialist::SystemResources {
        crate::federation::specialist::SystemResources::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStats;
