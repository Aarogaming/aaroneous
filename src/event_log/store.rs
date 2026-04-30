use crate::event_log::types::{FederationEvent, EventLogError, LogOffset};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory event store (RocksDB integration to come in optimization phase)
/// For Phase 6A.2, we use memory-backed store with serialization capability
pub struct EventLogStore {
    /// Events indexed by offset
    events: Arc<RwLock<Vec<FederationEvent>>>,
    /// Trace ID → offsets mapping for fast queries
    trace_index: Arc<RwLock<BTreeMap<String, Vec<LogOffset>>>>,
    /// Next event offset
    next_offset: Arc<AtomicU64>,
    /// Database path (for future RocksDB integration)
    db_path: std::path::PathBuf,
}

impl EventLogStore {
    /// Open or create event log store
    pub async fn open(db_path: impl AsRef<Path>) -> Result<Self, EventLogError> {
        let path = db_path.as_ref().to_path_buf();
        
        // Create directory if not exists
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| EventLogError::IoError(e.to_string()))?;
        }

        Ok(Self {
            events: Arc::new(RwLock::new(Vec::new())),
            trace_index: Arc::new(RwLock::new(BTreeMap::new())),
            next_offset: Arc::new(AtomicU64::new(0)),
            db_path: path,
        })
    }

    /// Append event to log
    pub async fn append(&self, event: FederationEvent) -> Result<LogOffset, EventLogError> {
        let offset = self.next_offset.fetch_add(1, Ordering::SeqCst);

        // Add to events
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
        }

        // Update trace index
        {
            let mut trace_index = self.trace_index.write().await;
            trace_index
                .entry(event.trace_id.clone())
                .or_insert_with(Vec::new)
                .push(offset);
        }

        Ok(offset)
    }

    /// Read range of events
    pub async fn read_range(
        &self,
        start: LogOffset,
        end: LogOffset,
    ) -> Result<Vec<FederationEvent>, EventLogError> {
        let events = self.events.read().await;
        
        let start = start as usize;
        let end = (end as usize).min(events.len());

        if start > events.len() {
            return Err(EventLogError::InvalidOffset(
                format!("Start offset {} exceeds log size {}", start, events.len()),
            ));
        }

        Ok(events[start..end].to_vec())
    }

    /// Query events by trace ID
    pub async fn query_by_trace(&self, trace_id: &str) -> Result<Vec<FederationEvent>, EventLogError> {
        let trace_index = self.trace_index.read().await;
        let events = self.events.read().await;

        let offsets = trace_index
            .get(trace_id)
            .ok_or_else(|| EventLogError::NotFound(format!("Trace {}", trace_id)))?;

        let result = offsets
            .iter()
            .filter_map(|&offset| events.get(offset as usize).cloned())
            .collect();

        Ok(result)
    }

    /// Create snapshot of current state
    pub async fn create_snapshot(&self) -> Result<String, EventLogError> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let events = self.events.read().await;
        
        let snapshot_path = self.db_path.join(format!("snapshot_{}.json", snapshot_id));
        
        let snapshot_data = serde_json::json!({
            "snapshot_id": snapshot_id,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "events": events.clone(),
            "event_count": events.len(),
        });

        let json = serde_json::to_string_pretty(&snapshot_data)
            .map_err(|e| EventLogError::SerializationError(e.to_string()))?;

        std::fs::write(&snapshot_path, json)
            .map_err(|e| EventLogError::SnapshotError(e.to_string()))?;

        Ok(snapshot_id)
    }

    /// Restore from snapshot
    pub async fn restore_from_snapshot(&self, snapshot_id: &str) -> Result<(), EventLogError> {
        let snapshot_path = self.db_path.join(format!("snapshot_{}.json", snapshot_id));
        
        let data = std::fs::read_to_string(&snapshot_path)
            .map_err(|e| EventLogError::SnapshotError(e.to_string()))?;

        let snapshot: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| EventLogError::SerializationError(e.to_string()))?;

        let events: Vec<FederationEvent> = serde_json::from_value(
            snapshot["events"].clone(),
        )
        .map_err(|e| EventLogError::SerializationError(e.to_string()))?;

        // Clear and restore
        {
            let mut store_events = self.events.write().await;
            store_events.clear();
            store_events.extend(events.clone());
        }

        // Rebuild trace index
        {
            let mut trace_index = self.trace_index.write().await;
            trace_index.clear();
            for (offset, event) in events.iter().enumerate() {
                trace_index
                    .entry(event.trace_id.clone())
                    .or_insert_with(Vec::new)
                    .push(offset as u64);
            }
        }

        self.next_offset.store(events.len() as u64, Ordering::SeqCst);

        Ok(())
    }

    /// Get log statistics
    pub async fn get_stats(&self) -> Result<super::EventLogStats, EventLogError> {
        let events = self.events.read().await;
        let trace_index = self.trace_index.read().await;

        let total_events = events.len() as u64;
        let log_size_bytes = serde_json::to_string(&*events)
            .unwrap_or_default()
            .len() as u64;

        let (earliest_timestamp, latest_timestamp) = if events.is_empty() {
            (0, 0)
        } else {
            (
                events.first().unwrap().timestamp,
                events.last().unwrap().timestamp,
            )
        };

        // Count unique repos
        let mut repos = std::collections::HashSet::new();
        for event in events.iter() {
            repos.insert(event.source_repo.clone());
        }

        Ok(super::EventLogStats {
            total_events,
            log_size_bytes,
            earliest_timestamp,
            latest_timestamp,
            unique_traces: trace_index.len() as u64,
            unique_repos: repos.len() as u64,
        })
    }

    /// Get current log size
    pub async fn size(&self) -> u64 {
        self.events.read().await.len() as u64
    }

    /// Get next offset that would be assigned
    pub fn next_offset(&self) -> u64 {
        self.next_offset.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::types::{EventType, Operation};

    #[tokio::test]
    async fn test_store_creation() {
        let store = EventLogStore::open("/tmp/test_event_log").await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_append_and_read() {
        let store = EventLogStore::open("/tmp/test_event_log_append").await.unwrap();
        
        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("test".to_string()),
        );

        let offset = store.append(event.clone()).await.unwrap();
        assert_eq!(offset, 0);

        let events = store.read_range(0, 1).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[tokio::test]
    async fn test_trace_query() {
        let store = EventLogStore::open("/tmp/test_event_log_trace").await.unwrap();
        
        let event1 = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("test".to_string()),
        );
        let event2 = FederationEvent::new(
            "trace-1",
            "Guild",
            "execution",
            EventType::PluginExec,
            Operation::Update("result".to_string()),
        );

        store.append(event1.clone()).await.unwrap();
        store.append(event2.clone()).await.unwrap();

        let events = store.query_by_trace("trace-1").await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_snapshot() {
        let store = EventLogStore::open("/tmp/test_event_log_snapshot").await.unwrap();
        
        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("test".to_string()),
        );

        store.append(event).await.unwrap();

        let snapshot_id = store.create_snapshot().await.unwrap();
        assert!(!snapshot_id.is_empty());

        let stats = store.get_stats().await.unwrap();
        assert_eq!(stats.total_events, 1);
    }
}
