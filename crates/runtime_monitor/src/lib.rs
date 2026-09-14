//! Runtime Monitor fast-path crate.

mod types;
// mod streaming_adaptation; // TEMPORARILY DISABLED
mod error_interceptor;

pub use hypervisor::state::telemetry::{Trigger, push_telemetry, telemetry_ring_buffer};

/// Push a trigger onto the telemetry ring buffer using lock-free SWMR API.
pub fn push_trigger(trigger: Trigger) {
    // SAFETY: Single-writer guarantee - this is the only writer in runtime_monitor crate
    push_telemetry(trigger);  // Returns bool but we don't need to check in single-writer context
}
