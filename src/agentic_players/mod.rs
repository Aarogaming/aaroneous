/// Agentic Players Module - User Emulation Service
///
/// This module implements the Shadow Agent and Emulation framework that allows
/// Aaroneous to learn from player gameplay and autonomously emulate learned playstyles.
///
/// Key Components:
/// - ShadowAgent: Observes and records player behavior
/// - IntentAnalyzer: Uses LLM to infer player intent from actions
/// - PolicyBuilder: Builds a learned policy from observations
/// - EmulationAgent: Executes the learned policy with humanization

pub mod types;
pub mod shadow_agent;
pub mod intent_analyzer;
pub mod policy;
pub mod emulation;

pub use types::*;
pub use shadow_agent::ShadowAgent;
pub use intent_analyzer::IntentAnalyzer;
pub use policy::PolicyBuilder;
pub use emulation::EmulationAgent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emulation_stats() {
        let mut stats = EmulationStats::new();
        stats.record_action(true);
        stats.record_action(false);
        assert_eq!(stats.actions_attempted, 2);
        assert_eq!(stats.actions_succeeded, 1);
        assert_eq!(stats.success_rate(), 0.5);
    }
}
