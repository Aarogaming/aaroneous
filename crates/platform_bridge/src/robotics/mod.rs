// crates/platform_bridge/src/robotics/mod.rs
//! Robotics and physical cybernetic machine control interfaces.

pub mod boebot;

pub use boebot::{
    BoeBotCommand, BoeBotOcularNavigator, CorridorCorridorAnalysis, OcularPerspective,
};
