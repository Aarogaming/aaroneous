//! spatial_coord.rs
//! 3D Semantic Coordinate System for the Omni Galaxy.
//! X: Domain Spectrum (Theory/Spec <-> Execution/HID)
//! Y: Temporal Phase (Archived History <-> Active/Future)
//! Z: Priority & Depth (Deep Background <-> Critical/Immediate)

use serde::{Deserialize, Serialize};

/// 3D coordinate point in Omni semantic space
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpatialCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl SpatialCoord {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Calculate Euclidean distance to another spatial coordinate
    pub fn distance_to(&self, other: &SpatialCoord) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate weighted semantic distance
    pub fn semantic_distance_to(&self, other: &SpatialCoord) -> f64 {
        let euclidean = self.distance_to(other);
        let domain_proximity = (self.x - other.x).abs() / 2000.0;
        let temporal_alignment = (self.y - other.y).abs() / 2000.0;
        let priority_alignment = (self.z - other.z).abs() / 2000.0;

        let semantic_weight = (domain_proximity * 0.3 + temporal_alignment * 0.3 + priority_alignment * 0.4) * 0.5;
        euclidean * (1.0 + semantic_weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_distance() {
        let a = SpatialCoord::new(0.0, 0.0, 0.0);
        let b = SpatialCoord::new(3.0, 4.0, 0.0);
        assert_eq!(a.distance_to(&b), 5.0);
    }
}
