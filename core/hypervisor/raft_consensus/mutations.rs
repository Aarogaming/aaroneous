/// Mutation handling and quorum-based atomic operations
///
/// Implements atomic mutations with:
/// - Quorum-based acknowledgment
/// - Deduplication via client_id + sequence
/// - Committed entry advancement
/// - State machine application

use super::types::*;
use std::collections::HashMap;
use chrono::Utc;

/// Check if we have quorum (more than half)
pub fn is_quorum(votes: usize, total_nodes: usize) -> bool {
    votes > total_nodes / 2
}

/// Deduplication key for mutations
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientCommand {
    pub client_id: String,
    pub sequence: u64,
}

impl ClientCommand {
    pub fn new(client_id: String, sequence: u64) -> Self {
        Self {
            client_id,
            sequence,
        }
    }
}

/// State of a replicated mutation
#[derive(Clone, Debug, PartialEq)]
pub enum MutationState {
    /// Entry is in leader's log, not yet replicated
    Pending,
    /// Entry has been replicated to quorum
    Committed,
    /// Entry has been applied to state machine
    Applied,
}

/// Mutation request from client
#[derive(Clone, Debug)]
pub struct MutationRequest {
    /// Client that sent the mutation
    pub client_id: String,
    /// Sequence number (for deduplication)
    pub sequence: u64,
    /// Data to apply
    pub data: serde_json::Value,
}

impl MutationRequest {
    pub fn new(client_id: String, sequence: u64, data: serde_json::Value) -> Self {
        Self {
            client_id,
            sequence,
            data,
        }
    }

    /// Convert to log entry
    pub fn to_log_entry(self) -> LogEntry {
        LogEntry {
            index: 0, // Will be set by leader
            term: 0,   // Will be set by leader
            data: self.data,
            client_id: self.client_id,
            sequence: self.sequence,
            created_at: Utc::now(),
        }
    }
}

/// Mutation tracker for leader
#[derive(Clone, Debug)]
pub struct MutationTracker {
    /// Track which mutations have been applied
    applied_mutations: HashMap<ClientCommand, LogIndex>,
    /// Track acknowledgments for pending mutations
    ack_counts: HashMap<LogIndex, usize>,
    /// Total nodes in cluster
    total_nodes: usize,
}

impl MutationTracker {
    pub fn new(total_nodes: usize) -> Self {
        Self {
            applied_mutations: HashMap::new(),
            ack_counts: HashMap::new(),
            total_nodes,
        }
    }

    /// Record an acknowledgment for a log entry
    pub fn record_ack(&mut self, log_index: LogIndex) {
        *self.ack_counts.entry(log_index).or_insert(0) += 1;
    }

    /// Check if entry has quorum acknowledgment
    /// The leader always acknowledges its own log entries
    pub fn has_quorum(&self, log_index: LogIndex) -> bool {
        let follower_acks = self.ack_counts.get(&log_index).copied().unwrap_or(0);
        let total_acks = follower_acks + 1; // +1 for leader
        is_quorum(total_acks, self.total_nodes)
    }

    /// Check if mutation was already applied
    pub fn is_duplicate(&self, client_cmd: &ClientCommand) -> bool {
        self.applied_mutations.contains_key(client_cmd)
    }

    /// Mark mutation as applied
    pub fn mark_applied(&mut self, client_cmd: ClientCommand, log_index: LogIndex) {
        self.applied_mutations.insert(client_cmd, log_index);
    }

    /// Get applied index for a client command
    pub fn get_applied_index(&self, client_cmd: &ClientCommand) -> Option<LogIndex> {
        self.applied_mutations.get(client_cmd).copied()
    }

    /// Clean up acknowledged entries (for snapshot)
    pub fn prune_before(&mut self, index: LogIndex) {
        self.ack_counts.retain(|&k, _| k > index);
    }
}

/// Calculate the new commitIndex for leader based on replicas
pub fn calculate_new_commit_index(
    leader_state: &LeaderState,
    current_commit_index: LogIndex,
    total_nodes: usize,
) -> LogIndex {
    let mut indices: Vec<LogIndex> = leader_state.match_index.values().copied().collect();
    indices.push(std::u64::MAX); // Add leader's match index (all entries)
    indices.sort_by(|a, b| b.cmp(a)); // Sort descending
    
    // Get the index of the (n/2 + 1)-th highest match_index
    let quorum_idx = total_nodes / 2;
    if quorum_idx < indices.len() {
        indices[quorum_idx]
    } else {
        current_commit_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_3_nodes() {
        assert!(!is_quorum(1, 3)); // 1 out of 3 is not quorum
        assert!(is_quorum(2, 3));  // 2 out of 3 is quorum
        assert!(is_quorum(3, 3));  // 3 out of 3 is quorum
    }

    #[test]
    fn test_quorum_5_nodes() {
        assert!(!is_quorum(2, 5)); // 2 out of 5 is not quorum
        assert!(is_quorum(3, 5));  // 3 out of 5 is quorum
        assert!(is_quorum(4, 5));  // 4 out of 5 is quorum
        assert!(is_quorum(5, 5));  // 5 out of 5 is quorum
    }

    #[test]
    fn test_quorum_1_node() {
        assert!(is_quorum(1, 1)); // Single node is quorum
    }

    #[test]
    fn test_quorum_7_nodes() {
        assert!(!is_quorum(3, 7)); // 3 out of 7 is not quorum
        assert!(is_quorum(4, 7));  // 4 out of 7 is quorum
        assert!(is_quorum(5, 7));  // 5 out of 7 is quorum
    }

    #[test]
    fn test_client_command_dedup() {
        let cmd1 = ClientCommand::new("client1".to_string(), 1);
        let cmd2 = ClientCommand::new("client1".to_string(), 1);
        let cmd3 = ClientCommand::new("client1".to_string(), 2);

        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[test]
    fn test_mutation_request_to_log_entry() {
        let req = MutationRequest::new(
            "client1".to_string(),
            1,
            serde_json::json!({"cmd": "write", "value": "test"}),
        );

        let entry = req.to_log_entry();
        assert_eq!(entry.client_id, "client1");
        assert_eq!(entry.sequence, 1);
    }

    #[test]
    fn test_mutation_tracker_quorum() {
        let mut tracker = MutationTracker::new(3);

        // For 3 nodes, quorum is > 3/2 = 1.5, so need 2 votes total
        // Record 0 acks from followers
        assert!(!tracker.has_quorum(1)); // 0 follower acks + 1 leader = 1 (not > 1.5)

        tracker.record_ack(1); // 1 follower ack
        assert!(tracker.has_quorum(1)); // 1 follower ack + 1 leader = 2 (> 1.5) ✓
    }

    #[test]
    fn test_mutation_tracker_5_nodes() {
        let mut tracker = MutationTracker::new(5);

        // For 5 nodes, quorum is > 5/2 = 2.5, so need 3 votes total
        assert!(!tracker.has_quorum(1)); // 0 acks + leader = 1 (not > 2.5)

        tracker.record_ack(1); // 1 follower ack
        assert!(!tracker.has_quorum(1)); // 1 ack + leader = 2 (not > 2.5)

        tracker.record_ack(1); // 2 follower acks
        assert!(tracker.has_quorum(1)); // 2 acks + leader = 3 (> 2.5) ✓
    }

    #[test]
    fn test_mutation_deduplication() {
        let mut tracker = MutationTracker::new(3);

        let cmd = ClientCommand::new("client1".to_string(), 1);
        assert!(!tracker.is_duplicate(&cmd));

        tracker.mark_applied(cmd.clone(), 1);
        assert!(tracker.is_duplicate(&cmd));
        assert_eq!(tracker.get_applied_index(&cmd), Some(1));
    }

    #[test]
    fn test_mutation_tracker_multiple_commands() {
        let mut tracker = MutationTracker::new(3);

        let cmd1 = ClientCommand::new("client1".to_string(), 1);
        let cmd2 = ClientCommand::new("client1".to_string(), 2);
        let cmd3 = ClientCommand::new("client2".to_string(), 1);

        tracker.mark_applied(cmd1.clone(), 1);
        tracker.mark_applied(cmd2.clone(), 2);
        tracker.mark_applied(cmd3.clone(), 3);

        assert!(tracker.is_duplicate(&cmd1));
        assert!(tracker.is_duplicate(&cmd2));
        assert!(tracker.is_duplicate(&cmd3));
        
        assert_eq!(tracker.get_applied_index(&cmd1), Some(1));
        assert_eq!(tracker.get_applied_index(&cmd2), Some(2));
        assert_eq!(tracker.get_applied_index(&cmd3), Some(3));
    }

    #[test]
    fn test_calculate_commit_index() {
        let mut leader_state = LeaderState::new(
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
            0,
        );

        // Set match indices
        leader_state.match_index.insert("node2".to_string(), 2);
        leader_state.match_index.insert("node3".to_string(), 1);

        let new_commit = calculate_new_commit_index(&leader_state, 0, 3);
        // With match_index [2, 1] + leader (all), sorted desc: [inf, 2, 1]
        // Quorum idx = 3/2 = 1
        // indices[1] = 2
        assert_eq!(new_commit, 2);
    }

    #[test]
    fn test_calculate_commit_index_5_nodes() {
        let mut leader_state = LeaderState::new(
            vec![
                "node1".to_string(),
                "node2".to_string(),
                "node3".to_string(),
                "node4".to_string(),
                "node5".to_string(),
            ],
            0,
        );

        // Set match indices for 4 followers
        leader_state.match_index.insert("node2".to_string(), 10);
        leader_state.match_index.insert("node3".to_string(), 8);
        leader_state.match_index.insert("node4".to_string(), 5);
        leader_state.match_index.insert("node5".to_string(), 3);

        let new_commit = calculate_new_commit_index(&leader_state, 0, 5);
        // With match_index [10, 8, 5, 3] + leader (inf), sorted desc: [inf, 10, 8, 5, 3]
        // Quorum idx = 5/2 = 2
        // indices[2] = 8
        assert_eq!(new_commit, 8);
    }

    #[test]
    fn test_mutation_tracker_prune() {
        let mut tracker = MutationTracker::new(3);

        // Record acks for multiple indices
        tracker.record_ack(1);
        tracker.record_ack(2);
        tracker.record_ack(3);
        tracker.record_ack(4);

        // Prune before index 2 (keep only entries with k > 2, i.e., 3, 4)
        tracker.prune_before(2);

        assert_eq!(tracker.ack_counts.len(), 2);
        assert!(!tracker.ack_counts.contains_key(&1));
        assert!(!tracker.ack_counts.contains_key(&2));
        assert!(tracker.ack_counts.contains_key(&3));
        assert!(tracker.ack_counts.contains_key(&4));
    }

    #[test]
    fn test_mutation_state_enum() {
        let states = vec![
            MutationState::Pending,
            MutationState::Committed,
            MutationState::Applied,
        ];

        assert_ne!(states[0], states[1]);
        assert_ne!(states[1], states[2]);
        assert_eq!(states[0].clone(), MutationState::Pending);
    }
}
