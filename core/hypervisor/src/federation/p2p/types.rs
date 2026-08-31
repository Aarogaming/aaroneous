/// Shared types for the P2P module
///
/// These types are used regardless of whether the `p2p-iroh` feature is
/// enabled, so the Omnipresent specialist can have a stable API.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors that can occur during P2P operations
#[derive(Debug, thiserror::Error)]
pub enum P2pError {
    #[error("P2P network error: {0}")]
    Network(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid endpoint ID: {0}")]
    InvalidEndpoint(String),

    #[error("Operation timed out after {0}ms")]
    Timeout(u64),

    #[error("P2P feature not enabled (compile with --features p2p-iroh)")]
    FeatureNotEnabled,

    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for P2pError {
    fn from(e: std::io::Error) -> Self {
        P2pError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for P2pError {
    fn from(e: serde_json::Error) -> Self {
        P2pError::Serialization(e.to_string())
    }
}

/// A node identifier in the P2P network.
///
/// When `p2p-iroh` is enabled, this wraps an Iroh `EndpointId` (Ed25519 public key).
/// In stub mode, it's a random hex string for testing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct P2pNodeId(pub String);

impl P2pNodeId {
    /// Create a new random node ID for testing/stub mode
    pub fn random() -> Self {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(format!("stub-{:032x}", nanos))
    }

    /// Get a short display version (first 8 chars, UTF-8 safe)
    pub fn short(&self) -> String {
        if self.0.len() <= 12 {
            self.0.clone()
        } else {
            // Find the 12th character boundary for UTF-8 safety
            match self.0.char_indices().nth(12) {
                Some((idx, _)) => format!("{}…", &self.0[..idx]),
                None => self.0.clone(),
            }
        }
    }
}

impl fmt::Display for P2pNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message format for syncing Intent state between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    /// Schema version for forward compatibility
    pub version: u32,
    /// Sender's node ID
    pub from: P2pNodeId,
    /// Logical timestamp (Lamport clock or wall clock seconds)
    pub timestamp: u64,
    /// Intent version on sender
    pub intent_version: u32,
    /// Payload type discriminator
    pub kind: SyncMessageKind,
    /// Opaque payload (CRDT delta, full state, etc.)
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncMessageKind {
    /// Initial sync request: "give me your state"
    SyncRequest,
    /// Full state response
    FullState,
    /// CRDT delta update
    Delta,
    /// Heartbeat / device-alive ping
    Heartbeat,
    /// Conflict notification
    ConflictDetected,
}

impl SyncMessage {
    pub fn heartbeat(from: P2pNodeId, intent_version: u32) -> Self {
        Self {
            version: 1,
            from,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            intent_version,
            kind: SyncMessageKind::Heartbeat,
            payload: vec![],
        }
    }

    pub fn full_state(from: P2pNodeId, intent_version: u32, state: Vec<u8>) -> Self {
        Self {
            version: 1,
            from,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            intent_version,
            kind: SyncMessageKind::FullState,
            payload: state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_random() {
        let id1 = P2pNodeId::random();
        let id2 = P2pNodeId::random();
        assert_ne!(id1, id2, "Random IDs should be unique");
    }

    #[test]
    fn test_node_id_short() {
        let id = P2pNodeId("abcdef0123456789abcdef0123456789".to_string());
        assert_eq!(id.short(), "abcdef012345…");

        let short_id = P2pNodeId("short".to_string());
        assert_eq!(short_id.short(), "short");
    }

    #[test]
    fn test_sync_message_heartbeat() {
        let id = P2pNodeId::random();
        let msg = SyncMessage::heartbeat(id.clone(), 42);
        assert_eq!(msg.from, id);
        assert_eq!(msg.intent_version, 42);
        assert_eq!(msg.kind, SyncMessageKind::Heartbeat);
        assert!(msg.payload.is_empty());
        assert_eq!(msg.version, 1);
    }

    #[test]
    fn test_sync_message_full_state() {
        let id = P2pNodeId::random();
        let payload = vec![1, 2, 3, 4, 5];
        let msg = SyncMessage::full_state(id.clone(), 7, payload.clone());
        assert_eq!(msg.from, id);
        assert_eq!(msg.intent_version, 7);
        assert_eq!(msg.kind, SyncMessageKind::FullState);
        assert_eq!(msg.payload, payload);
    }

    #[test]
    fn test_sync_message_serialization() {
        let id = P2pNodeId::random();
        let msg = SyncMessage::heartbeat(id, 42);
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.intent_version, 42);
        assert_eq!(parsed.kind, SyncMessageKind::Heartbeat);
    }

    #[test]
    fn test_p2p_error_display() {
        let err = P2pError::Timeout(5000);
        assert_eq!(err.to_string(), "Operation timed out after 5000ms");

        let err = P2pError::FeatureNotEnabled;
        assert!(err.to_string().contains("p2p-iroh"));
    }
}
