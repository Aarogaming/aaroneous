/// Kernel Fusion Module for Phase H+ Optimization
/// 
/// Combines multiple operations into single kernels to reduce:
/// - Memory bandwidth
/// - Kernel launch overhead
/// - Data movement between operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kernel operation that can be fused
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum KernelOperation {
    // Linear algebra
    Matmul,
    Gemm,      // General matrix multiply
    
    // Activation functions
    ReLU,
    Softmax,
    Sigmoid,
    GeLU,
    
    // Normalization
    BatchNorm,
    LayerNorm,
    GroupNorm,
    
    // Pooling
    MaxPool,
    AvgPool,
    
    // Elementwise
    Add,
    Multiply,
    Scale,
    
    // Reduction
    Sum,
    Mean,
    Max,
}

impl KernelOperation {
    /// Memory intensity (relative flops/byte)
    pub fn flops_per_byte(&self) -> f32 {
        match self {
            KernelOperation::Matmul => 10.0,        // Compute-bound
            KernelOperation::Add => 0.1,             // Memory-bound
            KernelOperation::ReLU => 0.05,           // Very memory-bound
            KernelOperation::Softmax => 2.0,
            KernelOperation::LayerNorm => 1.5,
            KernelOperation::BatchNorm => 1.5,
            _ => 1.0,
        }
    }

    /// Estimated latency in microseconds (on GPU)
    pub fn base_latency_us(&self) -> f32 {
        match self {
            KernelOperation::Matmul => 1000.0,
            KernelOperation::Add => 50.0,
            KernelOperation::ReLU => 10.0,
            KernelOperation::Softmax => 200.0,
            KernelOperation::LayerNorm => 150.0,
            _ => 50.0,
        }
    }

    /// Launch overhead (fixed cost per kernel)
    pub fn launch_overhead_us(&self) -> f32 {
        5.0  // ~5 microseconds per kernel launch
    }

    /// Can this operation be fused with another?
    pub fn can_fuse_with(&self, other: &KernelOperation) -> bool {
        matches!(
            (self, other),
            // Matmul + activation
            (KernelOperation::Matmul, KernelOperation::ReLU)
                | (KernelOperation::Matmul, KernelOperation::Sigmoid)
                | (KernelOperation::Matmul, KernelOperation::GeLU)
                // Activation + scale
                | (KernelOperation::ReLU, KernelOperation::Scale)
                // Norm + activation
                | (KernelOperation::LayerNorm, KernelOperation::GeLU)
                | (KernelOperation::BatchNorm, KernelOperation::ReLU)
                // Any + Add for skip connections
                | (_, KernelOperation::Add)
                | (KernelOperation::Add, _)
        )
    }
}

/// Fusion plan for a sequence of operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionPlan {
    pub operations: Vec<KernelOperation>,
    pub fused_groups: Vec<Vec<KernelOperation>>,
    pub estimated_speedup: f32,
    pub memory_saved_mb: u32,
    pub launch_overhead_reduced_us: f32,
}

impl FusionPlan {
    /// Analyze a sequence of operations for fusion opportunities
    pub fn analyze(operations: Vec<KernelOperation>) -> Self {
        let mut fused_groups = Vec::new();
        let mut current_group = Vec::new();

        for op in &operations {
            if current_group.is_empty() {
                current_group.push(*op);
            } else if current_group.last().unwrap().can_fuse_with(op) {
                current_group.push(*op);
            } else {
                fused_groups.push(current_group.clone());
                current_group = vec![*op];
            }
        }

        if !current_group.is_empty() {
            fused_groups.push(current_group);
        }

        // Calculate speedup
        let original_latency: f32 = operations.iter().map(|op| op.base_latency_us() + op.launch_overhead_us()).sum();
        let mut fused_latency = 0.0;

        for group in &fused_groups {
            let group_compute: f32 = group.iter().map(|op| op.base_latency_us()).sum();
            let launch_cost = group[0].launch_overhead_us(); // Single launch for group
            fused_latency += group_compute + launch_cost;
        }

        let speedup = original_latency / fused_latency.max(0.1);

        let launch_overhead_saved: f32 = operations.iter().skip(1).map(|op| op.launch_overhead_us()).sum();

        Self {
            operations: operations.clone(),
            fused_groups,
            estimated_speedup: speedup,
            memory_saved_mb: ((operations.len() as u32) * 10), // Estimate
            launch_overhead_reduced_us: launch_overhead_saved,
        }
    }

    /// Check if fusion is worthwhile
    pub fn is_worth_fusing(&self) -> bool {
        self.estimated_speedup > 1.05  // At least 5% speedup
    }

    /// Get fusion efficiency score
    pub fn efficiency_score(&self) -> f32 {
        (self.estimated_speedup - 1.0) * self.launch_overhead_reduced_us / 100.0
    }

    /// Decompose fused groups back into individual operations.
    ///
    /// Inverse of `analyze()` — flattens all fused groups back into
    /// the original operation sequence in order.
    pub fn defuse(&self) -> Vec<KernelOperation> {
        self.fused_groups.iter().flat_map(|group| group.iter().copied()).collect()
    }

    /// Get the number of fused groups.
    pub fn group_count(&self) -> usize {
        self.fused_groups.len()
    }

    /// Get operations in each fused group.
    pub fn groups(&self) -> &[Vec<KernelOperation>] {
        &self.fused_groups
    }
}

/// Kernel fusion engine for runtime optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelFusionEngine {
    pub fusion_plans: HashMap<String, FusionPlan>,
    pub total_operations_fused: u64,
    pub total_speedup_achieved: f64,
}

impl KernelFusionEngine {
    pub fn new() -> Self {
        Self {
            fusion_plans: HashMap::new(),
            total_operations_fused: 0,
            total_speedup_achieved: 0.0,
        }
    }

    /// Register and optimize a kernel sequence
    pub fn optimize_sequence(&mut self, name: String, ops: Vec<KernelOperation>) -> FusionPlan {
        let plan = FusionPlan::analyze(ops);
        
        if plan.is_worth_fusing() {
            self.total_operations_fused += plan.operations.len() as u64;
            self.total_speedup_achieved += plan.estimated_speedup as f64;
            self.fusion_plans.insert(name, plan.clone());
        }

        plan
    }

    /// Get average speedup achieved
    pub fn avg_speedup(&self) -> f32 {
        if self.fusion_plans.is_empty() {
            return 1.0;
        }
        let total: f32 = self.fusion_plans.values().map(|p| p.estimated_speedup).sum();
        total / self.fusion_plans.len() as f32
    }

    /// Get best optimization opportunity
    pub fn best_opportunity(&self) -> Option<(&String, &FusionPlan)> {
        self.fusion_plans.iter().max_by(|a, b| {
            a.1.efficiency_score().partial_cmp(&b.1.efficiency_score()).unwrap()
        })
    }
}

impl Default for KernelFusionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Tensor core specialization support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorCoreConfig {
    /// Use Tensor Cores for supported operations
    pub enabled: bool,
    /// Minimum tensor size to use Tensor Cores (dimensions)
    pub min_tensor_size: u32,
    /// Precision for Tensor Core operations
    pub tensor_precision: TensorPrecision,
    /// Target TFLOPS (trillion floating point operations per second)
    pub target_tflops: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TensorPrecision {
    TensorFloat32,  // 32-bit (high precision, lower throughput)
    TensorFloat16,  // 16-bit (balanced)
    TensorInt8,     // 8-bit (highest throughput)
    TensorInt4,     // 4-bit (ultra high throughput)
}

impl TensorCoreConfig {
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            min_tensor_size: 16,
            tensor_precision: TensorPrecision::TensorInt8,
            target_tflops: 10000.0, // 10 POPS on modern GPUs
        }
    }

    pub fn balanced() -> Self {
        Self {
            enabled: true,
            min_tensor_size: 32,
            tensor_precision: TensorPrecision::TensorFloat16,
            target_tflops: 5000.0,
        }
    }

    pub fn conservative() -> Self {
        Self {
            enabled: true,
            min_tensor_size: 64,
            tensor_precision: TensorPrecision::TensorFloat32,
            target_tflops: 1000.0,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            min_tensor_size: u32::MAX,
            tensor_precision: TensorPrecision::TensorFloat32,
            target_tflops: 0.0,
        }
    }

    /// Estimate throughput for this configuration
    pub fn estimated_tflops(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        
        match self.tensor_precision {
            TensorPrecision::TensorFloat32 => 1000.0,
            TensorPrecision::TensorFloat16 => 5000.0,
            TensorPrecision::TensorInt8 => 10000.0,
            TensorPrecision::TensorInt4 => 20000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_operation_flops() {
        assert!(KernelOperation::Matmul.flops_per_byte() > KernelOperation::Add.flops_per_byte());
    }

    #[test]
    fn test_kernel_fusion_matmul_relu() {
        assert!(KernelOperation::Matmul.can_fuse_with(&KernelOperation::ReLU));
    }

    #[test]
    fn test_fusion_plan_single_op() {
        let ops = vec![KernelOperation::Matmul];
        let plan = FusionPlan::analyze(ops);
        assert_eq!(plan.fused_groups.len(), 1);
    }

    #[test]
    fn test_fusion_plan_multiple_ops() {
        let ops = vec![
            KernelOperation::Matmul,
            KernelOperation::ReLU,
            KernelOperation::Add,
        ];
        let plan = FusionPlan::analyze(ops);
        assert!(plan.estimated_speedup > 1.0);
    }

    #[test]
    fn test_fusion_plan_efficiency() {
        let ops = vec![
            KernelOperation::Matmul,
            KernelOperation::ReLU,
            KernelOperation::Softmax,
        ];
        let plan = FusionPlan::analyze(ops);
        assert!(plan.efficiency_score() > 0.0);
    }

    #[test]
    fn test_kernel_fusion_engine() {
        let mut engine = KernelFusionEngine::new();
        let ops = vec![KernelOperation::Add, KernelOperation::Add, KernelOperation::Add, KernelOperation::Add];
        
        engine.optimize_sequence("matmul_relu".to_string(), ops);
        assert_eq!(engine.fusion_plans.len(), 1);
        assert!(engine.avg_speedup() > 1.0);
    }

    #[test]
    fn test_tensor_core_config_aggressive() {
        let config = TensorCoreConfig::aggressive();
        assert!(config.enabled);
        assert_eq!(config.tensor_precision, TensorPrecision::TensorInt8);
        assert!(config.estimated_tflops() > 1000.0);
    }

    #[test]
    fn test_tensor_core_config_speedup() {
        let aggressive = TensorCoreConfig::aggressive();
        let conservative = TensorCoreConfig::conservative();
        
        assert!(aggressive.estimated_tflops() > conservative.estimated_tflops());
    }
}
