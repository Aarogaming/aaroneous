use parking_lot::RwLock;
/// Specialist Memory - persistent memory entries for federation specialists.
///
/// Each specialist can store and retrieve memory entries that persist
/// across sessions and restarts. Enables learning from experience and
/// consultation during task execution.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

/// Wall-clock seconds since UNIX_EPOCH. Wrapped here so
/// the eviction tests can stub it via a feature flag if
/// needed; for now it is a thin wrapper over the standard
/// library.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Type of memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Factual knowledge learned from execution
    Factual,
    /// Procedural knowledge (how to do something)
    Procedural,
    /// Episodic memory (specific event)
    Episodic,
    /// Relational memory (about other specialists)
    Relational,
    /// Meta-cognitive (about own thinking)
    Metacognitive,
}

impl MemoryType {
    pub fn relevance_to_task(&self, _task_type: &str) -> f32 {
        match self {
            MemoryType::Procedural => 0.95,    // Highly relevant for learning how
            MemoryType::Factual => 0.85,       // Good for understanding what
            MemoryType::Episodic => 0.70,      // Moderately relevant - past experience
            MemoryType::Relational => 0.60,    // Helpful for collaboration
            MemoryType::Metacognitive => 0.50, // Generally applicable
        }
    }
}

/// A single memory entry stored by a specialist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub specialist_id: String,
    pub title: String,
    pub description: String,
    pub memory_type: MemoryType,
    pub confidence: f32, // 0.0-1.0: how confident in this memory
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
    pub tags: Vec<String>, // For semantic search
    /// Monotonic insertion sequence assigned by the
    /// store on first insert. Used as a tie-breaker
    /// when multiple entries share the same
    /// `last_accessed` second. Serialised so saved
    /// memory snapshots remain deterministic.
    #[serde(default)]
    pub seq: u64,
}

impl MemoryEntry {
    pub fn new(
        id: String,
        specialist_id: String,
        title: String,
        description: String,
        memory_type: MemoryType,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            id,
            specialist_id,
            title,
            description,
            memory_type,
            confidence: 0.5,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            tags: Vec::new(),
            seq: 0,
        }
    }

    /// Update access tracking when memory is used
    pub fn record_access(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_accessed = now;
        self.access_count += 1;
    }

    /// Calculate relevance score for a query
    pub fn relevance_score(&self, query: &str, task_type: &str) -> f32 {
        let type_relevance = self.memory_type.relevance_to_task(task_type);

        // Check title and description for keyword matches
        let title_match = self.title.to_lowercase().contains(&query.to_lowercase());
        let desc_match = self
            .description
            .to_lowercase()
            .contains(&query.to_lowercase());
        let tags_match = self
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&query.to_lowercase()));

        let keyword_score = if title_match {
            0.9
        } else if desc_match {
            0.6
        } else if tags_match {
            0.7
        } else {
            0.0
        };

        // Higher access count = more useful memory
        let recency_factor = (self.access_count as f32 / 100.0).min(1.0);

        // Weight: type (40%) + keywords (40%) + recency (20%)
        (type_relevance * 0.4) + (keyword_score * 0.4) + (recency_factor * 0.2)
    }
}

/// Query result when consulting specialist memory
#[derive(Debug, Clone, Serialize)]
pub struct MemoryQueryResult {
    pub entries: Vec<MemoryEntry>,
    pub total_score: f32,
    pub recommendation: String,
}

/// Specialist Memory Store - manages memory for a single specialist
#[derive(Clone)]
pub struct SpecialistMemoryStore {
    specialist_id: String,
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
    /// Bounds applied at `store_memory` time. The store
    /// never exceeds `max_entries`; entries older than
    /// `ttl` are evicted lazily on access and explicitly
    /// via `evict_expired`.
    config: MemoryConfig,
    /// Monotonic insertion sequence. Combined with
    /// `last_accessed` to break ties when multiple
    /// entries share the same wall-clock second.
    /// Without this, the LRU test would be
    /// non-deterministic because `last_accessed` is
    /// stored at second resolution and a tight loop
    /// can produce identical timestamps.
    next_seq: Arc<AtomicU64>,
}

/// Memory store configuration. Defaults: `ttl = None`
/// (no expiry), `max_entries = None` (unbounded). Pass a
/// config with both set to bound memory in long-running
/// deployments.
#[derive(Debug, Clone, Default)]
pub struct MemoryConfig {
    /// Drop entries whose `last_accessed` is older than
    /// `now - ttl`. `None` disables TTL eviction.
    pub ttl: Option<std::time::Duration>,
    /// Maximum number of entries. When the store would
    /// grow beyond this, the least-recently-accessed entry
    /// is dropped. `None` disables LRU eviction.
    pub max_entries: Option<usize>,
}

impl SpecialistMemoryStore {
    /// Create new memory store for specialist with
    /// default (unbounded) config. Long-running callers
    /// should prefer `with_config` to bound memory.
    pub fn new(specialist_id: String) -> Self {
        Self::with_config(specialist_id, MemoryConfig::default())
    }

    /// Create with explicit memory bounds.
    pub fn with_config(specialist_id: String, config: MemoryConfig) -> Self {
        Self {
            specialist_id,
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
            next_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Current config. Test-only inspection.
    pub fn config(&self) -> MemoryConfig {
        self.config.clone()
    }

    /// Store a memory entry. Applies the configured
    /// bounds: if `max_entries` is set, drops the
    /// least-recently-accessed entry first; if `ttl`
    /// is set, expired entries are reaped before the
    /// insert so the new entry is not immediately
    /// evicted.
    pub fn store_memory(&self, mut entry: MemoryEntry) {
        entry.specialist_id = self.specialist_id.clone();
        // Assign the monotonic seq before any check
        // that might drop the entry. The seq is part
        // of the entry and used by the LRU comparator
        // as a tie-breaker.
        if entry.seq == 0 {
            entry.seq = self.next_seq.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        }
        let mut store = self.entries.write();

        // Lazy TTL eviction on insert: cheap when there
        // are no expired entries (one pass through values).
        if let Some(ttl) = self.config.ttl {
            let now = now_secs();
            store.retain(|_, e| now.saturating_sub(e.last_accessed) < ttl.as_secs());
        }

        // LRU bound: if the store is at the cap, drop the
        // least-recently-accessed entry. The retained
        // entry's id is computed by a single pass and the
        // map.remove is O(1). O(n) per insert in the
        // worst case, but `n` is bounded by `max_entries`
        // which the caller chose deliberately.
        // Tie-break by `seq` so two entries inserted in
        // the same second have a deterministic order.
        if let Some(cap) = self.config.max_entries {
            while store.len() >= cap {
                if let Some(oldest_id) = store
                    .iter()
                    .min_by_key(|(_, e)| (e.last_accessed, e.seq))
                    .map(|(id, _)| id.clone())
                {
                    store.remove(&oldest_id);
                } else {
                    break;
                }
            }
        }

        store.insert(entry.id.clone(), entry);
    }

    /// Retrieve a specific memory by ID. Refreshes
    /// `last_accessed` on hit; the LRU bound therefore
    /// keeps "hot" entries while letting cold ones
    /// fall out.
    pub fn get_memory(&self, memory_id: &str) -> Option<MemoryEntry> {
        let mut store = self.entries.write();
        if let Some(entry) = store.get_mut(memory_id) {
            entry.record_access();
        }
        store.get(memory_id).cloned()
    }

    /// Drop entries older than `config.ttl`. Returns the
    /// number of entries removed. Call from a periodic
    /// sweeper; idempotent and cheap.
    pub fn evict_expired(&self) -> usize {
        let ttl = match self.config.ttl {
            Some(t) => t,
            None => return 0,
        };
        let mut store = self.entries.write();
        let now = now_secs();
        let before = store.len();
        store.retain(|_, e| now.saturating_sub(e.last_accessed) < ttl.as_secs());
        before - store.len()
    }

    /// Drop the least-recently-accessed entries until
    /// the store is at or below `config.max_entries`.
    /// Returns the number removed. Safe to call when
    /// the cap is unset (no-op).
    pub fn evict_lru(&self) -> usize {
        let cap = match self.config.max_entries {
            Some(c) => c,
            None => return 0,
        };
        let mut store = self.entries.write();
        let mut removed = 0;
        while store.len() > cap {
            if let Some(oldest_id) = store
                .iter()
                .min_by_key(|(_, e)| (e.last_accessed, e.seq))
                .map(|(id, _)| id.clone())
            {
                store.remove(&oldest_id);
                removed += 1;
            } else {
                break;
            }
        }
        removed
    }

    /// Query memory by keyword and task type
    pub fn query_memory(&self, query: &str, task_type: &str, limit: usize) -> MemoryQueryResult {
        let store = self.entries.read();

        let mut results: Vec<_> = store
            .values()
            .map(|entry| {
                let score = entry.relevance_score(query, task_type);
                (entry.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by relevance score (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N results
        let entries: Vec<MemoryEntry> = results
            .iter()
            .take(limit)
            .map(|(entry, _)| entry.clone())
            .collect();

        let total_score: f32 = results.iter().take(limit).map(|(_, score)| score).sum();

        let avg_score = if !entries.is_empty() {
            total_score / entries.len() as f32
        } else {
            0.0
        };

        // Generate recommendation based on results
        let recommendation = if avg_score > 0.8 {
            "High confidence guidance available from past experience".to_string()
        } else if avg_score > 0.5 {
            "Moderate guidance available, proceed with caution".to_string()
        } else if avg_score > 0.0 {
            "Limited relevant experience, review carefully".to_string()
        } else {
            "No relevant memory found, use external expertise".to_string()
        };

        MemoryQueryResult {
            entries,
            total_score,
            recommendation,
        }
    }

    /// Get all memories of a specific type
    pub fn get_memories_by_type(&self, memory_type: MemoryType) -> Vec<MemoryEntry> {
        let store = self.entries.read();
        store
            .values()
            .filter(|e| e.memory_type == memory_type)
            .cloned()
            .collect()
    }

    /// Get most frequently accessed memories (most useful)
    pub fn get_frequently_used(&self, limit: usize) -> Vec<MemoryEntry> {
        let store = self.entries.read();
        let mut entries: Vec<_> = store.values().cloned().collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.access_count));
        entries.into_iter().take(limit).collect()
    }

    /// Clear all memories
    pub fn clear_memories(&self) {
        self.entries.write().clear();
    }

    /// Total number of stored entries. Test-only
    /// inspection.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }
}

/// Shared memory registry. Wraps the per-specialist
/// store map in an `Arc<Mutex<...>>` so multiple
/// components (`autonomic_loop`, `decision_engine`,
/// future federation hooks) can read and write the
/// *same* store for a given specialist, eliminating
/// the "two parallel paths" bug where one component
/// stored a memory and the other could not see it.
///
/// Construct with `SharedMemoryRegistry::new()` for
/// a registry with unbounded per-specialist stores,
/// or `SharedMemoryRegistry::with_default_config`
/// to apply a config to every new specialist's
/// store. The default config can also be applied
/// later to existing entries by calling
/// `apply_default_config_to` (currently a no-op;
/// the per-call config in `get_or_create_with`
/// takes precedence).
#[derive(Clone, Default)]
pub struct SharedMemoryRegistry {
    stores: Arc<parking_lot::Mutex<HashMap<String, SpecialistMemoryStore>>>,
    default_config: MemoryConfig,
}

impl SharedMemoryRegistry {
    /// New registry with the default (unbounded)
    /// per-specialist config.
    pub fn new() -> Self {
        Self {
            stores: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            default_config: MemoryConfig::default(),
        }
    }

    /// New registry whose per-specialist stores are
    /// created with `config` by default. Callers
    /// that need a per-specialist override can use
    /// `get_or_create_with`.
    pub fn with_default_config(config: MemoryConfig) -> Self {
        Self {
            stores: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            default_config: config,
        }
    }

    /// Get the existing store for `specialist_id`, or
    /// create one with the registry's default config
    /// and insert it. Cheap clone — the returned
    /// `SpecialistMemoryStore` shares the inner
    /// `Arc<RwLock<HashMap>>` with the registry.
    pub fn get_or_create(&self, specialist_id: &str) -> SpecialistMemoryStore {
        self.get_or_create_with(specialist_id, self.default_config.clone())
    }

    /// Get the existing store, or create one with
    /// an explicit config. If a store already
    /// exists, the existing instance is returned
    /// unchanged; this method cannot retroactively
    /// apply a config to a store that was created
    /// earlier.
    pub fn get_or_create_with(
        &self,
        specialist_id: &str,
        config: MemoryConfig,
    ) -> SpecialistMemoryStore {
        let mut stores = self.stores.lock();
        if let Some(existing) = stores.get(specialist_id) {
            return existing.clone();
        }
        let store = SpecialistMemoryStore::with_config(specialist_id.to_string(), config);
        stores.insert(specialist_id.to_string(), store.clone());
        store
    }

    /// List of specialist IDs with a registered
    /// store. Test-only inspection.
    #[cfg(test)]
    pub fn specialist_ids(&self) -> Vec<String> {
        self.stores.lock().keys().cloned().collect()
    }

    /// Total number of registered stores (one per
    /// specialist). Test-only inspection.
    #[cfg(test)]
    pub fn store_count(&self) -> usize {
        self.stores.lock().len()
    }
}

/// Get memory store statistics (re-opens the
/// SpecialistMemoryStore impl to host the method
/// that was stranded when `clear_memories` was
/// refactored).
impl SpecialistMemoryStore {
    /// Get memory store statistics
    pub fn get_stats(&self) -> MemoryStats {
        let store = self.entries.read();
        let total_entries = store.len();
        let total_accesses: u32 = store.values().map(|e| e.access_count).sum();
        let avg_confidence: f32 = if total_entries > 0 {
            store.values().map(|e| e.confidence).sum::<f32>() / total_entries as f32
        } else {
            0.0
        };

        let type_counts = {
            let mut counts = HashMap::new();
            for entry in store.values() {
                *counts.entry(entry.memory_type.clone()).or_insert(0) += 1;
            }
            counts
        };

        MemoryStats {
            total_entries,
            total_accesses,
            avg_confidence,
            type_counts,
        }
    }
}

/// Statistics about specialist memory
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_accesses: u32,
    pub avg_confidence: f32,
    pub type_counts: HashMap<MemoryType, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "How to parse JSON".to_string(),
            "Use serde_json crate".to_string(),
            MemoryType::Procedural,
        );
        assert_eq!(entry.title, "How to parse JSON");
        assert_eq!(entry.memory_type, MemoryType::Procedural);
        assert_eq!(entry.access_count, 0);
    }

    #[test]
    fn test_memory_relevance_score() {
        let mut entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "JSON parsing".to_string(),
            "Using serde_json for deserialization".to_string(),
            MemoryType::Procedural,
        );
        entry.tags = vec!["json".to_string(), "parsing".to_string()];

        let score = entry.relevance_score("json", "parsing_task");
        assert!(score > 0.0);
    }

    #[test]
    fn test_memory_store_operations() {
        let store = SpecialistMemoryStore::new("specialist-1".to_string());

        let entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "Test memory".to_string(),
            "Test description".to_string(),
            MemoryType::Factual,
        );

        store.store_memory(entry.clone());
        let retrieved = store.get_memory("mem-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test memory");
    }

    #[test]
    fn test_memory_query() {
        let store = SpecialistMemoryStore::new("specialist-1".to_string());

        let entry1 = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "How to debug errors".to_string(),
            "Use logging and assertions".to_string(),
            MemoryType::Procedural,
        );

        let entry2 = MemoryEntry::new(
            "mem-2".to_string(),
            "specialist-1".to_string(),
            "Optimization tips".to_string(),
            "Cache results when possible".to_string(),
            MemoryType::Factual,
        );

        store.store_memory(entry1);
        store.store_memory(entry2);

        let result = store.query_memory("debug", "debugging_task", 5);
        assert!(!result.entries.is_empty());
    }

    #[test]
    fn test_memory_stats() {
        let store = SpecialistMemoryStore::new("specialist-1".to_string());

        let entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            MemoryType::Procedural,
        );

        store.store_memory(entry);
        let stats = store.get_stats();
        assert_eq!(stats.total_entries, 1);
    }

    fn make_entry(id: &str) -> MemoryEntry {
        let mut e = MemoryEntry::new(
            id.to_string(),
            "test-specialist".to_string(),
            format!("title-{id}"),
            "desc".to_string(),
            MemoryType::Factual,
        );
        e.seq = 0; // let the store assign
        e
    }

    #[test]
    fn test_ttl_eviction() {
        // Insert one entry, then sweep. With ttl=1s and
        // the test running immediately, the entry is
        // still hot — should not be evicted. To test
        // eviction we'd need to either wait or backdate
        // the entry. We pick a 0-second TTL, which is
        // treated as "any access older than 0s is gone":
        // every entry is immediately eligible.
        let store = SpecialistMemoryStore::with_config(
            "test".to_string(),
            MemoryConfig {
                ttl: Some(std::time::Duration::from_secs(0)),
                max_entries: None,
            },
        );
        store.store_memory(make_entry("mem-0"));
        // last_accessed == now; with ttl=0 the entry
        // is exactly at the boundary. The retain logic
        // is strict-less-than, so a 0-second ttl evicts
        // every entry whose last_accessed is now or
        // earlier. Because `now` advances between the
        // store and the evict, mem-0 should be dropped.
        let removed = store.evict_expired();
        assert!(removed >= 1, "expected at least 1 eviction, got {removed}");
        assert!(store.get_memory("mem-0").is_none());
    }

    #[test]
    fn test_unbounded_config_behaves_like_default() {
        // No config: entries grow without bound. The
        // existing test_memory_store_operations covers
        // this implicitly, but we add an explicit check
        // that `evict_expired` and `evict_lru` are no-ops
        // when the config is unset.
        let store = SpecialistMemoryStore::new("test".to_string());
        for i in 0..10 {
            store.store_memory(make_entry(&format!("mem-{i}")));
        }
        assert_eq!(store.evict_expired(), 0);
        assert_eq!(store.evict_lru(), 0);
        assert_eq!(store.get_stats().total_entries, 10);
    }

    #[test]
    fn test_shared_registry_returns_same_store_for_same_specialist() {
        // Two components get the same store for the
        // same specialist id. A memory stored through
        // one handle is visible through the other.
        let registry = SharedMemoryRegistry::new();
        let store_a = registry.get_or_create("alice");
        let store_b = registry.get_or_create("alice");
        // Both handles point to the same underlying
        // Arc<RwLock<HashMap>>.
        assert!(Arc::ptr_eq(&store_a.entries, &store_b.entries,));
        store_a.store_memory(make_entry("shared-mem-0"));
        assert!(store_b.get_memory("shared-mem-0").is_some());
    }

    #[test]
    fn test_shared_registry_isolates_different_specialists() {
        let registry = SharedMemoryRegistry::new();
        let alice = registry.get_or_create("alice");
        let bob = registry.get_or_create("bob");
        alice.store_memory(make_entry("alice-only"));
        bob.store_memory(make_entry("bob-only"));
        assert!(alice.get_memory("alice-only").is_some());
        assert!(alice.get_memory("bob-only").is_none());
        assert!(bob.get_memory("bob-only").is_some());
        assert!(bob.get_memory("alice-only").is_none());
    }

    #[test]
    fn test_shared_registry_applies_default_config() {
        // Registry with a tight default config: every
        // new store inherits the cap.
        let registry = SharedMemoryRegistry::with_default_config(MemoryConfig {
            ttl: None,
            max_entries: Some(2),
        });
        let s = registry.get_or_create("capped");
        for i in 0..5 {
            s.store_memory(make_entry(&format!("m-{i}")));
        }
        // Cap of 2: only the most recent 2 survive.
        let stats = s.get_stats();
        assert_eq!(stats.total_entries, 2);
    }
}
