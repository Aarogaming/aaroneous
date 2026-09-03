// crates/platform_bridge/src/robotics/boebot.rs
//! Dual-Ocular Autonomous BOE-Bot Navigator & Wireless Bridge.
//!
//! Supports:
//! 1. "Driver's Seat" First-Person POV: Front-facing mobile/mini camera detecting corridor
//!    openings, wall vanishing points, and dead-ends in real-time.
//! 2. Optional "Satellite Eye" Top-Down Overlay: Overhead global camera solving global
//!    maze topology and fusing coordinates with local driver perspective.
//! 3. High-Speed Wireless Serial Bridge: Streams Fitts's Law kinetic motion commands
//!    ('F', 'B', 'L', 'R', 'S') and receives whisker collision telemetry.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::kinetic_synthesizer::{KineticTrajectorySynthesizer, Point2D};

/// Navigation direction commanded by Aaroneous
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoeBotCommand {
    Forward,
    Reverse,
    PivotLeft,
    PivotRight,
    Stop,
}

impl BoeBotCommand {
    /// Serial byte sent over Bluetooth/Wi-Fi to BS2
    pub fn to_serial_byte(&self) -> u8 {
        match self {
            Self::Forward => b'F',
            Self::Reverse => b'B',
            Self::PivotLeft => b'L',
            Self::PivotRight => b'R',
            Self::Stop => b'S',
        }
    }
}

/// Vision perspective active on the robot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcularPerspective {
    DriversSeatFirstPerson,
    SatelliteTopDown,
    DualSensorFusion,
}

/// Visual corridor classification from driver's seat camera
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorCorridorAnalysis {
    pub left_clearance: f32,
    pub center_clearance: f32,
    pub right_clearance: f32,
    pub detected_dead_end: bool,
    pub recommended_action: BoeBotCommand,
}

/// The Dual-Ocular BOE-Bot Autonomous Navigator
pub struct BoeBotOcularNavigator {
    perspective: OcularPerspective,
    pub kinetic_synthesizer: KineticTrajectorySynthesizer,
    pub is_wireless_active: AtomicBool,
    last_dispatched_command: AtomicU8,
}

impl Default for BoeBotOcularNavigator {
    fn default() -> Self {
        Self::new(OcularPerspective::DriversSeatFirstPerson)
    }
}

impl BoeBotOcularNavigator {
    pub fn new(perspective: OcularPerspective) -> Self {
        Self {
            perspective,
            kinetic_synthesizer: KineticTrajectorySynthesizer::default(),
            is_wireless_active: AtomicBool::new(true),
            last_dispatched_command: AtomicU8::new(b'S'),
        }
    }

    pub fn set_perspective(&mut self, perspective: OcularPerspective) {
        self.perspective = perspective;
    }

    pub fn perspective(&self) -> OcularPerspective {
        self.perspective
    }

    /// Ingests a camera frame (Driver's Seat POV or Satellite) and computes real-time maze navigation command
    pub fn evaluate_ocular_frame(&self, frame_rgb: &[u8], width: u32, height: u32) -> Result<CorridorCorridorAnalysis> {
        if frame_rgb.is_empty() || width == 0 || height == 0 {
            bail!("Invalid camera frame dimensions");
        }

        // Divide lower visual horizon into 3 spatial sectors: Left, Center, Right corridors
        let bytes_per_pixel = (frame_rgb.len() / (width as usize * height as usize)).max(1);
        let row_stride = width as usize * bytes_per_pixel;

        // Sample the forward bottom 40% region of the camera frame (the corridor floor & walls)
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
                    // Simple grayscale luma: Y = 0.299R + 0.587G + 0.114B
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

        // Wall proximity threshold: lower luma / darker obstacle threshold
        let dead_end = center_clearance < 0.18 && left_clearance < 0.18 && right_clearance < 0.18;

        let recommended_action = if dead_end {
            BoeBotCommand::PivotLeft // 180 spin on dead end
        } else if center_clearance >= left_clearance && center_clearance >= right_clearance && center_clearance > 0.25 {
            BoeBotCommand::Forward // Open corridor ahead
        } else if left_clearance > right_clearance {
            BoeBotCommand::PivotLeft // Left corridor opening
        } else {
            BoeBotCommand::PivotRight // Right corridor opening
        };

        self.last_dispatched_command
            .store(recommended_action.to_serial_byte(), Ordering::Release);

        Ok(CorridorCorridorAnalysis {
            left_clearance,
            center_clearance,
            right_clearance,
            detected_dead_end: dead_end,
            recommended_action,
        })
    }

    /// Fuses Satellite Overhead coordinates with Driver's Seat POV
    pub fn fuse_satellite_target(
        &self,
        driver_analysis: &CorridorCorridorAnalysis,
        robot_pos: Point2D,
        satellite_exit_target: Point2D,
    ) -> BoeBotCommand {
        // If driver POV detects imminent collision, prioritize driver reflex
        if driver_analysis.detected_dead_end || driver_analysis.center_clearance < 0.15 {
            return driver_analysis.recommended_action;
        }

        // Otherwise, steer smoothly towards satellite global waypoint
        let dx = satellite_exit_target.x - robot_pos.x;
        let dy = satellite_exit_target.y - robot_pos.y;

        if dx.abs() > dy.abs() {
            if dx > 0.0 {
                BoeBotCommand::PivotRight
            } else {
                BoeBotCommand::PivotLeft
            }
        } else if dy > 0.0 {
            BoeBotCommand::Forward
        } else {
            BoeBotCommand::Reverse
        }
    }

    /// Gets the most recent command ready to transmit wirelessly
    pub fn active_command_byte(&self) -> u8 {
        self.last_dispatched_command.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocular_navigator_drivers_seat_analysis() {
        let nav = BoeBotOcularNavigator::new(OcularPerspective::DriversSeatFirstPerson);

        // Generate synthetic camera frame (320x240 RGB) with clear center corridor
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
        assert_eq!(analysis.recommended_action, BoeBotCommand::Forward);
        assert!(!analysis.detected_dead_end);
        assert_eq!(nav.active_command_byte(), b'F');
    }

    #[test]
    fn test_ocular_navigator_satellite_fusion() {
        let nav = BoeBotOcularNavigator::new(OcularPerspective::DualSensorFusion);
        let clear_analysis = CorridorCorridorAnalysis {
            left_clearance: 0.5,
            center_clearance: 0.8,
            right_clearance: 0.5,
            detected_dead_end: false,
            recommended_action: BoeBotCommand::Forward,
        };

        let robot_pos = Point2D::new(10.0, 10.0);
        let goal_pos = Point2D::new(50.0, 10.0); // Exit is to the right

        let cmd = nav.fuse_satellite_target(&clear_analysis, robot_pos, goal_pos);
        assert_eq!(cmd, BoeBotCommand::PivotRight);
    }
}
