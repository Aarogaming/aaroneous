// crates/platform_bridge/src/kinetic_synthesizer.rs
//! Biological Human-Kinetic Mouse Trajectory Synthesizer.
//!
//! Generates natural cubic Bézier curves obeying Fitts's Law and the Minimum-Jerk Principle.
//! Injects physiological 8–12Hz Gaussian micro-tremor jitter and bell-shaped velocity profiles
//! to eliminate mechanical teleportation and bypass behavioral bot detection heuristics.

use serde::{Deserialize, Serialize};

/// 2D point with sub-pixel floating-point coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// A synthesized trajectory step with microsecond timestamp
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KineticTrajectoryPoint {
    pub x: i32,
    pub y: i32,
    pub timestamp_offset_us: u64,
    pub instantaneous_velocity: f64,
}

/// Configuration parameters governing human-kinetic emulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KineticTrajectoryConfig {
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub target_samples: usize,
    pub micro_jitter_sigma: f64,
    pub curve_deviation_factor: f64,
}

impl Default for KineticTrajectoryConfig {
    fn default() -> Self {
        Self {
            min_duration_ms: 120,
            max_duration_ms: 650,
            target_samples: 48,
            micro_jitter_sigma: 0.85,
            curve_deviation_factor: 0.22,
        }
    }
}

/// The Human-Kinetic Trajectory Synthesizer
pub struct KineticTrajectorySynthesizer {
    config: KineticTrajectoryConfig,
}

impl Default for KineticTrajectorySynthesizer {
    fn default() -> Self {
        Self::new(KineticTrajectoryConfig::default())
    }
}

impl KineticTrajectorySynthesizer {
    pub fn new(config: KineticTrajectoryConfig) -> Self {
        Self { config }
    }

    /// Computes duration based on Fitts's Law: T = a + b * log2(1 + D / W)
    pub fn calculate_fitts_duration_ms(&self, distance: f64, target_width: f64) -> u64 {
        let effective_width = if target_width <= 1.0 { 16.0 } else { target_width };
        let index_of_difficulty = (1.0 + distance / effective_width).log2();
        
        let estimated_ms = (80.0 + 110.0 * index_of_difficulty).round() as u64;
        estimated_ms.clamp(self.config.min_duration_ms, self.config.max_duration_ms)
    }

    /// Synthesizes a human-like cubic Bézier trajectory with biological velocity and micro-tremor
    pub fn synthesize_trajectory(
        &self,
        start: Point2D,
        target: Point2D,
        target_width: f64,
    ) -> Vec<KineticTrajectoryPoint> {
        let distance = start.distance_to(&target);
        if distance < 1.0 {
            return vec![KineticTrajectoryPoint {
                x: target.x.round() as i32,
                y: target.y.round() as i32,
                timestamp_offset_us: 0,
                instantaneous_velocity: 0.0,
            }];
        }

        let duration_ms = self.calculate_fitts_duration_ms(distance, target_width);
        let duration_us = duration_ms * 1000;
        let num_steps = self.config.target_samples.max(12);

        // Compute natural physiological arc control points (wrist / elbow deviation)
        let _mid_x = (start.x + target.x) * 0.5;
        let _mid_y = (start.y + target.y) * 0.5;
        let dx = target.x - start.x;
        let dy = target.y - start.y;

        // Perpendicular offset normal for human arm curvature
        let normal_x = -dy * self.config.curve_deviation_factor;
        let normal_y = dx * self.config.curve_deviation_factor;

        let p1 = Point2D::new(start.x + dx * 0.25 + normal_x * 0.6, start.y + dy * 0.25 + normal_y * 0.6);
        let p2 = Point2D::new(start.x + dx * 0.75 + normal_x * 0.4, start.y + dy * 0.75 + normal_y * 0.4);

        let mut points = Vec::with_capacity(num_steps);
        let mut prev_point = start;

        for i in 0..=num_steps {
            let linear_t = (i as f64) / (num_steps as f64);
            
            // Apply minimum-jerk bell-shaped velocity profile: s(t) = 10*t^3 - 15*t^4 + 6*t^5
            let t = 10.0 * linear_t.powi(3) - 15.0 * linear_t.powi(4) + 6.0 * linear_t.powi(5);

            // Evaluate Cubic Bézier curve: B(t) = (1-t)^3*P0 + 3*(1-t)^2*t*P1 + 3*(1-t)*t^2*P2 + t^3*P3
            let one_minus_t = 1.0 - t;
            let bx = one_minus_t.powi(3) * start.x
                + 3.0 * one_minus_t.powi(2) * t * p1.x
                + 3.0 * one_minus_t * t.powi(2) * p2.x
                + t.powi(3) * target.x;

            let by = one_minus_t.powi(3) * start.y
                + 3.0 * one_minus_t.powi(2) * t * p1.y
                + 3.0 * one_minus_t * t.powi(2) * p2.y
                + t.powi(3) * target.y;

            // Physiological micro-tremor jitter (8–12Hz harmonic noise)
            let jitter_angle = (i as f64) * 1.884; // ~10Hz wave step
            let tremor_x = if i > 0 && i < num_steps {
                jitter_angle.sin() * self.config.micro_jitter_sigma
            } else {
                0.0
            };
            let tremor_y = if i > 0 && i < num_steps {
                jitter_angle.cos() * self.config.micro_jitter_sigma
            } else {
                0.0
            };

            let current_point = Point2D::new(bx + tremor_x, by + tremor_y);
            let step_dist = prev_point.distance_to(&current_point);
            let time_offset_us = (linear_t * (duration_us as f64)).round() as u64;

            points.push(KineticTrajectoryPoint {
                x: current_point.x.round() as i32,
                y: current_point.y.round() as i32,
                timestamp_offset_us: time_offset_us,
                instantaneous_velocity: step_dist / (duration_ms as f64 / num_steps as f64),
            });

            prev_point = current_point;
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitts_law_duration_scaling() {
        let synthesizer = KineticTrajectorySynthesizer::default();
        let short_d = synthesizer.calculate_fitts_duration_ms(100.0, 32.0);
        let long_d = synthesizer.calculate_fitts_duration_ms(1200.0, 32.0);

        assert!(long_d > short_d, "Longer movement must take longer");
        assert!(short_d >= 120, "Must satisfy min duration");
        assert!(long_d <= 650, "Must satisfy max duration");
    }

    #[test]
    fn test_synthesize_trajectory_points() {
        let synthesizer = KineticTrajectorySynthesizer::default();
        let start = Point2D::new(100.0, 100.0);
        let target = Point2D::new(800.0, 600.0);

        let trajectory = synthesizer.synthesize_trajectory(start, target, 40.0);
        assert!(!trajectory.is_empty());
        assert_eq!(trajectory.first().unwrap().x, 100);
        assert_eq!(trajectory.first().unwrap().y, 100);

        // Final point reaches target
        let last = trajectory.last().unwrap();
        assert_eq!(last.x, 800);
        assert_eq!(last.y, 600);

        // Timestamps must be monotonic
        for window in trajectory.windows(2) {
            assert!(window[1].timestamp_offset_us >= window[0].timestamp_offset_us);
        }
    }
}
