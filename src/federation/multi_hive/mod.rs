/// Phase I: Advanced Federation - Multi-Hive Coordination
/// 
/// Enable federation of independent Aaroneous hives:
/// - Multi-hive clustering and discovery
/// - P2P networking between hives
/// - Distributed consensus (gossip protocol)
/// - Federated learning (gradient exchange)
/// - Cross-hive specialist coordination
/// - Distributed DNA Bank synchronization

pub mod hive_cluster;
pub mod p2p_network;
pub mod consensus;
pub mod federated_learning;
pub mod distributed_registry;

pub use hive_cluster::{HiveCluster, HiveNode, HiveNodeStatus, ClusterConfig};
pub use p2p_network::{P2PNetwork, PeerMessage, MessageType, NetworkConfig};
pub use consensus::{GossipMessage, ConsensusEngine, ConsensusState};
pub use federated_learning::{GradientUpdate, ModelMerger, FederatedLearningEngine};
pub use distributed_registry::{DistributedSpecialistRegistry, RemoteSpecialist};

/// Multi-hive federation context
#[derive(Debug, Clone)]
pub struct MultihiveFederation {
    pub cluster: HiveCluster,
    pub network: P2PNetwork,
    pub consensus: ConsensusEngine,
    pub fed_learning: FederatedLearningEngine,
    pub registry: DistributedSpecialistRegistry,
}

impl MultihiveFederation {
    pub fn new(config: ClusterConfig) -> Self {
        let cluster = HiveCluster::new(config.clone());
        let network = P2PNetwork::new(config.clone().into());
        let consensus = ConsensusEngine::new();
        let fed_learning = FederatedLearningEngine::new();
        let registry = DistributedSpecialistRegistry::new();

        Self {
            cluster,
            network,
            consensus,
            fed_learning,
            registry,
        }
    }

    /// Join a hive to the federation
    pub fn join_hive(&mut self, node: HiveNode) -> Result<(), String> {
        self.cluster.add_node(node.clone())?;
        self.registry.register_node(node)?;
        Ok(())
    }

    /// Get status of all hives
    pub fn cluster_status(&self) -> Vec<(String, HiveNodeStatus)> {
        self.cluster
            .nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.status.clone()))
            .collect()
    }

    /// Get healthy hives for load balancing
    pub fn healthy_hives(&self) -> Vec<HiveNode> {
        self.cluster
            .nodes
            .values()
            .filter(|node| matches!(node.status, HiveNodeStatus::Healthy))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multihive_federation_creation() {
        let config = ClusterConfig::default();
        let fed = MultihiveFederation::new(config);
        assert_eq!(fed.cluster.nodes.len(), 0);
    }

    #[test]
    fn test_multihive_join_hive() {
        let config = ClusterConfig::default();
        let mut fed = MultihiveFederation::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        let result = fed.join_hive(node.clone());
        assert!(result.is_ok());
        assert_eq!(fed.cluster.nodes.len(), 1);
    }

    #[test]
    fn test_multihive_cluster_status() {
        let config = ClusterConfig::default();
        let mut fed = MultihiveFederation::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        fed.join_hive(node).ok();

        let status = fed.cluster_status();
        assert_eq!(status.len(), 1);
    }

    #[test]
    fn test_multihive_healthy_hives() {
        let config = ClusterConfig::default();
        let mut fed = MultihiveFederation::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        fed.join_hive(node).ok();

        let healthy = fed.healthy_hives();
        assert_eq!(healthy.len(), 1);
    }
}
