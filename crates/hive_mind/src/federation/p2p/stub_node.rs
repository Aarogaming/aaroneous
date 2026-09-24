/// Stub P2P node implementation (no-op, used when `p2p-iroh` feature is disabled)
///
/// This implementation:
/// - Provides the same API as the real Iroh-backed node
/// - Records calls for testing
/// - Returns deterministic data instead of real network operations
/// - Allows tests to run without requiring network access or Iroh dependencies

use super::types::{P2pError, P2pNodeId, SyncMessage};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Stub P2P node - records operations but does no real networking
pub struct P2pNode {
    node_id: P2pNodeId,
    alpn: Vec<u8>,
    /// Records of operations performed (for test inspection)
    pub call_log: Arc<Mutex<Vec<StubCall>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StubCall {
    Spawn,
    Connect(P2pNodeId),
    SendMessage(P2pNodeId),
    Broadcast(usize), // peer count
    Shutdown,
}

impl P2pNode {
    /// Spawn a new stub P2P node with the given ALPN identifier
    pub async fn spawn(alpn: &[u8]) -> Result<Self, P2pError> {
        let node = Self {
            node_id: P2pNodeId::random(),
            alpn: alpn.to_vec(),
            call_log: Arc::new(Mutex::new(vec![StubCall::Spawn])),
        };
        Ok(node)
    }

    /// Get this node's identifier
    pub fn endpoint_id(&self) -> &P2pNodeId {
        &self.node_id
    }

    /// Get the ALPN this node is listening on
    pub fn alpn(&self) -> &[u8] {
        &self.alpn
    }

    /// Connect to another peer (stub: just records the call)
    pub async fn connect(&self, peer: &P2pNodeId) -> Result<(), P2pError> {
        self.call_log.lock().await.push(StubCall::Connect(peer.clone()));
        Ok(())
    }

    /// Send a sync message to a peer (stub: records call, returns success)
    pub async fn send(&self, peer: &P2pNodeId, _msg: SyncMessage) -> Result<(), P2pError> {
        self.call_log.lock().await.push(StubCall::SendMessage(peer.clone()));
        Ok(())
    }

    /// Broadcast a sync message to multiple peers (stub: records peer count)
    pub async fn broadcast(&self, peers: &[P2pNodeId], _msg: SyncMessage) -> Result<usize, P2pError> {
        self.call_log.lock().await.push(StubCall::Broadcast(peers.len()));
        Ok(peers.len())
    }

    /// Receive next sync message (stub: returns FeatureNotEnabled to signal real impl needed)
    pub async fn recv(&self) -> Result<SyncMessage, P2pError> {
        Err(P2pError::FeatureNotEnabled)
    }

    /// Shut down the node gracefully
    pub async fn shutdown(self) -> Result<(), P2pError> {
        self.call_log.lock().await.push(StubCall::Shutdown);
        Ok(())
    }

    /// Test helper: get a snapshot of call log
    #[cfg(test)]
    pub async fn calls(&self) -> Vec<StubCall> {
        self.call_log.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_spawn() {
        let node = P2pNode::spawn(b"test/v1").await.unwrap();
        assert_eq!(node.alpn(), b"test/v1");
        let calls = node.calls().await;
        assert_eq!(calls, vec![StubCall::Spawn]);
    }

    #[tokio::test]
    async fn test_stub_connect_records_call() {
        let node = P2pNode::spawn(b"test/v1").await.unwrap();
        let peer = P2pNodeId::random();
        node.connect(&peer).await.unwrap();
        let calls = node.calls().await;
        assert_eq!(calls.len(), 2);
        assert!(matches!(&calls[1], StubCall::Connect(p) if p == &peer));
    }

    #[tokio::test]
    async fn test_stub_send_message() {
        let node = P2pNode::spawn(b"test/v1").await.unwrap();
        let peer = P2pNodeId::random();
        let msg = SyncMessage::heartbeat(node.endpoint_id().clone(), 1);
        node.send(&peer, msg).await.unwrap();
        let calls = node.calls().await;
        assert!(calls.iter().any(|c| matches!(c, StubCall::SendMessage(p) if p == &peer)));
    }

    #[tokio::test]
    async fn test_stub_broadcast() {
        let node = P2pNode::spawn(b"test/v1").await.unwrap();
        let peers = vec![
            P2pNodeId::random(),
            P2pNodeId::random(),
            P2pNodeId::random(),
        ];
        let msg = SyncMessage::heartbeat(node.endpoint_id().clone(), 1);
        let n = node.broadcast(&peers, msg).await.unwrap();
        assert_eq!(n, 3);
        let calls = node.calls().await;
        assert!(calls.iter().any(|c| matches!(c, StubCall::Broadcast(3))));
    }

    #[tokio::test]
    async fn test_stub_recv_signals_feature_disabled() {
        let node = P2pNode::spawn(b"test/v1").await.unwrap();
        let result = node.recv().await;
        assert!(matches!(result, Err(P2pError::FeatureNotEnabled)));
    }

    #[tokio::test]
    async fn test_stub_shutdown() {
        let node = P2pNode::spawn(b"test/v1").await.unwrap();
        let log = node.call_log.clone();
        node.shutdown().await.unwrap();
        let calls = log.lock().await.clone();
        assert!(calls.contains(&StubCall::Shutdown));
    }

    #[tokio::test]
    async fn test_stub_node_ids_are_unique() {
        let node1 = P2pNode::spawn(b"test/v1").await.unwrap();
        let node2 = P2pNode::spawn(b"test/v1").await.unwrap();
        assert_ne!(node1.endpoint_id(), node2.endpoint_id());
    }
}
