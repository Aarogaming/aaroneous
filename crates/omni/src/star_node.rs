//! star_node.rs
//! Atomic StarNode data unit representing minimized data, code, or capabilities in the Omni Galaxy.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::spatial_coord::SpatialCoord;

/// Node category types in the Omni galaxy
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StarNodeType {
    Feature,
    Bug,
    Roadmap,
    Decision,
    Architecture,
    Lore,
    Incident,
    Reference,
    KnowledgeGap,
    Memory,
    Specialist,
    NeuralSignal,
    LatentPulse,
    Resource,
    TestCase,
}

/// Lifecycle status of a star node
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StarNodeStatus {
    Planned,
    InProgress,
    Completed,
    Archived,
    Dormant,
}

/// Content-based relationship between star nodes
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LinkType {
    DependsOn,
    Implements,
    Documents,
    RelatesTo,
    Synthesizes,
}

/// Priority level of a star node
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// An atomic StarNode in the Omni Galaxy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StarNode {
    pub id: String,
    pub title: String,
    pub node_type: StarNodeType,
    pub status: StarNodeStatus,
    pub priority: Priority,
    pub domain: String,
    pub spatial_coord: SpatialCoord,
    pub activity_pulse: f32, // 0.0 to 1.0 (visual pulse/glow intensity)
    pub latent_vector: Option<[f32; 32]>, // 32-dim embedding for semantic gravitational clustering
    pub links: HashMap<String, LinkType>,
    pub tags: HashSet<String>,
    pub metadata: HashMap<String, String>,
    pub payload_uri: String,
    pub updated_at: DateTime<Utc>,
}

impl StarNode {
    pub fn new(
        id: &str,
        title: &str,
        node_type: StarNodeType,
        domain: &str,
        spatial_coord: SpatialCoord,
        payload_uri: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            node_type,
            status: StarNodeStatus::Planned,
            priority: Priority::Medium,
            domain: domain.to_string(),
            spatial_coord,
            activity_pulse: 0.5,
            latent_vector: None,
            links: HashMap::new(),
            tags: HashSet::new(),
            metadata: HashMap::new(),
            payload_uri: payload_uri.to_string(),
            updated_at: Utc::now(),
        }
    }

    /// Add a content-based relationship link to another star node
    pub fn link_to(&mut self, target_node_id: &str, link_type: LinkType) {
        self.links.insert(target_node_id.to_string(), link_type);
        self.updated_at = Utc::now();
    }

    /// Pulse the activity of the node when accessed
    pub fn pulse(&mut self, intensity: f32) {
        self.activity_pulse = intensity.clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    /// Builder method to attach a 32-dim latent embedding
    pub fn with_latent(mut self, vec: [f32; 32]) -> Self {
        self.latent_vector = Some(vec);
        self
    }

    /// Builder method to set status
    pub fn with_status(mut self, status: StarNodeStatus) -> Self {
        self.status = status;
        self
    }

    /// Builder method to set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Compute Euclidean distance to another StarNode in 3D space
    pub fn distance_to(&self, other: &StarNode) -> f64 {
        self.spatial_coord.distance_to(&other.spatial_coord)
    }

    /// Computes cosine similarity between this node's latent vector and a query vector
    pub fn cosine_similarity(&self, query_latent: &[f32; 32]) -> f32 {
        if let Some(vec) = &self.latent_vector {
            let mut dot = 0.0f32;
            let mut norm_a = 0.0f32;
            let mut norm_b = 0.0f32;

            for i in 0..32 {
                dot += vec[i] * query_latent[i];
                norm_a += vec[i] * vec[i];
                norm_b += query_latent[i] * query_latent[i];
            }

            if norm_a <= 1e-6 || norm_b <= 1e-6 {
                0.0
            } else {
                dot / (norm_a.sqrt() * norm_b.sqrt())
            }
        } else {
            0.0
        }
    }
}
