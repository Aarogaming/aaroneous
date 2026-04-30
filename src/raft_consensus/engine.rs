/// Raft consensus engine - multi-node coordinator
///
/// Manages leader election, log replication, and state machine across cluster

use super::types::*;
use super::node::RaftNode;
use std::collections::HashMap;

/// Raft cluster engine
pub struct RaftEngine {
    nodes: HashMap<NodeId, RaftNode>,
}

impl RaftEngine {
    /// Create new Raft cluster
    pub fn new(node_ids: Vec<NodeId>) -> Self {
        let mut nodes = HashMap::new();
        let config = RaftConfig::new(
            node_ids[0].clone(),
            node_ids.clone(),
        );

        for node_id in node_ids {
            let mut node_config = config.clone();
            node_config.node_id = node_id.clone();
            nodes.insert(node_id, RaftNode::new(node_config));
        }

        Self { nodes }
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &NodeId) -> Option<RaftNode> {
        self.nodes.get(node_id).cloned()
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<RaftNode> {
        self.nodes.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let node_ids = vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ];
        let engine = RaftEngine::new(node_ids);

        assert_eq!(engine.get_all_nodes().len(), 3);
    }

    #[test]
    fn test_get_node() {
        let node_ids = vec![
            "node1".to_string(),
            "node2".to_string(),
        ];
        let engine = RaftEngine::new(node_ids);

        assert!(engine.get_node(&"node1".to_string()).is_some());
        assert!(engine.get_node(&"node2".to_string()).is_some());
        assert!(engine.get_node(&"node3".to_string()).is_none());
    }
}
