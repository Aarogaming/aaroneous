/// Individual Raft node state machine
/// 
/// Manages a single node's state in the Raft cluster

use super::types::*;
use super::log::RaftLog;
use std::sync::{Arc, RwLock};
use chrono::Utc;

/// Raft node - maintains state machine for a single node
#[derive(Clone)]
pub struct RaftNode {
    config: RaftConfig,
    
    // Persistent state (must be durably stored)
    persistent: Arc<RwLock<PersistentState>>,
    
    // Volatile state
    volatile: Arc<RwLock<VolatileState>>,
    
    // Current state (Follower/Candidate/Leader)
    state: Arc<RwLock<RaftState>>,
    
    // Log
    log: RaftLog,
    
    // Leader state (only valid if this node is leader)
    leader_state: Arc<RwLock<Option<LeaderState>>>,
}

impl RaftNode {
    /// Create new Raft node
    pub fn new(config: RaftConfig) -> Self {
        Self {
            config,
            persistent: Arc::new(RwLock::new(PersistentState::default())),
            volatile: Arc::new(RwLock::new(VolatileState::default())),
            state: Arc::new(RwLock::new(RaftState::Follower {
                leader_id: None,
                last_heartbeat: Utc::now(),
            })),
            log: RaftLog::new(),
            leader_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current state (Follower/Candidate/Leader)
    pub fn get_state(&self) -> Result<RaftState, String> {
        self.state.read()
            .map(|s| s.clone())
            .map_err(|_| "Failed to acquire lock".to_string())
    }

    /// Get current term
    pub fn get_term(&self) -> Result<Term, String> {
        self.persistent.read()
            .map(|s| s.current_term)
            .map_err(|_| "Failed to acquire lock".to_string())
    }

    /// Get who we voted for in current term (or None)
    pub fn get_voted_for(&self) -> Result<Option<NodeId>, String> {
        self.persistent.read()
            .map(|s| s.voted_for.clone())
            .map_err(|_| "Failed to acquire lock".to_string())
    }

    /// Get commit index
    pub fn get_commit_index(&self) -> Result<LogIndex, String> {
        self.volatile.read()
            .map(|s| s.commit_index)
            .map_err(|_| "Failed to acquire lock".to_string())
    }

    /// Get last applied index
    pub fn get_last_applied(&self) -> Result<LogIndex, String> {
        self.volatile.read()
            .map(|s| s.last_applied)
            .map_err(|_| "Failed to acquire lock".to_string())
    }

    /// Get log
    pub fn get_log(&self) -> RaftLog {
        self.log.clone()
    }

    /// Get cluster configuration
    pub fn get_config(&self) -> RaftConfig {
        self.config.clone()
    }

    /// Update term and reset vote
    pub fn update_term(&self, new_term: Term) -> Result<(), String> {
        let mut persistent = self.persistent.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        if new_term > persistent.current_term {
            persistent.current_term = new_term;
            persistent.voted_for = None; // Lost vote when term changes
        }
        
        Ok(())
    }

    /// Record a vote for a candidate
    pub fn vote_for(&self, candidate_id: NodeId) -> Result<bool, String> {
        let mut persistent = self.persistent.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        if persistent.voted_for.is_none() {
            persistent.voted_for = Some(candidate_id);
            Ok(true)
        } else {
            Ok(persistent.voted_for.as_ref() == Some(&candidate_id))
        }
    }

    /// Update commit index
    pub fn set_commit_index(&self, new_commit: LogIndex) -> Result<(), String> {
        let mut volatile = self.volatile.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        if new_commit > volatile.commit_index {
            volatile.commit_index = new_commit;
        }
        
        Ok(())
    }

    /// Become follower
    pub fn become_follower(&self, term: Term, leader: Option<NodeId>) -> Result<(), String> {
        self.update_term(term)?;
        
        let mut state = self.state.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        *state = RaftState::Follower {
            leader_id: leader,
            last_heartbeat: Utc::now(),
        };
        
        let mut leader_state = self.leader_state.write()
            .map_err(|_| "Failed to acquire lock")?;
        *leader_state = None; // Clear leader state
        
        Ok(())
    }

    /// Become candidate
    pub fn become_candidate(&self) -> Result<(), String> {
        let mut persistent = self.persistent.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        persistent.current_term += 1;
        persistent.voted_for = Some(self.config.node_id.clone());
        
        drop(persistent); // Release lock
        
        let votes_needed = self.config.quorum_size() as u32;
        let mut state = self.state.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        *state = RaftState::Candidate {
            votes_received: 1, // Vote for self
            votes_needed,
        };
        
        Ok(())
    }

    /// Become leader
    pub fn become_leader(&self) -> Result<(), String> {
        let last_log_index = self.log.last_index()?;
        
        let leader_state = LeaderState::new(
            self.config.all_nodes.clone(),
            last_log_index,
        );
        
        let mut state = self.state.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        *state = RaftState::Leader {
            elected_at: Utc::now(),
        };
        
        let mut ls = self.leader_state.write()
            .map_err(|_| "Failed to acquire lock")?;
        *ls = Some(leader_state);
        
        Ok(())
    }

    /// Get leader state (only valid if leader)
    pub fn get_leader_state(&self) -> Result<Option<LeaderState>, String> {
        self.leader_state.read()
            .map(|s| s.clone())
            .map_err(|_| "Failed to acquire lock".to_string())
    }

    /// Update leader state (only valid if leader)
    pub fn update_leader_state<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut LeaderState),
    {
        let mut ls = self.leader_state.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        if let Some(ref mut leader_state) = *ls {
            f(leader_state);
        }
        
        Ok(())
    }

    /// Handle AppendEntriesRpc from leader
    /// 
    /// Implements the follower side of log replication
    pub fn handle_append_entries(&self, rpc: &AppendEntriesRpc) -> Result<AppendEntriesResponse, String> {
        let persistent = self.persistent.read()
            .map_err(|_| "Failed to acquire lock")?;

        // 1. Check if RPC term is stale
        if rpc.term < persistent.current_term {
            return Ok(AppendEntriesResponse {
                term: persistent.current_term,
                success: false,
                conflict_index: None,
            });
        }

        drop(persistent);
        
        // 2. Update term if needed and become follower
        if rpc.term > self.get_term()? {
            self.update_term(rpc.term)?;
            self.become_follower(rpc.term, Some(rpc.leader_id.clone()))?;
        } else {
            // Update leader ID in follower state
            let mut state = self.state.write()
                .map_err(|_| "Failed to acquire lock")?;
            if let RaftState::Follower { leader_id, last_heartbeat } = &mut *state {
                *leader_id = Some(rpc.leader_id.clone());
                *last_heartbeat = Utc::now();
            }
        }

        // Re-acquire locks after potential state changes
        let mut persistent = self.persistent.write()
            .map_err(|_| "Failed to acquire lock")?;
        let volatile = self.volatile.read()
            .map_err(|_| "Failed to acquire lock")?;

        // 3. Check log consistency (prev_log_index and prev_log_term must match)
        let log_len = persistent.log_entries.len();
        
        // Convert logical index to position in vec (accounting for snapshot)
        let prev_index_in_vec = (rpc.prev_log_index as usize).saturating_sub(
            persistent.last_included_index as usize
        );
        
        // Check if we have entry at prev_log_index
        let has_matching_prefix = if rpc.prev_log_index == 0 {
            // Requesting from beginning
            true
        } else if rpc.prev_log_index <= persistent.last_included_index {
            // Check against snapshot
            rpc.prev_log_term == persistent.last_included_term
        } else if prev_index_in_vec > 0 && prev_index_in_vec <= log_len {
            // Check against log entry
            persistent.log_entries[prev_index_in_vec - 1].term == rpc.prev_log_term
        } else {
            false
        };

        if !has_matching_prefix {
            // Conflict! Find first index where we differ
            let conflict_index = if rpc.prev_log_index > persistent.last_included_index {
                if prev_index_in_vec > log_len {
                    log_len as LogIndex + persistent.last_included_index
                } else {
                    rpc.prev_log_index
                }
            } else {
                persistent.last_included_index
            };

            return Ok(AppendEntriesResponse {
                term: persistent.current_term,
                success: false,
                conflict_index: Some(conflict_index),
            });
        }

        // 4. Delete conflicting entries (entries with same index but different term)
        if !rpc.entries.is_empty() {
            let first_new_index = rpc.entries[0].index;
            
            // Remove entries from first_new_index onward
            let remove_from = (first_new_index as usize).saturating_sub(
                persistent.last_included_index as usize
            );
            
            if remove_from <= persistent.log_entries.len() {
                persistent.log_entries.truncate(remove_from);
            }
        }

        // 5. Append new entries
        for entry in &rpc.entries {
            // Sanity check: entry index should match expected position
            let expected_index = persistent.last_included_index + persistent.log_entries.len() as LogIndex + 1;
            if entry.index == expected_index {
                persistent.log_entries.push(entry.clone());
            }
        }

        // 6. Update commit index
        drop(volatile);
        let mut volatile = self.volatile.write()
            .map_err(|_| "Failed to acquire lock")?;
        
        if rpc.leader_commit > volatile.commit_index {
            let last_log_index = persistent.last_included_index + persistent.log_entries.len() as LogIndex;
            volatile.commit_index = rpc.leader_commit.min(last_log_index);
        }

        Ok(AppendEntriesResponse {
            term: persistent.current_term,
            success: true,
            conflict_index: None,
        })
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        assert_eq!(node.get_term().unwrap(), 0);
        assert!(node.get_voted_for().unwrap().is_none());
        assert!(node.get_state().unwrap().is_follower());
    }

    #[test]
    fn test_become_candidate() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        node.become_candidate().unwrap();

        assert_eq!(node.get_term().unwrap(), 1);
        assert!(node.get_state().unwrap().is_candidate());
        assert_eq!(node.get_voted_for().unwrap().unwrap(), "node1");
    }

    #[test]
    fn test_become_follower() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );
        let node = RaftNode::new(config);

        node.become_candidate().unwrap();
        node.become_follower(2, Some("node2".to_string())).unwrap();

        assert_eq!(node.get_term().unwrap(), 2);
        assert!(node.get_state().unwrap().is_follower());
    }

    #[test]
    fn test_become_leader() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        node.become_leader().unwrap();

        assert!(node.get_state().unwrap().is_leader());
        assert!(node.get_leader_state().unwrap().is_some());
    }

    #[test]
    fn test_handle_append_entries_stale_term() {
        let config = RaftConfig::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        // Set node to term 2
        node.update_term(2).unwrap();

        // Receive RPC from term 1 (stale)
        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "node1".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        assert!(!response.success);
        assert_eq!(response.term, 2);
    }

    #[test]
    fn test_handle_append_entries_heartbeat() {
        let config = RaftConfig::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        // Send heartbeat (empty entries) from node1
        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "node1".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        assert!(response.success);
        assert_eq!(response.term, 1);

        // Verify node became follower and knows the leader
        let state = node.get_state().unwrap();
        assert!(state.is_follower());
        if let RaftState::Follower { leader_id, .. } = state {
            assert_eq!(leader_id, Some("node1".to_string()));
        }
    }

    #[test]
    fn test_handle_append_entries_with_entries() {
        let config = RaftConfig::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        // Create a log entry to replicate
        let entry = LogEntry {
            index: 1,
            term: 1,
            data: serde_json::json!({"cmd": "write", "value": "test"}),
            client_id: "client1".to_string(),
            sequence: 1,
            created_at: Utc::now(),
        };

        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "node1".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![entry.clone()],
            leader_commit: 1,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        assert!(response.success);

        // Verify commit index was updated
        let commit = node.get_commit_index().unwrap();
        assert_eq!(commit, 1);
    }

    #[test]
    fn test_handle_append_entries_log_conflict() {
        let config = RaftConfig::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        // Pre-populate log with entry at index 1, term 1
        let entry1 = LogEntry {
            index: 1,
            term: 1,
            data: serde_json::json!({"cmd": "write"}),
            client_id: "client1".to_string(),
            sequence: 1,
            created_at: Utc::now(),
        };
        node.get_log().append(entry1.clone()).unwrap();

        // Try to append with wrong prev_log_term
        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "node1".to_string(),
            prev_log_index: 1,
            prev_log_term: 2, // Wrong term! Should be 1
            entries: vec![],
            leader_commit: 0,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        assert!(!response.success);
        assert!(response.conflict_index.is_some());
    }

    #[test]
    fn test_handle_append_entries_update_commit() {
        let config = RaftConfig::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        let entry1 = LogEntry {
            index: 1,
            term: 1,
            data: serde_json::json!({"cmd": "write"}),
            client_id: "client1".to_string(),
            sequence: 1,
            created_at: Utc::now(),
        };

        let entry2 = LogEntry {
            index: 2,
            term: 1,
            data: serde_json::json!({"cmd": "write"}),
            client_id: "client1".to_string(),
            sequence: 2,
            created_at: Utc::now(),
        };

        // Replicate two entries with leader_commit=2
        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "node1".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![entry1, entry2],
            leader_commit: 2,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        assert!(response.success);

        // Verify commit index updated
        let commit = node.get_commit_index().unwrap();
        assert_eq!(commit, 2);
    }

    #[test]
    fn test_handle_append_entries_delete_conflicts() {
        let config = RaftConfig::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        // First, establish baseline with entries 1 and 2 at term 1
        let entry1 = LogEntry {
            index: 1,
            term: 1,
            data: serde_json::json!({"cmd": "old"}),
            client_id: "client1".to_string(),
            sequence: 1,
            created_at: Utc::now(),
        };
        let entry2 = LogEntry {
            index: 2,
            term: 1,
            data: serde_json::json!({"cmd": "old"}),
            client_id: "client1".to_string(),
            sequence: 2,
            created_at: Utc::now(),
        };

        let rpc1 = AppendEntriesRpc {
            term: 1,
            leader_id: "node1".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![entry1, entry2],
            leader_commit: 0,
        };

        let resp1 = node.handle_append_entries(&rpc1).unwrap();
        assert!(resp1.success);

        // Now leader wants to replace entry at index 2 with a new term
        let new_entry2 = LogEntry {
            index: 2,
            term: 2,
            data: serde_json::json!({"cmd": "new"}),
            client_id: "client1".to_string(),
            sequence: 1,
            created_at: Utc::now(),
        };

        let rpc2 = AppendEntriesRpc {
            term: 2,
            leader_id: "node1".to_string(),
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![new_entry2],
            leader_commit: 2,
        };

        let resp2 = node.handle_append_entries(&rpc2).unwrap();
        assert!(resp2.success);
        assert_eq!(resp2.term, 2);
    }
}
