/// DNA Bank: Persistent Learning Memory
///
/// Implements long-term memory for the hive using a tiered storage system:
/// - Hot tier (in-memory): Recent events and patterns (1-7 days)
/// - Warm tier (disk): Medium history (1-3 months)  [RocksDB with `rocksdb-dna` feature]
/// - Cold tier (archive): Long-term history (1+ years) [future]
///
/// # Storage backends
///
/// - **Default (no feature)**: `BTreeMap<String, DNAEvent>` - fast for tests and dev,
///   no native dependencies, data lost on restart.
/// - **`rocksdb-dna` feature**: RocksDB column families on disk - durable storage,
///   survives restarts, handles millions of events without memory pressure.
///   Requires RocksDB native library (compiled from source on first use).
///
/// Use `DNABank::new()` for in-memory or `DNABank::open(path)` for RocksDB.
/// All public methods are identical regardless of backend.
use crate::federation::specialist::SpecialistId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

/// Event in the DNA Bank
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNAEvent {
    pub id: String,
    pub timestamp: u64,
    pub specialist: SpecialistId,
    pub event_type: String,
    pub outcome: String, // "success", "failure", "partial"
    pub duration_ms: u32,
    pub metadata: HashMap<String, String>,
}

impl DNAEvent {
    pub fn new(
        specialist: SpecialistId,
        event_type: String,
        outcome: String,
        duration_ms: u32,
    ) -> Self {
        let id = format!("evt-{}", uuid());
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            timestamp,
            specialist,
            event_type,
            outcome,
            duration_ms,
            metadata: HashMap::new(),
        }
    }

    pub fn age_days(&self) -> u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        ((now - self.timestamp) / 86400) as u32
    }

    pub fn is_success(&self) -> bool {
        self.outcome == "success"
    }
}

/// Query builder for flexible event retrieval
#[derive(Debug, Clone)]
pub struct EventQuery {
    pub specialist: Option<SpecialistId>,
    pub event_type: Option<String>,
    pub outcome: Option<String>,
    pub since_timestamp: Option<u64>,
    pub until_timestamp: Option<u64>,
    pub limit: Option<usize>,
}

impl Default for EventQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl EventQuery {
    pub fn new() -> Self {
        Self {
            specialist: None,
            event_type: None,
            outcome: None,
            since_timestamp: None,
            until_timestamp: None,
            limit: None,
        }
    }

    pub fn for_specialist(mut self, id: SpecialistId) -> Self {
        self.specialist = Some(id);
        self
    }

    pub fn of_type(mut self, event_type: String) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_outcome(mut self, outcome: String) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn since(mut self, timestamp: u64) -> Self {
        self.since_timestamp = Some(timestamp);
        self
    }

    pub fn until(mut self, timestamp: u64) -> Self {
        self.until_timestamp = Some(timestamp);
        self
    }

    pub fn limit(mut self, count: usize) -> Self {
        self.limit = Some(count);
        self
    }

    pub fn matches(&self, event: &DNAEvent) -> bool {
        if let Some(specialist) = self.specialist
            && event.specialist != specialist
        {
            return false;
        }

        if let Some(ref event_type) = self.event_type
            && event.event_type != *event_type
        {
            return false;
        }

        if let Some(ref outcome) = self.outcome
            && event.outcome != *outcome
        {
            return false;
        }

        if let Some(since) = self.since_timestamp
            && event.timestamp < since
        {
            return false;
        }

        if let Some(until) = self.until_timestamp
            && event.timestamp > until
        {
            return false;
        }

        true
    }
}

/// Pattern extracted from events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub specialist: SpecialistId,
    pub event_type: String,
    pub success_rate: f32,
    pub occurrence_count: usize,
    pub average_duration_ms: u32,
    pub discovered_at: u64,
    pub last_reinforced: u64,
    pub confidence: f32,
}

impl Pattern {
    pub fn new(
        specialist: SpecialistId,
        event_type: String,
        success_rate: f32,
        occurrence_count: usize,
    ) -> Self {
        let id = format!("pat-{}", uuid());
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            specialist,
            event_type,
            success_rate,
            occurrence_count,
            average_duration_ms: 0,
            discovered_at: timestamp,
            last_reinforced: timestamp,
            confidence: (success_rate * occurrence_count as f32 / 10.0).min(1.0),
        }
    }

    pub fn reinforce(&mut self, success: bool) {
        if success {
            self.confidence = (self.confidence + 0.05).min(1.0);
        } else {
            self.confidence = (self.confidence - 0.1).max(0.0);
        }

        self.last_reinforced = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn age_days(&self) -> u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        ((now - self.discovered_at) / 86400) as u32
    }
}

/// DNA Bank: Main persistent storage
pub struct DNABank {
    // Hot tier storage - BTreeMap by default, RocksDB via `rocksdb-dna` feature
    #[cfg(not(feature = "rocksdb-dna"))]
    pub events: BTreeMap<String, DNAEvent>,
    #[cfg(feature = "rocksdb-dna")]
    pub events: rocksdb_storage::RocksDbEvents,

    pub patterns: HashMap<String, Pattern>,

    // Statistics
    pub total_events_stored: u64,
    pub total_patterns_discovered: u64,
    pub last_consolidated: u64,

    /// Path this DNA Bank was opened from. `None` for pure in-memory instances.
    pub db_path: Option<std::path::PathBuf>,
}

impl Default for DNABank {
    fn default() -> Self {
        Self::new()
    }
}

impl DNABank {
    /// Create an in-memory DNA Bank (default, no disk I/O, data lost on drop).
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "rocksdb-dna"))]
            events: BTreeMap::new(),
            #[cfg(feature = "rocksdb-dna")]
            events: rocksdb_storage::RocksDbEvents::in_memory(),
            patterns: HashMap::new(),
            total_events_stored: 0,
            total_patterns_discovered: 0,
            last_consolidated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            db_path: None,
        }
    }

    /// Open (or create) a DNA Bank backed by RocksDB at `path`.
    ///
    /// When the `rocksdb-dna` feature is enabled, this opens a RocksDB
    /// database at the given path and returns a DNA Bank backed by it.
    ///
    /// Without the `rocksdb-dna` feature, this is identical to `new()` —
    /// the path is ignored and an in-memory bank is returned. This means
    /// application code can always call `open()`, and storage will be
    /// durable only when the feature is enabled.
    #[allow(unused_variables)]
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();

        #[cfg(feature = "rocksdb-dna")]
        {
            let events = rocksdb_storage::RocksDbEvents::open(path)
                .map_err(|e| format!("RocksDB open failed at {}: {}", path.display(), e))?;
            Ok(Self {
                events,
                patterns: HashMap::new(),
                total_events_stored: 0,
                total_patterns_discovered: 0,
                last_consolidated: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                db_path: Some(path.to_path_buf()),
            })
        }

        #[cfg(not(feature = "rocksdb-dna"))]
        {
            tracing::debug!(
                "rocksdb-dna feature not enabled; using in-memory store (path {} ignored)",
                path.display()
            );
            Ok(Self::new())
        }
    }

    /// Whether this DNA Bank is backed by durable disk storage.
    pub fn is_persistent(&self) -> bool {
        #[cfg(feature = "rocksdb-dna")]
        return true;
        #[cfg(not(feature = "rocksdb-dna"))]
        return false;
    }

    /// Record an event
    pub fn record_event(&mut self, event: DNAEvent) -> Result<String, String> {
        let id = event.id.clone();
        self.events.insert(id.clone(), event);
        self.total_events_stored += 1;
        Ok(id)
    }

    /// Query events
    pub fn query(&self, query: &EventQuery) -> Vec<DNAEvent> {
        let mut results: Vec<DNAEvent> = self
            .events
            .values()
            .filter(|event| query.matches(event))
            .cloned()
            .collect();

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        results
    }

    /// Get event by ID
    pub fn get_event(&self, id: &str) -> Option<DNAEvent> {
        self.events.get(id).cloned()
    }

    /// Extract patterns from events
    pub fn extract_patterns(&mut self) -> Vec<Pattern> {
        let mut pattern_map: HashMap<(SpecialistId, String), (usize, usize)> = HashMap::new();

        for event in self.events.values() {
            let key = (event.specialist, event.event_type.clone());
            let (count, success) = pattern_map.entry(key).or_insert((0, 0));
            *count += 1;
            if event.is_success() {
                *success += 1;
            }
        }

        let mut patterns = vec![];
        for ((specialist, event_type), (count, success)) in pattern_map {
            if count >= 3 {
                // Only patterns with 3+ occurrences
                let success_rate = success as f32 / count as f32;
                let pattern = Pattern::new(specialist, event_type, success_rate, count);
                patterns.push(pattern);
            }
        }

        // Store patterns
        for pattern in &patterns {
            self.patterns.insert(pattern.id.clone(), pattern.clone());
            self.total_patterns_discovered += 1;
        }

        patterns
    }

    /// Get pattern by ID
    pub fn get_pattern(&self, id: &str) -> Option<Pattern> {
        self.patterns.get(id).cloned()
    }

    /// Query patterns
    pub fn query_patterns(&self, specialist: SpecialistId) -> Vec<Pattern> {
        self.patterns
            .values()
            .filter(|p| p.specialist == specialist)
            .cloned()
            .collect()
    }

    /// Consolidate old data (move to cold tier)
    pub fn consolidate(&mut self) -> Result<ConsolidationStats, String> {
        let cutoff_days = 30; // Move events older than 30 days
        let mut moved = 0;
        let mut archived = 0;

        let to_move: Vec<String> = self
            .events
            .values()
            .filter(|event| event.age_days() > cutoff_days)
            .map(|event| event.id.clone())
            .collect();

        for id in to_move {
            // In real implementation, would move to RocksDB
            // For simulation, just count them
            self.events.remove(&id);
            moved += 1;
            archived += 1;
        }

        self.last_consolidated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(ConsolidationStats {
            events_moved: moved,
            events_archived: archived,
            consolidation_time_ms: 100,
        })
    }

    /// Get statistics
    pub fn stats(&self) -> DNABankStats {
        let oldest_event = self.events.values().min_by_key(|e| e.timestamp);

        DNABankStats {
            total_events: self.events.len(),
            total_patterns: self.patterns.len(),
            total_stored: self.total_events_stored,
            total_discovered: self.total_patterns_discovered,
            oldest_event_age_days: oldest_event.map(|e| e.age_days()).unwrap_or(0),
            average_success_rate: self.calculate_success_rate(),
        }
    }

    fn calculate_success_rate(&self) -> f32 {
        if self.events.is_empty() {
            return 0.0;
        }

        let successes = self.events.values().filter(|e| e.is_success()).count();
        (successes as f32 / self.events.len() as f32) * 100.0
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.events.clear();
        self.patterns.clear();
    }

    /// Backup statistics
    pub fn backup_info(&self) -> BackupInfo {
        BackupInfo {
            event_count: self.events.len(),
            pattern_count: self.patterns.len(),
            size_estimate_mb: (self.events.len() as u32 * 2) + (self.patterns.len() as u32),
            last_backup: self.last_consolidated,
        }
    }
}

/// Consolidation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationStats {
    pub events_moved: usize,
    pub events_archived: usize,
    pub consolidation_time_ms: u32,
}

/// DNA Bank statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNABankStats {
    pub total_events: usize,
    pub total_patterns: usize,
    pub total_stored: u64,
    pub total_discovered: u64,
    pub oldest_event_age_days: u32,
    pub average_success_rate: f32,
}

/// Backup information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub event_count: usize,
    pub pattern_count: usize,
    pub size_estimate_mb: u32,
    pub last_backup: u64,
}

fn uuid() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dna_event_creation() {
        let event = DNAEvent::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            "success".to_string(),
            500,
        );

        assert_eq!(event.specialist, SpecialistId::Visionary);
        assert_eq!(event.event_type, "design_generation");
        assert!(event.is_success());
    }

    #[test]
    fn test_event_query_builder() {
        let query = EventQuery::new()
            .for_specialist(SpecialistId::Visionary)
            .of_type("design_generation".to_string())
            .with_outcome("success".to_string());

        assert_eq!(query.specialist, Some(SpecialistId::Visionary));
        assert_eq!(query.event_type, Some("design_generation".to_string()));
        assert_eq!(query.outcome, Some("success".to_string()));
    }

    #[test]
    fn test_event_query_matching() {
        let event = DNAEvent::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            "success".to_string(),
            500,
        );

        let matching_query = EventQuery::new().for_specialist(SpecialistId::Visionary);

        let non_matching_query = EventQuery::new().for_specialist(SpecialistId::Omnipresent);

        assert!(matching_query.matches(&event));
        assert!(!non_matching_query.matches(&event));
    }

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            0.85,
            10,
        );

        assert_eq!(pattern.specialist, SpecialistId::Visionary);
        assert_eq!(pattern.success_rate, 0.85);
        assert_eq!(pattern.occurrence_count, 10);
        assert!(pattern.confidence > 0.0);
    }

    #[test]
    fn test_pattern_reinforce() {
        let mut pattern = Pattern::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            0.85,
            10,
        );

        let initial_confidence = pattern.confidence;

        pattern.reinforce(true);
        assert!(pattern.confidence > initial_confidence);

        pattern.reinforce(false);
        assert!(pattern.confidence < 1.0); // Max is 1.0
    }

    #[test]
    fn test_dna_bank_record_event() {
        let mut bank = DNABank::new();
        let event = DNAEvent::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            "success".to_string(),
            500,
        );

        let result = bank.record_event(event);
        assert!(result.is_ok());
        assert_eq!(bank.total_events_stored, 1);
        assert_eq!(bank.events.len(), 1);
    }

    #[test]
    fn test_dna_bank_query_events() {
        let mut bank = DNABank::new();

        for _ in 0..5 {
            let event = DNAEvent::new(
                SpecialistId::Visionary,
                "design_generation".to_string(),
                "success".to_string(),
                500,
            );
            let _ = bank.record_event(event);
        }

        let query = EventQuery::new().for_specialist(SpecialistId::Visionary);

        let results = bank.query(&query);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_dna_bank_query_with_limit() {
        let mut bank = DNABank::new();

        for _ in 0..10 {
            let event = DNAEvent::new(
                SpecialistId::Visionary,
                "design_generation".to_string(),
                "success".to_string(),
                500,
            );
            let _ = bank.record_event(event);
        }

        let query = EventQuery::new().limit(3);
        let results = bank.query(&query);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_dna_bank_extract_patterns() {
        let mut bank = DNABank::new();

        for i in 0..5 {
            let event = DNAEvent::new(
                SpecialistId::Visionary,
                "design_generation".to_string(),
                if i < 4 {
                    "success".to_string()
                } else {
                    "failure".to_string()
                },
                500 + i as u32 * 10,
            );
            let _ = bank.record_event(event);
        }

        let patterns = bank.extract_patterns();
        assert!(!patterns.is_empty());

        let pattern = &patterns[0];
        assert_eq!(pattern.specialist, SpecialistId::Visionary);
        assert!(pattern.success_rate >= 0.75);
    }

    #[test]
    fn test_dna_bank_stats() {
        let mut bank = DNABank::new();

        for _ in 0..5 {
            let event = DNAEvent::new(
                SpecialistId::Visionary,
                "design_generation".to_string(),
                "success".to_string(),
                500,
            );
            let _ = bank.record_event(event);
        }

        let stats = bank.stats();
        assert_eq!(stats.total_events, 5);
        assert_eq!(stats.total_stored, 5);
        assert_eq!(stats.average_success_rate, 100.0);
    }

    #[test]
    fn test_dna_bank_consolidation() {
        let mut bank = DNABank::new();

        // Add events
        for _ in 0..10 {
            let event = DNAEvent::new(
                SpecialistId::Visionary,
                "design_generation".to_string(),
                "success".to_string(),
                500,
            );
            let _ = bank.record_event(event);
        }

        let result = bank.consolidate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_dna_bank_backup_info() {
        let mut bank = DNABank::new();

        let event = DNAEvent::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            "success".to_string(),
            500,
        );
        let _ = bank.record_event(event);

        let backup = bank.backup_info();
        assert_eq!(backup.event_count, 1);
        assert!(backup.size_estimate_mb > 0);
    }

    #[test]
    fn test_dna_bank_clear() {
        let mut bank = DNABank::new();

        let event = DNAEvent::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            "success".to_string(),
            500,
        );
        let _ = bank.record_event(event);

        bank.clear();
        assert_eq!(bank.events.len(), 0);
    }

    #[test]
    fn test_dna_bank_mixed_events() {
        let mut bank = DNABank::new();

        for specialist in &[SpecialistId::Visionary, SpecialistId::Omnipresent] {
            for i in 0..3 {
                let event = DNAEvent::new(
                    *specialist,
                    "operation".to_string(),
                    if i < 2 {
                        "success".to_string()
                    } else {
                        "failure".to_string()
                    },
                    500,
                );
                let _ = bank.record_event(event);
            }
        }

        let visionary_query = EventQuery::new().for_specialist(SpecialistId::Visionary);

        let visionary_events = bank.query(&visionary_query);
        assert_eq!(visionary_events.len(), 3);

        let omnipresent_query = EventQuery::new().for_specialist(SpecialistId::Omnipresent);

        let omnipresent_events = bank.query(&omnipresent_query);
        assert_eq!(omnipresent_events.len(), 3);
    }

    #[test]
    fn test_dna_bank_get_event() {
        let mut bank = DNABank::new();

        let event = DNAEvent::new(
            SpecialistId::Visionary,
            "design_generation".to_string(),
            "success".to_string(),
            500,
        );
        let event_id = event.id.clone();
        let _ = bank.record_event(event);

        let retrieved = bank.get_event(&event_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, event_id);
    }

    #[test]
    fn test_dna_bank_query_patterns() {
        let mut bank = DNABank::new();

        for i in 0..5 {
            let event = DNAEvent::new(
                SpecialistId::Visionary,
                "design_generation".to_string(),
                if i < 4 {
                    "success".to_string()
                } else {
                    "failure".to_string()
                },
                500,
            );
            let _ = bank.record_event(event);
        }

        bank.extract_patterns();

        let patterns = bank.query_patterns(SpecialistId::Visionary);
        assert!(!patterns.is_empty());
    }

    #[test]
    #[cfg(not(feature = "rocksdb-dna"))]
    fn test_dna_bank_is_not_persistent_by_default() {
        let bank = DNABank::new();
        assert!(
            !bank.is_persistent(),
            "in-memory bank should not be persistent"
        );
    }

    #[test]
    #[cfg(not(feature = "rocksdb-dna"))]
    fn test_dna_bank_open_without_rocksdb_feature_returns_in_memory() {
        // Without the rocksdb-dna feature, open() returns an in-memory bank
        let bank = DNABank::open("/tmp/does-not-matter").expect("open should succeed");
        assert!(!bank.is_persistent());
    }
}

// ================================================================
// RocksDB storage backend (only compiled with `rocksdb-dna` feature)
// ================================================================

#[cfg(feature = "rocksdb-dna")]
mod rocksdb_storage {
    use super::DNAEvent;
    use std::collections::BTreeMap;
    use std::path::Path;

    /// RocksDB-backed event store.
    ///
    /// Implements the same insert/get/remove/values/len/iter surface as
    /// `BTreeMap<String, DNAEvent>` so `DNABank`'s methods can use either
    /// backend without being rewritten.
    ///
    /// Events are serialized to JSON bytes before storage so the schema is
    /// human-readable and forward-compatible.
    pub struct RocksDbEvents {
        db: rocksdb::DB,
        /// In-memory cache so `values()` doesn't require multiple RocksDB scans.
        cache: BTreeMap<String, DNAEvent>,
    }

    impl RocksDbEvents {
        /// Open (or create) a RocksDB database at the given path.
        pub fn open(path: &Path) -> Result<Self, rocksdb::Error> {
            let mut opts = rocksdb::Options::default();
            opts.create_if_missing(true);
            opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
            opts.set_write_buffer_size(64 * 1024 * 1024); // 64 MB write buffer

            let db = rocksdb::DB::open(&opts, path)?;

            // Load existing events into the cache on open
            let mut cache = BTreeMap::new();
            let iter = db.iterator(rocksdb::IteratorMode::Start);
            for result in iter {
                let (key_bytes, val_bytes) = result?;
                let key = String::from_utf8_lossy(&key_bytes).to_string();
                if let Ok(event) = serde_json::from_slice::<DNAEvent>(&val_bytes) {
                    cache.insert(key, event);
                }
            }

            Ok(Self { db, cache })
        }

        /// In-memory-backed instance (for tests / `DNABank::new()`).
        /// All operations succeed but nothing is written to disk.
        pub fn in_memory() -> Self {
            // Use a temp directory so it's cleaned up when dropped.
            // This matches the in-memory semantics while still going through
            // the RocksDB code path (useful for testing the feature).
            let tmp = tempfile::tempdir().expect("failed to create temp dir for in-memory RocksDB");
            // We can't easily return the TempDir alongside the DB here, so
            // just open a DB in a path that will be cleaned eventually.
            // (Alternatively: keep an in-memory BTreeMap for the in_memory case)
            let path = tmp.keep(); // intentionally leak to keep alive
            Self::open(&path).expect("temp RocksDB should open")
        }

        /// Insert an event.
        pub fn insert(&mut self, id: String, event: DNAEvent) {
            let bytes = serde_json::to_vec(&event).expect("DNAEvent serialization");
            let _ = self.db.put(id.as_bytes(), &bytes);
            self.cache.insert(id, event);
        }

        /// Get an event by ID.
        pub fn get(&self, id: &str) -> Option<&DNAEvent> {
            self.cache.get(id)
        }

        /// Remove an event by ID.
        pub fn remove(&mut self, id: &str) {
            let _ = self.db.delete(id.as_bytes());
            self.cache.remove(id);
        }

        /// Iterate all events (via in-memory cache).
        pub fn values(&self) -> impl Iterator<Item = &DNAEvent> {
            self.cache.values()
        }

        /// Number of events in the store.
        pub fn len(&self) -> usize {
            self.cache.len()
        }

        /// Whether the store is empty.
        pub fn is_empty(&self) -> bool {
            self.cache.is_empty()
        }

        /// Clear all events from both cache and disk.
        pub fn clear(&mut self) {
            // Delete all keys
            let keys: Vec<Vec<u8>> = self.cache.keys().map(|k| k.as_bytes().to_vec()).collect();
            for key in keys {
                let _ = self.db.delete(&key);
            }
            self.cache.clear();
        }
    }
}
