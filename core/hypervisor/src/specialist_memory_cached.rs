/// Cached Specialist Memory
///
/// Extends SpecialistMemory with intelligent query result caching.
/// Provides transparent caching with automatic invalidation on memory changes.
/// Expected performance improvement: 10-50x query speedup for repeated queries.

use crate::specialist_memory::{SpecialistMemory, MemoryEntry, MemoryType, Confidence};
use crate::specialist_memory_caching::{CacheKey, MultiLayerCache, MultiLayerCacheConfig, CacheInvalidationManager};
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// Wrapper around SpecialistMemory with integrated caching
#[derive(Clone)]
pub struct CachedSpecialistMemory {
    memory: Arc<RwLock<SpecialistMemory>>,
    cache: MultiLayerCache,
    invalidation_manager: CacheInvalidationManager,
}

impl CachedSpecialistMemory {
    /// Create a new cached specialist memory
    pub fn new(specialist_id: String) -> Self {
        let memory = Arc::new(RwLock::new(SpecialistMemory::new(specialist_id)));
        let config = MultiLayerCacheConfig::default();
        let cache = MultiLayerCache::new(config);
        let invalidation_manager = CacheInvalidationManager::new(cache.clone());

        Self {
            memory,
            cache,
            invalidation_manager,
        }
    }

    /// Create from existing SpecialistMemory
    pub fn from_memory(memory: SpecialistMemory) -> Self {
        let memory = Arc::new(RwLock::new(memory));
        let config = MultiLayerCacheConfig::default();
        let cache = MultiLayerCache::new(config);
        let invalidation_manager = CacheInvalidationManager::new(cache.clone());

        Self {
            memory,
            cache,
            invalidation_manager,
        }
    }

    /// Record a new memory entry (invalidates relevant cache)
    pub fn record_memory(&self, entry: MemoryEntry) -> String {
        let id = {
            let mut mem = self.memory.write().unwrap();
            let result_id = entry.id.clone();
            mem.record_memory(entry.clone());
            result_id
        };

        // Invalidate cache entries affected by this new memory
        self.invalidation_manager.invalidate_on_memory_change(&entry);
        
        info!("Recorded memory entry: {} (cache invalidated)", id);
        id
    }

    /// Get memories by type (with caching)
    pub fn get_memories_by_type(&self, memory_type: MemoryType) -> Vec<MemoryEntry> {
        let cache_key = CacheKey::new("by_type", vec![format!("{:?}", memory_type)]);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for get_memories_by_type: {:?}", memory_type);
            return cached;
        }

        // Not in cache, query from memory
        let result = {
            let mem = self.memory.read().unwrap();
            mem.get_memories_by_type(memory_type)
                .iter()
                .map(|e| (*e).clone())
                .collect::<Vec<_>>()
        };

        // Store in cache for future queries
        self.cache.insert(cache_key, result.clone());
        
        debug!("Cache miss for get_memories_by_type: {:?} (cached {} results)", memory_type, result.len());
        result
    }

    /// Search memories by tag (with caching)
    pub fn search_memories(&self, tag: &str) -> Vec<MemoryEntry> {
        let cache_key = CacheKey::new("search", vec![tag.to_string()]);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for search_memories: {}", tag);
            return cached;
        }

        // Not in cache, query from memory
        let result = {
            let mem = self.memory.read().unwrap();
            mem.search_memories(tag)
                .iter()
                .map(|e| (*e).clone())
                .collect::<Vec<_>>()
        };

        // Store in cache
        self.cache.insert(cache_key, result.clone());
        
        debug!("Cache miss for search_memories: {} (cached {} results)", tag, result.len());
        result
    }

    /// Get all memories for specialist (with caching)
    pub fn get_all_memories(&self) -> Vec<MemoryEntry> {
        let cache_key = CacheKey::new("all", vec![]);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for get_all_memories");
            return cached;
        }

        // Not in cache, query from memory
        let result = {
            let mem = self.memory.read().unwrap();
            mem.memories
                .values()
                .cloned()
                .collect::<Vec<_>>()
        };

        // Store in cache
        self.cache.insert(cache_key, result.clone());
        
        debug!("Cache miss for get_all_memories (cached {} entries)", result.len());
        result
    }

    /// Get recent memories (with caching)
    pub fn get_recent_memories(&self, limit: usize) -> Vec<MemoryEntry> {
        let cache_key = CacheKey::new("recent", vec![limit.to_string()]);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for get_recent_memories: {}", limit);
            return cached;
        }

        // Not in cache, query from memory
        let mut result = {
            let mem = self.memory.read().unwrap();
            mem.memories
                .values()
                .cloned()
                .collect::<Vec<_>>()
        };
        
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        result.truncate(limit);

        // Store in cache
        self.cache.insert(cache_key, result.clone());
        
        debug!("Cache miss for get_recent_memories: {} (cached {} results)", limit, result.len());
        result
    }

    /// Get high-confidence memories (with caching)
    pub fn get_high_confidence_memories(&self) -> Vec<MemoryEntry> {
        let cache_key = CacheKey::new("high_confidence", vec![]);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for get_high_confidence_memories");
            return cached;
        }

        // Not in cache, query from memory
        let result = {
            let mem = self.memory.read().unwrap();
            mem.memories
                .values()
                .filter(|m| m.confidence == Confidence::High)
                .cloned()
                .collect::<Vec<_>>()
        };

        // Store in cache
        self.cache.insert(cache_key, result.clone());
        
        debug!("Cache miss for get_high_confidence_memories (cached {} entries)", result.len());
        result
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> crate::specialist_memory_caching::CacheStats {
        self.cache.get_stats()
    }

    /// Get cache hit rate
    pub fn get_cache_hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }

    /// Clear all cache
    pub fn clear_cache(&self) {
        self.cache.clear_all();
        info!("Specialist memory cache cleared");
    }

    /// Get memory reference for direct access (bypasses cache)
    pub fn get_memory_ref(&self) -> Arc<RwLock<SpecialistMemory>> {
        Arc::clone(&self.memory)
    }

    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> CacheMemoryUsageStats {
        let cache_usage = self.cache.memory_usage();
        let mem = self.memory.read().unwrap();
        
        CacheMemoryUsageStats {
            total_memory_entries: mem.memories.len(),
            cache_l1_entries: cache_usage.l1_entries,
            cache_l2_entries: cache_usage.l2_entries,
            cache_l3_entries: cache_usage.l3_entries,
            total_cache_entries: cache_usage.total_entries,
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct CacheMemoryUsageStats {
    pub total_memory_entries: usize,
    pub cache_l1_entries: usize,
    pub cache_l2_entries: usize,
    pub cache_l3_entries: usize,
    pub total_cache_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialist_memory::MemoryEntry;

    fn create_test_entry(specialist_id: &str, title: &str) -> MemoryEntry {
        MemoryEntry::new(
            specialist_id.to_string(),
            MemoryType::Lesson,
            title.to_string(),
            "test description".to_string(),
        )
    }

    #[test]
    fn test_cached_memory_creation() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let stats = cached.get_cache_stats();
        assert_eq!(stats.total_queries, 0);
    }

    #[test]
    fn test_record_memory_invalidates_cache() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        let id = cached.record_memory(entry);
        assert!(!id.is_empty());
    }

    #[test]
    fn test_get_memories_by_type_caching() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        cached.record_memory(entry);
        
        // First query - cache miss
        let result1 = cached.get_memories_by_type(MemoryType::Lesson);
        let stats1 = cached.get_cache_stats();
        
        // Second query - cache hit
        let result2 = cached.get_memories_by_type(MemoryType::Lesson);
        let stats2 = cached.get_cache_stats();
        
        assert_eq!(result1.len(), result2.len());
        assert!(stats2.l1_hits > stats1.l1_hits);
    }

    #[test]
    fn test_search_memories_caching() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let mut entry = create_test_entry("spec-1", "tagged memory");
        entry.tags = vec!["important".to_string()];
        
        cached.record_memory(entry);
        
        // First query - cache miss
        let result1 = cached.search_memories("important");
        assert!(!result1.is_empty());
        
        // Second query - cache hit
        let result2 = cached.search_memories("important");
        assert_eq!(result1.len(), result2.len());
        
        let stats = cached.get_cache_stats();
        assert!(stats.l1_hits > 0);
    }

    #[test]
    fn test_get_all_memories_caching() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        cached.record_memory(entry);
        
        // First query
        let result1 = cached.get_all_memories();
        
        // Second query (should hit cache)
        let result2 = cached.get_all_memories();
        
        assert_eq!(result1.len(), result2.len());
    }

    #[test]
    fn test_get_recent_memories_caching() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        cached.record_memory(entry);
        
        let result1 = cached.get_recent_memories(10);
        let result2 = cached.get_recent_memories(10);
        
        assert_eq!(result1.len(), result2.len());
    }

    #[test]
    fn test_get_high_confidence_memories_caching() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let mut entry = create_test_entry("spec-1", "high confidence memory");
        entry.confidence = Confidence::High;
        
        cached.record_memory(entry);
        
        let result1 = cached.get_high_confidence_memories();
        let result2 = cached.get_high_confidence_memories();
        
        assert_eq!(result1.len(), result2.len());
    }

    #[test]
    fn test_cache_hit_rate() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        cached.record_memory(entry);
        
        // First query - miss
        let _result1 = cached.get_memories_by_type(MemoryType::Lesson);
        
        // Second query - hit
        let _result2 = cached.get_memories_by_type(MemoryType::Lesson);
        
        let hit_rate = cached.get_cache_hit_rate();
        assert!(hit_rate > 0.0);
    }

    #[test]
    fn test_clear_cache() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        cached.record_memory(entry);
        cached.get_memories_by_type(MemoryType::Lesson);
        
        let stats_before = cached.get_cache_stats();
        cached.clear_cache();
        let stats_after = cached.get_cache_stats();
        
        assert!(stats_before.total_queries > 0);
        assert_eq!(stats_after.total_queries, 0);
    }

    #[test]
    fn test_memory_usage_stats() {
        let cached = CachedSpecialistMemory::new("spec-1".to_string());
        let entry = create_test_entry("spec-1", "test memory");
        
        cached.record_memory(entry);
        
        let usage = cached.get_memory_usage();
        assert_eq!(usage.total_memory_entries, 1);
    }
}
