// dev/tools/afc/src/lib.rs
pub mod config;
pub mod delivery;
pub mod engine;
pub mod gatekeeper;
pub mod git;
pub mod gui;
pub mod hardware;
pub mod llm;
pub mod model_probe;
pub mod queue;
pub mod recipe;
pub mod repl;
pub mod router;
pub mod state;

pub use config::FlightConfig;
pub use engine::FlightEngine;
pub use gui::launch_gui;
pub use model_probe::{ModelEndpointStatus, ModelProbe};
pub use repl::{ReplSummary, SovereignRepl};
