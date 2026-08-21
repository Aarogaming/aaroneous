pub mod consensus;
pub mod distributed_registry;
pub mod federated_learning;
pub mod hive_cluster;
pub mod live_daemon;
pub mod p2p_network;
pub mod swarm_offloader;

pub use consensus::{
    ConsensusEngine, ConsensusInstance, ConsensusState, ConsensusStats, GossipMessage,
};
pub use distributed_registry::{DistributedSpecialistRegistry, RemoteSpecialist};
pub use federated_learning::{FederatedLearningEngine, GradientUpdate, ModelMerger};
pub use hive_cluster::{ClusterConfig, HiveCluster, HiveNode, HiveNodeStatus};
pub use live_daemon::{DaemonWirePacket, LiveP2PConfig, LiveP2PDaemon, LivePeerInfo};
pub use p2p_network::{MessageType, P2PNetwork, PeerMessage};
pub use swarm_offloader::{SwarmExecutionOutcome, SwarmOffloader, SwarmTask};

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
        self.clusters
            .iter()
            .flat_map(|c| {
                c.nodes
                    .iter()
                    .map(|(id, node)| (id.clone(), node.status))
                    .collect::<Vec<_>>()
            })
            .collect()
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
    pub async fn sync(&self) {}
}
