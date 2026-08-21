//! crates/marionette/src/event_recorder.rs
//! Microsecond-Precision HID Event Recording, Framebuffer Delta Analysis, and Action Replay Engine
//! inspired by Playwright, Selenium, and Win32 Input Journaling.

use serde::{Deserialize, Serialize};

/// An input event recorded with microsecond timing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordedInputEvent {
    MouseMove { x: i32, y: i32, timestamp_us: u64 },
    MouseDown { button: u8, timestamp_us: u64 },
    MouseUp { button: u8, timestamp_us: u64 },
    KeyDown { key_code: u32, timestamp_us: u64 },
    KeyUp { key_code: u32, timestamp_us: u64 },
    FrameCapture { frame_hash: u64, timestamp_us: u64 },
}

/// A recorded interaction session timeline
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionRecording {
    pub session_id: String,
    pub events: Vec<RecordedInputEvent>,
    pub start_time_us: u64,
    pub end_time_us: u64,
}

impl SessionRecording {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            events: Vec::new(),
            start_time_us: 0,
            end_time_us: 0,
        }
    }

    pub fn record_event(&mut self, event: RecordedInputEvent) {
        if self.events.is_empty() {
            self.start_time_us = match &event {
                RecordedInputEvent::MouseMove { timestamp_us, .. } => *timestamp_us,
                RecordedInputEvent::MouseDown { timestamp_us, .. } => *timestamp_us,
                RecordedInputEvent::MouseUp { timestamp_us, .. } => *timestamp_us,
                RecordedInputEvent::KeyDown { timestamp_us, .. } => *timestamp_us,
                RecordedInputEvent::KeyUp { timestamp_us, .. } => *timestamp_us,
                RecordedInputEvent::FrameCapture { timestamp_us, .. } => *timestamp_us,
            };
        }
        self.events.push(event);
    }

    pub fn duration_us(&self) -> u64 {
        if self.events.len() < 2 {
            return 0;
        }
        let last_time = match self.events.last().unwrap() {
            RecordedInputEvent::MouseMove { timestamp_us, .. } => *timestamp_us,
            RecordedInputEvent::MouseDown { timestamp_us, .. } => *timestamp_us,
            RecordedInputEvent::MouseUp { timestamp_us, .. } => *timestamp_us,
            RecordedInputEvent::KeyDown { timestamp_us, .. } => *timestamp_us,
            RecordedInputEvent::KeyUp { timestamp_us, .. } => *timestamp_us,
            RecordedInputEvent::FrameCapture { timestamp_us, .. } => *timestamp_us,
        };
        last_time.saturating_sub(self.start_time_us)
    }
}

/// Framebuffer visual difference detector
pub struct FramebufferAnalyzer;

impl FramebufferAnalyzer {
    /// Computes a 64-bit hash of framebuffer pixels to detect visual mutations
    pub fn compute_frame_hash(frame_bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325u64; // FNV-1a offset basis
        for &byte in frame_bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV-1a prime
        }
        hash
    }

    /// Computes percentage of mutated pixels between two framebuffers
    pub fn compute_frame_diff_percentage(frame_a: &[u8], frame_b: &[u8]) -> f32 {
        if frame_a.is_empty() || frame_b.is_empty() || frame_a.len() != frame_b.len() {
            return 100.0;
        }

        let mut diff_count = 0usize;
        for i in 0..frame_a.len() {
            if frame_a[i] != frame_b[i] {
                diff_count += 1;
            }
        }

        (diff_count as f32 / frame_a.len() as f32) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_event_recording_duration() {
        let mut session = SessionRecording::new("session_login");
        session.record_event(RecordedInputEvent::MouseMove { x: 100, y: 200, timestamp_us: 1000 });
        session.record_event(RecordedInputEvent::MouseDown { button: 1, timestamp_us: 1500 });
        session.record_event(RecordedInputEvent::MouseUp { button: 1, timestamp_us: 2000 });

        assert_eq!(session.events.len(), 3);
        assert_eq!(session.duration_us(), 1000); // 2000 - 1000
    }

    #[test]
    fn test_framebuffer_diff_calculation() {
        let frame_1 = vec![0u8; 1000];
        let mut frame_2 = vec![0u8; 1000];
        // Mutate 100 bytes (10%)
        for i in 0..100 {
            frame_2[i] = 255;
        }

        let hash_1 = FramebufferAnalyzer::compute_frame_hash(&frame_1);
        let hash_2 = FramebufferAnalyzer::compute_frame_hash(&frame_2);
        assert_ne!(hash_1, hash_2);

        let diff = FramebufferAnalyzer::compute_frame_diff_percentage(&frame_1, &frame_2);
        assert!((diff - 10.0).abs() < 1e-4);
    }
}
