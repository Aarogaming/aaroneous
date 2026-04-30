/// Event Log Module
///
/// Provides a distributed, append-only event log for federation-wide transaction tracking.
/// The event log is the single source of truth for federation state, enabling:
/// - Distributed tracing with causal ordering
/// - State machine replication via Raft consensus
/// - Checkpoint/restore for disaster recovery
/// - Audit trails for compliance
///
/// # Architecture
///
/// ```text
/// Event Sources                Event Log              Consumers
/// ├─ AAS mutations        ────→ ├─ RocksDB       ────→ ├─ Raft consensus
/// ├─ Guild actions            │  │ (local)           ├─ Distributed tracing
/// ├─ Merlin analysis          │  ├─ Snapshots       ├─ Distillation pipeline
/// └─ Monitoring alerts    ────→ ├─ Replication   ────→ └─ Recovery engine
///                              │  (to siblings)
///                              └─ Compaction
/// ```
///
/// # Event Types
///
/// - `Boot` - Federation startup or node join
/// - `PluginLoad` - Plugin loading/initialization
/// - `PluginExec` - Plugin execution result
/// - `Mutation` - State machine mutation (requires consensus)
/// - `HealthCheck` - Periodic health status
/// - `Validation` - Critic loop validation result
/// - `Repair` - Autonomous repair action
/// - `Distillation` - Knowledge extraction and compression
/// - `Failure` - Cascade or error event
/// - `Recovery` - Recovery operation

pub mod types;
pub mod store;
pub mod replicator;
pub mod compactor;

pub use types::{FederationEvent, EventType, Operation, EventLogError, EventId, LogOffset};
pub use store::EventLogStore;
pub use replicator::EventLogReplicator;
pub use compactor::EventCompactor;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Main event log interface combining store + replication
pub struct EventLog {
    store: Arc<EventLogStore>,
    replicator: Arc<EventLogReplicator>,
    compactor: Arc<EventCompactor>,
}

impl EventLog {
    /// Create new event log
    pub async fn new(
        db_path: impl AsRef<std::path::Path>,
        nats_url: &str,
        repo_id: &str,
        peers: Vec<String>,
    ) -> Result<Self, EventLogError> {
        let store = Arc::new(EventLogStore::open(db_path).await?);
        let replicator = Arc::new(EventLogReplicator::new(nats_url, repo_id, peers).await?);
        let compactor = Arc::new(EventCompactor::new(store.clone()));

        Ok(Self {
            store,
            replicator,
            compactor,
        })
    }

    /// Append event to log
    pub async fn append(&self, mut event: FederationEvent) -> Result<LogOffset, EventLogError> {
        let offset = self.store.append(event.clone()).await?;
        
        // Replicate to peers in background
        let replicator = self.replicator.clone();
        tokio::spawn(async move {
            let _ = replicator.replicate(&event).await;
        });

        Ok(offset)
    }

    /// Read range of events
    pub async fn read_range(
        &self,
        start: LogOffset,
        end: LogOffset,
    ) -> Result<Vec<FederationEvent>, EventLogError> {
        self.store.read_range(start, end).await
    }

    /// Query events by trace ID
    pub async fn query_by_trace(&self, trace_id: &str) -> Result<Vec<FederationEvent>, EventLogError> {
        self.store.query_by_trace(trace_id).await
    }

    /// Create checkpoint snapshot
    pub async fn checkpoint(&self) -> Result<String, EventLogError> {
        self.store.create_snapshot().await
    }

    /// Restore from checkpoint
    pub async fn restore(&self, snapshot_id: &str) -> Result<(), EventLogError> {
        self.store.restore_from_snapshot(snapshot_id).await
    }

    /// Get log statistics
    pub async fn stats(&self) -> Result<EventLogStats, EventLogError> {
        self.store.get_stats().await
    }

    /// Run compaction (remove old events, create snapshot)
    pub async fn compact(&self) -> Result<CompactionResult, EventLogError> {
        self.compactor.compact().await
    }

    /// Get reference to store for direct access
    pub fn store(&self) -> &EventLogStore {
        &self.store
    }

    /// Get reference to replicator for manual replication
    pub fn replicator(&self) -> &EventLogReplicator {
        &self.replicator
    }
}

/// Event log statistics
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventLogStats {
    pub total_events: u64,
    pub log_size_bytes: u64,
    pub earliest_timestamp: i64,
    pub latest_timestamp: i64,
    pub unique_traces: u64,
    pub unique_repos: u64,
}

/// Compaction result
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompactionResult {
    pub events_removed: u64,
    pub space_freed_bytes: u64,
    pub new_snapshot_id: String,
    pub duration_ms: u64,
}
