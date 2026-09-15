//! crates/orchestrator/src/assimilation.rs
//! Pure, Event-Driven Typestate Machine and Reactive Reducer for Asset Assimilation.
//! Consumes and emits zero-copy frames over the `ipc_bus`.

#![deny(unsafe_code)]

use core::marker::PhantomData;
use ipc_bus::universal_protocol::{
    AssimilationPhase, AssimilationRecord, FixedString64, FixedString256, UniversalClientRequest,
    UniversalServerBroadcast,
};

// Re-export wire types for convenience
pub use ipc_bus::universal_protocol::{AssimilationPhase as Phase, AssimilationRecord as Record};

/// Error types occurring during assimilation event processing
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AssimilationError {
    FrameTooSmall,
    CastFailed,
    InvalidPhaseTransition(u32),
    InvalidAuditPayload,
    RetryExhausted,
}

impl core::fmt::Display for AssimilationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FrameTooSmall => write!(f, "Binary frame smaller than AssimilationRecord"),
            Self::CastFailed => write!(f, "Zero-copy bytemuck cast failed"),
            Self::InvalidPhaseTransition(p) => write!(f, "Invalid phase transition from phase {p}"),
            Self::InvalidAuditPayload => write!(f, "Audit payload failed validation"),
            Self::RetryExhausted => write!(f, "Maximum certification retries exhausted"),
        }
    }
}

impl std::error::Error for AssimilationError {}

/// Typestate phantom markers
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Idle;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Quarantined;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Auditing;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Synthesizing;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Certifying;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Committed;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rejected;

/// Typestate-governed assimilation / component onboarding task
#[derive(Copy, Clone, Debug)]
pub struct AssimilationTask<State> {
    pub record: AssimilationRecord,
    pub _marker: PhantomData<State>,
}

/// Normalized systems alias for component onboarding tasks.
pub type ComponentOnboardingTask<State> = AssimilationTask<State>;
/// Normalized systems alias for onboarding error.
pub type OnboardingError = AssimilationError;
/// Normalized systems alias for onboarding phase.
pub type OnboardingPhase = ipc_bus::universal_protocol::AssimilationPhase;
/// Normalized systems alias for onboarding record.
pub type OnboardingRecord = ipc_bus::universal_protocol::AssimilationRecord;
/// Normalized systems alias for staged sandbox state marker.
pub type Staged = Quarantined;
/// Normalized systems alias for staged sandbox state marker.
pub type StagedSandbox = Quarantined;

impl AssimilationTask<Idle> {
    pub fn new(source_id: [u8; 16], timestamp_us: u64) -> Self {
        Self {
            record: AssimilationRecord::new(source_id, timestamp_us),
            _marker: PhantomData,
        }
    }

    pub fn quarantine(mut self, path: FixedString256) -> AssimilationTask<Quarantined> {
        self.record.phase = AssimilationPhase::Quarantined as u32;
        self.record.mount_path = path;
        AssimilationTask {
            record: self.record,
            _marker: PhantomData,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuditResult {
    Pass(FixedString64),
    Fail,
}

impl AssimilationTask<Quarantined> {
    pub fn begin_audit(mut self) -> AssimilationTask<Auditing> {
        self.record.phase = AssimilationPhase::Auditing as u32;
        AssimilationTask {
            record: self.record,
            _marker: PhantomData,
        }
    }
}

impl AssimilationTask<Auditing> {
    #[allow(
        clippy::result_large_err,
        reason = "Typestate transitions return the owned fixed-size record without heap allocation"
    )]
    pub fn conclude_audit(
        mut self,
        result: AuditResult,
    ) -> Result<AssimilationTask<Synthesizing>, AssimilationTask<Rejected>> {
        match result {
            AuditResult::Pass(hash) => {
                self.record.phase = AssimilationPhase::Synthesizing as u32;
                self.record.ir_hash = hash;
                Ok(AssimilationTask {
                    record: self.record,
                    _marker: PhantomData,
                })
            }
            AuditResult::Fail => {
                self.record.phase = AssimilationPhase::Rejected as u32;
                Err(AssimilationTask {
                    record: self.record,
                    _marker: PhantomData,
                })
            }
        }
    }
}

impl AssimilationTask<Synthesizing> {
    pub fn finalize_synthesis(mut self) -> AssimilationTask<Certifying> {
        self.record.phase = AssimilationPhase::Certifying as u32;
        AssimilationTask {
            record: self.record,
            _marker: PhantomData,
        }
    }
}

impl AssimilationTask<Certifying> {
    #[allow(
        clippy::result_large_err,
        reason = "Typestate transitions return the owned fixed-size record without heap allocation"
    )]
    pub fn certify(
        mut self,
        passed: bool,
        max_retries: u32,
    ) -> Result<
        AssimilationTask<Committed>,
        Result<AssimilationTask<Synthesizing>, AssimilationTask<Rejected>>,
    > {
        if passed {
            self.record.phase = AssimilationPhase::Committed as u32;
            Ok(AssimilationTask {
                record: self.record,
                _marker: PhantomData,
            })
        } else if self.record.retries < max_retries {
            self.record.retries += 1;
            self.record.phase = AssimilationPhase::Synthesizing as u32;
            Err(Ok(AssimilationTask {
                record: self.record,
                _marker: PhantomData,
            }))
        } else {
            self.record.phase = AssimilationPhase::Rejected as u32;
            Err(Err(AssimilationTask {
                record: self.record,
                _marker: PhantomData,
            }))
        }
    }
}

/// Pure, non-allocating deterministic state reducer.
/// Ingests an incoming binary slice, casts it zero-copy to `AssimilationRecord`,
/// and deterministically advances the typestate machine by one step.
pub fn handle_assimilation_event(bytes: &[u8]) -> Result<AssimilationRecord, AssimilationError> {
    if bytes.len() < core::mem::size_of::<AssimilationRecord>() {
        return Err(AssimilationError::FrameTooSmall);
    }

    let record = bytemuck::try_from_bytes::<AssimilationRecord>(
        &bytes[..core::mem::size_of::<AssimilationRecord>()],
    )
    .map_err(|_| AssimilationError::CastFailed)?;

    let mut transitioned = *record;
    match transitioned.phase {
        0 => {
            // Idle -> Quarantined
            let task = AssimilationTask::<Idle> {
                record: transitioned,
                _marker: PhantomData,
            };
            let quarantined = task.quarantine(transitioned.mount_path);
            transitioned = quarantined.record;
        }
        1 => {
            // Quarantined -> Auditing
            let task = AssimilationTask::<Quarantined> {
                record: transitioned,
                _marker: PhantomData,
            };
            let auditing = task.begin_audit();
            transitioned = auditing.record;
        }
        2 => {
            // Auditing -> Synthesizing or Rejected
            let task = AssimilationTask::<Auditing> {
                record: transitioned,
                _marker: PhantomData,
            };
            match task.conclude_audit(AuditResult::Pass(transitioned.ir_hash)) {
                Ok(synthesizing) => transitioned = synthesizing.record,
                Err(rejected) => transitioned = rejected.record,
            }
        }
        3 => {
            // Synthesizing -> Certifying
            let task = AssimilationTask::<Synthesizing> {
                record: transitioned,
                _marker: PhantomData,
            };
            let certifying = task.finalize_synthesis();
            transitioned = certifying.record;
        }
        4 => {
            // Certifying -> Committed, Synthesizing, or Rejected
            let task = AssimilationTask::<Certifying> {
                record: transitioned,
                _marker: PhantomData,
            };
            match task.certify(true, 3) {
                Ok(committed) => transitioned = committed.record,
                Err(Ok(synthesizing)) => transitioned = synthesizing.record,
                Err(Err(rejected)) => transitioned = rejected.record,
            }
        }
        _ => {
            // Terminal states (Committed = 5, Rejected = 6) remain stable
        }
    }

    Ok(transitioned)
}

/// Process a universal client request for assimilation and produce a zero-copy server broadcast.
pub fn process_client_request(
    req: &UniversalClientRequest,
) -> Result<UniversalServerBroadcast, AssimilationError> {
    let initial_record = AssimilationRecord::from_client_request(req)
        .ok_or(AssimilationError::InvalidPhaseTransition(req.req_type))?;

    let bytes = bytemuck::bytes_of(&initial_record);
    let transitioned = handle_assimilation_event(bytes)?;
    Ok(transitioned.to_broadcast(req.sequence, 0))
}

/// Normalized systems alias for `handle_assimilation_event`.
pub use handle_assimilation_event as handle_onboarding_event;

/// Normalized systems alias for `process_client_request`.
pub use process_client_request as process_onboarding_request;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_transitions_happy_path() {
        let task = AssimilationTask::<Idle>::new([1u8; 16], 100);
        assert_eq!(task.record.phase, AssimilationPhase::Idle as u32);

        let path = FixedString256::new("crates/test").expect("valid path");
        let quarantined = task.quarantine(path);
        assert_eq!(
            quarantined.record.phase,
            AssimilationPhase::Quarantined as u32
        );
        assert_eq!(
            quarantined.record.mount_path.as_str().expect("utf8"),
            "crates/test"
        );

        let auditing = quarantined.begin_audit();
        assert_eq!(auditing.record.phase, AssimilationPhase::Auditing as u32);

        let ir_hash = FixedString64::new("hash_12345").expect("valid hash");
        let synthesizing = auditing
            .conclude_audit(AuditResult::Pass(ir_hash))
            .expect("audit should pass");
        assert_eq!(
            synthesizing.record.phase,
            AssimilationPhase::Synthesizing as u32
        );
        assert_eq!(
            synthesizing.record.ir_hash.as_str().expect("utf8"),
            "hash_12345"
        );

        let certifying = synthesizing.finalize_synthesis();
        assert_eq!(
            certifying.record.phase,
            AssimilationPhase::Certifying as u32
        );

        let committed = certifying
            .certify(true, 3)
            .expect("certification should pass");
        assert_eq!(committed.record.phase, AssimilationPhase::Committed as u32);
    }

    #[test]
    fn test_typestate_retry_and_rejection() {
        let task = AssimilationTask::<Idle>::new([2u8; 16], 200);
        let path = FixedString256::new("crates/fail").expect("valid path");
        let quarantined = task.quarantine(path);
        let auditing = quarantined.begin_audit();

        // Fail audit -> Rejected
        let rejected = auditing
            .conclude_audit(AuditResult::Fail)
            .expect_err("audit must fail");
        assert_eq!(rejected.record.phase, AssimilationPhase::Rejected as u32);

        // Test retry exhaustion during certification
        let certifying = AssimilationTask::<Synthesizing> {
            record: AssimilationRecord {
                source_id: [3u8; 16],
                phase: AssimilationPhase::Synthesizing as u32,
                retries: 2,
                started_at_us: 300,
                mount_path: FixedString256::default(),
                ir_hash: FixedString64::default(),
            },
            _marker: PhantomData,
        }
        .finalize_synthesis();

        // Retry 2 -> 3 (retries < 3: 2 < 3, so retries becomes 3 and loops back to Synthesizing)
        let retry_synthesizing = certifying
            .certify(false, 3)
            .expect_err("certify must retry")
            .expect("still has retries");
        assert_eq!(retry_synthesizing.record.retries, 3);
        assert_eq!(
            retry_synthesizing.record.phase,
            AssimilationPhase::Synthesizing as u32
        );

        // Next certification with retries == 3 -> Rejected
        let certifying_again = retry_synthesizing.finalize_synthesis();
        let rejected_final = certifying_again
            .certify(false, 3)
            .expect_err("certify must fail")
            .expect_err("retries exhausted");
        assert_eq!(
            rejected_final.record.phase,
            AssimilationPhase::Rejected as u32
        );
    }

    #[test]
    fn test_event_driven_assimilation_reducer() {
        let initial = AssimilationRecord::new([7u8; 16], 500);
        let bytes0 = bytemuck::bytes_of(&initial);

        // Step 1: Idle -> Quarantined
        let step1 = handle_assimilation_event(bytes0).expect("step 1 succeeds");
        assert_eq!(step1.phase, AssimilationPhase::Quarantined as u32);

        // Step 2: Quarantined -> Auditing
        let bytes1 = bytemuck::bytes_of(&step1);
        let step2 = handle_assimilation_event(bytes1).expect("step 2 succeeds");
        assert_eq!(step2.phase, AssimilationPhase::Auditing as u32);

        // Step 3: Auditing -> Synthesizing
        let bytes2 = bytemuck::bytes_of(&step2);
        let step3 = handle_assimilation_event(bytes2).expect("step 3 succeeds");
        assert_eq!(step3.phase, AssimilationPhase::Synthesizing as u32);

        // Step 4: Synthesizing -> Certifying
        let bytes3 = bytemuck::bytes_of(&step3);
        let step4 = handle_assimilation_event(bytes3).expect("step 4 succeeds");
        assert_eq!(step4.phase, AssimilationPhase::Certifying as u32);

        // Step 5: Certifying -> Committed
        let bytes4 = bytemuck::bytes_of(&step4);
        let step5 = handle_assimilation_event(bytes4).expect("step 5 succeeds");
        assert_eq!(step5.phase, AssimilationPhase::Committed as u32);

        // Terminal state does not transition further
        let bytes5 = bytemuck::bytes_of(&step5);
        let step6 = handle_assimilation_event(bytes5).expect("step 6 succeeds");
        assert_eq!(step6.phase, AssimilationPhase::Committed as u32);
    }
}
