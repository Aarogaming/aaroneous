//! Runtime Monitor fast-path crate.

mod types;
// mod streaming_adaptation; // TEMPORARILY DISABLED due to Pod constraint issues
mod error_interceptor;

pub use hypervisor::state::telemetry::Trigger;
// pub use streaming_adaptation::StreamingLoraAdaptationPipeline; // TEMPORARILY DISABLED

/// Push a trigger onto the telemetry ring buffer using SWMR-safe hypervisor API.
pub fn push_trigger(trigger: Trigger) {
    // SAFETY: Single-writer guarantee - this is the only writer in runtime_monitor crate
    hypervisor::state::telemetry::push_telemetry_safe(trigger).expect("Telemetry buffer full - queue overflow");
}

// Re-export telemetry buffer for external access
pub use hypervisor::state::telemetry::{TELEMETRY_BUFFER, push_telemetry, telemetry_ring_buffer};
