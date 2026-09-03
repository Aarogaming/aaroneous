/// Shared types for the P2P module & Fleet Swarm Federation
///
/// These types are used regardless of whether the `fleet` / `p2p-iroh` feature is
/// enabled, so the Omnipresent specialist and FleetScheduler have a stable API.

use serde::{Deserialize, Serialize};
use si_ir::NativeComputationalGraph;
use std::fmt;

/// Errors that can occur during P2P operations
#[derive(Debug, thiserror::Error)]
pub enum P2pError {
    #[error("P2P network error: {0}")]
    Network(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid endpoint ID: {0}")]
    InvalidEndpoint(String),

    #[error("Operation timed out after {0}ms")]
    Timeout(u64),

    #[error("P2P feature not enabled (compile with --features fleet / p2p-iroh)")]
    FeatureNotEnabled,

    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for P2pError {
    fn from(e: std::io::Error) -> Self {
        P2pError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for P2pError {
    fn from(e: serde_json::Error) -> Self {
        P2pError::SerializationError(e.to_string())
    }
}

/// A node identifier in the P2P network.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct P2pNodeId(pub String);

impl P2pNodeId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    /// Create a new random node ID for testing/stub mode
    pub fn random() -> Self {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self(format!("fleet-{:032x}", nanos))
    }

    /// Get a short display version (first 12 chars, UTF-8 safe)
    pub fn short(&self) -> String {
        let char_count = self.0.chars().count();
        if char_count <= 12 {
            self.0.clone()
        } else {
            let truncated: String = self.0.chars().take(12).collect();
            format!("{}…", truncated)
        }
    }
}

impl fmt::Display for P2pNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Work-stealing request sent by an idle or overloaded fleet node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkStealRequest {
    pub requester_node_id: P2pNodeId,
    pub max_nodes: usize,
    pub min_free_energy: f64,
}

/// Work-stealing response containing offloaded computation graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkStealResponse {
    pub donor_node_id: P2pNodeId,
    pub task_id: u64,
    pub graph: NativeComputationalGraph,
}

/// Completed execution result returned by a remote fleet worker node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkResult {
    pub worker_node_id: P2pNodeId,
    pub task_id: u64,
    pub execution_trace: Vec<u8>,
    pub result_status: u32,
    pub thermodynamic_free_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMessageKind {
    FullState,
    Delta,
    StateSync,
    Heartbeat,
    ConflictDetected,
    SyncRequest,
    Request,
    Response,
    WorkStealRequest,
    WorkStealResponse,
    WorkResult,
    CartridgeLoraDeltaSync,
}

/// Dynamic .si LoRA Delta Synchronization Payload for P2P Mesh Weight Propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartridgeLoraDeltaSync {
    pub cartridge_id: String,
    pub adaptation_cycle: u64,
    pub rank: usize,
    pub lora_b_delta: Vec<f32>,
    pub orthogonality_score: f32,
    pub free_energy_reduction: f32,
}

/// Target operating system and compute backend of a heterogeneous cluster node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformOs {
    WindowsDirectX,
    LinuxVulkan,
    DarwinMetal,
    BareMetalMicrokernel,
}

/// Hardware specification describing compute capabilities of a cluster node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterNodeHardwareSpec {
    pub os: PlatformOs,
    pub cpu_cores: u32,
    pub gpu_device_name: String,
    pub total_vram_mb: u32,
    pub supports_fp16: bool,
    pub supports_simd_warp_scan: bool,
}

impl Default for ClusterNodeHardwareSpec {
    fn default() -> Self {
        Self {
            os: PlatformOs::WindowsDirectX,
            cpu_cores: 16,
            gpu_device_name: "Host Accelerated Device".to_string(),
            total_vram_mb: 8192,
            supports_fp16: true,
            supports_simd_warp_scan: true,
        }
    }
}

/// Message format for syncing Intent state and Work-Stealing between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub kind: SyncMessageKind,
    pub payload: Vec<u8>,
    pub from: String,
    pub timestamp: u64,
    pub intent_version: u32,
}

impl SyncMessage {
    pub fn heartbeat() -> Self {
        Self {
            kind: SyncMessageKind::Heartbeat,
            payload: vec![],
            from: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            intent_version: 0,
        }
    }

    pub fn full_state(data: Vec<u8>) -> Self {
        Self {
            kind: SyncMessageKind::FullState,
            payload: data,
            from: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            intent_version: 0,
        }
    }

    pub fn work_steal_request(from: P2pNodeId, req: &WorkStealRequest) -> Self {
        let payload = serde_json::to_vec(req).unwrap_or_default();
        Self {
            kind: SyncMessageKind::WorkStealRequest,
            payload,
            from: from.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            intent_version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_random() {
        let id1 = P2pNodeId::random();
        let id2 = P2pNodeId::random();
        assert_ne!(id1, id2, "Random IDs should be unique");
    }

    #[test]
    fn test_work_steal_types_roundtrip() {
        let req = WorkStealRequest {
            requester_node_id: P2pNodeId("node-alpha".to_string()),
            max_nodes: 10,
            min_free_energy: 0.05,
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: WorkStealRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_nodes, 10);

        let res = WorkStealResponse {
            donor_node_id: P2pNodeId("node-beta".to_string()),
            task_id: 42,
            graph: NativeComputationalGraph::new(),
        };

        let json_res = serde_json::to_string(&res).unwrap();
        let parsed_res: WorkStealResponse = serde_json::from_str(&json_res).unwrap();
        assert_eq!(parsed_res.task_id, 42);
    }
}
