// src/state/telemetry.rs

use super::ring_buffer::RingBuffer;

/// Telemetry trigger used for fast-path error reporting.
/// 
/// SAFETY: All fields are POD-compliant primitives or fixed-size arrays with no padding.
/// Layout (repr(C)): anomaly(4) + context(64) + timestamp(8) = 76 bytes total, aligned to 8 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trigger {
    pub anomaly: u32,
    pub context: [u8; 64],
    pub timestamp: u64,
}

// SAFETY: Fixed-size arrays require manual Pod/Zeroable verification since bytemuck
// cannot verify padding for generic array types at compile time.
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
// SAFETY: RingBuffer uses atomic head/tail indices for lock-free SWMR access.
pub static TELEMETRY_BUFFER: RingBuffer<Trigger, 1024> = RingBuffer::new();

/// Return a reference to the global telemetry ring buffer (for reads).
pub fn telemetry_ring_buffer() -> &'static RingBuffer<Trigger, 1024> {
    &TELEMETRY_BUFFER
}

/// Push a trigger onto the buffer with exclusive write access. Returns Err if buffer full.
// SAFETY: This function takes &mut TELEMETRY_BUFFER (via transmute) to guarantee no concurrent writes.
// Called only from single-writer contexts (runtime_monitor crate).
pub fn push_telemetry(trigger: Trigger) -> Result<(), Trigger> {
    // Get mutable reference via unsafe transmute (single-writer guarantee)
    let mut buffer = unsafe { 
        &mut *(&TELEMETRY_BUFFER as *const RingBuffer<Trigger, 1024> as *mut RingBuffer<Trigger, 1024>) 
    };
    
    let tail = buffer.tail();
    let head = buffer.head();
    
    if (tail + 1) % 1024 == head % 1024 {
        return Err(trigger);  // Buffer full
    }
    
    let idx = tail % 1024;
    
    // SAFETY: We have exclusive write access (checked buffer is not full)
    unsafe { 
        let slot = &mut buffer.buffer_mut()[idx];
        *slot.as_ptr() = trigger;
    }
    
    buffer.tail.store(tail.wrapping_add(1), std::sync::atomic::Ordering::Release);
    Ok(())
}

/// Push a trigger with safety wrapper for single-writer guarantee.
pub fn push_telemetry_safe(trigger: Trigger) -> Result<(), Trigger> {
    push_telemetry(trigger)
}
