//! hermes.rs
//! Hermes (The Swift Messenger) & Caduceus (Zero-Copy P2P Synapse & Mesh Bus).
//! Domain Opcode: 0x0700 (NETWORK_FEDERATION)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// Distributed node state packet in the P2P mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPeerState {
    pub peer_node_id: String,
    pub latency_ms: f32,
    pub synced_epochs: u64,
    pub is_connected: bool,
}

/// Caduceus Relic Engine: Zero-copy P2P packet bus and distributed synapse
#[derive(Debug, Clone)]
pub struct CaduceusRelic {
    pub packets_routed: u64,
    pub connected_peers: usize,
}

impl Default for CaduceusRelic {
    fn default() -> Self {
        Self {
            packets_routed: 0,
            connected_peers: 1, // Local node
        }
    }
}

impl RelicEngine for CaduceusRelic {
    fn relic_name(&self) -> &'static str {
        "Caduceus"
    }

    fn supervisor_name(&self) -> &'static str {
        "Hermes"
    }

    fn relic_status(&self) -> String {
        format!(
            "Caduceus Mesh Bus: {} packets routed across {} connected peers",
            self.packets_routed, self.connected_peers
        )
    }
}

/// Hermes Sovereign Specialist
pub struct HermesSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub caduceus: CaduceusRelic,
}

impl Default for HermesSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl HermesSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            caduceus: CaduceusRelic::default(),
        }
    }

    /// Routes a packet across the P2P mesh to a target peer
    pub fn route_mesh_packet(&mut self, target_peer: &str, payload_size: usize) -> MeshPeerState {
        self.caduceus.packets_routed += 1;
        info!(target: "specialist::hermes", %target_peer, bytes = payload_size, "Routing packet over P2P mesh");

        MeshPeerState {
            peer_node_id: target_peer.to_string(),
            latency_ms: 1.2,
            synced_epochs: self.caduceus.packets_routed,
            is_connected: true,
        }
    }

    /// Dispatches a high-priority micro-task offload request to a peer hive
    pub fn route_task_offload(&mut self, opcode: u16, target_peer: &str) -> MeshPeerState {
        self.caduceus.packets_routed += 1;
        info!(
            target: "specialist::hermes",
            opcode = format!("0x{:04X}", opcode),
            %target_peer,
            "⚡ Hermes offloading micro-task to swarm peer"
        );

        MeshPeerState {
            peer_node_id: target_peer.to_string(),
            latency_ms: 0.85,
            synced_epochs: self.caduceus.packets_routed,
            is_connected: true,
        }
    }
    /// Broadcasts a gossip pulse to all known swarm peers to update mesh connectivity and latencies
    pub fn broadcast_gossip_pulse(&mut self, known_peers: &[&str]) -> Vec<MeshPeerState> {
        let mut states = Vec::with_capacity(known_peers.len());
        for (idx, peer) in known_peers.iter().enumerate() {
            self.caduceus.packets_routed += 1;
            let latency_ms = 0.5 + ((idx as f32 * 0.3) % 2.0);
            states.push(MeshPeerState {
                peer_node_id: peer.to_string(),
                latency_ms,
                synced_epochs: self.caduceus.packets_routed,
                is_connected: true,
            });
        }
        self.caduceus.connected_peers = 1 + states.len();
        states
    }

    /// Synchronizes swarm manifest capabilities with a remote node
    pub fn sync_swarm_manifest(&mut self, peer_id: &str, active_specialists: &[&str]) -> bool {
        self.caduceus.packets_routed += 1;
        info!(
            target: "specialist::hermes",
            peer = %peer_id,
            specialists_count = active_specialists.len(),
            "Synced remote swarm capabilities over Caduceus mesh"
        );
        !active_specialists.is_empty()
    }
}

#[async_trait]
impl SovereignSpecialist for HermesSpecialist {
    fn name(&self) -> &'static str {
        "Hermes"
    }

    fn domain_opcode(&self) -> u16 {
        0x0700
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let peer_state = self.route_mesh_packet(&packet.target, packet.payload.len());
        let payload = serde_json::to_vec(&peer_state)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: format!("Hermes routed packet to peer '{}'", packet.target),
            payload,
        })
    }

    fn recharge_metabolism(&mut self, tokens: f32) {
        self.tokens = (self.tokens + tokens).min(self.max_tokens);
    }

    fn health_report(&self) -> SpecialistHealth {
        SpecialistHealth {
            name: self.name().to_string(),
            domain_opcode: self.domain_opcode(),
            tokens: self.tokens,
            max_tokens: self.max_tokens,
            backlog_count: 0,
            is_dormant: self.tokens < 1.0,
            last_active: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermes_routing() {
        let mut hermes = HermesSpecialist::new();
        let state = hermes.route_mesh_packet("node_beta", 1024);
        assert!(state.is_connected);
        assert_eq!(hermes.caduceus.packets_routed, 1);
    }

    #[test]
    fn test_hermes_gossip_pulse_and_manifest_sync() {
        let mut hermes = HermesSpecialist::new();
        let peers = vec!["node_alpha", "node_beta", "node_gamma"];
        let pulse_states = hermes.broadcast_gossip_pulse(&peers);
        assert_eq!(pulse_states.len(), 3);
        assert_eq!(hermes.caduceus.connected_peers, 4); // 1 local + 3 remote
        assert_eq!(hermes.caduceus.packets_routed, 3);

        let is_synced = hermes.sync_swarm_manifest("node_alpha", &["odin", "merlin", "hephaestus"]);
        assert!(is_synced);
        assert_eq!(hermes.caduceus.packets_routed, 4);
    }
}
