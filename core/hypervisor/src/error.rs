// Error handling for hypervisor ACC

use bytemuck::{Pod, Zeroable};
use thiserror::Error;

/// Central error type for the hypervisor ACC.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Error, Pod, Zeroable)]
pub enum HypervisorError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    #[error("Unexpected condition: {0}")]
    Unexpected(String),
}
