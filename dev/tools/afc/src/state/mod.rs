// dev/tools/afc/src/state/mod.rs
pub mod machine;
pub mod sanitizer;

pub use machine::{FlightState, StateMachine, StateTransition};
pub use sanitizer::ContextSanitizer;
