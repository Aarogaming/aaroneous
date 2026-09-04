//! crates/omni/src/spatial_coord.rs
//! 3D Semantic Coordinate System and Vector Algebra for the Omni Galaxy.
//! X: Domain Spectrum (Theory/Spec <-> Execution/HID)
//! Y: Temporal Phase (Archived History <-> Active/Future)
//! Z: Priority & Depth (Deep Background <-> Critical/Immediate)

use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

/// 3D coordinate point and vector in Omni semantic space
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpatialCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Default for SpatialCoord {
    fn default() -> Self {
        Self::ORIGIN
    }
}

impl SpatialCoord {
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn from_array(arr: [f64; 3]) -> Self {
        Self { x: arr[0], y: arr[1], z: arr[2] }
    }

    pub fn to_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    pub fn from_f32(arr: [f32; 3]) -> Self {
        Self { x: arr[0] as f64, y: arr[1] as f64, z: arr[2] as f64 }
    }

    pub fn to_f32(&self) -> [f32; 3] {
        [self.x as f32, self.y as f32, self.z as f32]
    }

    /// Calculate Euclidean distance to another spatial coordinate
    pub fn distance_to(&self, other: &SpatialCoord) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate squared Euclidean distance (avoids sqrt for fast comparisons)
    pub fn distance_squared_to(&self, other: &SpatialCoord) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Calculate L1 Manhattan distance
    pub fn manhattan_distance_to(&self, other: &SpatialCoord) -> f64 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }

    /// Calculate Chebyshev distance (maximum coordinate delta)
    pub fn chebyshev_distance_to(&self, other: &SpatialCoord) -> f64 {
        (self.x - other.x).abs().max((self.y - other.y).abs()).max((self.z - other.z).abs())
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

    /// Vector length (magnitude)
    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Squared vector length
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Returns unit vector in the same direction, or ORIGIN if length is zero
    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len > 1e-12 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self::ORIGIN
        }
    }

    /// Dot product with another coordinate vector
    pub fn dot(&self, other: &SpatialCoord) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product with another vector
    pub fn cross(&self, other: &SpatialCoord) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Linear interpolation between two coordinates: self * (1 - t) + other * t
    pub fn lerp(&self, other: &SpatialCoord, t: f64) -> Self {
        let clamped_t = t.clamp(0.0, 1.0);
        Self {
            x: self.x + (other.x - self.x) * clamped_t,
            y: self.y + (other.y - self.y) * clamped_t,
            z: self.z + (other.z - self.z) * clamped_t,
        }
    }

    /// Midpoint between self and other
    pub fn midpoint(&self, other: &SpatialCoord) -> Self {
        self.lerp(other, 0.5)
    }

    /// Checks if coordinate is inside an axis-aligned bounding box [min, max]
    pub fn is_inside_bounds(&self, min: &SpatialCoord, max: &SpatialCoord) -> bool {
        self.x >= min.x && self.x <= max.x
            && self.y >= min.y && self.y <= max.y
            && self.z >= min.z && self.z <= max.z
    }
}

impl Add for SpatialCoord {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for SpatialCoord {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f64> for SpatialCoord {
    type Output = Self;
    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl std::ops::Div<f64> for SpatialCoord {
    type Output = Self;
    fn div(self, scalar: f64) -> Self {
        if scalar.abs() < 1e-15 {
            Self::ORIGIN
        } else {
            Self {
                x: self.x / scalar,
                y: self.y / scalar,
                z: self.z / scalar,
            }
        }
    }
}

impl std::ops::Neg for SpatialCoord {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_distance_and_metrics() {
        let a = SpatialCoord::new(0.0, 0.0, 0.0);
        let b = SpatialCoord::new(3.0, 4.0, 0.0);
        assert_eq!(a.distance_to(&b), 5.0);
        assert_eq!(a.distance_squared_to(&b), 25.0);
        assert_eq!(a.manhattan_distance_to(&b), 7.0);
        assert_eq!(a.chebyshev_distance_to(&b), 4.0);
    }

    #[test]
    fn test_vector_algebra() {
        let u = SpatialCoord::new(1.0, 0.0, 0.0);
        let v = SpatialCoord::new(0.0, 1.0, 0.0);
        let cross = u.cross(&v);
        assert_eq!(cross, SpatialCoord::new(0.0, 0.0, 1.0));
        assert_eq!(u.dot(&v), 0.0);

        let sum = u + v;
        assert_eq!(sum, SpatialCoord::new(1.0, 1.0, 0.0));

        let scaled = sum * 2.0;
        assert_eq!(scaled, SpatialCoord::new(2.0, 2.0, 0.0));

        let norm = scaled.normalized();
        assert!((norm.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_lerp_and_bounds() {
        let a = SpatialCoord::new(0.0, 0.0, 0.0);
        let b = SpatialCoord::new(10.0, 20.0, 30.0);
        let mid = a.midpoint(&b);
        assert_eq!(mid, SpatialCoord::new(5.0, 10.0, 15.0));

        let min_bound = SpatialCoord::new(0.0, 0.0, 0.0);
        let max_bound = SpatialCoord::new(10.0, 20.0, 30.0);
        assert!(mid.is_inside_bounds(&min_bound, &max_bound));
        assert!(!SpatialCoord::new(15.0, 5.0, 5.0).is_inside_bounds(&min_bound, &max_bound));
    }

    #[test]
    fn test_vector_division_and_negation() {
        let v = SpatialCoord::new(10.0, -20.0, 30.0);
        let divided = v / 2.0;
        assert_eq!(divided, SpatialCoord::new(5.0, -10.0, 15.0));

        let div_zero = v / 0.0;
        assert_eq!(div_zero, SpatialCoord::ORIGIN);

        let negated = -v;
        assert_eq!(negated, SpatialCoord::new(-10.0, 20.0, -30.0));
    }
}
