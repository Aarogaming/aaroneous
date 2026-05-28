pub mod hive_cluster;
pub mod consensus;
pub mod distributed_registry;
pub mod federated_learning;
pub mod p2p_network;

pub use hive_cluster::{HiveCluster, HiveNode, ClusterConfig, HiveNodeStatus};
pub use consensus::{ConsensusEngine, ConsensusState, ConsensusInstance, ConsensusStats, GossipMessage};
pub use distributed_registry::{DistributedSpecialistRegistry, RemoteSpecialist};
pub use federated_learning::{FederatedLearningEngine, GradientUpdate, ModelMerger};
pub use p2p_network::{P2PNetwork, PeerMessage, MessageType};

pub struct MultihiveFederation {
    clusters: Vec<HiveCluster>,
    cluster: Option<HiveCluster>,
}

impl MultihiveFederation {
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            clusters: Vec::new(),
            cluster: Some(HiveCluster::new(config)),
        }
    }
    pub fn cluster_status(&self) -> Vec<(String, HiveNodeStatus)> {
        self.clusters.iter().map(|c| {
            c.nodes.iter().map(|(id, node)| (id.clone(), node.status)).collect::<Vec<_>>()
        }).flatten().collect()
    }
    pub fn join_hive(&mut self, node: HiveNode) -> Result<(), String> {
        if let Some(ref mut cluster) = self.cluster {
            cluster.add_node(node)
        } else if let Some(first) = self.clusters.first_mut() {
            first.add_node(node)
        } else {
            Err("No cluster available".to_string())
        }
    }
    pub async fn sync(&self) { }
}
