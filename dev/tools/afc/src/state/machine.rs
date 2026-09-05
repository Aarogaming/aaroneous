// dev/tools/afc/src/state/machine.rs
use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlightState {
    Idle,
    Planning {
        spec_focus: String,
    },
    Auditing {
        category: String,
    },
    IsolatedRemediation {
        task_id: String,
        target_file: PathBuf,
        target_lines: (usize, usize),
        defect_description: String,
        compiler_feedback: Option<String>,
    },
    VerificationGate {
        modified_files: Vec<PathBuf>,
    },
    CommitLedger {
        commit_message: String,
    },
    Completed,
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: FlightState,
    pub to: FlightState,
    pub timestamp: String,
    pub reason: String,
}

pub struct StateMachine {
    pub current_state: FlightState,
    pub history: Vec<StateTransition>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current_state: FlightState::Idle,
            history: Vec::new(),
        }
    }

    /// Transition to a new micro-state
    pub fn transition_to(&mut self, next: FlightState, reason: impl Into<String>) -> Result<()> {
        let transition = StateTransition {
            from: self.current_state.clone(),
            to: next.clone(),
            timestamp: Local::now().to_rfc3339(),
            reason: reason.into(),
        };

        self.history.push(transition);
        self.current_state = next;
        Ok(())
    }

    /// Check if state machine can accept a new task
    pub fn is_idle(&self) -> bool {
        matches!(
            self.current_state,
            FlightState::Idle | FlightState::Completed
        )
    }

    /// Reset state machine back to Idle
    pub fn reset(&mut self) {
        let _ = self.transition_to(FlightState::Idle, "Manual reset to idle state");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_transitions() {
        let mut sm = StateMachine::new();
        assert!(sm.is_idle());

        sm.transition_to(
            FlightState::Planning {
                spec_focus: "Refactor".into(),
            },
            "Start plan",
        )
        .expect("Transition should succeed");
        assert!(!sm.is_idle());
        assert_eq!(sm.history.len(), 1);

        sm.transition_to(FlightState::Completed, "Done")
            .expect("Transition should succeed");
        assert!(sm.is_idle());
        assert_eq!(sm.history.len(), 2);
    }
}
