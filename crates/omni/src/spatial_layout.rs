//! crates/omni/src/spatial_layout.rs
//! Mathematical Non-Overlapping Spatial Window Layout & AABB Collision Solvers.
//!
//! Provides deterministic non-overlapping placement strategies for 2D/3D presentation canvases
//! using axis-aligned bounding box (AABB) intersection and repulsion mathematics.

use serde::{Deserialize, Serialize};

/// Rectangular Area for Axis-Aligned Bounding Box (AABB) Collision Prevention
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RectAabb {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl RectAabb {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn intersects(&self, other: &RectAabb, padding: f32) -> bool {
        let left1 = self.x - padding;
        let right1 = self.x + self.width + padding;
        let top1 = self.y - padding;
        let bottom1 = self.y + self.height + padding;

        let left2 = other.x - padding;
        let right2 = other.x + other.width + padding;
        let top2 = other.y - padding;
        let bottom2 = other.y + other.height + padding;

        !(left1 >= right2 || right1 <= left2 || top1 >= bottom2 || bottom1 <= top2)
    }
}

/// Window Arranger Strategy to Mathematically Guarantee Non-Overlapping GUI Layouts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowArrangementStrategy {
    Cascade,
    TileHorizontal,
    TileVertical,
    TileGrid { columns: usize },
    AabbRepulsion,
}

/// Spatial Non-Overlap Solver for calculating layout positions across bounds
pub struct NonOverlapSolver;

impl NonOverlapSolver {
    /// Computes guaranteed non-overlapping positions for a set of window sizes within bounds
    pub fn compute_non_overlapping_layout(
        window_sizes: &[(f32, f32)],
        strategy: WindowArrangementStrategy,
        screen_bounds: RectAabb,
        padding: f32,
    ) -> Vec<RectAabb> {
        let n = window_sizes.len();
        if n == 0 {
            return Vec::new();
        }

        match strategy {
            WindowArrangementStrategy::TileHorizontal => {
                let total_padding = padding * (n as f32 + 1.0);
                let available_w = (screen_bounds.width - total_padding).max(100.0);
                let tile_w = available_w / n as f32;
                let tile_h = screen_bounds.height - (padding * 2.0);

                (0..n)
                    .map(|i| {
                        let x = screen_bounds.x + padding + (i as f32 * (tile_w + padding));
                        let y = screen_bounds.y + padding;
                        RectAabb::new(x, y, tile_w, tile_h)
                    })
                    .collect()
            }
            WindowArrangementStrategy::TileVertical => {
                let total_padding = padding * (n as f32 + 1.0);
                let available_h = (screen_bounds.height - total_padding).max(100.0);
                let tile_h = available_h / n as f32;
                let tile_w = screen_bounds.width - (padding * 2.0);

                (0..n)
                    .map(|i| {
                        let x = screen_bounds.x + padding;
                        let y = screen_bounds.y + padding + (i as f32 * (tile_h + padding));
                        RectAabb::new(x, y, tile_w, tile_h)
                    })
                    .collect()
            }
            WindowArrangementStrategy::TileGrid { columns } => {
                let cols = columns.max(1);
                let rows = (n + cols - 1) / cols;

                let total_pad_x = padding * (cols as f32 + 1.0);
                let total_pad_y = padding * (rows as f32 + 1.0);

                let cell_w = (screen_bounds.width - total_pad_x).max(100.0) / cols as f32;
                let cell_h = (screen_bounds.height - total_pad_y).max(100.0) / rows as f32;

                (0..n)
                    .map(|i| {
                        let col = i % cols;
                        let row = i / cols;
                        let x = screen_bounds.x + padding + (col as f32 * (cell_w + padding));
                        let y = screen_bounds.y + padding + (row as f32 * (cell_h + padding));
                        RectAabb::new(x, y, cell_w, cell_h)
                    })
                    .collect()
            }
            WindowArrangementStrategy::Cascade => {
                let offset_step = 32.0;
                let (default_w, default_h) = window_sizes.first().copied().unwrap_or((400.0, 300.0));

                (0..n)
                    .map(|i| {
                        let x = screen_bounds.x + padding + (i as f32 * offset_step);
                        let y = screen_bounds.y + padding + (i as f32 * offset_step);
                        let (w, h) = window_sizes.get(i).copied().unwrap_or((default_w, default_h));
                        RectAabb::new(x, y, w, h)
                    })
                    .collect()
            }
            WindowArrangementStrategy::AabbRepulsion => {
                let mut boxes: Vec<RectAabb> = window_sizes
                    .iter()
                    .enumerate()
                    .map(|(i, &(w, h))| {
                        let x = screen_bounds.x + padding + (i as f32 * 20.0);
                        let y = screen_bounds.y + padding + (i as f32 * 20.0);
                        RectAabb::new(x, y, w, h)
                    })
                    .collect();

                let iterations = 15;
                for _ in 0..iterations {
                    for i in 0..n {
                        for j in (i + 1)..n {
                            if boxes[i].intersects(&boxes[j], padding) {
                                let c1_x = boxes[i].x + boxes[i].width / 2.0;
                                let c1_y = boxes[i].y + boxes[i].height / 2.0;
                                let c2_x = boxes[j].x + boxes[j].width / 2.0;
                                let c2_y = boxes[j].y + boxes[j].height / 2.0;

                                let mut dx = c2_x - c1_x;
                                let dy = c2_y - c1_y;
                                if dx.abs() < 1e-4 && dy.abs() < 1e-4 {
                                    dx = 1.0;
                                }

                                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                                let push_dist = 12.0;
                                let push_x = (dx / dist) * push_dist;
                                let push_y = (dy / dist) * push_dist;

                                boxes[i].x -= push_x * 0.5;
                                boxes[i].y -= push_y * 0.5;
                                boxes[j].x += push_x * 0.5;
                                boxes[j].y += push_y * 0.5;

                                boxes[i].x = boxes[i]
                                    .x
                                    .clamp(screen_bounds.x, screen_bounds.x + screen_bounds.width - boxes[i].width);
                                boxes[i].y = boxes[i]
                                    .y
                                    .clamp(screen_bounds.y, screen_bounds.y + screen_bounds.height - boxes[i].height);
                                boxes[j].x = boxes[j]
                                    .x
                                    .clamp(screen_bounds.x, screen_bounds.x + screen_bounds.width - boxes[j].width);
                                boxes[j].y = boxes[j]
                                    .y
                                    .clamp(screen_bounds.y, screen_bounds.y + screen_bounds.height - boxes[j].height);
                            }
                        }
                    }
                }
                boxes
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_aabb_intersection() {
        let r1 = RectAabb::new(0.0, 0.0, 100.0, 100.0);
        let r2 = RectAabb::new(50.0, 50.0, 100.0, 100.0);
        let r3 = RectAabb::new(200.0, 200.0, 50.0, 50.0);

        assert!(r1.intersects(&r2, 0.0));
        assert!(!r1.intersects(&r3, 0.0));
    }

    #[test]
    fn test_tile_horizontal_layout() {
        let sizes = vec![(200.0, 150.0), (200.0, 150.0)];
        let bounds = RectAabb::new(0.0, 0.0, 1000.0, 600.0);
        let layout = NonOverlapSolver::compute_non_overlapping_layout(
            &sizes,
            WindowArrangementStrategy::TileHorizontal,
            bounds,
            10.0,
        );

        assert_eq!(layout.len(), 2);
        assert!(!layout[0].intersects(&layout[1], 0.0));
    }

    #[test]
    fn test_tile_vertical_layout() {
        let sizes = vec![(200.0, 150.0), (200.0, 150.0)];
        let bounds = RectAabb::new(0.0, 0.0, 1000.0, 600.0);
        let layout = NonOverlapSolver::compute_non_overlapping_layout(
            &sizes,
            WindowArrangementStrategy::TileVertical,
            bounds,
            10.0,
        );

        assert_eq!(layout.len(), 2);
        assert!(!layout[0].intersects(&layout[1], 0.0));
    }
}