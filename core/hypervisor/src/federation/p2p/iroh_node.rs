#![cfg(feature = "p2p-iroh")]
/// Real Iroh-backed P2P node implementation
///
/// This module is only compiled when the `p2p-iroh` feature is enabled.
/// It wraps an Iroh `Endpoint` and provides the same API as the stub node.
///
/// # Architecture
///
/// - One Iroh `Endpoint` per `P2pNode` instance
/// - Connections are dialed by `EndpointId` (the peer's public key)
/// - QUIC streams carry serialized `SyncMessage` payloads
/// - The node listens on the configured ALPN for incoming connections
///
/// # Connection Lifecycle
///
/// 1. `spawn()` creates an `Endpoint` bound to the default Iroh relays
/// 2. `connect()` opens a QUIC connection to a peer by their `EndpointId`
/// 3. `send()` opens a uni-directional stream and writes a serialized message
/// 4. `recv()` accepts the next inbound connection and reads one message
/// 5. `shutdown()` closes all connections and the endpoint gracefully
///
/// # Limitations of v1
///
/// - One connection per send (no connection pooling yet)
/// - No automatic peer discovery (requires known `EndpointId`)
/// - No CRDT integration yet (callers send/receive opaque messages)
use super::types::{P2pError, P2pNodeId, SyncMessage};
use iroh::{Endpoint, EndpointAddr, PublicKey, endpoint::presets};
use n0_future::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// ALPN-prefix sanity check max
const MAX_ALPN_LEN: usize = 256;

/// Default connection timeout
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Iroh-backed P2P node
pub struct P2pNode {
    endpoint: Endpoint,
    alpn: Vec<u8>,
    /// Cached node ID string for fast access
    node_id_str: String,
    /// Set of currently-known peer endpoint IDs (for tracking)
    known_peers: Arc<Mutex<Vec<P2pNodeId>>>,
}

impl P2pNode {
    /// Spawn a new P2P node listening on the given ALPN
    ///
    /// The ALPN is the application-layer protocol name and must be agreed
    /// upon by both ends of the connection. For Aaroneous Intent sync, use
    /// something like `b"aaroneous/sync/v1"`.
    pub async fn spawn(alpn: &[u8]) -> Result<Self, P2pError> {
        if alpn.is_empty() || alpn.len() > MAX_ALPN_LEN {
            return Err(P2pError::Network(format!(
                "ALPN must be 1-{} bytes, got {}",
                MAX_ALPN_LEN,
                alpn.len()
            )));
        }

        let alpn_vec = alpn.to_vec();

        info!(
            "Spawning Iroh P2P endpoint with ALPN: {:?}",
            String::from_utf8_lossy(alpn)
        );

        let endpoint = Endpoint::builder(presets::N0)
            .alpns(vec![alpn_vec.clone()])
            .bind()
            .await
            .map_err(|e| P2pError::Network(format!("Failed to bind Iroh endpoint: {}", e)))?;

        let node_id_str = endpoint.id().to_string();
        info!("Iroh P2P endpoint spawned with ID: {}", node_id_str);

        Ok(Self {
            endpoint,
            alpn: alpn_vec,
            node_id_str,
            known_peers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Get this node's endpoint ID (public key as string)
    pub fn endpoint_id(&self) -> P2pNodeId {
        P2pNodeId(self.node_id_str.clone())
    }

    /// Get the ALPN this node is listening on
    pub fn alpn(&self) -> &[u8] {
        &self.alpn
    }

    /// Connect to a peer by endpoint ID (without sending data)
    ///
    /// This establishes a QUIC connection but does not send any messages.
    /// Useful for warming up the connection before a burst of sends.
    pub async fn connect(&self, peer: &P2pNodeId) -> Result<(), P2pError> {
        let public_key = parse_endpoint_id(&peer.0)?;
        let addr = EndpointAddr::from(public_key);

        debug!("Connecting to peer: {}", peer.short());

        let conn = tokio::time::timeout(CONNECT_TIMEOUT, self.endpoint.connect(addr, &self.alpn))
            .await
            .map_err(|_| P2pError::Timeout(CONNECT_TIMEOUT.as_millis() as u64))?
            .map_err(|e| {
                P2pError::ConnectionFailed(format!("connect to {}: {}", peer.short(), e))
            })?;

        // Track the peer
        self.known_peers.lock().await.push(peer.clone());

        // Close the warm-up connection cleanly
        conn.close(0u8.into(), b"warmup-done");

        Ok(())
    }

    /// Send a sync message to a peer
    pub async fn send(&self, peer: &P2pNodeId, msg: SyncMessage) -> Result<(), P2pError> {
        let public_key = parse_endpoint_id(&peer.0)?;
        let addr = EndpointAddr::from(public_key);

        debug!("Sending {:?} to peer {}", msg.kind, peer.short());

        let conn = tokio::time::timeout(CONNECT_TIMEOUT, self.endpoint.connect(addr, &self.alpn))
            .await
            .map_err(|_| P2pError::Timeout(CONNECT_TIMEOUT.as_millis() as u64))?
            .map_err(|e| P2pError::ConnectionFailed(format!("send to {}: {}", peer.short(), e)))?;

        let mut send_stream = conn
            .open_uni()
            .await
            .map_err(|e| P2pError::Network(format!("open uni stream: {}", e)))?;

        let bytes = serde_json::to_vec(&msg)?;
        send_stream
            .write_all(&bytes)
            .await
            .map_err(|e| P2pError::Network(format!("write: {}", e)))?;
        send_stream
            .finish()
            .map_err(|e| P2pError::Network(format!("finish: {}", e)))?;

        // Wait briefly for graceful close, then drop the connection
        let _ = tokio::time::timeout(Duration::from_secs(1), conn.closed()).await;

        Ok(())
    }

    /// Broadcast a sync message to multiple peers
    ///
    /// Sends are performed sequentially in v1. Returns the number of successful
    /// sends. Failed sends are logged but not propagated (best-effort delivery).
    pub async fn broadcast(
        &self,
        peers: &[P2pNodeId],
        msg: SyncMessage,
    ) -> Result<usize, P2pError> {
        if peers.is_empty() {
            return Ok(0);
        }

        let mut success_count = 0;
        for peer in peers {
            match self.send(peer, msg.clone()).await {
                Ok(()) => success_count += 1,
                Err(e) => warn!("Broadcast send to {} failed: {}", peer.short(), e),
            }
        }

        Ok(success_count)
    }

    /// Accept the next inbound connection and read one sync message
    ///
    /// This is a blocking await: it waits for an inbound connection,
    /// accepts it, reads one uni-directional stream, parses the message,
    /// and returns it.
    ///
    /// For long-running listeners, prefer running this in a loop within
    /// a dedicated tokio task.
    pub async fn recv(&self) -> Result<SyncMessage, P2pError> {
        // Accept incoming connection
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or_else(|| P2pError::Network("endpoint closed".to_string()))?;

        let conn = incoming
            .await
            .map_err(|e| P2pError::ConnectionFailed(format!("accept conn: {}", e)))?;

        // Accept uni-directional stream
        let mut recv_stream = conn
            .accept_uni()
            .await
            .map_err(|e| P2pError::Network(format!("accept uni: {}", e)))?;

        // Read with size limit (1 MiB max per message)
        let bytes = recv_stream
            .read_to_end(1024 * 1024)
            .await
            .map_err(|e| P2pError::Network(format!("read: {}", e)))?;

        let msg: SyncMessage = serde_json::from_slice(&bytes)?;
        debug!("Received {:?} from peer", msg.kind);

        Ok(msg)
    }

    /// Get a list of known peers (those we've connected to)
    pub async fn known_peers(&self) -> Vec<P2pNodeId> {
        self.known_peers.lock().await.clone()
    }

    /// Shut down the endpoint gracefully
    pub async fn shutdown(self) -> Result<(), P2pError> {
        info!(
            "Shutting down Iroh P2P endpoint {}",
            self.endpoint_id().short()
        );
        self.endpoint.close().await;
        Ok(())
    }
}

/// Parse a string endpoint ID into an Iroh PublicKey
fn parse_endpoint_id(s: &str) -> Result<PublicKey, P2pError> {
    s.parse::<PublicKey>()
        .map_err(|e| P2pError::InvalidEndpoint(format!("{}: {}", s, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iroh_spawn() {
        let node = P2pNode::spawn(b"aaroneous/test/v1").await;
        assert!(node.is_ok(), "Failed to spawn Iroh node: {:?}", node.err());

        let node = node.unwrap();
        assert_eq!(node.alpn(), b"aaroneous/test/v1");

        let id = node.endpoint_id();
        assert!(!id.0.is_empty(), "Endpoint ID should not be empty");

        // Cleanup
        node.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_iroh_alpn_validation() {
        // Empty ALPN should be rejected
        let result = P2pNode::spawn(b"").await;
        assert!(matches!(result, Err(P2pError::Network(_))));

        // Too-long ALPN should be rejected
        let long = vec![b'x'; 257];
        let result = P2pNode::spawn(&long).await;
        assert!(matches!(result, Err(P2pError::Network(_))));
    }

    #[tokio::test]
    async fn test_iroh_invalid_endpoint_id() {
        let node = P2pNode::spawn(b"aaroneous/test/v1").await.unwrap();
        let bad_peer = P2pNodeId("not-a-real-key".to_string());

        let result = node.connect(&bad_peer).await;
        assert!(matches!(result, Err(P2pError::InvalidEndpoint(_))));

        node.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_iroh_two_nodes_can_be_created() {
        let node1 = P2pNode::spawn(b"aaroneous/test/v1").await.unwrap();
        let node2 = P2pNode::spawn(b"aaroneous/test/v1").await.unwrap();

        assert_ne!(
            node1.endpoint_id(),
            node2.endpoint_id(),
            "Two nodes should have different IDs"
        );

        node1.shutdown().await.unwrap();
        node2.shutdown().await.unwrap();
    }

    /// Integration test: two nodes connect and exchange a message.
    /// This requires actual network access (Iroh relay servers).
    #[tokio::test]
    #[ignore = "requires network access to Iroh relay servers"]
    async fn test_iroh_send_recv_round_trip() {
        let listener = P2pNode::spawn(b"aaroneous/test/v1").await.unwrap();
        let listener_id = listener.endpoint_id();

        let sender = P2pNode::spawn(b"aaroneous/test/v1").await.unwrap();
        let sender_id = sender.endpoint_id();

        // Listener task
        let recv_handle = tokio::spawn(async move {
            let msg = listener.recv().await.unwrap();
            listener.shutdown().await.unwrap();
            msg
        });

        // Give the listener time to start
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Send a message
        let msg = SyncMessage::heartbeat(sender_id, 42);
        sender.send(&listener_id, msg.clone()).await.unwrap();

        // Wait for recv
        let received = tokio::time::timeout(Duration::from_secs(30), recv_handle)
            .await
            .expect("recv timed out")
            .expect("recv task panicked");

        assert_eq!(received.intent_version, 42);
        assert_eq!(received.kind, msg.kind);

        sender.shutdown().await.unwrap();
    }
}
