/// Advanced Memory Pooling for Phase H+ Optimization
/// 
/// Implements pooling strategies to reduce allocation overhead
/// and fragmentation during inference

use serde::{Deserialize, Serialize};

/// Memory pool for efficient allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPool {
    pub pool_name: String,
    pub total_bytes: u64,
    pub allocated_bytes: u64,
    pub block_size: u32,
    pub blocks_in_use: u32,
    pub blocks_available: u32,
    pub allocation_count: u64,
    pub reuse_count: u64,
}

impl MemoryPool {
    pub fn new(name: &str, total_bytes: u64, block_size: u32) -> Self {
        let total_blocks = (total_bytes / block_size as u64) as u32;
        
        Self {
            pool_name: name.to_string(),
            total_bytes,
            allocated_bytes: 0,
            block_size,
            blocks_in_use: 0,
            blocks_available: total_blocks,
            allocation_count: 0,
            reuse_count: 0,
        }
    }

    /// Allocate blocks from pool
    pub fn allocate(&mut self, num_blocks: u32) -> Result<u64, String> {
        if num_blocks > self.blocks_available {
            return Err(format!(
                "Not enough blocks: need {}, have {}",
                num_blocks, self.blocks_available
            ));
        }

        let bytes = (num_blocks as u64) * (self.block_size as u64);
        self.allocated_bytes += bytes;
        self.blocks_in_use += num_blocks;
        self.blocks_available -= num_blocks;
        self.allocation_count += 1;

        // Prefer reuse of old allocations
        if self.allocation_count > 1 {
            self.reuse_count += 1;
        }

        Ok(bytes)
    }

    /// Free blocks back to pool
    pub fn free(&mut self, num_blocks: u32) {
        let bytes = (num_blocks as u64) * (self.block_size as u64);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(bytes);
        self.blocks_in_use = self.blocks_in_use.saturating_sub(num_blocks);
        self.blocks_available += num_blocks;
    }

    /// Get utilization percentage
    pub fn utilization_percent(&self) -> f32 {
        (self.blocks_in_use as f32 / (self.blocks_in_use as u32 + self.blocks_available) as f32) * 100.0
    }

    /// Get fragmentation ratio (0 = no fragmentation, 1 = fully fragmented)
    pub fn fragmentation_ratio(&self) -> f32 {
        let max_contiguous = self.blocks_available;

        if max_contiguous == 0 {
            1.0
        } else {
            1.0 - (max_contiguous as f32 / (self.block_size as f32))
        }
    }

    /// Should defragment?
    pub fn should_defragment(&self) -> bool {
        self.fragmentation_ratio() > 0.5
    }
}

/// Tiered memory pooling (fast, normal, slow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredMemoryPool {
    pub fast_pool: MemoryPool,    // L1 cache / VRAM
    pub normal_pool: MemoryPool,  // System RAM
    pub slow_pool: MemoryPool,    // Disk cache
}

impl TieredMemoryPool {
    pub fn new(fast_mb: u32, normal_mb: u32, slow_mb: u32) -> Self {
        Self {
            fast_pool: MemoryPool::new("Fast", (fast_mb as u64) * 1024 * 1024, 4096),
            normal_pool: MemoryPool::new("Normal", (normal_mb as u64) * 1024 * 1024, 4096),
            slow_pool: MemoryPool::new("Slow", (slow_mb as u64) * 1024 * 1024, 4096),
        }
    }

    /// Allocate with tiered strategy (try fast first)
    pub fn allocate(&mut self, num_blocks: u32) -> Result<PoolTier, String> {
        // Try fast tier first
        if self.fast_pool.allocate(num_blocks).is_ok() {
            return Ok(PoolTier::Fast);
        }

        // Fall back to normal
        if self.normal_pool.allocate(num_blocks).is_ok() {
            return Ok(PoolTier::Normal);
        }

        // Last resort: slow
        if self.slow_pool.allocate(num_blocks).is_ok() {
            return Ok(PoolTier::Slow);
        }

        Err("All memory pools exhausted".to_string())
    }

    /// Promote allocation from slow to fast tier
    pub fn promote(&mut self, from: PoolTier, to: PoolTier, num_blocks: u32) -> Result<(), String> {
        match (from, to) {
            (PoolTier::Slow, PoolTier::Normal) => {
                self.slow_pool.free(num_blocks);
                self.normal_pool.allocate(num_blocks)?;
                Ok(())
            }
            (PoolTier::Normal, PoolTier::Fast) => {
                self.normal_pool.free(num_blocks);
                self.fast_pool.allocate(num_blocks)?;
                Ok(())
            }
            (PoolTier::Slow, PoolTier::Fast) => {
                self.slow_pool.free(num_blocks);
                self.fast_pool.allocate(num_blocks)?;
                Ok(())
            }
            _ => Err("Cannot promote in this direction".to_string()),
        }
    }

    /// Get overall utilization
    pub fn overall_utilization_percent(&self) -> f32 {
        let total_blocks = self.fast_pool.blocks_in_use
            + self.normal_pool.blocks_in_use
            + self.slow_pool.blocks_in_use;
        let max_blocks = (self.fast_pool.total_bytes
            + self.normal_pool.total_bytes
            + self.slow_pool.total_bytes)
            / 4096;

        (total_blocks as f32 / max_blocks as f32) * 100.0
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum PoolTier {
    Fast,
    Normal,
    Slow,
}

/// Memory allocation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocationStrategy {
    pub name: String,
    pub pool_sizes: (u32, u32, u32), // (fast, normal, slow) in MB
    pub eagerness: f32, // 0.0 = conservative, 1.0 = aggressive
    pub defragment_on_threshold: f32,
    pub promote_frequently_used: bool,
}

impl MemoryAllocationStrategy {
    /// Aggressive strategy: Maximize fast memory usage
    pub fn aggressive() -> Self {
        Self {
            name: "Aggressive".to_string(),
            pool_sizes: (2048, 4096, 8192),
            eagerness: 0.9,
            defragment_on_threshold: 0.3,
            promote_frequently_used: true,
        }
    }

    /// Balanced strategy
    pub fn balanced() -> Self {
        Self {
            name: "Balanced".to_string(),
            pool_sizes: (1024, 2048, 4096),
            eagerness: 0.5,
            defragment_on_threshold: 0.5,
            promote_frequently_used: true,
        }
    }

    /// Conservative strategy: Minimal memory usage
    pub fn conservative() -> Self {
        Self {
            name: "Conservative".to_string(),
            pool_sizes: (512, 1024, 2048),
            eagerness: 0.1,
            defragment_on_threshold: 0.8,
            promote_frequently_used: false,
        }
    }
}

/// Memory access statistics for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccessStats {
    pub total_accesses: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub fast_tier_accesses: u64,
    pub normal_tier_accesses: u64,
    pub slow_tier_accesses: u64,
    pub average_access_latency_ns: f64,
}

impl MemoryAccessStats {
    pub fn new() -> Self {
        Self {
            total_accesses: 0,
            cache_hits: 0,
            cache_misses: 0,
            fast_tier_accesses: 0,
            normal_tier_accesses: 0,
            slow_tier_accesses: 0,
            average_access_latency_ns: 0.0,
        }
    }

    pub fn cache_hit_rate(&self) -> f32 {
        if self.total_accesses == 0 {
            return 0.0;
        }
        self.cache_hits as f32 / self.total_accesses as f32
    }

    pub fn record_access(&mut self, tier: PoolTier, latency_ns: f64, hit: bool) {
        self.total_accesses += 1;
        
        if hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }

        match tier {
            PoolTier::Fast => self.fast_tier_accesses += 1,
            PoolTier::Normal => self.normal_tier_accesses += 1,
            PoolTier::Slow => self.slow_tier_accesses += 1,
        }

        // Update moving average latency
        self.average_access_latency_ns =
            (self.average_access_latency_ns * 0.99) + (latency_ns * 0.01);
    }
}

impl Default for MemoryAccessStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_allocation() {
        let mut pool = MemoryPool::new("test", 1024 * 1024, 4096);
        let result = pool.allocate(10);
        assert!(result.is_ok());
        assert_eq!(pool.blocks_in_use, 10);
    }

    #[test]
    fn test_memory_pool_exhaustion() {
        let mut pool = MemoryPool::new("test", 4096, 4096);
        let result = pool.allocate(2);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_pool_utilization() {
        let mut pool = MemoryPool::new("test", 1024 * 1024, 4096);
        pool.allocate(50).unwrap();
        assert!(pool.utilization_percent() > 0.0);
    }

    #[test]
    fn test_tiered_pool_allocation() {
        let mut tiered = TieredMemoryPool::new(256, 512, 1024);
        let result = tiered.allocate(10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_allocation_strategy_aggressive() {
        let strategy = MemoryAllocationStrategy::aggressive();
        assert!(strategy.eagerness > 0.5);
        assert!(strategy.promote_frequently_used);
    }

    #[test]
    fn test_memory_access_stats() {
        let mut stats = MemoryAccessStats::new();
        stats.record_access(PoolTier::Fast, 50.0, true);
        stats.record_access(PoolTier::Normal, 200.0, false);
        
        assert_eq!(stats.total_accesses, 2);
        assert_eq!(stats.cache_hits, 1);
        assert!(stats.cache_hit_rate() > 0.0);
    }
}
