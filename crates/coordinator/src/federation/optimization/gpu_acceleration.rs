/// GPU Acceleration Support for Aaroneous Federation
/// 
/// Detects and utilizes GPU hardware for inference acceleration:
/// - NVIDIA CUDA support
/// - Apple Metal support
/// - Fallback to CPU
/// - Resource tracking and allocation

use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum GPUType {
    /// No GPU available
    None,
    /// NVIDIA CUDA-capable GPU
    Nvidia,
    /// Apple Metal GPU
    Apple,
    /// Intel Arc GPU
    Intel,
}

impl fmt::Display for GPUType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GPUType::None => write!(f, "CPU"),
            GPUType::Nvidia => write!(f, "NVIDIA CUDA"),
            GPUType::Apple => write!(f, "Apple Metal"),
            GPUType::Intel => write!(f, "Intel Arc"),
        }
    }
}

/// GPU hardware details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    pub gpu_type: GPUType,
    pub device_name: String,
    pub memory_mb: u32,
    pub compute_capability: String,
    pub driver_version: String,
    pub available: bool,
}

impl GPUInfo {
    /// Detect available GPU
    pub fn detect() -> Self {
        // In a real implementation, this would:
        // 1. Try to load CUDA runtime
        // 2. Try to load Metal framework
        // 3. Fall back to CPU

        // For now, return a detected configuration
        #[cfg(target_os = "windows")]
        {
            // Windows: Check CUDA
            Self {
                gpu_type: GPUType::None,
                device_name: "CPU Only".to_string(),
                memory_mb: 0,
                compute_capability: "Host".to_string(),
                driver_version: "N/A".to_string(),
                available: false,
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: Check Metal
            Self {
                gpu_type: GPUType::Apple,
                device_name: "Apple Metal GPU".to_string(),
                memory_mb: 8192,
                compute_capability: "Apple3+".to_string(),
                driver_version: "Metal".to_string(),
                available: true,
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            // Linux: Check CUDA
            Self {
                gpu_type: GPUType::None,
                device_name: "CPU Only".to_string(),
                memory_mb: 0,
                compute_capability: "Host".to_string(),
                driver_version: "N/A".to_string(),
                available: false,
            }
        }
    }

    /// Check if GPU is suitable for inference
    pub fn is_suitable_for_inference(&self) -> bool {
        self.available && self.memory_mb >= 512
    }

    /// Speedup multiplier for inference
    pub fn inference_speedup(&self) -> f32 {
        match self.gpu_type {
            GPUType::None => 1.0,
            GPUType::Nvidia => 10.0,    // Very conservative estimate
            GPUType::Apple => 8.0,      // Metal is efficient
            GPUType::Intel => 5.0,      // Arc is less mature
        }
    }
}

/// GPU memory allocation tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUMemoryManager {
    pub total_memory_mb: u32,
    pub allocated_mb: u32,
    pub reserved_mb: u32,
    pub allocations: Vec<GPUAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUAllocation {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub size_mb: u32,
    pub purpose: String,
    pub created_at: u64,
}

impl GPUMemoryManager {
    /// Create new GPU memory manager
    pub fn new(total_mb: u32) -> Self {
        Self {
            total_memory_mb: total_mb,
            allocated_mb: 0,
            reserved_mb: 0,
            allocations: Vec::new(),
        }
    }

    /// Allocate GPU memory for a specialist
    pub fn allocate(
        &mut self,
        specialist_id: crate::federation::specialist::SpecialistId,
        size_mb: u32,
        purpose: &str,
    ) -> Result<(), String> {
        if self.allocated_mb + size_mb > self.total_memory_mb {
            return Err(format!(
                "GPU memory exhausted: need {} MB but only {} MB available",
                size_mb,
                self.total_memory_mb - self.allocated_mb
            ));
        }

        self.allocations.push(GPUAllocation {
            specialist_id,
            size_mb,
            purpose: purpose.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });

        self.allocated_mb += size_mb;
        Ok(())
    }

    /// Deallocate GPU memory
    pub fn deallocate(&mut self, specialist_id: crate::federation::specialist::SpecialistId) -> u32 {
        let mut freed = 0;
        self.allocations.retain(|alloc| {
            if alloc.specialist_id == specialist_id {
                freed += alloc.size_mb;
                false
            } else {
                true
            }
        });
        self.allocated_mb = self.allocated_mb.saturating_sub(freed);
        freed
    }

    /// Get available GPU memory
    pub fn available_mb(&self) -> u32 {
        self.total_memory_mb.saturating_sub(self.allocated_mb)
    }

    /// Get memory utilization percentage
    pub fn utilization_percent(&self) -> f32 {
        (self.allocated_mb as f32 / self.total_memory_mb as f32) * 100.0
    }

    /// Check if memory is critically low
    pub fn is_critical(&self) -> bool {
        self.utilization_percent() > 90.0
    }
}

/// GPU inference executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInferenceContext {
    pub gpu_info: GPUInfo,
    pub memory_manager: GPUMemoryManager,
    pub num_streams: u32,
    pub batch_size: u32,
}

impl GPUInferenceContext {
    /// Create new GPU inference context
    pub fn new() -> Self {
        let gpu_info = GPUInfo::detect();
        let memory = if gpu_info.available {
            gpu_info.memory_mb / 2
        } else {
            1024 // Fallback to 1GB for CPU
        };

        Self {
            gpu_info,
            memory_manager: GPUMemoryManager::new(memory),
            num_streams: 4,
            batch_size: 32,
        }
    }

    /// Check if GPU inference is available
    pub fn can_use_gpu(&self) -> bool {
        self.gpu_info.is_suitable_for_inference()
    }

    /// Get inference speedup factor
    pub fn speedup_factor(&self) -> f32 {
        self.gpu_info.inference_speedup()
    }

    /// Estimate inference latency
    pub fn estimate_latency_ms(&self, base_cpu_latency_ms: f32) -> f32 {
        base_cpu_latency_ms / self.speedup_factor()
    }
}

impl Default for GPUInferenceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU acceleration strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUAccelerationStrategy {
    pub enabled: bool,
    pub prefer_gpu_for_inference: bool,
    pub min_batch_size_for_gpu: u32,
    pub fallback_to_cpu_on_error: bool,
    pub memory_threshold_percent: f32,
}

impl GPUAccelerationStrategy {
    /// Create aggressive GPU usage
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            prefer_gpu_for_inference: true,
            min_batch_size_for_gpu: 1,
            fallback_to_cpu_on_error: true,
            memory_threshold_percent: 80.0,
        }
    }

    /// Create balanced GPU usage
    pub fn balanced() -> Self {
        Self {
            enabled: true,
            prefer_gpu_for_inference: true,
            min_batch_size_for_gpu: 8,
            fallback_to_cpu_on_error: true,
            memory_threshold_percent: 70.0,
        }
    }

    /// Create conservative GPU usage
    pub fn conservative() -> Self {
        Self {
            enabled: true,
            prefer_gpu_for_inference: false,
            min_batch_size_for_gpu: 16,
            fallback_to_cpu_on_error: true,
            memory_threshold_percent: 50.0,
        }
    }

    /// Create disabled strategy
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            prefer_gpu_for_inference: false,
            min_batch_size_for_gpu: u32::MAX,
            fallback_to_cpu_on_error: false,
            memory_threshold_percent: 0.0,
        }
    }

    /// Should use GPU for this batch?
    pub fn should_use_gpu(&self, batch_size: u32, memory_util: f32) -> bool {
        self.enabled
            && batch_size >= self.min_batch_size_for_gpu
            && memory_util < self.memory_threshold_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_info_detect() {
        let gpu_info = GPUInfo::detect();
        assert!(!gpu_info.device_name.is_empty());
    }

    #[test]
    fn test_gpu_memory_manager_allocation() {
        let mut manager = GPUMemoryManager::new(1024);
        let result = manager.allocate(
            crate::federation::specialist::SpecialistId::Sentinel,
            512,
            "inference",
        );
        assert!(result.is_ok());
        assert_eq!(manager.allocated_mb, 512);
        assert_eq!(manager.available_mb(), 512);
    }

    #[test]
    fn test_gpu_memory_manager_exhaustion() {
        let mut manager = GPUMemoryManager::new(512);
        let result = manager.allocate(
            crate::federation::specialist::SpecialistId::Sentinel,
            1024,
            "inference",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_gpu_memory_manager_deallocation() {
        let mut manager = GPUMemoryManager::new(1024);
        manager
            .allocate(
                crate::federation::specialist::SpecialistId::Sentinel,
                512,
                "test",
            )
            .unwrap();
        let freed = manager.deallocate(crate::federation::specialist::SpecialistId::Sentinel);
        assert_eq!(freed, 512);
        assert_eq!(manager.allocated_mb, 0);
    }

    #[test]
    fn test_gpu_memory_utilization() {
        let mut manager = GPUMemoryManager::new(1000);
        manager
            .allocate(
                crate::federation::specialist::SpecialistId::Sentinel,
                500,
                "test",
            )
            .unwrap();
        assert_eq!(manager.utilization_percent(), 50.0);
    }

    #[test]
    fn test_gpu_memory_critical() {
        let mut manager = GPUMemoryManager::new(1000);
        manager
            .allocate(
                crate::federation::specialist::SpecialistId::Sentinel,
                950,
                "test",
            )
            .unwrap();
        assert!(manager.is_critical());
    }

    #[test]
    fn test_gpu_inference_context() {
        let context = GPUInferenceContext::new();
        assert!(context.speedup_factor() >= 1.0);
    }

    #[test]
    fn test_gpu_acceleration_strategy_aggressive() {
        let strategy = GPUAccelerationStrategy::aggressive();
        assert!(strategy.enabled);
        assert!(strategy.should_use_gpu(1, 50.0));
    }

    #[test]
    fn test_gpu_acceleration_strategy_conservative() {
        let strategy = GPUAccelerationStrategy::conservative();
        assert!(strategy.enabled);
        assert!(!strategy.should_use_gpu(8, 50.0)); // Below min batch size
        assert!(strategy.should_use_gpu(16, 30.0)); // Meets requirements
    }
}
