use crate::event_log::types::EventLogError;
use std::sync::Arc;

/// Compacts event log by removing old events and creating snapshots
pub struct EventCompactor {
    store: Arc<super::EventLogStore>,
}

impl EventCompactor {
    /// Create new compactor
    pub fn new(store: Arc<super::EventLogStore>) -> Self {
        Self { store }
    }

    /// Run compaction - keep only recent events, create snapshot
    pub async fn compact(&self) -> Result<super::CompactionResult, EventLogError> {
        let start_time = std::time::Instant::now();

        // Create snapshot before compaction
        let snapshot_id = self.store.create_snapshot().await?;

        // In Phase 6A.2, we keep all events
        // In Phase 6D Recovery Engine, we'll implement selective pruning based on:
        // - Event age (keep last 7 days)
        // - Event type (always keep Mutations)
        // - Snapshot intervals (keep events since last snapshot)

        let stats = self.store.get_stats().await?;

        Ok(super::CompactionResult {
            events_removed: 0,
            space_freed_bytes: 0,
            new_snapshot_id: snapshot_id,
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Schedule periodic compaction
    pub async fn start_compaction_scheduler(
        self,
        interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval_secs)
            );

            loop {
                interval.tick().await;
                
                if let Err(e) = self.compact().await {
                    tracing::warn!("Compaction error: {}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::types::{EventType, Operation};

    #[tokio::test]
    async fn test_compactor_creation() {
        let store = Arc::new(
            super::super::EventLogStore::open("/tmp/test_compactor")
                .await
                .unwrap(),
        );
        let compactor = EventCompactor::new(store);
        let result = compactor.compact().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compaction_result() {
        let store = Arc::new(
            super::super::EventLogStore::open("/tmp/test_compactor_result")
                .await
                .unwrap(),
        );

        // Add some events
        for i in 0..5 {
            let event = crate::event_log::types::FederationEvent::new(
                format!("trace-{}", i),
                "AAS",
                "leadership",
                EventType::Mutation,
                Operation::Create(format!("test-{}", i)),
            );
            store.append(event).await.unwrap();
        }

        let compactor = EventCompactor::new(store);
        let result = compactor.compact().await.unwrap();

        assert!(!result.new_snapshot_id.is_empty());
        assert!(result.duration_ms >= 0);
    }
}
