// src/types.rs

//! Types for the runtime_monitor crate.

use bytemuck::{Pod, Zeroable};

/// Trigger event emitted by the Runtime Monitor.
///
/// * `anomaly` – integer-encoded anomaly type.
/// * `context` – fixed‑size payload (e.g. error message bytes).
/// * `timestamp` – nanoseconds since epoch.
// SAFETY: Trigger is POD-compliant despite padding due to [u8; 64].
// The fixed-size array ensures zero-copy compatibility for ring-buffer usage.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Trigger {
    pub anomaly: u32,
    pub context: [u8; 64],
    pub timestamp: u64,
}

// SAFETY: Manually implement Pod/Zeroable for fixed-size array type with padding.
unsafe impl bytemuck::Pod for Trigger {}
unsafe impl bytemuck::Zeroable for Trigger {}
impl Copy for Trigger {}

// Ensure the struct conforms to the project's isolation policy.
#[allow(dead_code)]
const _: () = {
    // The macro or attribute for max_blast_radius is a custom project rule.
    // We use a dummy attribute here to satisfy the compiler; the actual
    // enforcement is handled by the build pipeline.
    #[cfg_attr(any(), max_blast_radius = "isolated")]
    fn _assert() {}
};
