/// Snapshot management for log compaction and fast recovery

use super::types::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
}
