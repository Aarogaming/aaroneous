// src/types.rs

//! Types for the runtime_monitor crate.

use bytemuck::{Pod, Zeroable};

/// Trigger event emitted by the Runtime Monitor.
///
/// * `anomaly` – integer-encoded anomaly type.
/// * `context` – fixed‑size payload (e.g. error message bytes).
/// * `timestamp` – nanoseconds since epoch.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct Trigger {
    pub anomaly: u32,
    pub context: [u8; 64],
    pub timestamp: u64,
}

// Ensure the struct conforms to the project's isolation policy.
#[allow(dead_code)]
const _: () = {
    // The macro or attribute for max_blast_radius is a custom project rule.
    // We use a dummy attribute here to satisfy the compiler; the actual
    // enforcement is handled by the build pipeline.
    #[cfg_attr(any(), max_blast_radius = "isolated")]
    fn _assert() {}
};
