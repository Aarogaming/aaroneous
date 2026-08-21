// P2P Networking stub
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct P2pNodeId(pub String);

impl P2pNodeId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn short(&self) -> String {
        if self.0.len() <= 12 {
            self.0.clone()
        } else {
            format!("{}…", &self.0[..12])
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2pError {
    ConnectionFailed(String),
    Timeout(u64),
    SerializationError(String),
    Network(String),
    InvalidEndpoint(String),
}

impl std::fmt::Display for P2pError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            P2pError::ConnectionFailed(s) => write!(f, "Connection failed: {}", s),
            P2pError::Timeout(ms) => write!(f, "Timeout after {}ms", ms),
            P2pError::SerializationError(s) => write!(f, "Serialization error: {}", s),
            P2pError::Network(s) => write!(f, "P2P network error: {}", s),
            P2pError::InvalidEndpoint(s) => write!(f, "Invalid endpoint: {}", s),
        }
    }
}

impl From<serde_json::Error> for P2pError {
    fn from(e: serde_json::Error) -> Self {
        P2pError::SerializationError(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessageKind {
    FullState,
    Delta,
    StateSync,
    Heartbeat,
    ConflictDetected,
    SyncRequest,
    Request,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub kind: SyncMessageKind,
    pub payload: Vec<u8>,
    pub from: String,
    pub timestamp: u64,
    pub intent_version: u32,
}

impl SyncMessage {
    pub fn full_state(data: Vec<u8>) -> Self {
        Self {
            kind: SyncMessageKind::FullState,
            payload: data,
            from: String::new(),
            timestamp: 0,
            intent_version: 0,
        }
    }
    pub fn heartbeat() -> Self {
        Self {
            kind: SyncMessageKind::Heartbeat,
            payload: vec![],
            from: String::new(),
            timestamp: 0,
            intent_version: 0,
        }
    }
}

pub struct P2pNode;
impl P2pNode {
    pub fn new(_id: P2pNodeId) -> Self {
        Self
    }
    pub async fn start(&mut self) -> Result<(), P2pError> {
        Ok(())
    }
    pub async fn stop(&mut self) {}
    pub async fn send(&self, _to: P2pNodeId, _msg: SyncMessage) -> Result<(), P2pError> {
        Ok(())
    }
    pub async fn receive(&self) -> Option<(P2pNodeId, SyncMessage)> {
        None
    }
    pub async fn spawn(_id: P2pNodeId) -> Result<Self, P2pError> {
        Ok(Self)
    }
    pub fn endpoint_id(&self) -> String {
        String::new()
    }
    pub async fn broadcast(&self, _msg: SyncMessage) {}
}

pub mod types {
    pub use super::{P2pError, P2pNodeId, SyncMessage, SyncMessageKind};
}

#[cfg(feature = "p2p-iroh")]
pub mod iroh_node;
