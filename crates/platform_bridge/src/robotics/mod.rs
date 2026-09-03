// crates/platform_bridge/src/robotics/mod.rs
//! Robotics and physical cybernetic machine control interfaces.

pub mod canbus;
pub mod differential_drive;

pub use canbus::{AutomotiveBusBridge, CanFrame, ProtocolEntropyAnalyzer};
pub use differential_drive::{
    CorridorClearanceAnalysis, DifferentialDriveCommand, DualPerspectiveOcularNavigator,
    OcularPerspective,
};

// Aliases for transition
pub type BoeBotCommand = DifferentialDriveCommand;
pub type BoeBotOcularNavigator = DualPerspectiveOcularNavigator;
pub type CorridorCorridorAnalysis = CorridorClearanceAnalysis;
