pub mod election;
pub mod engine;
#[cfg(test)]
pub mod integration_tests;
pub mod log;
pub mod mutations;
pub mod node;
pub mod snapshot;
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

pub use election::{
    ElectionOutcome, ElectionTimeout, HeartbeatTimer, handle_request_vote, random_election_timeout,
};
pub use engine::RaftEngine;
pub use log::RaftLog;
pub use node::RaftNode;
pub use snapshot::Snapshot;
pub use types::*;

// Specific types for replication
pub use types::{AppendEntriesResponse, AppendEntriesRpc, ReplicationResult};

// Mutation types
pub use mutations::{
    ClientCommand, MutationRequest, MutationState, MutationTracker, calculate_new_commit_index,
    is_quorum,
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
