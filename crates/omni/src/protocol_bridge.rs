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

/// Generic 3D Spatial Visual Primitive for Any Presentation Layer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UniversalSpatialPoint {
    pub id: String,
    pub label: String,
    pub coords: [f32; 3],
    pub color_rgba: [u8; 4],
    pub radius: f32,
    pub energy_level: f32,
}

/// Generic Link Primitive between two spatial nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UniversalSpatialLink {
    pub source_id: String,
    pub target_id: String,
    pub intensity: f32,
}

/// Universal Spatial Canvas Sink Trait (decouples 3D simulation from rendering engine)
pub trait UniversalSpatialCanvasSink: Send + Sync {
    fn render_scene(&mut self, points: &[UniversalSpatialPoint], links: &[UniversalSpatialLink]) -> Result<()>;
}

/// Headless in-memory sink for unit tests, CLI dumps, or remote WebSocket streams
#[derive(Default)]
pub struct InMemorySpatialCanvasSink {
    pub last_points: Vec<UniversalSpatialPoint>,
    pub last_links: Vec<UniversalSpatialLink>,
}

impl UniversalSpatialCanvasSink for InMemorySpatialCanvasSink {
    fn render_scene(&mut self, points: &[UniversalSpatialPoint], links: &[UniversalSpatialLink]) -> Result<()> {
        self.last_points = points.to_vec();
        self.last_links = links.to_vec();
        Ok(())
    }
}
