pub use nervous_system;

/// The Rust SDK for Aaroneous.
/// Provides high-level wrappers for the Synapse and AgentBus.
pub struct SynapseBridge;

impl SynapseBridge {
    pub fn connect() -> Self {
        println!("[SDK] Connected to Aaroneous Synapse");
        Self
    }
}
