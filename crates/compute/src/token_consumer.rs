// Token Consumer - Continuous State Adaptation from MachineToken frames

use anyhow::Result;

/// Error type for state adaptation failures
#[derive(Debug)]
pub enum AdaptationError {
    InvalidTokenFormat,
    ParameterDriftExceeded(f32),
    CovarianceSingularity,
}

impl std::fmt::Display for AdaptationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AdaptationError::InvalidTokenFormat => write!(f, "Token format invalid"),
            AdaptationError::ParameterDriftExceeded(drift) => write!(f, "Parameter drift exceeded: {}", drift),
            AdaptationError::CovarianceSingularity => write!(f, "Covariance matrix singular"),
        }
    }
}

impl std::error::Error for AdaptationError {}

/// 32-byte MachineToken for telemetry data
#[derive(Debug, Clone, Copy)]
pub struct MachineToken([u8; 32]);

impl MachineToken {
    pub fn new(data: [u8; 32]) -> Self {
        Self(data)
    }
}

/// Continuous State Adaptor using Recursive Least Squares (RLS)
pub struct StateAdaptor<const DIM: usize, const DIM_SQ: usize> {
    pub covariance_matrix: [f32; DIM_SQ],
    pub parameters: [f32; DIM],
    pub lambda: f32,
}

impl<const DIM: usize, const DIM_SQ: usize> StateAdaptor<DIM, DIM_SQ> {
    pub fn new(initial_covariance: [f32; DIM_SQ], foraging_gain: f32) -> Self {
        Self {
            covariance_matrix: initial_covariance,
            parameters: [0.0f32; DIM],
            lambda: if foraging_gain > 0.99 { 0.99 } else { foraging_gain }.max(0.95),
        }
    }

    pub fn process_token(&mut self, _token: &MachineToken) -> Result<f32, AdaptationError> {
        // Placeholder implementation
        Ok(0.0)
    }
}
