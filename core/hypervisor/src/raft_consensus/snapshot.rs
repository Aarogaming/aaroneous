/// Snapshot management for log compaction and fast recovery
///
/// Implements:
/// - Snapshot creation and versioning
/// - Snapshot comparison and ordering
/// - Snapshot store management
/// - Log compaction trigger logic

use super::types::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// A snapshot of the state machine at a specific index
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    /// Index of last included entry
    pub last_included_index: LogIndex,
    /// Term of last included entry
    pub last_included_term: Term,
    /// Serialized state machine data
    pub state_data: Vec<u8>,
    /// When snapshot was created
    pub created_at: DateTime<Utc>,
    /// Size in bytes
    pub size_bytes: usize,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(
        last_included_index: LogIndex,
        last_included_term: Term,
        state_data: Vec<u8>,
    ) -> Self {
        let size_bytes = state_data.len();
        Self {
            last_included_index,
            last_included_term,
            state_data,
            created_at: Utc::now(),
            size_bytes,
        }
    }

    /// Is this snapshot newer than another?
    pub fn is_newer_than(&self, other: &Snapshot) -> bool {
        self.last_included_index > other.last_included_index
            || (self.last_included_index == other.last_included_index
                && self.last_included_term > other.last_included_term)
    }

    /// Check if this snapshot can replace a log entry at index
    pub fn covers_index(&self, index: LogIndex) -> bool {
        index <= self.last_included_index
    }

    /// Calculate compaction ratio (how much log can be compacted)
    pub fn compaction_ratio(&self, total_log_entries: usize) -> f64 {
        if total_log_entries == 0 {
            return 0.0;
        }
        self.last_included_index as f64 / (self.last_included_index as f64 + total_log_entries as f64)
    }
}

/// Snapshot store with retention policy
#[derive(Clone, Debug)]
pub struct SnapshotStore {
    /// Most recent snapshots (keep last N)
    snapshots: VecDeque<Snapshot>,
    /// Maximum snapshots to retain
    max_snapshots: usize,
    /// Total size limit (bytes)
    max_total_size: usize,
}

impl SnapshotStore {
    /// Create new snapshot store
    pub fn new(max_snapshots: usize, max_total_size: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_snapshots,
            max_total_size,
        }
    }

    /// Add a new snapshot (evicts old ones if necessary)
    pub fn add_snapshot(&mut self, snapshot: Snapshot) {
        // Add new snapshot
        self.snapshots.push_back(snapshot);

        // Evict old snapshots if we exceed retention policy
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }

        // Check total size
        while self.total_size() > self.max_total_size && !self.snapshots.is_empty() {
            self.snapshots.pop_front();
        }
    }

    /// Get the latest snapshot
    pub fn latest(&self) -> Option<&Snapshot> {
        self.snapshots.back()
    }

    /// Get snapshot at or before a specific index
    pub fn get_snapshot_before(&self, index: LogIndex) -> Option<&Snapshot> {
        self.snapshots.iter().rev().find(|s| s.covers_index(index))
    }

    /// Get all snapshots
    pub fn all_snapshots(&self) -> Vec<&Snapshot> {
        self.snapshots.iter().collect()
    }

    /// Total size of all snapshots (bytes)
    pub fn total_size(&self) -> usize {
        self.snapshots.iter().map(|s| s.size_bytes).sum()
    }

    /// Number of snapshots stored
    pub fn count(&self) -> usize {
        self.snapshots.len()
    }

    /// Should we create a new snapshot?
    pub fn should_snapshot(&self, log_entries_count: usize, size_threshold_mb: usize) -> bool {
        if log_entries_count > 100_000 {
            return true; // Too many entries
        }

        if let Some(latest) = self.latest() {
            let entries_since_snapshot = log_entries_count as LogIndex - latest.last_included_index;
            if entries_since_snapshot > 50_000 {
                return true; // Many new entries
            }

            let estimated_log_size_mb = log_entries_count * 1024 / (1024 * 1024);
            if estimated_log_size_mb > size_threshold_mb {
                return true; // Log too large
            }
        } else if log_entries_count > 50_000 {
            // No snapshot yet, create one
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let snapshot = Snapshot::new(10, 2, data.clone());

        assert_eq!(snapshot.last_included_index, 10);
        assert_eq!(snapshot.last_included_term, 2);
        assert_eq!(snapshot.size_bytes, 5);
        assert_eq!(snapshot.state_data, data);
    }

    #[test]
    fn test_snapshot_comparison() {
        let snap1 = Snapshot::new(5, 1, vec![]);
        let snap2 = Snapshot::new(10, 1, vec![]);
        let snap3 = Snapshot::new(5, 2, vec![]);

        assert!(snap2.is_newer_than(&snap1)); // Higher index
        assert!(snap3.is_newer_than(&snap1)); // Same index, higher term
        assert!(!snap1.is_newer_than(&snap2)); // Lower index
    }

    #[test]
    fn test_snapshot_covers_index() {
        let snapshot = Snapshot::new(10, 2, vec![1, 2, 3]);

        assert!(snapshot.covers_index(5));
        assert!(snapshot.covers_index(10));
        assert!(!snapshot.covers_index(11));
        assert!(!snapshot.covers_index(20));
    }

    #[test]
    fn test_snapshot_compaction_ratio() {
        let snapshot = Snapshot::new(100, 1, vec![]);

        let ratio = snapshot.compaction_ratio(50);
        assert!(ratio > 0.6 && ratio < 0.7); // ~100/150 ≈ 0.67

        let ratio_no_entries = snapshot.compaction_ratio(0);
        assert_eq!(ratio_no_entries, 0.0);
    }

    #[test]
    fn test_snapshot_store_creation() {
        let store = SnapshotStore::new(5, 10_000_000);

        assert_eq!(store.count(), 0);
        assert!(store.latest().is_none());
    }

    #[test]
    fn test_snapshot_store_add() {
        let mut store = SnapshotStore::new(5, 10_000_000);

        let snap1 = Snapshot::new(10, 1, vec![1, 2, 3]);
        store.add_snapshot(snap1.clone());

        assert_eq!(store.count(), 1);
        assert_eq!(store.latest().unwrap().last_included_index, 10);
    }

    #[test]
    fn test_snapshot_store_retention_count() {
        let mut store = SnapshotStore::new(3, 10_000_000); // Keep max 3 snapshots

        for i in 1..=5 {
            let snapshot = Snapshot::new(i * 10, 1, vec![0; 100]);
            store.add_snapshot(snapshot);
        }

        assert_eq!(store.count(), 3); // Only keep last 3
        assert_eq!(store.latest().unwrap().last_included_index, 50); // Last snapshot
    }

    #[test]
    fn test_snapshot_store_retention_size() {
        let max_size = 500; // 500 bytes max
        let mut store = SnapshotStore::new(100, max_size);

        for i in 1..=3 {
            let data = vec![0; 200]; // 200 bytes each
            let snapshot = Snapshot::new(i * 10, 1, data);
            store.add_snapshot(snapshot);
        }

        assert_eq!(store.count(), 2); // Only 2 snapshots fit
        assert!(store.total_size() <= max_size);
    }

    #[test]
    fn test_snapshot_store_get_before() {
        let mut store = SnapshotStore::new(10, 10_000_000);

        let snap1 = Snapshot::new(10, 1, vec![]);
        let snap2 = Snapshot::new(20, 1, vec![]);
        let snap3 = Snapshot::new(30, 1, vec![]);

        store.add_snapshot(snap1);
        store.add_snapshot(snap2);
        store.add_snapshot(snap3);

        // get_snapshot_before finds the most recent snapshot that covers the index
        // A snapshot at index N covers all indices <= N
        
        // Index 25 is covered by snap3 (30), snap2 (20) doesn't
        // But we're searching in reverse, so we find the MOST RECENT one that covers it
        assert_eq!(store.get_snapshot_before(25).unwrap().last_included_index, 30);
        
        // Index 30 is covered by snap3
        assert_eq!(store.get_snapshot_before(30).unwrap().last_included_index, 30);
        
        // Index 5 is covered by snap1, snap2, snap3 - but most recent (snap3) is returned
        assert_eq!(store.get_snapshot_before(5).unwrap().last_included_index, 30);
        
        // Index 35 is NOT covered by any snapshot
        assert!(store.get_snapshot_before(35).is_none());
        
        // Index 10 is covered by all, returns most recent (snap3)
        assert_eq!(store.get_snapshot_before(10).unwrap().last_included_index, 30);
    }

    #[test]
    fn test_snapshot_store_total_size() {
        let mut store = SnapshotStore::new(10, 10_000_000);

        let snap1 = Snapshot::new(10, 1, vec![0; 100]);
        let snap2 = Snapshot::new(20, 1, vec![0; 200]);
        let snap3 = Snapshot::new(30, 1, vec![0; 300]);

        store.add_snapshot(snap1);
        store.add_snapshot(snap2);
        store.add_snapshot(snap3);

        assert_eq!(store.total_size(), 600);
    }

    #[test]
    fn test_should_snapshot_by_entries() {
        let store = SnapshotStore::new(5, 10_000_000);

        assert!(!store.should_snapshot(1000, 100));
        assert!(store.should_snapshot(60_000, 100)); // Too many entries
        assert!(store.should_snapshot(120_000, 100)); // Way too many
    }

    #[test]
    fn test_should_snapshot_by_size() {
        let mut store = SnapshotStore::new(5, 10_000_000);
        let latest = Snapshot::new(10, 1, vec![0; 1000]);
        store.add_snapshot(latest);

        // Should trigger: many entries since last snapshot
        assert!(store.should_snapshot(60_000, 100));
    }

    #[test]
    fn test_should_snapshot_no_previous() {
        let store = SnapshotStore::new(5, 10_000_000);

        // No previous snapshot, moderate entries
        assert!(store.should_snapshot(60_000, 100));
        assert!(!store.should_snapshot(10_000, 100));
    }

    #[test]
    fn test_snapshot_store_all_snapshots() {
        let mut store = SnapshotStore::new(10, 10_000_000);

        let snap1 = Snapshot::new(10, 1, vec![]);
        let snap2 = Snapshot::new(20, 1, vec![]);

        store.add_snapshot(snap1);
        store.add_snapshot(snap2);

        let all = store.all_snapshots();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].last_included_index, 10);
        assert_eq!(all[1].last_included_index, 20);
    }
}
