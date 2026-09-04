use serde::{Deserialize, Serialize, de::DeserializeOwned};
/// Unified Registry — single abstraction for all dynamic registration.
///
/// Provides a common interface for registering, looking up, listing,
/// and managing any type of component in the system. Supports:
/// - Dynamic registration/deregistration at runtime
/// - O(1) HashMap lookup by key
/// - O(n) iteration for discovery
/// - Optional JSON persistence
/// - TTL-based expiry for stale entries
/// - Health status tracking
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Health status of a registered entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EntryHealth {
    Healthy,
    Degraded,
    Failed,
    #[default]
    Unknown,
}

/// Metadata for a registered entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMeta {
    /// When this entry was registered
    pub registered_at: u64,
    /// Last time this entry was accessed or health-checked
    pub last_seen: u64,
    /// Current health status
    pub health: EntryHealth,
    /// Optional tags for filtering
    pub tags: Vec<String>,
    /// Version string (SemVer)
    pub version: String,
    /// Optional TTL in seconds (0 = no expiry)
    pub ttl_secs: u64,
}

impl EntryMeta {
    pub fn new(version: &str) -> Self {
        let now = now_secs();
        Self {
            registered_at: now,
            last_seen: now,
            health: EntryHealth::Healthy,
            tags: Vec::new(),
            version: version.to_string(),
            ttl_secs: 0,
        }
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn is_expired(&self) -> bool {
        if self.ttl_secs == 0 {
            return false;
        }
        now_secs() > self.last_seen + self.ttl_secs
    }

    pub fn touch(&mut self) {
        self.last_seen = now_secs();
    }
}

/// A registered entry with its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry<T: Clone> {
    pub id: String,
    pub data: T,
    pub meta: EntryMeta,
}

/// Configuration for a registry instance.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Optional path for JSON persistence
    pub persist_path: Option<PathBuf>,
    /// Maximum entries (0 = unlimited)
    pub max_entries: usize,
    /// Default TTL for new entries (0 = no expiry)
    pub default_ttl_secs: u64,
    /// Auto-evict expired entries on access
    pub auto_evict: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            persist_path: None,
            max_entries: 0,
            default_ttl_secs: 0,
            auto_evict: true,
        }
    }
}

/// Unified registry with async RwLock for concurrent access.
pub struct Registry<T: Clone + Serialize + DeserializeOwned> {
    entries: HashMap<String, RegistryEntry<T>>,
    config: RegistryConfig,
}

impl<T: Clone + Serialize + DeserializeOwned> Registry<T> {
    /// Create a new empty registry.
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
        }
    }

    /// Create a registry and load persisted entries from disk.
    pub fn with_persistence(config: RegistryConfig) -> Self {
        let mut registry = Self::new(config.clone());
        if let Some(ref path) = config.persist_path
            && let Err(e) = registry.load_from_file(path)
        {
            warn!("Failed to load registry from {}: {}", path.display(), e);
        }
        registry
    }

    /// Register a new entry. Returns the assigned ID.
    ///
    /// If an entry with the same ID already exists, it is updated.
    pub fn register(&mut self, id: String, data: T, meta: EntryMeta) -> Result<(), String> {
        // Check capacity
        if self.config.max_entries > 0
            && self.entries.len() >= self.config.max_entries
            && !self.entries.contains_key(&id)
        {
            return Err(format!(
                "Registry full ({}/{})",
                self.entries.len(),
                self.config.max_entries
            ));
        }

        let entry = RegistryEntry {
            id: id.clone(),
            data,
            meta,
        };
        self.entries.insert(id, entry);

        // Persist if configured
        if let Some(ref path) = self.config.persist_path
            && let Err(e) = self.save_to_file(path)
        {
            warn!("Failed to persist registry: {}", e);
        }

        Ok(())
    }

    /// Register with auto-generated metadata.
    pub fn register_simple(&mut self, id: String, data: T, version: &str) -> Result<(), String> {
        let mut meta = EntryMeta::new(version);
        meta.ttl_secs = self.config.default_ttl_secs;
        self.register(id, data, meta)
    }

    /// Unregister an entry by ID. Returns true if it existed.
    pub fn unregister(&mut self, id: &str) -> bool {
        let existed = self.entries.remove(id).is_some();

        if existed
            && self.config.persist_path.is_some()
            && let Some(ref path) = self.config.persist_path.clone()
            && let Err(e) = self.save_to_file(path)
        {
            warn!("Failed to persist registry after unregister: {}", e);
        }

        existed
    }

    /// Get an entry by ID (cloned).
    pub fn get(&self, id: &str) -> Option<RegistryEntry<T>> {
        let entry = self.entries.get(id)?;

        // Check expiry
        if self.config.auto_evict && entry.meta.is_expired() {
            // Can't evict here since we only have &self; evict_expired() handles it
            return None;
        }

        Some(entry.clone())
    }

    /// Get an entry by ID and update its last_seen timestamp.
    pub fn get_mut(&mut self, id: &str) -> Option<RegistryEntry<T>> {
        let entry = self.entries.get_mut(id)?;

        if self.config.auto_evict && entry.meta.is_expired() {
            self.entries.remove(id);
            return None;
        }

        entry.meta.touch();
        Some(entry.clone())
    }

    /// List all entry IDs.
    pub fn list_ids(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// List all entries.
    pub fn list(&self) -> Vec<&RegistryEntry<T>> {
        self.entries.values().collect()
    }

    /// Find entries matching a predicate.
    pub fn find<F>(&self, predicate: F) -> Vec<&RegistryEntry<T>>
    where
        F: Fn(&RegistryEntry<T>) -> bool,
    {
        self.entries.values().filter(|e| predicate(e)).collect()
    }

    /// Find entries by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&RegistryEntry<T>> {
        self.find(|e| e.meta.tags.iter().any(|t| t == tag))
    }

    /// Find healthy entries only.
    pub fn healthy(&self) -> Vec<&RegistryEntry<T>> {
        self.find(|e| e.meta.health == EntryHealth::Healthy)
    }

    /// Update health status of an entry.
    pub fn set_health(&mut self, id: &str, health: EntryHealth) -> bool {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.meta.health = health;
            entry.meta.touch();
            true
        } else {
            false
        }
    }

    /// Remove all expired entries. Returns count removed.
    pub fn evict_expired(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| !entry.meta.is_expired());
        let removed = before - self.entries.len();
        if removed > 0 {
            info!("Evicted {} expired entries from registry", removed);
        }
        removed
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Save registry to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    /// Load registry from a JSON file.
    pub fn load_from_file(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
        self.entries =
            serde_json::from_str(&json).map_err(|e| format!("Deserialize error: {}", e))?;
        info!(
            "Loaded {} entries from {}",
            self.entries.len(),
            path.display()
        );
        Ok(())
    }
}

/// Async wrapper for concurrent access.
pub struct AsyncRegistry<T: Clone + Serialize + DeserializeOwned> {
    inner: Arc<RwLock<Registry<T>>>,
    _config: RegistryConfig,
}

impl<T: Clone + Serialize + DeserializeOwned + 'static> AsyncRegistry<T> {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Registry::new(config.clone()))),
            _config: config,
        }
    }

    pub fn with_persistence(config: RegistryConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Registry::with_persistence(config.clone()))),
            _config: config,
        }
    }

    pub async fn register(&self, id: String, data: T, meta: EntryMeta) -> Result<(), String> {
        self.inner.write().await.register(id, data, meta)
    }

    pub async fn register_simple(&self, id: String, data: T, version: &str) -> Result<(), String> {
        self.inner.write().await.register_simple(id, data, version)
    }

    pub async fn unregister(&self, id: &str) -> bool {
        self.inner.write().await.unregister(id)
    }

    pub async fn get(&self, id: &str) -> Option<RegistryEntry<T>> {
        self.inner.write().await.get(id)
    }

    pub async fn list(&self) -> Vec<RegistryEntry<T>> {
        self.inner
            .read()
            .await
            .list()
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn find_by_tag(&self, tag: &str) -> Vec<RegistryEntry<T>> {
        self.inner
            .read()
            .await
            .find_by_tag(tag)
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn set_health(&self, id: &str, health: EntryHealth) -> bool {
        self.inner.write().await.set_health(id, health)
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    pub async fn evict_expired(&self) -> usize {
        self.inner.write().await.evict_expired()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestEntry {
        name: String,
        value: i32,
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default());
        reg.register_simple(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            "1.0.0",
        )
        .unwrap();
        assert_eq!(reg.len(), 1);

        let entry = reg.get("a").unwrap();
        assert_eq!(entry.data.name, "alpha");
        assert_eq!(entry.meta.version, "1.0.0");
    }

    #[test]
    fn test_unregister() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default());
        reg.register_simple(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            "1.0.0",
        )
        .unwrap();
        assert!(reg.unregister("a"));
        assert_eq!(reg.len(), 0);
        assert!(!reg.unregister("a"));
    }

    #[test]
    fn test_find_by_tag() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default());
        reg.register(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            EntryMeta::new("1.0.0").with_tags(vec!["fast".into()]),
        )
        .unwrap();
        reg.register(
            "b".into(),
            TestEntry {
                name: "beta".into(),
                value: 2,
            },
            EntryMeta::new("1.0.0").with_tags(vec!["slow".into()]),
        )
        .unwrap();

        let fast = reg.find_by_tag("fast");
        assert_eq!(fast.len(), 1);
        assert_eq!(fast[0].data.name, "alpha");
    }

    #[test]
    fn test_health_status() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default());
        reg.register_simple(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            "1.0.0",
        )
        .unwrap();

        reg.set_health("a", EntryHealth::Degraded);
        let entry = reg.get("a").unwrap();
        assert_eq!(entry.meta.health, EntryHealth::Degraded);
    }

    #[test]
    fn test_evict_expired() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default());
        reg.register(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            EntryMeta::new("1.0.0").with_ttl(0),
        )
        .unwrap(); // No expiry
        reg.register(
            "b".into(),
            TestEntry {
                name: "beta".into(),
                value: 2,
            },
            EntryMeta::new("1.0.0"),
        )
        .unwrap(); // Default (no expiry)

        assert_eq!(reg.evict_expired(), 0);
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("test_registry");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.json");

        {
            let mut reg = Registry::<TestEntry>::new(RegistryConfig {
                persist_path: Some(path.clone()),
                ..Default::default()
            });
            reg.register_simple(
                "a".into(),
                TestEntry {
                    name: "alpha".into(),
                    value: 1,
                },
                "1.0.0",
            )
            .unwrap();
        }

        {
            let reg = Registry::<TestEntry>::with_persistence(RegistryConfig {
                persist_path: Some(path.clone()),
                ..Default::default()
            });
            assert_eq!(reg.len(), 1);
            assert_eq!(reg.get("a").unwrap().data.name, "alpha");
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
