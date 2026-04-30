/// Sparse Tensor Optimization for Phase H+ 
/// 
/// Optimizes inference with sparse tensors (many zero values)
/// Common in transformer networks, attention mechanisms, etc.

use serde::{Deserialize, Serialize};

/// Sparsity pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparsityPattern {
    pub tensor_name: String,
    pub total_elements: u64,
    pub nonzero_elements: u64,
    pub sparsity_percent: f32,
    pub structured: bool,  // Regular pattern vs random
    pub block_size: u32,   // For structured sparsity
}

impl SparsityPattern {
    pub fn new(tensor_name: &str, total: u64, nonzero: u64) -> Self {
        let sparsity = 100.0 * (1.0 - (nonzero as f32 / total as f32));
        
        Self {
            tensor_name: tensor_name.to_string(),
            total_elements: total,
            nonzero_elements: nonzero,
            sparsity_percent: sparsity,
            structured: sparsity > 70.0,  // If very sparse, likely structured
            block_size: 16,
        }
    }

    /// Is this tensor worth optimizing for sparsity?
    pub fn worth_optimizing(&self) -> bool {
        self.sparsity_percent > 50.0  // More than 50% sparse
    }

    /// Estimate memory savings from sparse representation
    pub fn estimated_memory_savings_percent(&self) -> f32 {
        self.sparsity_percent * 0.8  // 80% of sparsity is saved
    }

    /// Estimate computation speedup from sparsity
    pub fn estimated_compute_speedup(&self) -> f32 {
        // Only compute nonzero elements
        (self.total_elements as f32 / self.nonzero_elements.max(1) as f32).min(10.0)
    }
}

/// Sparse tensor format
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SparseFormat {
    /// Coordinate format (row, col, value)
    COO,
    /// Compressed Row Storage
    CSR,
    /// Compressed Column Storage
    CSC,
    /// Blocked ELLpack (for GPU)
    BELL,
    /// Jagged Diagonal Storage
    JDS,
}

impl SparseFormat {
    /// Memory overhead multiplier vs dense
    pub fn memory_multiplier(&self, sparsity_percent: f32) -> f32 {
        let dense_percent = 100.0 - sparsity_percent;
        let overhead = 1.2; // 20% overhead for indices

        match self {
            SparseFormat::COO => (dense_percent / 100.0) * overhead + 0.3,
            SparseFormat::CSR => (dense_percent / 100.0) * overhead + 0.2,
            SparseFormat::CSC => (dense_percent / 100.0) * overhead + 0.2,
            SparseFormat::BELL => (dense_percent / 100.0) * overhead + 0.15,
            SparseFormat::JDS => (dense_percent / 100.0) * overhead + 0.1,
        }
    }

    /// Compute speedup for this format
    pub fn speedup_multiplier(&self, sparsity_percent: f32) -> f32 {
        let density = (100.0 - sparsity_percent) / 100.0;

        match self {
            SparseFormat::COO => density * 2.0,        // Simple but slower
            SparseFormat::CSR => density * 3.0,        // Better for rows
            SparseFormat::CSC => density * 3.0,        // Better for cols
            SparseFormat::BELL => density * 5.0,       // GPU optimized
            SparseFormat::JDS => density * 4.0,        // Balanced
        }
    }

    /// Is GPU-friendly?
    pub fn is_gpu_friendly(&self) -> bool {
        matches!(self, SparseFormat::BELL | SparseFormat::CSR)
    }
}

/// Sparse optimization config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseOptimizationConfig {
    pub enabled: bool,
    pub min_sparsity_percent: f32,
    pub preferred_format: SparseFormat,
    pub automatic_detection: bool,
    pub auto_conversion_threshold: f32,
}

impl SparseOptimizationConfig {
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            min_sparsity_percent: 40.0,
            preferred_format: SparseFormat::BELL,
            automatic_detection: true,
            auto_conversion_threshold: 0.5,
        }
    }

    pub fn balanced() -> Self {
        Self {
            enabled: true,
            min_sparsity_percent: 60.0,
            preferred_format: SparseFormat::CSR,
            automatic_detection: true,
            auto_conversion_threshold: 0.7,
        }
    }

    pub fn conservative() -> Self {
        Self {
            enabled: true,
            min_sparsity_percent: 80.0,
            preferred_format: SparseFormat::CSC,
            automatic_detection: false,
            auto_conversion_threshold: 0.9,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            min_sparsity_percent: 100.0,
            preferred_format: SparseFormat::COO,
            automatic_detection: false,
            auto_conversion_threshold: 0.0,
        }
    }
}

/// Sparse optimization engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseOptimizationEngine {
    pub config: SparseOptimizationConfig,
    pub detected_patterns: Vec<SparsityPattern>,
    pub optimized_tensors: u32,
    pub total_memory_saved_mb: f32,
    pub total_compute_speedup: f32,
}

impl SparseOptimizationEngine {
    pub fn new(config: SparseOptimizationConfig) -> Self {
        Self {
            config,
            detected_patterns: Vec::new(),
            optimized_tensors: 0,
            total_memory_saved_mb: 0.0,
            total_compute_speedup: 0.0,
        }
    }

    /// Analyze tensor for sparsity
    pub fn analyze_tensor(&mut self, pattern: SparsityPattern) -> bool {
        if !self.config.enabled {
            return false;
        }

        if pattern.sparsity_percent >= self.config.min_sparsity_percent && pattern.worth_optimizing()
        {
            let memory_savings_percent = pattern.estimated_memory_savings_percent();
            let compute_speedup = pattern.estimated_compute_speedup();

            self.total_memory_saved_mb += memory_savings_percent;
            self.total_compute_speedup += compute_speedup;
            self.optimized_tensors += 1;

            self.detected_patterns.push(pattern);
            true
        } else {
            false
        }
    }

    /// Get best format for sparsity level
    pub fn recommend_format(&self, sparsity_percent: f32) -> SparseFormat {
        if sparsity_percent > 90.0 {
            SparseFormat::BELL
        } else if sparsity_percent > 70.0 {
            SparseFormat::CSR
        } else if sparsity_percent > 50.0 {
            SparseFormat::CSC
        } else {
            SparseFormat::COO
        }
    }

    /// Get average speedup achieved
    pub fn avg_speedup(&self) -> f32 {
        if self.optimized_tensors == 0 {
            return 1.0;
        }
        self.total_compute_speedup / self.optimized_tensors as f32
    }
}

impl Default for SparseOptimizationEngine {
    fn default() -> Self {
        Self::new(SparseOptimizationConfig::balanced())
    }
}

/// Sparse matrix multiply optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseMatmulOptimization {
    pub left_sparsity: f32,
    pub right_sparsity: f32,
    pub output_sparsity: f32,
    pub estimated_speedup: f32,
    pub format: SparseFormat,
}

impl SparseMatmulOptimization {
    pub fn new(left_sparsity: f32, right_sparsity: f32) -> Self {
        // When both are sparse, output tends to be sparse too
        let output_sparsity = (left_sparsity + right_sparsity) / 2.0;
        
        // Speedup from avoiding computation with zeros
        let density = (100.0 - left_sparsity.max(right_sparsity)) / 100.0;
        let speedup = (1.0 / density.max(0.1)).min(10.0);

        Self {
            left_sparsity,
            right_sparsity,
            output_sparsity,
            estimated_speedup: speedup,
            format: SparseFormat::BELL,
        }
    }

    /// Is sparse matmul worth using?
    pub fn is_beneficial(&self) -> bool {
        self.estimated_speedup > 1.5  // At least 50% speedup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparsity_pattern_creation() {
        let pattern = SparsityPattern::new("test", 1000, 100);
        assert_eq!(pattern.sparsity_percent, 90.0);
    }

    #[test]
    fn test_sparsity_worth_optimizing() {
        let pattern = SparsityPattern::new("test", 1000, 200);
        assert!(pattern.worth_optimizing()); // 80% sparse
    }

    #[test]
    fn test_sparse_format_memory() {
        let memory_csr = SparseFormat::CSR.memory_multiplier(80.0);
        let memory_coo = SparseFormat::COO.memory_multiplier(80.0);
        assert!(memory_csr < memory_coo); // CSR is more efficient
    }

    #[test]
    fn test_sparse_format_speedup() {
        let speedup_bell = SparseFormat::BELL.speedup_multiplier(80.0);
        let speedup_coo = SparseFormat::COO.speedup_multiplier(80.0);
        assert!(speedup_bell > speedup_coo);
    }

    #[test]
    fn test_sparse_optimization_config_aggressive() {
        let config = SparseOptimizationConfig::aggressive();
        assert!(config.enabled);
        assert!(config.min_sparsity_percent < 50.0);
    }

    #[test]
    fn test_sparse_optimization_engine() {
        let config = SparseOptimizationConfig::balanced();
        let mut engine = SparseOptimizationEngine::new(config);
        
        let pattern = SparsityPattern::new("test", 10000, 1000);
        engine.analyze_tensor(pattern);
        
        assert_eq!(engine.optimized_tensors, 1);
        assert!(engine.avg_speedup() > 1.0);
    }

    #[test]
    fn test_sparse_matmul_optimization() {
        let opt = SparseMatmulOptimization::new(80.0, 75.0);
        assert!(opt.is_beneficial());
        assert!(opt.estimated_speedup > 1.0);
    }
}
