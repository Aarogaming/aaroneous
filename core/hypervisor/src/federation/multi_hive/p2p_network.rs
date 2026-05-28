/// P2P Network: Cross-Hive Communication
/// 
/// Enables peer-to-peer communication between Aaroneous hives

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub node_id: String,
    pub listen_addr: String,
    pub max_peers: usize,
    pub message_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub max_message_size_kb: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            listen_addr: "127.0.0.1:8001".to_string(),
            max_peers: 100,
            message_timeout_ms: 5000,
            heartbeat_interval_ms: 1000,
            max_message_size_kb: 10240,  // 10MB
        }
    }
}

impl From<crate::federation::multi_hive::ClusterConfig> for NetworkConfig {
    fn from(config: crate::federation::multi_hive::ClusterConfig) -> Self {
        Self {
            node_id: config.node_id,
            listen_addr: config.listen_addr,
            max_peers: config.max_cluster_size,
            message_timeout_ms: config.heartbeat_timeout_ms,
            heartbeat_interval_ms: config.health_check_interval_ms,
            max_message_size_kb: 10240,
        }
    }
}

/// Message type for inter-hive communication
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    /// Heartbeat/keepalive
    Ping,
    Pong,
    /// Specialist proposal from another hive
    ProposalSync,
    /// Decision from another hive
    DecisionSync,
    /// Gradient update for federated learning
    GradientUpdate,
    /// Model merge request
    ModelMerge,
    /// Consensus gossip message
    Gossip,
    /// Event log synchronization
    EventSync,
    /// Health status update
    StatusUpdate,
    /// Custom message
    Custom(String),
}

/// Message from another hive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMessage {
    pub message_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub timestamp_ms: u64,
    pub priority: u32,
    pub requires_ack: bool,
}

impl PeerMessage {
    pub fn new(
        from_node_id: String,
        to_node_id: String,
        message_type: MessageType,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from_node_id,
            to_node_id,
            message_type,
            payload,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            priority: 50,
            requires_ack: true,
        }
    }

    /// Get message size in KB
    pub fn size_kb(&self) -> u32 {
        (self.payload.len() / 1024) as u32
    }

    /// Is message expired?
    pub fn is_expired(&self, timeout_ms: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now - self.timestamp_ms > timeout_ms
    }
}

/// P2P network peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPeer {
    pub peer_id: String,
    pub address: String,
    pub connected: bool,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub last_seen_ms: u64,
}

impl NetworkPeer {
    pub fn new(peer_id: String, address: String) -> Self {
        Self {
            peer_id,
            address,
            connected: false,
            messages_sent: 0,
            messages_received: 0,
            last_seen_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// P2P Network for inter-hive communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PNetwork {
    pub config: NetworkConfig,
    pub peers: std::collections::HashMap<String, NetworkPeer>,
    pub message_queue: VecDeque<PeerMessage>,
    pub ack_pending: std::collections::HashMap<String, PeerMessage>,
}

impl P2PNetwork {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            config,
            peers: std::collections::HashMap::new(),
            message_queue: VecDeque::new(),
            ack_pending: std::collections::HashMap::new(),
        }
    }

    /// Connect to a peer
    pub fn connect_peer(&mut self, peer_id: String, address: String) -> Result<(), String> {
        if self.peers.len() >= self.config.max_peers {
            return Err("Max peers reached".to_string());
        }

        let mut peer = NetworkPeer::new(peer_id, address);
        peer.connected = true;
        self.peers.insert(peer.peer_id.clone(), peer);
        Ok(())
    }

    /// Disconnect from a peer
    pub fn disconnect_peer(&mut self, peer_id: &str) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.connected = false;
        }
    }

    /// Send message to peer
    pub fn send_message(&mut self, message: PeerMessage) -> Result<(), String> {
        // Validate message size
        if message.size_kb() > self.config.max_message_size_kb {
            return Err(format!(
                "Message too large: {} KB (max {})",
                message.size_kb(),
                self.config.max_message_size_kb
            ));
        }

        // Check peer exists and is connected
        if !self
            .peers
            .get(&message.to_node_id)
            .map(|p| p.connected)
            .unwrap_or(false)
        {
            return Err(format!("Peer {} not connected", message.to_node_id));
        }

        // Track message
        if message.requires_ack {
            self.ack_pending.insert(message.message_id.clone(), message.clone());
        }

        // Add to queue
        self.message_queue.push_back(message.clone());

        // Update peer stats
        if let Some(peer) = self.peers.get_mut(&message.to_node_id) {
            peer.messages_sent += 1;
        }

        Ok(())
    }

    /// Receive message from peer
    pub fn receive_message(&mut self, message: PeerMessage) -> Result<(), String> {
        // Update peer stats
        if let Some(peer) = self.peers.get_mut(&message.from_node_id) {
            peer.messages_received += 1;
            peer.last_seen_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        } else {
            return Err(format!("Unknown peer {}", message.from_node_id));
        }

        self.message_queue.push_back(message);
        Ok(())
    }

    /// Get pending messages (FIFO)
    pub fn get_pending_messages(&mut self) -> Vec<PeerMessage> {
        let mut messages = Vec::new();
        while let Some(msg) = self.message_queue.pop_front() {
            messages.push(msg);
        }
        messages
    }

    /// Acknowledge a message
    pub fn acknowledge(&mut self, message_id: &str) -> Option<PeerMessage> {
        self.ack_pending.remove(message_id)
    }

    /// Get network statistics
    pub fn stats(&self) -> NetworkStats {
        let connected_peers = self.peers.values().filter(|p| p.connected).count();
        let total_messages_sent: u64 = self.peers.values().map(|p| p.messages_sent).sum();
        let total_messages_received: u64 = self.peers.values().map(|p| p.messages_received).sum();

        NetworkStats {
            connected_peers,
            total_peers: self.peers.len(),
            pending_acks: self.ack_pending.len(),
            messages_in_queue: self.message_queue.len(),
            total_messages_sent,
            total_messages_received,
        }
    }
}

impl Default for P2PNetwork {
    fn default() -> Self {
        Self::new(NetworkConfig::default())
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub total_peers: usize,
    pub pending_acks: usize,
    pub messages_in_queue: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_message_creation() {
        let msg = PeerMessage::new(
            "hive-1".to_string(),
            "hive-2".to_string(),
            MessageType::Ping,
            vec![],
        );
        assert_eq!(msg.from_node_id, "hive-1");
        assert!(!msg.is_expired(5000));
    }

    #[test]
    fn test_network_connect_peer() {
        let mut network = P2PNetwork::new(NetworkConfig::default());
        let result = network.connect_peer("hive-2".to_string(), "127.0.0.1:8002".to_string());
        assert!(result.is_ok());
        assert_eq!(network.peers.len(), 1);
    }

    #[test]
    fn test_network_send_message() {
        let mut network = P2PNetwork::new(NetworkConfig::default());
        network.connect_peer("hive-2".to_string(), "127.0.0.1:8002".to_string()).ok();

        let msg = PeerMessage::new(
            "hive-1".to_string(),
            "hive-2".to_string(),
            MessageType::Ping,
            vec![],
        );

        let result = network.send_message(msg);
        assert!(result.is_ok());
        assert_eq!(network.message_queue.len(), 1);
    }

    #[test]
    fn test_network_stats() {
        let mut network = P2PNetwork::new(NetworkConfig::default());
        network.connect_peer("hive-2".to_string(), "127.0.0.1:8002".to_string()).ok();

        let stats = network.stats();
        assert_eq!(stats.connected_peers, 1);
        assert_eq!(stats.total_peers, 1);
    }
}
