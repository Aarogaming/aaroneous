//! crates/marionette/src/game_player.rs
//! Autonomous Game Playthrough, User Emulation, and Demonstration Learning Engine.
//! Integrates vision-based action policies, reinforcement learning reward calculation,
//! and hardware HID injection with fail-safe safety killswitches.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::event_recorder::{RecordedInputEvent, SessionRecording};
use crate::traits::{HidAction, HidCommand, VisualObservation};

/// State of the Autonomous Playthrough Engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlaythroughState {
    Idle,
    Recording { session_id: String, frames_recorded: usize },
    AutonomousPlaying { steps_executed: usize, cumulative_reward: f32 },
    Paused,
    EmergencyHalted { reason: String },
}

/// A discretized game/task policy action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GamePolicyAction {
    Click,
    KeyPress { key_code: u16 },
    KeyRelease { key_code: u16 },
    MouseMove { delta_x: i32, delta_y: i32 },
    WaitMs { duration: u64 },
}

/// Autonomous Game & Task Emulation Agent
pub struct AutonomousGameAgent {
    pub state: PlaythroughState,
    pub active_recording: Option<SessionRecording>,
    pub learned_demonstrations: Vec<SessionRecording>,
    pub last_frame_sum: f32,
    pub cumulative_dopamine: f32,
    pub safety_killswitch_tripped: bool,
    start_time: Instant,
}

impl Default for AutonomousGameAgent {
    fn default() -> Self {
        Self {
            state: PlaythroughState::Idle,
            active_recording: None,
            learned_demonstrations: Vec::new(),
            last_frame_sum: 0.0,
            cumulative_dopamine: 0.0,
            safety_killswitch_tripped: false,
            start_time: Instant::now(),
        }
    }
}

impl AutonomousGameAgent {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts recording a human demonstration playthrough
    pub fn start_recording(&mut self, session_id: impl Into<String>) -> Result<()> {
        let id_str = session_id.into();
        self.active_recording = Some(SessionRecording::new(id_str.clone()));
        self.state = PlaythroughState::Recording {
            session_id: id_str,
            frames_recorded: 0,
        };
        self.safety_killswitch_tripped = false;
        Ok(())
    }

    /// Records an event during demonstration
    pub fn record_demonstration_event(&mut self, event: RecordedInputEvent) {
        if let Some(rec) = &mut self.active_recording {
            rec.record_event(event);
            if let PlaythroughState::Recording { session_id, frames_recorded } = &self.state {
                self.state = PlaythroughState::Recording {
                    session_id: session_id.clone(),
                    frames_recorded: *frames_recorded + 1,
                };
            }
        }
    }

    /// Stops recording and saves the demonstration into the learned skill pool
    pub fn stop_recording(&mut self) -> Result<usize> {
        if let Some(rec) = self.active_recording.take() {
            let event_count = rec.events.len();
            self.learned_demonstrations.push(rec);
            self.state = PlaythroughState::Idle;
            Ok(event_count)
        } else {
            Err(anyhow!("No active recording session"))
        }
    }

    /// Calculates visual dopamine reward from frame luminance deltas
    pub fn calculate_visual_reward(&mut self, grid: &[f32]) -> f32 {
        let current_sum: f32 = grid.iter().sum();
        if self.last_frame_sum == 0.0 {
            self.last_frame_sum = current_sum;
            return 0.0;
        }

        let delta = (current_sum - self.last_frame_sum).abs();
        self.last_frame_sum = current_sum;

        // Reward dynamic gameplay motion
        let reward = if delta > 0.01 { 0.1 } else { -0.01 };
        self.cumulative_dopamine += reward;
        reward
    }

    /// Evaluates visual perception and predicts next autonomous motor action
    pub fn evaluate_autonomous_step(&mut self, frame: &VisualObservation) -> Result<Option<HidCommand>> {
        if self.safety_killswitch_tripped {
            self.state = PlaythroughState::EmergencyHalted {
                reason: "Safety killswitch active".to_string(),
            };
            return Ok(None);
        }

        // Calculate visual feedback reward
        let _reward = self.calculate_visual_reward(&frame.grid);

        let steps = match self.state {
            PlaythroughState::AutonomousPlaying { steps_executed, .. } => steps_executed + 1,
            _ => 1,
        };

        self.state = PlaythroughState::AutonomousPlaying {
            steps_executed: steps,
            cumulative_reward: self.cumulative_dopamine,
        };

        // If we have learned demonstrations, follow imitation trajectory
        if let Some(demo) = self.learned_demonstrations.first() {
            if !demo.events.is_empty() {
                let event_idx = steps % demo.events.len();
                let event = &demo.events[event_idx];

                let actions = match event {
                    RecordedInputEvent::MouseMove { x, y, .. } => vec![HidAction::MouseMove { delta_x: *x, delta_y: *y }],
                    RecordedInputEvent::MouseDown { .. } => vec![HidAction::LeftClick],
                    RecordedInputEvent::MouseUp { .. } => vec![],
                    RecordedInputEvent::KeyDown { key_code, .. } => vec![HidAction::KeyPress { key_code: *key_code as u16 }],
                    RecordedInputEvent::KeyUp { key_code, .. } => vec![HidAction::KeyRelease { key_code: *key_code as u16 }],
                    RecordedInputEvent::FrameCapture { .. } => vec![],
                };

                if !actions.is_empty() {
                    return Ok(Some(HidCommand {
                        actions,
                        sequence_id: steps as u64,
                        timestamp_us: self.start_time.elapsed().as_micros() as u64,
                    }));
                }
            }
        }

        // Default exploratory action
        Ok(Some(HidCommand {
            actions: vec![HidAction::MouseMove { delta_x: (steps % 10) as i32, delta_y: (steps % 10) as i32 }],
            sequence_id: steps as u64,
            timestamp_us: self.start_time.elapsed().as_micros() as u64,
        }))
    }

    /// Triggers the emergency safety killswitch
    pub fn trigger_killswitch(&mut self, reason: &str) {
        self.safety_killswitch_tripped = true;
        self.state = PlaythroughState::EmergencyHalted {
            reason: reason.to_string(),
        };
    }

    /// Resets the agent back to Idle
    pub fn reset(&mut self) {
        self.state = PlaythroughState::Idle;
        self.safety_killswitch_tripped = false;
        self.active_recording = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_agent_recording_and_imitation_cycle() {
        let mut agent = AutonomousGameAgent::new();
        assert_eq!(agent.state, PlaythroughState::Idle);

        // 1. Record demonstration
        agent.start_recording("speedrun_level_1").unwrap();
        agent.record_demonstration_event(RecordedInputEvent::MouseMove { x: 100, y: 200, timestamp_us: 1000 });
        agent.record_demonstration_event(RecordedInputEvent::MouseDown { button: 1, timestamp_us: 2000 });

        let count = agent.stop_recording().unwrap();
        assert_eq!(count, 2);
        assert_eq!(agent.learned_demonstrations.len(), 1);

        // 2. Run autonomous imitation step
        let frame = VisualObservation::new(vec![0.5f32; 128 * 128], 128, 128, 3000);

        let cmd = agent.evaluate_autonomous_step(&frame).unwrap().unwrap();
        assert_eq!(cmd.actions.len(), 1);

        // 3. Test Killswitch
        agent.trigger_killswitch("User pressed emergency escape");
        assert!(matches!(agent.state, PlaythroughState::EmergencyHalted { .. }));
        let blocked = agent.evaluate_autonomous_step(&frame).unwrap();
        assert!(blocked.is_none());
    }
}
