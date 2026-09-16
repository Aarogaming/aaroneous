//! Wire-level IPC event type. All fields are naturally aligned; no padding needed
//! for this layout (8+2+2+4+4 = 20 bytes, but rounded to 24 for repr(C) alignment).

/// A single IPC bus event transmitted between components over the shared-memory bus.
///
/// All boundary types are `#[repr(C)]` with explicit padding for ABI safety.
/// Total size: 24 bytes (3 × cache-line-fraction, 8-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct IpcEvent {
    /// Monotonic nanosecond timestamp.
    pub timestamp: u64,
    /// Originating component ID (assigned at registration).
    pub component_id: u16,
    /// Event type opcode.
    pub event_type: u16,
    /// Byte offset into the shared payload region.
    pub payload_offset: u32,
    /// Byte length of the payload.
    pub payload_len: u32,
    /// Explicit padding to 24-byte natural alignment.
    pub _pad: u32,
}

impl IpcEvent {
    /// Construct a new `IpcEvent` with all fields set.
    #[inline]
    pub const fn new(
        timestamp: u64,
        component_id: u16,
        event_type: u16,
        payload_offset: u32,
        payload_len: u32,
    ) -> Self {
        Self {
            timestamp,
            component_id,
            event_type,
            payload_offset,
            payload_len,
            _pad: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn ipc_event_size_and_align() {
        assert_eq!(
            mem::size_of::<IpcEvent>(),
            24,
            "IpcEvent must be exactly 24 bytes"
        );
        assert_eq!(
            mem::align_of::<IpcEvent>(),
            8,
            "IpcEvent must be 8-byte aligned"
        );
    }

    #[test]
    fn ipc_event_pod_zeroable() {
        let z = bytemuck::Zeroable::zeroed();
        let _: IpcEvent = z;
    }
}
