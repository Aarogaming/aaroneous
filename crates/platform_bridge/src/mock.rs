//! mock.rs
//! Safe mock implementation of MarionetteHost for development, testing, and sandboxed execution.
//! Guaranteed to never move the host mouse cursor or inject OS keyboard strokes.

use anyhow::Result;
use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

use crate::traits::{HidCommand, MarionetteHost, ProbingTrace, VisualObservation};

/// Mock Marionette backend: 100% sandboxed and memory-isolated
#[derive(Debug, Clone)]
pub struct MockMarionette {
    pub frame_counter: u64,
    pub command_history: Vec<HidCommand>,
    pub trace_history: Vec<ProbingTrace>,
}

impl Default for MockMarionette {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMarionette {
    pub fn new() -> Self {
        Self {
            frame_counter: 0,
            command_history: Vec::new(),
            trace_history: Vec::new(),
        }
    }

    fn now_us() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

#[async_trait]
impl MarionetteHost for MockMarionette {
    async fn pull_visual_perception(&mut self) -> Result<VisualObservation> {
        self.frame_counter += 1;
        
        // Generate a 128x128 synthetic test pattern
        let mut grid = vec![0.0f32; 128 * 128];
        let phase = (self.frame_counter % 100) as f32 / 100.0;
        
        for y in 0..128 {
            for x in 0..128 {
                let val = ((x as f32 / 128.0) + (y as f32 / 128.0) + phase) % 1.0;
                grid[y * 128 + x] = val;
            }
        }

        debug!(target: "marionette::mock", frame = self.frame_counter, "Generated synthetic visual frame");

        Ok(VisualObservation {
            grid,
            width: 128,
            height: 128,
            timestamp_us: Self::now_us(),
            active_sectors_count: 256,
            compute_savings_pct: 0.0,
            gating_latency_us: 0,
        })
    }

    async fn pull_visual_perception_gated(&mut self, gate_mask: &[bool; 256]) -> Result<VisualObservation> {
        let mut observation = self.pull_visual_perception().await?;
        
        // Apply 16x16 sector gate mask
        let sector_size = 8;
        let sectors_per_row = 16;
        let mut active_count = 0usize;

        for (sector_idx, &active) in gate_mask.iter().enumerate() {
            if active {
                active_count += 1;
            } else {
                let sector_y = sector_idx / sectors_per_row;
                let sector_x = sector_idx % sectors_per_row;
                let y_start = sector_y * sector_size;
                let x_start = sector_x * sector_size;

                for dy in 0..sector_size {
                    for dx in 0..sector_size {
                        let y = y_start + dy;
                        let x = x_start + dx;
                        if y < 128 && x < 128 {
                            observation.grid[y * 128 + x] = 0.0;
                        }
                    }
                }
            }
        }

        observation.active_sectors_count = active_count;
        observation.compute_savings_pct = (1.0 - (active_count as f32 / 256.0)) * 100.0;
        observation.gating_latency_us = 12; // Simulated fast pass
        Ok(observation)
    }

    async fn inject_hid_event(&mut self, command: HidCommand) -> Result<()> {
        info!(
            target: "marionette::mock", 
            seq = command.sequence_id, 
            action_count = command.actions.len(), 
            "Recorded sandboxed motor intent (Zero OS side-effects)"
        );
        self.command_history.push(command);
        Ok(())
    }

    async fn log_probe_trace(&mut self, trace: ProbingTrace) -> Result<()> {
        info!(
            target: "marionette::mock",
            process = %trace.target_process,
            event = %trace.event_type,
            "Logged process probe trace"
        );
        self.trace_history.push(trace);
        Ok(())
    }

    fn is_live_emulation_active(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::HidAction;

    #[tokio::test]
    async fn test_mock_marionette_visual_capture() {
        let mut host = MockMarionette::new();
        let frame = host.pull_visual_perception().await.unwrap();
        assert_eq!(frame.grid.len(), 128 * 128);
        assert_eq!(frame.width, 128);
        assert_eq!(frame.height, 128);
    }

    #[tokio::test]
    async fn test_mock_marionette_hid_sandboxing() {
        let mut host = MockMarionette::new();
        let cmd = HidCommand {
            actions: vec![HidAction::MouseMove { delta_x: 100, delta_y: 200 }],
            sequence_id: 1,
            timestamp_us: MockMarionette::now_us(),
        };

        // Guaranteed to not move the physical mouse
        host.inject_hid_event(cmd).await.unwrap();
        assert_eq!(host.command_history.len(), 1);
        assert!(!host.is_live_emulation_active());
    }
}
