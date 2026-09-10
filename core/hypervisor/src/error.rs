// Error handling for hypervisor ACC

use bytemuck::{Pod, Zeroable};
use thiserror::Error;

/// Central error type for the hypervisor ACC.
#[repr(C)]
#[derive(Clone, Debug)]
pub enum HypervisorError {
    IoError(String),
    InvalidData(String),
    RuntimeError(String),
    Unexpected(String),
}

/// Error type for Constellation3D rendering
#[repr(C)]
#[derive(Clone, Debug)]
pub enum ConstellationError {
    Rendering(String),
    Gpu(String),
    Pipeline(String),
}

impl Default for HypervisorError {
    fn default() -> Self {
        Self::RuntimeError("".to_string())
    }
}

impl Default for ConstellationError {
    fn default() -> Self {
        Self::Rendering("".to_string())
    }
}

impl std::fmt::Display for HypervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HypervisorError::IoError(data) => write!(f, "{}", data),
            HypervisorError::InvalidData(data) => write!(f, "{}", data),
            HypervisorError::RuntimeError(data) => write!(f, "{}", data),
            HypervisorError::Unexpected(data) => write!(f, "{}", data),
        }
    }
}

impl std::fmt::Display for ConstellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstellationError::Rendering(data) => write!(f, "{}", data),
            ConstellationError::Gpu(data) => write!(f, "{}", data),
            ConstellationError::Pipeline(data) => write!(f, "{}", data),
        }
    }
}

impl std::error::Error for HypervisorError {}
impl std::error::Error for ConstellationError {}
