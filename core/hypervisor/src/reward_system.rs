use crate::autonomic_loop::SynapseState;

/// Machine-Native Reinforcement & Feedback Signal Processor
pub struct FeedbackSignalProcessor;

impl FeedbackSignalProcessor {
    /// Processes rewards/penalties based on worker execution results.
    /// This directly modifies the equilibrium meters in the Shared Interconnect Bus.
    pub fn process_event(&self, state: &mut SynapseState, event_type: FeedbackEvent) {
        match event_type {
            FeedbackEvent::SuccessfulIngestion(tier) => {
                // High reward for high-value public documents
                let reward = match tier {
                    0 => 15, // Public (High value for learning)
                    1 => 5,  // Restricted
                    _ => 0,  // Private (Neutral or risky)
                };
                state.curiosity_drive = state.curiosity_drive.saturating_sub(reward);
                state.understanding_score = state.understanding_score.saturating_add(reward / 2);
                state.integrity_score = state.integrity_score.saturating_add(2);
            }
            FeedbackEvent::ExecutionFailure(severity) => {
                // Penalty for failures (Integrity drop)
                let penalty = (severity as u32) * 10;
                state.integrity_score = state.integrity_score.saturating_sub(penalty);
                state.understanding_score = state.understanding_score.saturating_sub(5);
                // Failures increase exploration demand (need to determine failure cause)
                state.curiosity_drive = state.curiosity_drive.saturating_add(10);
            }
            FeedbackEvent::InternalCoherenceCheck(passed) => {
                if passed {
                    state.integrity_score = state.integrity_score.saturating_add(1);
                } else {
                    state.integrity_score = state.integrity_score.saturating_sub(5);
                    state.understanding_score = state.understanding_score.saturating_sub(2);
                }
            }
        }
    }
}

pub enum FeedbackEvent {
    SuccessfulIngestion(u8), // contains license_tier
    ExecutionFailure(u8),    // contains severity (1-10)
    InternalCoherenceCheck(bool),
}

// Backwards-compatible aliases
pub type DopamineSystem = FeedbackSignalProcessor;
pub type DopamineEvent = FeedbackEvent;
pub type RewardSignalProcessor = FeedbackSignalProcessor;
