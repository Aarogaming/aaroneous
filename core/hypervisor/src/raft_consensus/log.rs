/// Append-only Raft log with snapshot support
///
/// Maintains the log of state machine commands with:
/// - Immutable append-only property
/// - Snapshot integration for compaction
/// - Efficient lookups and range queries

use super::types::*;
use std::sync::{Arc, RwLock};

/// Raft log manager
#[derive(Clone, Debug)]
pub struct RaftLog {
    /// Log entries (index 1-based, index 0 is unused)
    entries: Arc<RwLock<Vec<LogEntry>>>,
    /// Last included index from snapshot
    last_included_index: Arc<RwLock<LogIndex>>,
    /// Last included term from snapshot
    last_included_term: Arc<RwLock<Term>>,
}

impl RaftLog {
    /// Create new empty log
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(vec![])),
            last_included_index: Arc::new(RwLock::new(0)),
            last_included_term: Arc::new(RwLock::new(0)),
        }
    }

    /// Append entry to log
    pub fn append(&self, entry: LogEntry) -> Result<LogIndex, String> {
        let mut entries = self.entries.write().map_err(|_| "Failed to acquire lock")?;

        let index = entry.index;
        let _term = entry.term;

        // Validate index is sequential
        let expected_index = if entries.is_empty() {
            *self.last_included_index.read().map_err(|_| "Failed to acquire lock")?+ 1
        } else {
            entries.last().unwrap().index + 1
        };

        if index != expected_index {
            return Err(format!(
                "Non-sequential append: expected {}, got {}",
                expected_index, index
            ));
        }

        entries.push(entry);
        Ok(index)
    }

    /// Append multiple entries (for batch operations)
    pub fn append_batch(&self, batch: Vec<LogEntry>) -> Result<LogIndex, String> {
        let mut entries = self.entries.write().map_err(|_| "Failed to acquire lock")?;

        if batch.is_empty() {
            return Err("Cannot append empty batch".to_string());
        }

        let mut last_index = if entries.is_empty() {
            *self.last_included_index.read().map_err(|_| "Failed to acquire lock")?
        } else {
            entries.last().unwrap().index
        };

        // Validate all entries are sequential
        for entry in &batch {
            last_index += 1;
            if entry.index != last_index {
                return Err(format!("Non-sequential batch: expected {}, got {}", last_index, entry.index));
            }
        }

        entries.extend(batch);
        Ok(last_index)
    }

    /// Get entry by index
    pub fn get(&self, index: LogIndex) -> Result<Option<LogEntry>, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;

        if entries.is_empty() {
            return Ok(None);
        }

        let first_index = entries[0].index;
        if index < first_index {
            return Ok(None); // Index is in compacted part
        }

        let offset = (index - first_index) as usize;
        Ok(entries.get(offset).cloned())
    }

    /// Get last entry
    pub fn last_entry(&self) -> Result<Option<LogEntry>, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;
        Ok(entries.last().cloned())
    }

    /// Get last log index
    pub fn last_index(&self) -> Result<LogIndex, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;

        if entries.is_empty() {
            return Ok(*self.last_included_index.read().map_err(|_| "Failed to acquire lock")?);
        }

        Ok(entries.last().unwrap().index)
    }

    /// Get last log term
    pub fn last_term(&self) -> Result<Term, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;

        if entries.is_empty() {
            return Ok(*self.last_included_term.read().map_err(|_| "Failed to acquire lock")?);
        }

        Ok(entries.last().unwrap().term)
    }

    /// Get entries in range [start_index, end_index)
    pub fn get_range(&self, start_index: LogIndex, end_index: LogIndex) -> Result<Vec<LogEntry>, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;

        if entries.is_empty() {
            return Ok(vec![]);
        }

        let first_index = entries[0].index;
        if end_index <= first_index {
            return Ok(vec![]);
        }

        let start_offset = if start_index < first_index {
            0
        } else {
            (start_index - first_index) as usize
        };

        let end_offset = ((end_index - first_index) as usize).min(entries.len());

        Ok(entries[start_offset..end_offset].to_vec())
    }

    /// Delete entries from index onwards (truncate)
    /// Used when follower's log diverges from leader's
    pub fn truncate_from(&self, index: LogIndex) -> Result<(), String> {
        let mut entries = self.entries.write().map_err(|_| "Failed to acquire lock")?;

        if entries.is_empty() {
            return Ok(());
        }

        let first_index = entries[0].index;
        if index <= first_index {
            entries.clear();
            return Ok(());
        }

        let offset = (index - first_index) as usize;
        entries.truncate(offset);
        Ok(())
    }

    /// Get entry term at index
    pub fn term_at(&self, index: LogIndex) -> Result<Option<Term>, String> {
        if index == 0 {
            return Ok(Some(0)); // Convention: term 0 for index 0
        }

        let last_included_index = *self.last_included_index.read().map_err(|_| "Failed to acquire lock")?;
        let last_included_term = *self.last_included_term.read().map_err(|_| "Failed to acquire lock")?;

        if index == last_included_index {
            return Ok(Some(last_included_term));
        }

        self.get(index).map(|entry| entry.map(|e| e.term))
    }

    /// Length of log (entries + snapshot)
    pub fn len(&self) -> Result<usize, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;
        let last_included = *self.last_included_index.read().map_err(|_| "Failed to acquire lock")?;
        Ok(entries.len() + last_included as usize)
    }

    /// Is log empty?
    pub fn is_empty(&self) -> Result<bool, String> {
        let entries = self.entries.read().map_err(|_| "Failed to acquire lock")?;
        let last_included = *self.last_included_index.read().map_err(|_| "Failed to acquire lock")?;
        Ok(entries.is_empty() && last_included == 0)
    }

    /// Apply snapshot (discards entries up to last_included_index)
    pub fn apply_snapshot(&self, last_included_index: LogIndex, last_included_term: Term) -> Result<(), String> {
        let mut entries = self.entries.write().map_err(|_| "Failed to acquire lock")?;
        let mut lis = self.last_included_index.write().map_err(|_| "Failed to acquire lock")?;
        let mut lit = self.last_included_term.write().map_err(|_| "Failed to acquire lock")?;

        // Remove all entries with index <= last_included_index
        while !entries.is_empty() && entries[0].index <= last_included_index {
            entries.remove(0);
        }

        *lis = last_included_index;
        *lit = last_included_term;

        Ok(())
    }

    /// Get last included index (from snapshot)
    pub fn last_included_index(&self) -> Result<LogIndex, String> {
        Ok(*self.last_included_index.read().map_err(|_| "Failed to acquire lock")?)
    }

    /// Get last included term (from snapshot)
    pub fn last_included_term(&self) -> Result<Term, String> {
        Ok(*self.last_included_term.read().map_err(|_| "Failed to acquire lock")?)
    }
}

impl Default for RaftLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_single_entry() {
        let log = RaftLog::new();
        let entry = LogEntry::new(1, 0, serde_json::json!({"cmd": "test"}), "c1".to_string(), 1);

        let result = log.append(entry.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Verify we can retrieve it
        let retrieved = log.get(1).unwrap().unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.term, 0);
    }

    #[test]
    fn test_append_sequential() {
        let log = RaftLog::new();

        for i in 1..=5 {
            let entry = LogEntry::new(i, i % 2, serde_json::json!({}), "c".to_string(), i);
            assert!(log.append(entry).is_ok());
        }

        // Verify all entries
        for i in 1..=5 {
            let entry = log.get(i).unwrap().unwrap();
            assert_eq!(entry.index, i);
        }
    }

    #[test]
    fn test_append_non_sequential_fails() {
        let log = RaftLog::new();

        let entry1 = LogEntry::new(1, 0, serde_json::json!({}), "c".to_string(), 1);
        assert!(log.append(entry1).is_ok());

        // Try to append index 3 instead of 2
        let entry3 = LogEntry::new(3, 0, serde_json::json!({}), "c".to_string(), 3);
        assert!(log.append(entry3).is_err());
    }

    #[test]
    fn test_last_entry() {
        let log = RaftLog::new();

        // Empty log
        assert!(log.last_entry().unwrap().is_none());

        // Add entry
        let entry = LogEntry::new(1, 0, serde_json::json!({}), "c".to_string(), 1);
        log.append(entry.clone()).unwrap();

        let last = log.last_entry().unwrap().unwrap();
        assert_eq!(last.index, 1);
    }

    #[test]
    fn test_get_range() {
        let log = RaftLog::new();

        for i in 1..=5 {
            let entry = LogEntry::new(i, 0, serde_json::json!({}), "c".to_string(), i);
            log.append(entry).unwrap();
        }

        let range = log.get_range(2, 4).unwrap();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].index, 2);
        assert_eq!(range[1].index, 3);
    }

    #[test]
    fn test_truncate_from() {
        let log = RaftLog::new();

        for i in 1..=5 {
            let entry = LogEntry::new(i, 0, serde_json::json!({}), "c".to_string(), i);
            log.append(entry).unwrap();
        }

        // Truncate from index 3 (keep 1, 2)
        log.truncate_from(3).unwrap();

        assert!(log.get(3).unwrap().is_none());
        assert!(log.get(2).unwrap().is_some());
    }

    #[test]
    fn test_snapshot_compaction() {
        let log = RaftLog::new();

        for i in 1..=10 {
            let entry = LogEntry::new(i, 0, serde_json::json!({}), "c".to_string(), i);
            log.append(entry).unwrap();
        }

        // Apply snapshot up to index 5
        log.apply_snapshot(5, 0).unwrap();

        // Entries 1-5 should be gone
        assert!(log.get(5).unwrap().is_none());
        assert!(log.get(6).unwrap().is_some());

        // But we should track the snapshot
        assert_eq!(log.last_included_index().unwrap(), 5);
    }

    #[test]
    fn test_term_at() {
        let log = RaftLog::new();

        let entry1 = LogEntry::new(1, 1, serde_json::json!({}), "c".to_string(), 1);
        let entry2 = LogEntry::new(2, 2, serde_json::json!({}), "c".to_string(), 2);

        log.append(entry1).unwrap();
        log.append(entry2).unwrap();

        assert_eq!(log.term_at(0).unwrap(), Some(0));
        assert_eq!(log.term_at(1).unwrap(), Some(1));
        assert_eq!(log.term_at(2).unwrap(), Some(2));
    }
}
