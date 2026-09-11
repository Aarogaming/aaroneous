//! Runtime Monitor fast-path crate.

mod types;
// mod streaming_adaptation; // TEMPORARILY DISABLED due to Pod constraint issues
mod error_interceptor;

pub use hypervisor::state::telemetry::Trigger;
// pub use streaming_adaptation::StreamingLoraAdaptationPipeline; // TEMPORARILY DISABLED

/// Push a trigger onto the telemetry ring buffer using safe hypervisor API.
pub fn push_trigger(trigger: Trigger) {
    // SAFETY: Caller must ensure exclusive access to TELEMETRY_BUFFER
    hypervisor::state::telemetry::push_telemetry(trigger).expect("Telemetry buffer full - queue overflow");
}

// Re-export telemetry buffer for external access
pub use hypervisor::state::telemetry::{TELEMETRY_BUFFER, push_telemetry, telemetry_ring_buffer};
