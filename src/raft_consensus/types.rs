/// Raft consensus types - RPCs, state enums, and core data structures
///
/// Implements types from the Raft consensus paper (Ongaro & Ousterhout, 2014)

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Node identifier in the cluster
pub type NodeId = String;

/// Term number in the cluster
pub type Term = u64;

/// Index in the log
pub type LogIndex = u64;

/// ===== STATE MACHINE =====

/// Raft node state - leader, follower, or candidate
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RaftState {
    /// Follower: accepts AppendEntriesRPC from leader
    Follower {
        leader_id: Option<NodeId>,
        last_heartbeat: DateTime<Utc>,
    },
    /// Candidate: participates in election
    Candidate {
        votes_received: u32,
        votes_needed: u32,
    },
    /// Leader: replicates log to followers
    Leader {
        elected_at: DateTime<Utc>,
    },
}

impl RaftState {
    pub fn is_leader(&self) -> bool {
        matches!(self, RaftState::Leader { .. })
    }

    pub fn is_follower(&self) -> bool {
        matches!(self, RaftState::Follower { .. })
    }

    pub fn is_candidate(&self) -> bool {
        matches!(self, RaftState::Candidate { .. })
    }

    pub fn as_str(&self) -> &str {
        match self {
            RaftState::Follower { .. } => "Follower",
            RaftState::Candidate { .. } => "Candidate",
            RaftState::Leader { .. } => "Leader",
        }
    }
}

/// ===== LOG ENTRIES =====

/// Entry in the Raft log
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log index of this entry
    pub index: LogIndex,
    /// Term when entry was received by leader
    pub term: Term,
    /// Actual data (FederationEvent)
    pub data: serde_json::Value,
    /// Client ID for deduplication
    pub client_id: String,
    /// Sequence number for deduplication
    pub sequence: u64,
    /// Timestamp when entry was created
    pub created_at: DateTime<Utc>,
}

impl LogEntry {
    pub fn new(
        index: LogIndex,
        term: Term,
        data: serde_json::Value,
        client_id: String,
        sequence: u64,
    ) -> Self {
        Self {
            index,
            term,
            data,
            client_id,
            sequence,
            created_at: Utc::now(),
        }
    }

    /// Check if this entry matches another by index and term
    pub fn matches(&self, index: LogIndex, term: Term) -> bool {
        self.index == index && self.term == term
    }
}

/// ===== RPC TYPES =====

/// RequestVoteRPC - request for leader election
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestVoteRpc {
    /// Current term of the sender
    pub term: Term,
    /// Candidate requesting votes
    pub candidate_id: NodeId,
    /// Index of candidate's last log entry
    pub last_log_index: LogIndex,
    /// Term of candidate's last log entry
    pub last_log_term: Term,
}

/// RequestVoteRPC Response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term of voter
    pub term: Term,
    /// true means candidate received vote
    pub vote_granted: bool,
}

/// AppendEntriesRPC - log replication and heartbeat
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendEntriesRpc {
    /// Current term of leader
    pub term: Term,
    /// So follower can redirect clients
    pub leader_id: NodeId,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: LogIndex,
    /// Term of prev_log_index entry
    pub prev_log_term: Term,
    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntry>,
    /// Leader's commit index
    pub leader_commit: LogIndex,
}

impl AppendEntriesRpc {
    pub fn is_heartbeat(&self) -> bool {
        self.entries.is_empty()
    }
}

/// AppendEntriesRPC Response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term of responder
    pub term: Term,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    /// (Optimization) Index of first conflicting entry
    pub conflict_index: Option<LogIndex>,
}

/// InstallSnapshotRPC - fast recovery for slow followers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallSnapshotRpc {
    /// Leader's term
    pub term: Term,
    /// So follower can redirect clients
    pub leader_id: NodeId,
    /// Index of last included entry
    pub last_included_index: LogIndex,
    /// Term of last included entry
    pub last_included_term: Term,
    /// Byte offset where chunk is positioned in the snapshot
    pub offset: u64,
    /// Raw bytes of the snapshot chunk
    pub data: Vec<u8>,
    /// True if this is the last chunk
    pub done: bool,
}

/// InstallSnapshotRPC Response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// Current term of responder
    pub term: Term,
}

/// ===== PERSISTENT STATE =====

/// State that must be persisted to stable storage
/// (before responding to RPCs)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistentState {
    /// Latest term server has seen
    pub current_term: Term,
    /// Candidate that received vote in current term (or None)
    pub voted_for: Option<NodeId>,
    /// Log entries
    #[serde(skip)]
    pub log_entries: Vec<LogEntry>,
    /// Last included index (from snapshot)
    pub last_included_index: LogIndex,
    /// Last included term (from snapshot)
    pub last_included_term: Term,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            log_entries: Vec::new(),
            last_included_index: 0,
            last_included_term: 0,
        }
    }
}

/// ===== VOLATILE STATE =====

/// State that is volatile (reset after restart)
#[derive(Clone, Debug)]
pub struct VolatileState {
    /// Index of highest log entry known to be committed
    pub commit_index: LogIndex,
    /// Index of highest log entry applied to state machine
    pub last_applied: LogIndex,
}

impl Default for VolatileState {
    fn default() -> Self {
        Self {
            commit_index: 0,
            last_applied: 0,
        }
    }
}

/// ===== LEADER STATE =====

/// State for leaders (reinitialized after election)
#[derive(Clone, Debug)]
pub struct LeaderState {
    /// For each server, index of next log entry to send to that server
    pub next_index: HashMap<NodeId, LogIndex>,
    /// For each server, index of highest log entry known to be replicated on server
    pub match_index: HashMap<NodeId, LogIndex>,
}

impl LeaderState {
    pub fn new(node_ids: Vec<NodeId>, last_log_index: LogIndex) -> Self {
        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();

        for id in node_ids {
            next_index.insert(id.clone(), last_log_index + 1);
            match_index.insert(id, 0);
        }

        Self {
            next_index,
            match_index,
        }
    }
}

/// ===== MUTATION HANDLING =====

/// Command to apply to state machine
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyCommand {
    /// The mutation data
    pub data: serde_json::Value,
    /// Client ID for deduplication
    pub client_id: String,
    /// Sequence number for ordered deduplication
    pub sequence: u64,
}

/// Result of applying a command
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Whether the command was successfully applied
    pub success: bool,
    /// Result data (or error message)
    pub result: serde_json::Value,
    /// Index at which command was applied
    pub applied_index: LogIndex,
}

/// ===== CONFIGURATION =====

/// Raft configuration
#[derive(Clone, Debug)]
pub struct RaftConfig {
    /// This node's ID
    pub node_id: NodeId,
    /// All node IDs in cluster
    pub all_nodes: Vec<NodeId>,
    /// Election timeout range (ms)
    pub election_timeout_min_ms: u64,
    pub election_timeout_max_ms: u64,
    /// Heartbeat interval (ms)
    pub heartbeat_interval_ms: u64,
}

impl RaftConfig {
    pub fn new(node_id: NodeId, all_nodes: Vec<NodeId>) -> Self {
        Self {
            node_id,
            all_nodes,
            election_timeout_min_ms: crate::raft_consensus::ELECTION_TIMEOUT_MIN_MS,
            election_timeout_max_ms: crate::raft_consensus::ELECTION_TIMEOUT_MAX_MS,
            heartbeat_interval_ms: crate::raft_consensus::HEARTBEAT_INTERVAL_MS,
        }
    }

    /// Number of nodes in cluster
    pub fn cluster_size(&self) -> usize {
        self.all_nodes.len()
    }

    /// Minimum votes needed for quorum
    pub fn quorum_size(&self) -> usize {
        self.cluster_size() / 2 + 1
    }

    /// Is this node in the cluster?
    pub fn is_valid_node(&self, node_id: &NodeId) -> bool {
        self.all_nodes.contains(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_state_checks() {
        let follower = RaftState::Follower {
            leader_id: Some("leader1".to_string()),
            last_heartbeat: Utc::now(),
        };

        assert!(follower.is_follower());
        assert!(!follower.is_leader());
        assert!(!follower.is_candidate());
        assert_eq!(follower.as_str(), "Follower");
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(
            1,
            0,
            serde_json::json!({"cmd": "test"}),
            "client1".to_string(),
            1,
        );

        assert_eq!(entry.index, 1);
        assert_eq!(entry.term, 0);
        assert_eq!(entry.client_id, "client1");
    }

    #[test]
    fn test_log_entry_matches() {
        let entry = LogEntry::new(5, 2, serde_json::json!({}), "c1".to_string(), 1);

        assert!(entry.matches(5, 2));
        assert!(!entry.matches(5, 3));
        assert!(!entry.matches(6, 2));
    }

    #[test]
    fn test_quorum_calculation() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );

        assert_eq!(config.cluster_size(), 3);
        assert_eq!(config.quorum_size(), 2); // majority of 3 is 2
    }

    #[test]
    fn test_quorum_5_nodes() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec![
                "node1".to_string(),
                "node2".to_string(),
                "node3".to_string(),
                "node4".to_string(),
                "node5".to_string(),
            ],
        );

        assert_eq!(config.cluster_size(), 5);
        assert_eq!(config.quorum_size(), 3); // majority of 5 is 3
    }

    #[test]
    fn test_heartbeat_detection() {
        let heartbeat = AppendEntriesRpc {
            term: 1,
            leader_id: "leader".to_string(),
            prev_log_index: 5,
            prev_log_term: 1,
            entries: vec![],
            leader_commit: 5,
        };

        assert!(heartbeat.is_heartbeat());

        let with_entries = AppendEntriesRpc {
            entries: vec![LogEntry::new(
                6,
                1,
                serde_json::json!({}),
                "c".to_string(),
                1,
            )],
            ..heartbeat
        };

        assert!(!with_entries.is_heartbeat());
    }
}
