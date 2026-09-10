// src/state/pod.rs
//! POD data structures for ACC compliance.

use bytemuck::{Pod, Zeroable};
use crate::types::TaskId;

/// Fixed-size task representation used in `AppState`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct Task {
    pub id: TaskId,
    pub priority: u8,
    pub state: u8,
    pub payload: [u8; 128],
}

/// Configuration slot used in the global config store.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct ConfigSlot {
    pub key: [u8; 32],
    pub value: [u8; 64],
}
