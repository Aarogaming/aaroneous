// src/state/telemetry.rs

use super::ring_buffer::SwmrRingBuffer;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Trigger {
    pub timestamp_qpc: u64,
    pub error_code: u32,
    pub severity: u16,
    pub reserved: u16,
    pub message: [u8; 64],
}

impl Default for Trigger {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Trigger {
    pub const ZERO: Self = Self {
        timestamp_qpc: 0,
        error_code: 0,
        severity: 0,
        reserved: 0,
        message: [0u8; 64],
    };
}

pub static TELEMETRY_BUFFER: SwmrRingBuffer<Trigger, 1024> = SwmrRingBuffer::new(Trigger::ZERO);

pub fn push_telemetry(trigger: Trigger) -> bool {
    TELEMETRY_BUFFER.push(trigger)
}
pub fn pop_telemetry() -> Option<Trigger> {
    TELEMETRY_BUFFER.pop()
}
pub fn telemetry_ring_buffer() -> &'static SwmrRingBuffer<Trigger, 1024> {
    &TELEMETRY_BUFFER
}
