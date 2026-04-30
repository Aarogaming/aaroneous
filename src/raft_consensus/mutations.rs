/// Mutation handling and quorum-based atomic operations

use super::types::*;

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
    fn test_client_command_dedup() {
        let cmd1 = ClientCommand::new("client1".to_string(), 1);
        let cmd2 = ClientCommand::new("client1".to_string(), 1);
        let cmd3 = ClientCommand::new("client1".to_string(), 2);

        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }
}
