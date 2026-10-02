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

/// Clock source function returning current time in seconds since UNIX epoch.
pub type ClockSource = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Monotonic system clock source returning seconds since UNIX epoch.
///
/// If system time steps backward or errors, holds the last known monotonic value rather
/// than resetting to zero, preventing premature expiration or non-expiring entries.
pub fn system_clock() -> ClockSource {
    use std::sync::atomic::{AtomicU64, Ordering};
    let initial = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last_good = Arc::new(AtomicU64::new(initial));
    let last_good_clone = last_good.clone();
    Arc::new(
        move || match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => {
                let s = d.as_secs();
                let mut prev = last_good_clone.load(Ordering::Relaxed);
                while s > prev {
                    match last_good_clone.compare_exchange_weak(
                        prev,
                        s,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(actual) => prev = actual,
                    }
                }
                last_good_clone.load(Ordering::Relaxed)
            }
            Err(_) => last_good_clone.load(Ordering::Relaxed),
        },
    )
}

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
    pub fn new(version: &str, registered_at: u64) -> Self {
        Self {
            registered_at,
            last_seen: registered_at,
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

    pub fn is_expired(&self, now_secs: u64) -> bool {
        if self.ttl_secs == 0 {
            return false;
        }
        now_secs > self.last_seen + self.ttl_secs
    }

    pub fn touch(&mut self, now_secs: u64) {
        self.last_seen = now_secs;
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
/// Unified registry with async RwLock for concurrent access.
pub struct Registry<T: Clone + Serialize + DeserializeOwned> {
    entries: HashMap<String, RegistryEntry<T>>,
    config: RegistryConfig,
    clock: ClockSource,
}

impl<T: Clone + Serialize + DeserializeOwned> Registry<T> {
    /// Create a new empty registry with an injected clock source.
    pub fn new(config: RegistryConfig, clock: ClockSource) -> Self {
        Self {
            entries: HashMap::new(),
            config,
            clock,
        }
    }

    /// Create a registry and load persisted entries from disk with an injected clock source.
    pub fn with_persistence(config: RegistryConfig, clock: ClockSource) -> Self {
        let mut registry = Self::new(config.clone(), clock);
        if let Some(ref path) = config.persist_path
            && let Err(e) = registry.load_from_file(path)
        {
            warn!("Failed to load registry from {}: {}", path.display(), e);
        }
        registry
    }

    /// Current timestamp in seconds from the injected clock.
    pub fn now_secs(&self) -> u64 {
        (self.clock)()
    }

    /// Create an `EntryMeta` stamped with the current timestamp from the injected clock.
    pub fn create_meta(&self, version: &str) -> EntryMeta {
        EntryMeta::new(version, self.now_secs())
    }

    /// Injected clock source handle.
    pub fn clock(&self) -> ClockSource {
        Arc::clone(&self.clock)
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
        let mut meta = self.create_meta(version);
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
        if self.config.auto_evict && entry.meta.is_expired(self.now_secs()) {
            // Can't evict here since we only have &self; evict_expired() handles it
            return None;
        }

        Some(entry.clone())
    }

    /// Get an entry by ID and update its last_seen timestamp.
    pub fn get_mut(&mut self, id: &str) -> Option<RegistryEntry<T>> {
        let now = self.now_secs();
        let entry = self.entries.get_mut(id)?;

        if self.config.auto_evict && entry.meta.is_expired(now) {
            self.entries.remove(id);
            return None;
        }

        entry.meta.touch(now);
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
        let now = self.now_secs();
        if let Some(entry) = self.entries.get_mut(id) {
            entry.meta.health = health;
            entry.meta.touch(now);
            true
        } else {
            false
        }
    }

    /// Remove all expired entries. Returns count removed.
    pub fn evict_expired(&mut self) -> usize {
        let now = self.now_secs();
        let before = self.entries.len();
        self.entries.retain(|_, entry| !entry.meta.is_expired(now));
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
    pub fn new(config: RegistryConfig, clock: ClockSource) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Registry::new(config.clone(), clock))),
            _config: config,
        }
    }

    pub fn with_persistence(config: RegistryConfig, clock: ClockSource) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Registry::with_persistence(
                config.clone(),
                clock,
            ))),
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

    pub async fn now_secs(&self) -> u64 {
        self.inner.read().await.now_secs()
    }

    pub async fn create_meta(&self, version: &str) -> EntryMeta {
        self.inner.read().await.create_meta(version)
    }

    pub async fn clock(&self) -> ClockSource {
        self.inner.read().await.clock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestEntry {
        name: String,
        value: i32,
    }

    fn fixed_clock(ts: u64) -> ClockSource {
        Arc::new(move || ts)
    }

    fn mutable_clock(initial: u64) -> (ClockSource, Arc<AtomicU64>) {
        let ts = Arc::new(AtomicU64::new(initial));
        let ts_clone = Arc::clone(&ts);
        (Arc::new(move || ts_clone.load(Ordering::SeqCst)), ts)
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default(), fixed_clock(1000));
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
        assert_eq!(entry.meta.registered_at, 1000);
        assert_eq!(entry.meta.last_seen, 1000);
    }

    #[test]
    fn test_unregister() {
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default(), fixed_clock(1000));
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
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default(), fixed_clock(1000));
        reg.register(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            EntryMeta::new("1.0.0", 1000).with_tags(vec!["fast".into()]),
        )
        .unwrap();
        reg.register(
            "b".into(),
            TestEntry {
                name: "beta".into(),
                value: 2,
            },
            EntryMeta::new("1.0.0", 1000).with_tags(vec!["slow".into()]),
        )
        .unwrap();

        let fast = reg.find_by_tag("fast");
        assert_eq!(fast.len(), 1);
        assert_eq!(fast[0].data.name, "alpha");
    }

    #[test]
    fn test_health_status() {
        let (clock, ts) = mutable_clock(1000);
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default(), clock);
        reg.register_simple(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            "1.0.0",
        )
        .unwrap();

        ts.store(1050, Ordering::SeqCst);
        reg.set_health("a", EntryHealth::Degraded);
        let entry = reg.get("a").unwrap();
        assert_eq!(entry.meta.health, EntryHealth::Degraded);
        assert_eq!(entry.meta.last_seen, 1050);
    }

    #[test]
    fn test_evict_expired() {
        let (clock, ts) = mutable_clock(1000);
        let mut reg = Registry::<TestEntry>::new(RegistryConfig::default(), clock);
        reg.register(
            "a".into(),
            TestEntry {
                name: "alpha".into(),
                value: 1,
            },
            EntryMeta::new("1.0.0", 1000).with_ttl(10),
        )
        .unwrap(); // Expires after 10s (at t > 1010)
        reg.register(
            "b".into(),
            TestEntry {
                name: "beta".into(),
                value: 2,
            },
            EntryMeta::new("1.0.0", 1000).with_ttl(0),
        )
        .unwrap(); // Default (no expiry)

        // At t = 1005: not expired
        ts.store(1005, Ordering::SeqCst);
        assert_eq!(reg.evict_expired(), 0);
        assert_eq!(reg.len(), 2);

        // Touch entry "a" at t = 1008 via get_mut
        ts.store(1008, Ordering::SeqCst);
        let touched = reg.get_mut("a").unwrap();
        assert_eq!(touched.meta.last_seen, 1008);

        // At t = 1015: since it was touched at 1008, ttl expires at 1018, so at 1015 it is still alive
        ts.store(1015, Ordering::SeqCst);
        assert_eq!(reg.evict_expired(), 0);
        assert_eq!(reg.len(), 2);

        // At t = 1019: entry "a" is now expired (1019 > 1008 + 10)
        ts.store(1019, Ordering::SeqCst);
        assert_eq!(reg.evict_expired(), 1);
        assert_eq!(reg.len(), 1);
        assert!(reg.get("a").is_none());
        assert!(reg.get("b").is_some());
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");

        {
            let mut reg = Registry::<TestEntry>::new(
                RegistryConfig {
                    persist_path: Some(path.clone()),
                    ..Default::default()
                },
                fixed_clock(1000),
            );
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
            let reg = Registry::<TestEntry>::with_persistence(
                RegistryConfig {
                    persist_path: Some(path.clone()),
                    ..Default::default()
                },
                fixed_clock(1000),
            );
            assert_eq!(reg.len(), 1);
            assert_eq!(reg.get("a").unwrap().data.name, "alpha");
        }
    }

    #[test]
    fn test_system_clock_monotonic_and_nonzero() {
        let clock = system_clock();
        let t1 = clock();
        assert!(t1 > 0, "system clock must return a non-zero timestamp");
        let t2 = clock();
        assert!(t2 >= t1, "system clock must be monotonic");
    }
}
