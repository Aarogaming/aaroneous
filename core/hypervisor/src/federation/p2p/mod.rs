pub mod types;
pub use types::*;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock, mpsc};

/// Sovereign P2P Hypervisor Node with Async Stream & Channel Multiplexing
pub struct P2pNode {
    id: P2pNodeId,
    inbox_tx: mpsc::Sender<(P2pNodeId, SyncMessage)>,
    inbox_rx: Arc<Mutex<mpsc::Receiver<(P2pNodeId, SyncMessage)>>>,
    peers: Arc<RwLock<HashMap<P2pNodeId, mpsc::Sender<SyncMessage>>>>,
    bound_addr: Arc<RwLock<Option<String>>>,
}

impl P2pNode {
    pub fn new(id: P2pNodeId) -> Self {
        let (tx, rx) = mpsc::channel(1024);
        Self {
            id,
            inbox_tx: tx,
            inbox_rx: Arc::new(Mutex::new(rx)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            bound_addr: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn spawn(id: P2pNodeId) -> Result<Self, P2pError> {
        let mut node = Self::new(id);
        node.start().await?;
        Ok(node)
    }

    pub fn id(&self) -> &P2pNodeId {
        &self.id
    }

    pub fn endpoint_id(&self) -> String {
        self.id.0.clone()
    }

    pub async fn start(&mut self) -> Result<(), P2pError> {
        Ok(())
    }

    /// Binds an asynchronous TCP listener for remote hypervisor peer connections
    pub async fn bind_listener(&self, addr: &str) -> Result<String, P2pError> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?.to_string();
        *self.bound_addr.write().await = Some(local_addr.clone());

        let inbox_tx = self.inbox_tx.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, peer_addr)) = listener.accept().await {
                let inbox = inbox_tx.clone();
                tokio::spawn(async move {
                    let mut len_buf = [0u8; 4];
                    loop {
                        if socket.read_exact(&mut len_buf).await.is_err() {
                            break;
                        }
                        let len = u32::from_be_bytes(len_buf) as usize;
                        let mut msg_buf = vec![0u8; len];
                        if socket.read_exact(&mut msg_buf).await.is_err() {
                            break;
                        }

                        if let Ok(msg) = serde_json::from_slice::<SyncMessage>(&msg_buf) {
                            let sender_id = if !msg.from.is_empty() {
                                P2pNodeId::new(&msg.from)
                            } else {
                                P2pNodeId::new(&peer_addr.to_string())
                            };
                            let _ = inbox.send((sender_id, msg)).await;
                        }
                    }
                });
            }
        });

        Ok(local_addr)
    }

    /// Connects to a remote peer over a network TCP stream
    pub async fn connect_remote_stream(&self, peer_id: P2pNodeId, target_addr: &str) -> Result<(), P2pError> {
        let mut stream = TcpStream::connect(target_addr).await.map_err(|e| {
            P2pError::ConnectionFailed(format!("Failed to connect to {}: {}", target_addr, e))
        })?;

        let (tx, mut rx) = mpsc::channel::<SyncMessage>(256);
        self.peers.write().await.insert(peer_id.clone(), tx);

        let self_id = self.id.0.clone();
        tokio::spawn(async move {
            while let Some(mut msg) = rx.recv().await {
                if msg.from.is_empty() {
                    msg.from = self_id.clone();
                }
                if let Ok(encoded) = serde_json::to_vec(&msg) {
                    let len = (encoded.len() as u32).to_be_bytes();
                    if stream.write_all(&len).await.is_err() {
                        break;
                    }
                    if stream.write_all(&encoded).await.is_err() {
                        break;
                    }
                    let _ = stream.flush().await;
                }
            }
        });

        Ok(())
    }

    /// Registers a direct in-memory peer channel for ultra-low latency local node meshes
    pub async fn register_direct_peer(&self, peer: &P2pNode) {
        let (self_to_peer_tx, mut self_to_peer_rx) = mpsc::channel::<SyncMessage>(256);
        let peer_inbox = peer.inbox_tx.clone();
        let self_id = self.id.clone();
        tokio::spawn(async move {
            while let Some(msg) = self_to_peer_rx.recv().await {
                let _ = peer_inbox.send((self_id.clone(), msg)).await;
            }
        });
        self.peers.write().await.insert(peer.id.clone(), self_to_peer_tx);

        let (peer_to_self_tx, mut peer_to_self_rx) = mpsc::channel::<SyncMessage>(256);
        let self_inbox = self.inbox_tx.clone();
        let peer_id = peer.id.clone();
        tokio::spawn(async move {
            while let Some(msg) = peer_to_self_rx.recv().await {
                let _ = self_inbox.send((peer_id.clone(), msg)).await;
            }
        });
        peer.peers.write().await.insert(self.id.clone(), peer_to_self_tx);
    }

    pub async fn stop(&mut self) {
        self.peers.write().await.clear();
    }

    pub async fn send(&self, to: P2pNodeId, mut msg: SyncMessage) -> Result<(), P2pError> {
        if msg.from.is_empty() {
            msg.from = self.id.0.clone();
        }
        let peers = self.peers.read().await;
        if let Some(peer_tx) = peers.get(&to) {
            peer_tx.send(msg).await.map_err(|_| {
                P2pError::ConnectionFailed(format!("Peer receiver dropped for {}", to.0))
            })?;
            Ok(())
        } else {
            Err(P2pError::InvalidEndpoint(format!("Peer not found: {}", to.0)))
        }
    }

    pub async fn receive(&self) -> Option<(P2pNodeId, SyncMessage)> {
        let mut rx = self.inbox_rx.lock().await;
        rx.recv().await
    }

    pub async fn broadcast(&self, mut msg: SyncMessage) {
        if msg.from.is_empty() {
            msg.from = self.id.0.clone();
        }
        let peers = self.peers.read().await;
        for peer_tx in peers.values() {
            let _ = peer_tx.send(msg.clone()).await;
        }
    }
}

#[cfg(any(feature = "p2p-iroh", feature = "fleet"))]
pub mod iroh_node;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_node_id_short_ascii() {
        let node_id_short = P2pNodeId::new("node-123");
        assert_eq!(node_id_short.short(), "node-123");

        let node_id_long = P2pNodeId::new("node-1234567890abcdef");
        assert_eq!(node_id_long.short(), "node-1234567…");
    }

    #[test]
    fn test_p2p_node_id_short_unicode_multibyte() {
        let unicode_id = P2pNodeId::new("🦀🔥⚡🌟🚀💻🎉✨🌌💎🛠️🎯🛡️");
        // Must not panic on UTF-8 multibyte boundary
        let short = unicode_id.short();
        assert!(short.ends_with('…'));
    }

    #[tokio::test]
    async fn test_p2p_direct_node_mesh_messaging() {
        let node_a = P2pNode::new(P2pNodeId::new("hive_node_a"));
        let node_b = P2pNode::new(P2pNodeId::new("hive_node_b"));

        node_a.register_direct_peer(&node_b).await;

        let msg = SyncMessage {
            kind: SyncMessageKind::Request,
            payload: b"Remote task payload".to_vec(),
            from: "hive_node_a".to_string(),
            timestamp: 123456,
            intent_version: 1,
        };

        node_a.send(P2pNodeId::new("hive_node_b"), msg).await.unwrap();

        let (from, received_msg) = node_b.receive().await.unwrap();
        assert_eq!(from.0, "hive_node_a");
        assert_eq!(received_msg.payload, b"Remote task payload");
    }

    #[tokio::test]
    async fn test_p2p_socket_stream_remote_transfer() {
        let receiver = P2pNode::new(P2pNodeId::new("socket_receiver"));
        let local_addr = receiver.bind_listener("127.0.0.1:0").await.unwrap();

        let sender = P2pNode::new(P2pNodeId::new("socket_sender"));
        sender.connect_remote_stream(P2pNodeId::new("socket_receiver"), &local_addr).await.unwrap();

        let msg = SyncMessage {
            kind: SyncMessageKind::StateSync,
            payload: b"Wire transport verified".to_vec(),
            from: "socket_sender".to_string(),
            timestamp: 789012,
            intent_version: 2,
        };

        sender.send(P2pNodeId::new("socket_receiver"), msg).await.unwrap();

        let (from, received) = receiver.receive().await.unwrap();
        assert_eq!(from.0, "socket_sender");
        assert_eq!(received.payload, b"Wire transport verified");
    }
}

