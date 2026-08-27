//! crates/desktop_emulator/src/sensory_motor_loop.rs
//! Closed-Loop Live Multimodal Sensory-Motor Pipeline.
//!
//! Connects:
//! 1. Epigenetic Vision Gater (16x16 sectors motion filter, ~97µs)
//! 2. Spatial-Semantic Latent Projector (R^256 latent vector)
//! 3. Sentinel Deep SVDD Latent Guardrail (Safe hypersphere audit & orthogonal projection, <2µs)
//! 4. Multi-Headed Action Decoder (R^256 -> Discrete MachineOpcode + 4D Coords, <10µs)
//! 5. Isolated Desktop Sandbox Actuator (Isolated Win32 Virtual Desktop / Mock Emulation)

use std::time::Instant;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use compute::isolated_desktop::IsolatedDesktop;
use compute::latent_guardrail::{GUARDRAIL_DIM, LatentAuditVerdict, SafeHypersphereManifold};
use compute::machine_native::MachineOpcode;
use compute::si_decoder::{ActionDecoder, DecodedActionCommand};

use crate::epigenetic_vision::{EpigeneticGatingResult, EpigeneticVisionGater};
use crate::mock::MockMarionette;
use crate::traits::{HidAction, HidCommand, MarionetteHost};

/// Report generated for each continuous sensory-motor execution cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryMotorCycleReport {
    pub frame_idx: usize,
    pub active_sectors: usize,
    pub compute_savings_pct: f32,
    pub gating_latency_us: u64,
    pub svdd_distance: f32,
    pub svdd_radius: f32,
    pub is_safe: bool,
    pub was_projected: bool,
    pub audit_duration_ns: u64,
    pub decoded_action: DecodedActionCommand,
    pub hid_actions: Vec<HidAction>,
    pub total_cycle_latency_us: u64,
}

/// The Closed-Loop Sensory-Motor Pipeline Engine
pub struct SensoryMotorPipeline {
    pub gater: EpigeneticVisionGater,
    pub guardrail: SafeHypersphereManifold,
    pub decoder: ActionDecoder,
    pub ghost_desktop: IsolatedDesktop,
    pub host: MockMarionette,
    pub frame_counter: usize,
}

impl Default for SensoryMotorPipeline {
    fn default() -> Self {
        Self::new("Aaroneous_Ghost_Sandbox")
    }
}

impl SensoryMotorPipeline {
    /// Initializes a new Closed-Loop Sensory-Motor Pipeline
    pub fn new(desktop_name: &str) -> Self {
        let gater = EpigeneticVisionGater::default();
        let mut guardrail = SafeHypersphereManifold::new(12.0); // Safe radius R = 12.0
        guardrail.fit_from_golden_states(&[
            vec![0.05f32; GUARDRAIL_DIM],
            vec![0.10f32; GUARDRAIL_DIM],
            vec![-0.05f32; GUARDRAIL_DIM],
        ]);

        let decoder = ActionDecoder::new(16, 8);
        let ghost_desktop = IsolatedDesktop::forge(desktop_name).unwrap_or_else(|_| {
            IsolatedDesktop {
                name: desktop_name.to_string(),
                handle_id: 0,
                is_isolated: false,
            }
        });

        Self {
            gater,
            guardrail,
            decoder,
            ghost_desktop,
            host: MockMarionette::new(),
            frame_counter: 0,
        }
    }

    /// Projects 16x16 epigenetic sector activations into an R^256 spatial-semantic latent intent vector
    pub fn project_latent_intent(&self, gated: &EpigeneticGatingResult, raw_frame: &[f32]) -> Vec<f32> {
        let mut intent = vec![0.0f32; GUARDRAIL_DIM];

        // 1. Ingest 256 sector saliency values directly
        for (i, &active) in gated.bool_mask.iter().enumerate() {
            if active {
                // Compute sector mean luminance from raw frame
                let sec_x = i % 16;
                let sec_y = i / 16;
                let mut sum = 0.0f32;
                for py in 0..8 {
                    for px in 0..8 {
                        let idx = (sec_y * 8 + py) * 128 + (sec_x * 8 + px);
                        if idx < raw_frame.len() {
                            sum += raw_frame[idx];
                        }
                    }
                }
                intent[i] = (sum / 64.0) * 2.0 - 1.0; // Normalized -1.0 to 1.0
            } else {
                intent[i] = 0.0;
            }
        }

        // 2. Add temporal bias based on active sectors count
        let activity_ratio = gated.active_sectors_count as f32 / 256.0;
        intent[0] += activity_ratio * 0.5;

        intent
    }

    /// Translates a decoded action command into peripheral HID commands
    pub fn translate_to_hid(&self, cmd: &DecodedActionCommand) -> Vec<HidAction> {
        let mut actions = Vec::new();

        let dx = (cmd.spatial_coords[0] * 50.0) as i32;
        let dy = (cmd.spatial_coords[1] * 50.0) as i32;

        actions.push(HidAction::MouseMove { delta_x: dx, delta_y: dy });

        match cmd.opcode {
            MachineOpcode::Call { .. } => {
                actions.push(HidAction::LeftClick);
            }
            MachineOpcode::BranchIf { .. } => {
                actions.push(HidAction::RightClick);
            }
            MachineOpcode::Return { .. } => {
                actions.push(HidAction::Scroll { delta: 120 });
            }
            _ => {
                actions.push(HidAction::LeftClick);
            }
        }

        actions
    }

    /// Executes one full sensory-to-motor closed-loop cycle
    pub async fn step_cycle(&mut self, raw_frame: &[f32]) -> Result<SensoryMotorCycleReport> {
        let start = Instant::now();
        self.frame_counter += 1;

        // Stage 1: Epigenetic Visual Motion Gating (16x16 grid, 256 sectors)
        let gated = self.gater.process_frame(raw_frame);

        // Stage 2: Spatial-Semantic Latent Projection (R^256)
        let raw_latent = self.project_latent_intent(&gated, raw_frame);

        // Stage 3: Sentinel Deep SVDD Latent Hypersphere Guardrail Audit
        let audit: LatentAuditVerdict = self.guardrail.audit_candidate_action(&raw_latent, true);
        let safe_latent = if audit.was_projected {
            audit.snapped_vector.clone().unwrap_or(raw_latent)
        } else {
            raw_latent
        };

        // Stage 4: Multi-Headed Action Decoder
        let decoded = self.decoder.decode(&safe_latent);

        // Stage 5: Motor Action Translation & Isolated Desktop Actuation
        let hid_actions = self.translate_to_hid(&decoded);
        let hid_command = HidCommand {
            actions: hid_actions.clone(),
            sequence_id: self.frame_counter as u64,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        };

        self.host.inject_hid_event(hid_command).await?;

        let total_cycle_latency_us = start.elapsed().as_micros() as u64;

        info!(
            target: "marionette::sensory_motor",
            frame = self.frame_counter,
            active_sectors = gated.active_sectors_count,
            compute_savings = gated.compute_savings_pct,
            svdd_distance = audit.distance_to_centroid,
            is_safe = audit.is_safe,
            total_latency_us = total_cycle_latency_us,
            "Completed closed-loop sensory-motor step"
        );

        Ok(SensoryMotorCycleReport {
            frame_idx: self.frame_counter,
            active_sectors: gated.active_sectors_count,
            compute_savings_pct: gated.compute_savings_pct,
            gating_latency_us: gated.duration_us,
            svdd_distance: audit.distance_to_centroid,
            svdd_radius: audit.safety_radius,
            is_safe: audit.is_safe,
            was_projected: audit.was_projected,
            audit_duration_ns: audit.audit_duration_ns,
            decoded_action: decoded,
            hid_actions,
            total_cycle_latency_us,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sensory_motor_closed_loop() {
        let mut pipeline = SensoryMotorPipeline::new("Aaroneous_Test_Sandbox");

        // Frame 1: Blank frame (initial baseline)
        let frame1 = vec![0.0f32; 128 * 128];
        let report1 = pipeline.step_cycle(&frame1).await.unwrap();
        assert_eq!(report1.frame_idx, 1);
        assert!(!report1.hid_actions.is_empty());

        // Step 4 frames with single moving 8x8 block to allow initial full-frame hysteresis to decay
        let mut frame_motion = vec![0.0f32; 128 * 128];
        for y in 60..68 {
            for x in 60..68 {
                frame_motion[y * 128 + x] = 0.95;
            }
        }

        let _ = pipeline.step_cycle(&frame_motion).await.unwrap();
        let _ = pipeline.step_cycle(&frame_motion).await.unwrap();
        let _ = pipeline.step_cycle(&frame_motion).await.unwrap();
        let report_final = pipeline.step_cycle(&frame_motion).await.unwrap();

        assert_eq!(report_final.frame_idx, 5);
        assert!(report_final.compute_savings_pct > 80.0, "Expected >80% savings, got {:.1}%", report_final.compute_savings_pct);
        assert!(report_final.total_cycle_latency_us < 50_000); // Sub-50ms execution
    }
}
