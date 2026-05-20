/// Raft Consensus Engine - Distributed consensus for federation
///
/// Implements the Raft consensus algorithm for multi-node federation,
/// ensuring strong consistency guarantees across the cluster.
///
/// Key Components:
/// - RaftNode: Individual node state machine
/// - RaftLog: Append-only log with entry management
/// - RaftEngine: Cluster coordinator (leader election, replication)
/// - Snapshots: Log compaction and fast recovery

pub mod types;
pub mod log;
pub mod node;
pub mod engine;
pub mod mutations;
pub mod snapshot;
pub mod election;
pub mod integration_tests;

pub use types::*;
pub use log::RaftLog;
pub use node::RaftNode;
pub use engine::RaftEngine;
pub use snapshot::Snapshot;
pub use election::{ElectionTimeout, HeartbeatTimer, ElectionOutcome, handle_request_vote, random_election_timeout};

// Specific types for replication
pub use types::{ReplicationResult, AppendEntriesRpc, AppendEntriesResponse};

// Mutation types
pub use mutations::{
    is_quorum, ClientCommand, MutationRequest, MutationTracker, MutationState,
    calculate_new_commit_index,
};

use std::time::Duration;

/// Default election timeout range (ms)
pub const ELECTION_TIMEOUT_MIN_MS: u64 = 150;
pub const ELECTION_TIMEOUT_MAX_MS: u64 = 300;

/// Default heartbeat interval (ms)
pub const HEARTBEAT_INTERVAL_MS: u64 = 50;

/// Default RPC timeout
pub const RPC_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeouts_configured() {
        assert!(ELECTION_TIMEOUT_MIN_MS < ELECTION_TIMEOUT_MAX_MS);
        assert!(HEARTBEAT_INTERVAL_MS < ELECTION_TIMEOUT_MIN_MS);
    }
}
