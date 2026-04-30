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
}
