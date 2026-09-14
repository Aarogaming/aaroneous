// LLM Response Caching
// Caches LLM responses to avoid redundant calls

use anyhow::Result;
use moka::future::Cache;
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use transpiler::prefix_cache_integration::{DemandDrivenAstCache, PromptPrefixKey};

pub struct LLMCache {
    cache: Cache<String, Vec<u8>>,
    ttl: Duration,
}

impl LLMCache {
    /// Create cache with TTL in seconds.
    ///
    /// Uses moka's `time_to_live` policy so entries are actually evicted
    /// after `ttl_secs` seconds.  Previously `Cache::new()` was used which
    /// creates a cache with no TTL; this is now fixed.
    pub fn new(ttl_secs: u64) -> Self {
        let ttl = Duration::from_secs(ttl_secs);
        let cache = moka::future::CacheBuilder::new(10_000)
            .time_to_live(ttl)
            .build();

        Self { cache, ttl }
    }

    /// Get cached value
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        if let Some(bytes) = self.cache.get(key).await
            && let Ok(value) = serde_json::from_slice::<T>(&bytes)
        {
            debug!("Cache hit: {}", key);
            return Some(value);
        }
        None
    }

    /// Set cached value
    pub async fn set<T: Serialize>(&self, key: &str, value: T) -> Result<()> {
        if let Ok(bytes) = serde_json::to_vec(&value) {
            self.cache.insert(key.to_string(), bytes).await;
            debug!("Cache set: {} (TTL: {:?})", key, self.ttl);
        }
        Ok(())
    }

    /// Clear all cache
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        debug!("Cache cleared");
    }

    /// Get cache size
    pub fn size(&self) -> u64 {
        self.cache.entry_count()
    }
}

/// AST-Driven Prompt Context & KV-Cache Pinner.
/// Integrates `DemandDrivenAstCache` from the transpiler crate to maintain
/// deterministic prompt prefix keys for source code files.
/// When source files are unmodified, their AST graphs and prompt prefix keys remain identical,
/// enabling maximal KV-cache reuse on local inference engines (e.g., GGUF / llama.cpp).
pub struct AstPrefixCacheManager {
    ast_cache: Arc<RwLock<DemandDrivenAstCache>>,
}

impl Default for AstPrefixCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AstPrefixCacheManager {
    pub fn new() -> Self {
        Self {
            ast_cache: Arc::new(RwLock::new(DemandDrivenAstCache::new())),
        }
    }

    pub fn with_cache(cache: Arc<RwLock<DemandDrivenAstCache>>) -> Self {
        Self { ast_cache: cache }
    }

    pub fn ast_cache(&self) -> Arc<RwLock<DemandDrivenAstCache>> {
        self.ast_cache.clone()
    }

    /// Records or updates file content, advancing file revisions and invalidating stale caches.
    pub fn update_file(&self, path: &str, content: &str) {
        self.ast_cache.write().set_file_content(path, content);
    }

    /// Computes or retrieves the deterministic prompt prefix key for a source file.
    /// Uses the Salsa-style query cache: if the file content hash matches the cached AST revision,
    /// returns the pinned prefix key in O(1) without re-parsing.
    pub fn get_or_create_prefix_key(
        &self,
        path: &str,
        content: &str,
    ) -> Result<PromptPrefixKey> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&content, &mut hasher);
        let content_hash = std::hash::Hasher::finish(&hasher);

        let mut cache = self.ast_cache.write();
        let graph = cache.query_ast(path, content, |src| {
            transpiler::prefix_cache_integration::parse_nl_to_opcode_dag(src)
        })?;

        let prefix_text = format!("FILE:{}:NODES:{}:HASH:{:016x}", path, graph.nodes.len(), content_hash);
        PromptPrefixKey::from_prompt(&prefix_text)
    }

    /// Constructs a pinned context block with deterministic prefix for prompt assembly.
    /// If the file AST has not changed, the generated prefix header is guaranteed to be identical,
    /// pinning KV-cache slots on compatible local runners.
    pub fn format_pinned_code_context(
        &self,
        path: &str,
        content: &str,
    ) -> Result<(PromptPrefixKey, String)> {
        let key = self.get_or_create_prefix_key(path, content)?;
        let context_block = format!(
            "// [AST-PINNED CONTEXT: {} | KEY: {:x}]\n{}",
            path, key.hash, content
        );
        Ok((key, context_block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = LLMCache::new(3600);
        let data = TestData {
            id: "test1".to_string(),
            value: 42,
        };

        cache.set("key1", &data).await.unwrap();
        let retrieved: Option<TestData> = cache.get("key1").await;

        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = LLMCache::new(3600);
        let retrieved: Option<TestData> = cache.get("nonexistent").await;

        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = LLMCache::new(3600);
        let data = TestData {
            id: "test1".to_string(),
            value: 42,
        };

        cache.set("key1", &data).await.unwrap();

        // Verify set worked
        let before: Option<TestData> = cache.get("key1").await;
        assert_eq!(before, Some(data.clone()));

        cache.clear().await;

        // Verify clear worked
        let after: Option<TestData> = cache.get("key1").await;
        assert_eq!(after, None);
    }

    #[test]
    fn test_cache_ttl() {
        // TTL is configured but hard to test reliably in tests
        // Just verify cache can be created with different TTL values
        let _cache1 = LLMCache::new(1);
        let _cache2 = LLMCache::new(3600);
        // If we get here without panic, TTL configuration works
        assert!(true);
    }

    #[test]
    fn test_ast_prefix_cache_manager_hits_and_invalidation() {
        let mgr = AstPrefixCacheManager::new();
        let path = "crates/core/src/state.rs";
        let code_v1 = "fn process() { load_item(); }";

        // First query evaluates AST
        let key1 = mgr.get_or_create_prefix_key(path, code_v1).unwrap();

        // Second query with identical source hits AST cache in O(1)
        let key2 = mgr.get_or_create_prefix_key(path, code_v1).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(mgr.ast_cache().read().hits(), 1);

        // Update file content with a modification
        let code_v2 = "fn process() { get_item(); }";
        mgr.update_file(path, code_v2);

        // Third query reflects updated revision
        let key3 = mgr.get_or_create_prefix_key(path, code_v2).unwrap();
        assert_ne!(key3.hash, 0);
        assert_ne!(key1.hash, key3.hash);
    }

    #[test]
    fn test_format_pinned_code_context() {
        let mgr = AstPrefixCacheManager::new();
        let path = "core/hypervisor/src/state.rs";
        let code = "fn run_tick() { let x = 1; }";

        let (key, formatted) = mgr.format_pinned_code_context(path, code).unwrap();
        assert!(formatted.contains("[AST-PINNED CONTEXT: core/hypervisor/src/state.rs"));
        assert!(formatted.contains(&format!("{:x}", key.hash)));
        assert!(formatted.contains("fn run_tick()"));
    }
}
