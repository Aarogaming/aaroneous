use crate::event_log::types::{FederationEvent, EventLogError};
use std::sync::Arc;

/// Replicates events to peer repositories via NATS
pub struct EventLogReplicator {
    nats_url: String,
    repo_id: String,
    peers: Vec<String>,
}

impl EventLogReplicator {
    /// Create new replicator
    pub async fn new(
        nats_url: &str,
        repo_id: &str,
        peers: Vec<String>,
    ) -> Result<Self, EventLogError> {
        Ok(Self {
            nats_url: nats_url.to_string(),
            repo_id: repo_id.to_string(),
            peers,
        })
    }

    /// Replicate event to peers
    pub async fn replicate(&self, event: &FederationEvent) -> Result<ReplicationAck, EventLogError> {
        // In Phase 6A.2, replication is simulated
        // In Phase 6B, this integrates with Raft consensus for atomic replication

        let mut acks = Vec::new();

        for peer in &self.peers {
            // Simulate replication delay
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            acks.push(peer.clone());
        }

        Ok(ReplicationAck {
            event_id: event.event_id.clone(),
            acked_by: acks,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    /// Get list of peers
    pub fn peers(&self) -> &[String] {
        &self.peers
    }

    /// Check if peer is reachable (simulated)
    pub async fn is_peer_healthy(&self, peer: &str) -> bool {
        // In Phase 6, integrate with actual NATS health checks
        self.peers.contains(&peer.to_string())
    }
}

/// Result of replication to peers
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReplicationAck {
    /// Event ID that was replicated
    pub event_id: String,
    /// Which peers acknowledged the event
    pub acked_by: Vec<String>,
    /// Timestamp when replication completed
    pub timestamp: i64,
}

impl ReplicationAck {
    /// Check if we have quorum (>50% of peers)
    pub fn has_quorum(&self, total_peers: usize) -> bool {
        self.acked_by.len() > total_peers / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::types::{EventType, Operation};

    #[tokio::test]
    async fn test_replicator_creation() {
        let replicator = EventLogReplicator::new(
            "nats://localhost:4222",
            "AAS",
            vec!["Guild".to_string(), "Merlin".to_string()],
        )
        .await;

        assert!(replicator.is_ok());
    }

    #[tokio::test]
    async fn test_replicate_event() {
        let replicator = EventLogReplicator::new(
            "nats://localhost:4222",
            "AAS",
            vec!["Guild".to_string(), "Merlin".to_string()],
        )
        .await
        .unwrap();

        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("test".to_string()),
        );

        let ack = replicator.replicate(&event).await.unwrap();
        assert_eq!(ack.acked_by.len(), 2);
        assert!(ack.has_quorum(2));
    }

    #[test]
    fn test_replication_ack_quorum() {
        let ack = ReplicationAck {
            event_id: "event-1".to_string(),
            acked_by: vec!["Guild".to_string(), "Merlin".to_string()],
            timestamp: 0,
        };

        // 2 out of 3 = quorum
        assert!(ack.has_quorum(3));
        
        // 1 out of 3 = no quorum
        let ack2 = ReplicationAck {
            event_id: "event-1".to_string(),
            acked_by: vec!["Guild".to_string()],
            timestamp: 0,
        };
        assert!(!ack2.has_quorum(3));
    }
}
