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

    /// Replicate log entry to all followers
    /// 
    /// Called by leader to send AppendEntriesRpc to all followers
    pub fn replicate_log_entry(
        &mut self,
        leader_id: &NodeId,
        prev_log_index: LogIndex,
        prev_log_term: Term,
        entries: Vec<LogEntry>,
        leader_commit: LogIndex,
    ) -> Result<ReplicationResult, String> {
        let leader = self.get_node(leader_id)
            .ok_or("Leader not found".to_string())?;

        let current_term = leader.get_term()?;
        let rpc = AppendEntriesRpc {
            term: current_term,
            leader_id: leader_id.clone(),
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        };

        let mut success_count = 0;
        let mut conflict_indices = Vec::new();

        // Send to all followers
        for (node_id, follower) in &self.nodes {
            if node_id == leader_id {
                success_count += 1; // Count leader as acknowledging
                continue;
            }

            match follower.handle_append_entries(&rpc) {
                Ok(response) => {
                    if response.success {
                        success_count += 1;
                    } else if let Some(idx) = response.conflict_index {
                        conflict_indices.push((node_id.clone(), idx));
                    }
                }
                Err(_) => {
                    // Network error - will retry later
                }
            }
        }

        // Check if we have quorum
        let quorum_size = (self.nodes.len() / 2) + 1;
        let is_quorum = success_count >= quorum_size;

        Ok(ReplicationResult {
            is_quorum,
            success_count,
            total_nodes: self.nodes.len(),
            conflict_indices,
        })
    }

    /// Get next index for a follower (leader only)
    pub fn get_next_index(&self, leader_id: &NodeId, follower_id: &NodeId) -> Result<LogIndex, String> {
        let leader = self.get_node(leader_id)
            .ok_or("Leader not found".to_string())?;

        let leader_state = leader.get_leader_state()?
            .ok_or("Node is not a leader".to_string())?;

        leader_state.next_index.get(follower_id)
            .copied()
            .ok_or("Follower not in cluster".to_string())
    }

    /// Decrement next index for a follower on conflict
    pub fn decrement_next_index(&mut self, leader_id: &NodeId, follower_id: &NodeId) -> Result<(), String> {
        let leader = self.get_node(leader_id)
            .ok_or("Leader not found".to_string())?;

        leader.update_leader_state(|ls| {
            if let Some(idx) = ls.next_index.get_mut(follower_id) {
                if *idx > 1 {
                    *idx -= 1;
                }
            }
        })
    }

    /// Update match index for a follower on success
    pub fn update_match_index(&mut self, leader_id: &NodeId, follower_id: &NodeId, match_index: LogIndex) -> Result<(), String> {
        let leader = self.get_node(leader_id)
            .ok_or("Leader not found".to_string())?;

        leader.update_leader_state(|ls| {
            if let Some(idx) = ls.match_index.get_mut(follower_id) {
                *idx = (*idx).max(match_index);
            }
            // Also update next_index
            if let Some(next_idx) = ls.next_index.get_mut(follower_id) {
                *next_idx = (*next_idx).max(match_index + 1);
            }
        })
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

    #[test]
    fn test_replicate_empty_entries() {
        let mut engine = RaftEngine::new(vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ]);

        // Replicate empty (heartbeat)
        let result = engine.replicate_log_entry(
            &"node1".to_string(),
            0,
            0,
            vec![],
            0,
        ).unwrap();

        assert!(result.is_quorum);
        assert_eq!(result.success_count, 3); // All nodes (including leader)
    }

    #[test]
    fn test_decrement_next_index() {
        let mut engine = RaftEngine::new(vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ]);

        let leader = engine.get_node(&"node1".to_string()).unwrap();
        leader.become_leader().unwrap();

        // First add some entries to give leader a non-zero last_log_index
        let entry1 = LogEntry {
            index: 1,
            term: 1,
            data: serde_json::json!({"cmd": "test"}),
            client_id: "client1".to_string(),
            sequence: 1,
            created_at: chrono::Utc::now(),
        };
        let entry2 = LogEntry {
            index: 2,
            term: 1,
            data: serde_json::json!({"cmd": "test"}),
            client_id: "client1".to_string(),
            sequence: 2,
            created_at: chrono::Utc::now(),
        };
        
        leader.get_log().append(entry1).unwrap();
        leader.get_log().append(entry2).unwrap();

        // Rebuild leader state with new last_log_index
        leader.become_leader().unwrap();

        let initial_next = engine.get_next_index(&"node1".to_string(), &"node2".to_string()).unwrap();
        assert_eq!(initial_next, 3); // 2 entries + 1
        
        engine.decrement_next_index(&"node1".to_string(), &"node2".to_string()).unwrap();
        
        let after_decr = engine.get_next_index(&"node1".to_string(), &"node2".to_string()).unwrap();
        assert_eq!(after_decr, 2); // Decremented by 1
    }

    #[test]
    fn test_update_match_index() {
        let mut engine = RaftEngine::new(vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ]);

        let leader = engine.get_node(&"node1".to_string()).unwrap();
        leader.become_leader().unwrap();

        engine.update_match_index(&"node1".to_string(), &"node2".to_string(), 5).unwrap();

        let next = engine.get_next_index(&"node1".to_string(), &"node2".to_string()).unwrap();
        assert_eq!(next, 6); // next_index = match_index + 1
    }
}
