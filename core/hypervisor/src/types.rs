// Shared type definitions for the hypervisor ACC

/// Primitive task identifier used throughout the hypervisor ACC.
pub type TaskId = u64;

/// Optional debug metadata compiled only when the `debug` feature is enabled.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DebugMeta {
    /// The associated task identifier.
    pub id: TaskId,
    /// Human‑readable name (up to 32 bytes) for diagnostics.
    pub name: Option<[u8; 32]>,
}
