//! core/hypervisor/src/onboarding.rs
//! Microkernel component onboarding adapter bridging to orchestrator::onboarding.

pub use ipc_bus::universal_protocol::{
    AssimilationPhase, AssimilationPhase as OnboardingPhase, AssimilationRecord,
    AssimilationRecord as OnboardingRecord,
};
pub use orchestrator::onboarding::{
    AssimilationError, AssimilationError as OnboardingError, AssimilationTask, AuditResult,
    Auditing, Certifying, Committed, ComponentOnboardingTask, Idle, Quarantined, Rejected, Staged,
    StagedSandbox, Synthesizing,
    handle_assimilation_event as orchestrator_handle_assimilation_event,
    handle_onboarding_event as orchestrator_handle_onboarding_event, process_client_request,
    process_onboarding_request,
};

use hypervisor::error::HypervisorError;

/// Reactive event listener: consumes an incoming binary frame from the IPC bus,
/// parses or casts it to an `OnboardingRecord`, and advances the typestate machine.
pub fn handle_onboarding_event(bytes: &[u8]) -> Result<OnboardingRecord, HypervisorError> {
    orchestrator::onboarding::handle_onboarding_event(bytes)
        .map_err(|e| HypervisorError::InvalidData(e.to_string()))
}

/// Backward-compatible alias for `handle_onboarding_event`.
pub use handle_onboarding_event as handle_assimilation_event;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_driven_onboarding_zero_copy() {
        let task = ComponentOnboardingTask::<Idle>::new([1u8; 16], 123456);
        let bytes = bytemuck::bytes_of(&task.record);

        let transitioned = handle_onboarding_event(bytes).expect("Event processing must succeed");
        assert_eq!(transitioned.phase, OnboardingPhase::Quarantined as u32);

        let bytes2 = bytemuck::bytes_of(&transitioned);
        let transitioned2 = handle_onboarding_event(bytes2).expect("Event processing must succeed");
        assert_eq!(transitioned2.phase, OnboardingPhase::Auditing as u32);
    }
}
