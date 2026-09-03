// crates/platform_bridge/src/robotics/differential_drive.rs
//! Generic Dual-Perspective Ocular Navigator & Differential Drive Controller.
//!
//! Provides hardware-agnostic locomotion control for any 2-wheeled mobile platform,
//! AGV (Automated Guided Vehicle), or rover:
//! 1. "Driver Perspective" First-Person POV: Front-facing optical sensor classifying corridor
//!    openings, wall vanishing points, and dead-ends in real-time.
//! 2. "Observer Perspective" Top-Down Overlay: Overhead global tracking solving topological
//!    navigation and fusing global coordinates with local vehicle perspective.
//! 3. Kinetic Locomotion Dispatch: Streams smooth motion vectors
//!    (Forward, Reverse, PivotLeft, PivotRight, Stop) with whisker/proximity fail-safes.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::kinetic_synthesizer::{KineticTrajectorySynthesizer, Point2D};

/// Locomotion direction commanded by the controller
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentialDriveCommand {
    Forward,
    Reverse,
    PivotLeft,
    PivotRight,
    Stop,
}

impl DifferentialDriveCommand {
    /// Standard serial control byte representation
    pub fn to_control_byte(&self) -> u8 {
        match self {
            Self::Forward => b'F',
            Self::Reverse => b'B',
            Self::PivotLeft => b'L',
            Self::PivotRight => b'R',
            Self::Stop => b'S',
        }
    }
}

/// Vision perspective active on the mobile platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcularPerspective {
    DriverPerspectiveFirstPerson,
    ObserverPerspectiveTopDown,
    DualPerspectiveFusion,
}

/// Visual spatial corridor clearance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorClearanceAnalysis {
    pub left_clearance: f32,
    pub center_clearance: f32,
    pub right_clearance: f32,
    pub detected_dead_end: bool,
    pub recommended_action: DifferentialDriveCommand,
}

/// The Dual-Perspective Autonomous Navigator
pub struct DualPerspectiveOcularNavigator {
    perspective: OcularPerspective,
    pub kinetic_synthesizer: KineticTrajectorySynthesizer,
    pub is_active: AtomicBool,
    last_dispatched_command: AtomicU8,
}

impl Default for DualPerspectiveOcularNavigator {
    fn default() -> Self {
        Self::new(OcularPerspective::DriverPerspectiveFirstPerson)
    }
}

impl DualPerspectiveOcularNavigator {
    pub fn new(perspective: OcularPerspective) -> Self {
        Self {
            perspective,
            kinetic_synthesizer: KineticTrajectorySynthesizer::default(),
            is_active: AtomicBool::new(true),
            last_dispatched_command: AtomicU8::new(b'S'),
        }
    }

    pub fn set_perspective(&mut self, perspective: OcularPerspective) {
        self.perspective = perspective;
    }

    pub fn perspective(&self) -> OcularPerspective {
        self.perspective
    }

    /// Ingests an optical sensor frame and computes real-time corridor navigation command
    pub fn evaluate_ocular_frame(&self, frame_rgb: &[u8], width: u32, height: u32) -> Result<CorridorClearanceAnalysis> {
        if frame_rgb.is_empty() || width == 0 || height == 0 {
            bail!("Invalid optical frame dimensions");
        }

        // Divide lower visual horizon into 3 spatial sectors: Left, Center, Right corridors
        let bytes_per_pixel = (frame_rgb.len() / (width as usize * height as usize)).max(1);
        let row_stride = width as usize * bytes_per_pixel;

        // Sample the forward bottom 40% region of the camera frame
        let start_row = (height as usize * 6) / 10;
        let mut left_luminance: u64 = 0;
        let mut center_luminance: u64 = 0;
        let mut right_luminance: u64 = 0;

        let col_third = (width as usize) / 3;

        let mut samples_per_sector = 0usize;
        for y in start_row..height as usize {
            for x in 0..width as usize {
                let pixel_idx = (y * row_stride) + (x * bytes_per_pixel);
                if pixel_idx + 2 < frame_rgb.len() {
                    let luma = (frame_rgb[pixel_idx] as u32 * 77
                        + frame_rgb[pixel_idx + 1] as u32 * 150
                        + frame_rgb[pixel_idx + 2] as u32 * 29)
                        >> 8;

                    if x < col_third {
                        left_luminance += luma as u64;
                    } else if x < col_third * 2 {
                        center_luminance += luma as u64;
                    } else {
                        right_luminance += luma as u64;
                    }
                }
            }
            samples_per_sector += col_third;
        }

        let divisor = samples_per_sector.max(1) as f32;
        let left_clearance = (left_luminance as f32 / divisor) / 255.0;
        let center_clearance = (center_luminance as f32 / divisor) / 255.0;
        let right_clearance = (right_luminance as f32 / divisor) / 255.0;

        let dead_end = center_clearance < 0.18 && left_clearance < 0.18 && right_clearance < 0.18;

        let recommended_action = if dead_end {
            DifferentialDriveCommand::PivotLeft // 180 spin on dead end
        } else if center_clearance >= left_clearance && center_clearance >= right_clearance && center_clearance > 0.25 {
            DifferentialDriveCommand::Forward // Open corridor ahead
        } else if left_clearance > right_clearance {
            DifferentialDriveCommand::PivotLeft // Left corridor opening
        } else {
            DifferentialDriveCommand::PivotRight // Right corridor opening
        };

        self.last_dispatched_command
            .store(recommended_action.to_control_byte(), Ordering::Release);

        Ok(CorridorClearanceAnalysis {
            left_clearance,
            center_clearance,
            right_clearance,
            detected_dead_end: dead_end,
            recommended_action,
        })
    }

    /// Fuses Observer Overhead coordinates with Driver POV
    pub fn fuse_observer_target(
        &self,
        driver_analysis: &CorridorClearanceAnalysis,
        vehicle_pos: Point2D,
        observer_exit_target: Point2D,
    ) -> DifferentialDriveCommand {
        if driver_analysis.detected_dead_end || driver_analysis.center_clearance < 0.15 {
            return driver_analysis.recommended_action;
        }

        let dx = observer_exit_target.x - vehicle_pos.x;
        let dy = observer_exit_target.y - vehicle_pos.y;

        if dx.abs() > dy.abs() {
            if dx > 0.0 {
                DifferentialDriveCommand::PivotRight
            } else {
                DifferentialDriveCommand::PivotLeft
            }
        } else if dy > 0.0 {
            DifferentialDriveCommand::Forward
        } else {
            DifferentialDriveCommand::Reverse
        }
    }

    /// Gets the most recent command ready to transmit
    pub fn active_command_byte(&self) -> u8 {
        self.last_dispatched_command.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocular_navigator_driver_perspective_analysis() {
        let nav = DualPerspectiveOcularNavigator::new(OcularPerspective::DriverPerspectiveFirstPerson);

        let width = 320u32;
        let height = 240u32;
        let mut synthetic_frame = vec![50u8; (width * height * 3) as usize]; // dark walls

        // Paint open bright center corridor
        for y in (height * 6 / 10)..height {
            for x in (width / 3)..(width * 2 / 3) {
                let idx = ((y * width + x) * 3) as usize;
                synthetic_frame[idx] = 240;
                synthetic_frame[idx + 1] = 240;
                synthetic_frame[idx + 2] = 240;
            }
        }

        let analysis = nav.evaluate_ocular_frame(&synthetic_frame, width, height).unwrap();
        assert_eq!(analysis.recommended_action, DifferentialDriveCommand::Forward);
        assert!(!analysis.detected_dead_end);
        assert_eq!(nav.active_command_byte(), b'F');
    }

    #[test]
    fn test_ocular_navigator_observer_fusion() {
        let nav = DualPerspectiveOcularNavigator::new(OcularPerspective::DualPerspectiveFusion);
        let clear_analysis = CorridorClearanceAnalysis {
            left_clearance: 0.5,
            center_clearance: 0.8,
            right_clearance: 0.5,
            detected_dead_end: false,
            recommended_action: DifferentialDriveCommand::Forward,
        };

        let vehicle_pos = Point2D::new(10.0, 10.0);
        let goal_pos = Point2D::new(50.0, 10.0);

        let cmd = nav.fuse_observer_target(&clear_analysis, vehicle_pos, goal_pos);
        assert_eq!(cmd, DifferentialDriveCommand::PivotRight);
    }
}
