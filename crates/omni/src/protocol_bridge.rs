//! protocol_bridge.rs
//! Machine-Native Linking Protocol (MNLP) galaxy map serialization and streaming for Omni.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::galaxy_cluster::GalaxyCluster;
use crate::star_node::StarNode;

/// Complete serializable snapshot of the Omni Galaxy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniGalaxySnapshot {
    pub omni_version: String,
    pub total_stars: usize,
    pub total_galaxies: usize,
    pub galaxies: Vec<GalaxyCluster>,
    pub star_nodes: Vec<StarNode>,
}

/// Machine-Native Linking Protocol bridge for Omni Galaxy snapshots
pub struct OmniProtocolBridge;

impl OmniProtocolBridge {
    /// Creates a complete snapshot of the Omni galaxy state
    pub fn create_snapshot(
        nodes: &HashMap<String, StarNode>,
        galaxies: &[GalaxyCluster],
    ) -> OmniGalaxySnapshot {
        OmniGalaxySnapshot {
            omni_version: "1.0.0".to_string(),
            total_stars: nodes.len(),
            total_galaxies: galaxies.len(),
            galaxies: galaxies.to_vec(),
            star_nodes: nodes.values().cloned().collect(),
        }
    }

    /// Serializes the galaxy snapshot to a JSON byte stream
    pub fn encode_snapshot_json(snapshot: &OmniGalaxySnapshot) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(snapshot)?;
        Ok(bytes)
    }

    /// Deserializes a galaxy snapshot from a JSON byte slice
    pub fn decode_snapshot_json(bytes: &[u8]) -> Result<OmniGalaxySnapshot> {
        let snapshot: OmniGalaxySnapshot = serde_json::from_slice(bytes)?;
        Ok(snapshot)
    }
}
