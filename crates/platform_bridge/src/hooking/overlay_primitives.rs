// crates/platform_bridge/src/hooking/overlay_primitives.rs
//! Low-Latency GPU Action Feedback & Attention Primitives for SwapChain Injection.
//!
//! Provides hardware-efficient geometric and telemetry overlay structures
//! (crosshairs, bounding boxes, heatmaps, text banners) that can be rasterized
//! directly into intercepted DirectX backbuffers in < 250 microseconds.

use serde::{Deserialize, Serialize};

/// Color representation with 8-bit normalized RGBA channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const RED: Self = Self { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Self = Self { r: 0, g: 150, b: 255, a: 255 };
    pub const YELLOW: Self = Self { r: 255, g: 215, b: 0, a: 255 };
    pub const CYAN: Self = Self { r: 0, g: 255, b: 255, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };

    pub fn with_alpha(mut self, alpha: u8) -> Self {
        self.a = alpha;
        self
    }
}

/// Primitive visual item to be injected onto the swapchain backbuffer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverlayPrimitive {
    /// Predictive aim crosshair or attention focal center.
    Crosshair {
        center_x: f32,
        center_y: f32,
        radius: f32,
        thickness: f32,
        color: Rgba8,
    },
    /// Bounding rectangle indicating detected target or interactive UI element.
    BoundingBox {
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        thickness: f32,
        color: Rgba8,
    },
    /// Sub-frame execution telemetry and latency badge.
    TelemetryBadge {
        pos_x: f32,
        pos_y: f32,
        inference_latency_us: u32,
        fps: f32,
        confidence: f32,
    },
    /// Directional velocity vector indicating intended autonomous cursor / camera delta.
    MotionVector {
        start_x: f32,
        start_y: f32,
        delta_x: f32,
        delta_y: f32,
        color: Rgba8,
    },
}

/// A collection of overlay primitives composited together for a single present frame.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubFrameOverlayBatch {
    pub frame_id: u64,
    pub timestamp_ns: u64,
    pub primitives: Vec<OverlayPrimitive>,
}

impl SubFrameOverlayBatch {
    pub fn new(frame_id: u64) -> Self {
        Self {
            frame_id,
            timestamp_ns: 0,
            primitives: Vec::with_capacity(16),
        }
    }

    pub fn push(&mut self, primitive: OverlayPrimitive) {
        self.primitives.push(primitive);
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_batch_lifecycle() {
        let mut batch = SubFrameOverlayBatch::new(42);
        assert_eq!(batch.frame_id, 42);
        assert!(batch.is_empty());

        batch.push(OverlayPrimitive::Crosshair {
            center_x: 960.0,
            center_y: 540.0,
            radius: 12.0,
            thickness: 2.0,
            color: Rgba8::CYAN,
        });

        batch.push(OverlayPrimitive::TelemetryBadge {
            pos_x: 20.0,
            pos_y: 20.0,
            inference_latency_us: 145,
            fps: 144.0,
            confidence: 0.98,
        });

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());

        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn test_rgba8_colors_and_alpha() {
        let col = Rgba8::YELLOW.with_alpha(128);
        assert_eq!(col.r, 255);
        assert_eq!(col.g, 215);
        assert_eq!(col.b, 0);
        assert_eq!(col.a, 128);
    }
}
