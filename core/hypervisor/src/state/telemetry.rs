// src/state/telemetry.rs

use std::sync::{Arc, Mutex};
use super::ring_buffer::RingBuffer;

/// Telemetry trigger used for fast-path error reporting.
/// 
/// SAFETY: All fields are POD-compliant primitives or fixed-size arrays with no padding.
/// Layout (repr(C)): anomaly(4) + context(64) + timestamp(8) = 76 bytes total.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trigger {
    pub anomaly: u32,
    pub context: [u8; 64],
    pub timestamp: u64,
}

// SAFETY: Manual Pod/Zeroable implementation since bytemuck can't verify POD for fixed-size arrays.
unsafe impl bytemuck::Pod for Trigger {}
unsafe impl bytemuck::Zeroable for Trigger {}

impl Default for Trigger {
    fn default() -> Self {
        Self {
            anomaly: 0,
            context: [0u8; 64],
            timestamp: 0,
        }
    }
}

// Static telemetry buffer with capacity 1024 entries.
lazy_static::lazy_static! {
    pub static ref TELEMETRY_BUFFER: Arc<Mutex<RingBuffer<Trigger, 1024>>> = {
        let buffer = RingBuffer::new();
        unsafe { Arc::new(std::sync::Mutex::new(buffer)) }
    };
}

/// Return a reference to the global telemetry ring buffer.
pub fn telemetry_ring_buffer() -> &'static Arc<Mutex<RingBuffer<Trigger, 1024>>> {
    &*TELEMETRY_BUFFER
}

/// Push a trigger onto the buffer (lock-free with mutex protection). Returns Err if buffer full.
pub fn push_telemetry(trigger: Trigger) -> Result<(), Trigger> {
    let mut guard = TELEMETRY_BUFFER.lock().unwrap();
    guard.push(trigger)
}
