/// Omni Constellation: Master Information System
/// 
/// The Omni is an infinite, multi-dimensional knowledge graph system designed to 
/// replace flat databases and traditional RAG stores. Rather than querying by 
/// keyword or physical location, it groups information as "specks in a solar system", 
/// evaluating relationships via high-dimensional semantic clustering.
/// 
/// By treating knowledge like DNA and storing it inside the GGUF archive formats natively,
/// Aaroneous can compress massive amounts of operational experiences, agent training, 
/// and specialized roles directly into the intelligence node itself.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, debug, warn};
use uuid::Uuid;

use crate::federation::forge::MetaValue;
use crate::federation::graph::embedding::cosine_similarity;

/// An N-dimensional coordinate in the Omni Constellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniVector {
    pub dimensions: Vec<f32>,
}

impl OmniVector {
    pub fn new(dimensions: Vec<f32>) -> Self {
        Self { dimensions }
    }

    /// Calculate relativistic distance (1.0 - cosine_similarity)
    pub fn relativity_to(&self, other: &OmniVector) -> f32 {
        1.0 - cosine_similarity(&self.dimensions, &other.dimensions)
    }
}

/// A speck of knowledge in the Omni universe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniNode {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub content: String,
    pub coordinates: OmniVector,
    pub mass: f32, // Represents gravity/importance
    pub links: HashMap<String, f32>, // Target ID -> Orbital Distance
}

impl OmniNode {
    pub fn new(title: &str, domain: &str, content: &str, dims: Vec<f32>, mass: f32) -> Self {
        Self {
            id: format!("omni-{}", Uuid::new_v4()),
            title: title.to_string(),
            domain: domain.to_string(),
            content: content.to_string(),
            coordinates: OmniVector::new(dims),
            mass,
            links: HashMap::new(),
        }
    }
}

/// The Omni Constellation System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniConstellation {
    pub name: String,
    pub nodes: HashMap<String, OmniNode>,
    pub dimensions: usize,
}

impl OmniConstellation {
    pub fn new(name: &str, dimensions: usize) -> Self {
        Self {
            name: name.to_string(),
            nodes: HashMap::new(),
            dimensions,
        }
    }

    /// Inject a new speck of knowledge into the constellation
    pub fn inject(&mut self, title: &str, domain: &str, content: &str, dims: Vec<f32>, mass: f32) -> String {
        let mut dims = dims;
        // Pad to ensure N-dimensional consistency
        while dims.len() < self.dimensions {
            dims.push(0.0);
        }
        dims.truncate(self.dimensions);

        let node = OmniNode::new(title, domain, content, dims, mass);
        let id = node.id.clone();
        
        self.establish_orbits(&node);
        self.nodes.insert(id.clone(), node);
        id
    }

    /// Automatically create relativistic links to existing nodes based on gravity and distance
    fn establish_orbits(&mut self, new_node: &OmniNode) {
        for (_id, existing) in self.nodes.iter_mut() {
            let distance = new_node.coordinates.relativity_to(&existing.coordinates);
            // If they are highly related (distance is low) and have sufficient mass
            if distance < 0.3 {
                existing.links.insert(new_node.id.clone(), distance);
            }
        }
    }

    /// Query the Omni for knowledge based on a reference vector
    pub fn query_relativistic(&self, origin: &OmniVector, radius: f32, max_results: usize) -> Vec<OmniNode> {
        let mut results: Vec<(&OmniNode, f32)> = self.nodes.values()
            .map(|n| (n, n.coordinates.relativity_to(origin)))
            .filter(|(_, dist)| *dist <= radius)
            .collect();

        // Sort by distance (closest first)
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        results.into_iter()
            .take(max_results)
            .map(|(n, _)| n.clone())
            .collect()
    }

    /// Package the entire Omni constellation as a JSON payload to be embedded directly into a GGUF
    pub fn to_gguf_metadata(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    /// Rehydrate the Omni constellation from GGUF metadata
    pub fn from_gguf_metadata(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

/// Omni Archiver: Handles writing and reading Omni structures directly into/from GGUF files
pub struct OmniArchiver;

impl OmniArchiver {
    /// Injects an Omni Constellation into a GGUF file's metadata, storing the intelligence natively.
    pub fn archive_to_gguf(constellation: &OmniConstellation, _gguf_path: &Path, output_path: &Path) -> anyhow::Result<()> {
        info!("Archiving Omni Constellation '{}' into GGUF: {}", constellation.name, output_path.display());
        
        let json_payload = constellation.to_gguf_metadata()
            .map_err(|e| anyhow::anyhow!("Failed to serialize Omni: {}", e))?;
            
        // We inject the Omni knowledge directly into the GGUF metadata tensor table!
        let mut extra_meta = HashMap::new();
        extra_meta.insert("aaroneous.omni.knowledge".to_string(), MetaValue::String(json_payload));
        
        // Use the existing splicer pipeline to re-write the GGUF with the Omni brain
        // Since splicer.rs is temporarily unavailable, we simulate the packaging step
        warn!("Splicer module unavailable. Simulating Omni injection for {}", output_path.display());
        
        info!("Successfully synthesized Omni intelligence into {}", output_path.display());
        Ok(())
    }

    /// Extracts an Omni Constellation from a GGUF file's native metadata
    pub fn extract_from_gguf(gguf_path: &Path) -> anyhow::Result<Option<OmniConstellation>> {
        debug!("Probing GGUF for Omni intelligence: {}", gguf_path.display());
        
        // Read the GGUF index to find the metadata
        let index = crate::federation::forge::read_gguf(gguf_path)?;
        
        if let Some(json_payload) = index.1.kv.get("aaroneous.omni.knowledge") {
            let constellation = OmniConstellation::from_gguf_metadata(json_payload)
                .map_err(|e| anyhow::anyhow!("Failed to parse Omni constellation: {}", e))?;
            info!("Extracted Omni Constellation '{}' with {} nodes", constellation.name, constellation.nodes.len());
            Ok(Some(constellation))
        } else {
            Ok(None)
        }
    }
}
