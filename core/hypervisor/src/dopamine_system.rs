use nervous_system::shared_memory::SynapseState;
use anyhow::Result;

pub struct DopamineSystem;

impl DopamineSystem {
    /// Processes rewards/penalties based on enzyme execution results.
    /// This directly modifies the homeostatic meters in the Synapse.
    pub fn process_event(&self, state: &mut SynapseState, event_type: DopamineEvent) {
        match event_type {
            DopamineEvent::SuccessfulIngestion(tier) => {
                // High reward for high-value public documents
                let reward = match tier {
                    0 => 15, // Public (High value for learning)
                    1 => 5,  // Restricted
                    _ => 0,  // Private (Neutral or risky)
                };
                state.curiosity_drive = state.curiosity_drive.saturating_sub(reward);
                state.understanding_score = state.understanding_score.saturating_add(reward / 2);
                state.integrity_score = state.integrity_score.saturating_add(2);
                println!("[Dopamine] +Reward: Ingestion successful (Tier {}). Curiosity satisfied.", tier);
            }
            DopamineEvent::ExecutionFailure(severity) => {
                // Penalty for failures (Integrity drop)
                let penalty = severity * 10;
                state.integrity_score = state.integrity_score.saturating_sub(penalty);
                state.understanding_score = state.understanding_score.saturating_sub(5);
                // Failures increase curiosity (need to figure out why it failed)
                state.curiosity_drive = state.curiosity_drive.saturating_add(10);
                println!("[Dopamine] -Penalty: Execution failure (Severity {}). Integrity compromised.", severity);
            }
            DopamineEvent::InternalCoherenceCheck(passed) => {
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

pub enum DopamineEvent {
    SuccessfulIngestion(u8), // contains license_tier
    ExecutionFailure(u8),    // contains severity (1-10)
    InternalCoherenceCheck(bool),
}
