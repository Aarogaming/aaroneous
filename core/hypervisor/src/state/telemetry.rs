// src/state/telemetry.rs

use bytemuck::{Pod, Zeroable};
use super::ring_buffer::RingBuffer;

/// Telemetry trigger used for fast‑path error reporting.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Trigger {
    pub anomaly: u32,
    pub context: [u8; 64],
    pub timestamp: u64,
}

// Static telemetry buffer with capacity 1024 entries.
static TELEMETRY_BUFFER: RingBuffer<Trigger, 1024> = RingBuffer::new();

/// Return a reference to the global telemetry ring buffer.
pub fn telemetry_ring_buffer() -> &'static RingBuffer<Trigger, 1024> {
    &TELEMETRY_BUFFER
}

/// Push a trigger onto the buffer (lock‑free). Returns Err if buffer full.
pub fn push_telemetry(trigger: Trigger) -> Result<(), Trigger> {
    TELEMETRY_BUFFER.push(trigger)
}
