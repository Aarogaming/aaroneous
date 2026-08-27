/// Agentic Players Module - User Emulation Service
///
/// This module implements the Background Agent and Emulation framework that allows
/// Aaroneous to learn from player gameplay and autonomously emulate learned playstyles.
///
/// Key Components:
/// - BackgroundAgent: Observes and records player behavior
/// - IntentAnalyzer: Uses LLM to infer player intent from actions
/// - PolicyBuilder: Builds a learned policy from observations
/// - EmulationAgent: Executes the learned policy with humanization

pub mod types;
pub mod background_agent;
pub mod intent_analyzer;
pub mod policy;
pub mod emulation;

pub use types::*;
pub use background_agent::BackgroundAgent;
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
        assert_eq!(stats.actions_taken, 2);
        assert!((stats.success_rate() - 0.5).abs() < std::f32::EPSILON);
        assert_eq!(stats.success_rate(), 0.5);
    }
}
