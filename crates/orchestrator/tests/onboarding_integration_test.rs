//! crates/orchestrator/tests/onboarding_integration_test.rs
//! End-to-End Integration Test for Component Onboarding Pipeline.
//! Verifies zero-copy IPC frame dispatch, typestate transitions, and broadcast emission.

#![deny(unsafe_code)]

use ipc_bus::universal_event_bus::UniversalEventBus;
use ipc_bus::universal_protocol::{
    AssimilationPhase, AssimilationRecord, FixedString64, FixedString256, UcpBroadcastType,
    UcpRequestType, UniversalClientRequest,
};
use orchestrator::onboarding::{
    AssimilationTask, Idle, Synthesizing, handle_assimilation_event, process_client_request,
};
use std::marker::PhantomData;

#[test]
fn test_e2e_assimilation_raw_binary_transitions() {
    let source_id = [0xAA; 16];
    let timestamp_us = 1_000_000u64;

    // Step 0: IDLE
    let initial_task = AssimilationTask::<Idle>::new(source_id, timestamp_us);
    assert_eq!(initial_task.record.phase, AssimilationPhase::Idle as u32);
    let raw_bytes_0 = bytemuck::bytes_of(&initial_task.record);
    assert_eq!(raw_bytes_0.len(), 360);

    // Transition 1: IDLE -> QUARANTINED
    let record_1 = handle_assimilation_event(raw_bytes_0).expect("Reducer step 1 must succeed");
    assert_eq!(record_1.phase, AssimilationPhase::Quarantined as u32);
    assert_eq!(record_1.source_id, source_id);

    // Transition 2: QUARANTINED -> AUDITING
    let raw_bytes_1 = bytemuck::bytes_of(&record_1);
    let record_2 = handle_assimilation_event(raw_bytes_1).expect("Reducer step 2 must succeed");
    assert_eq!(record_2.phase, AssimilationPhase::Auditing as u32);

    // Transition 3: AUDITING -> SYNTHESIZING
    let raw_bytes_2 = bytemuck::bytes_of(&record_2);
    let record_3 = handle_assimilation_event(raw_bytes_2).expect("Reducer step 3 must succeed");
    assert_eq!(record_3.phase, AssimilationPhase::Synthesizing as u32);

    // Transition 4: SYNTHESIZING -> CERTIFYING
    let raw_bytes_3 = bytemuck::bytes_of(&record_3);
    let record_4 = handle_assimilation_event(raw_bytes_3).expect("Reducer step 4 must succeed");
    assert_eq!(record_4.phase, AssimilationPhase::Certifying as u32);

    // Transition 5: CERTIFYING -> COMMITTED
    let raw_bytes_4 = bytemuck::bytes_of(&record_4);
    let record_5 = handle_assimilation_event(raw_bytes_4).expect("Reducer step 5 must succeed");
    assert_eq!(record_5.phase, AssimilationPhase::Committed as u32);

    // Verify broadcast emission
    let broadcast = record_5.to_broadcast(1, 45);
    assert_eq!(
        broadcast.broadcast_type,
        UcpBroadcastType::AssimilationState as u32
    );
    assert_eq!(broadcast.sequence, 1);
    assert_eq!(broadcast.boolean_flag, 1);
    assert_eq!(broadcast.cycle_latency_us, 45);
    let msg = broadcast.message.as_str().expect("valid utf8 message");
    assert_eq!(msg, "Assimilation completed successfully");
}

#[test]
fn test_e2e_universal_client_request_ingestion() {
    let mut req = UniversalClientRequest {
        req_type: UcpRequestType::AssimilationEvent as u32,
        _pad0: 0,
        sequence: 42,
        slot_id: 1,
        domain_id: 3,
        _pad1: [0u8; 3],
        payload_a: FixedString256::new("crates/test_asset").expect("valid mount path"),
        payload_b: FixedString256::new("sha256_mock_ir_hash").expect("valid hash payload"),
    };

    let broadcast =
        process_client_request(&req).expect("Client request must process into broadcast");
    assert_eq!(
        broadcast.broadcast_type,
        UcpBroadcastType::AssimilationState as u32
    );
    assert_eq!(broadcast.sequence, 42);
    // Initial transition from Idle advances to Quarantined (in progress)
    assert_eq!(broadcast.boolean_flag, 0);

    // Reject non-assimilation request types
    req.req_type = UcpRequestType::Ping as u32;
    assert!(process_client_request(&req).is_err());
}

#[test]
fn test_e2e_certification_retry_loop_and_rejection() {
    let record = AssimilationRecord {
        source_id: [0xEE; 16],
        phase: AssimilationPhase::Synthesizing as u32,
        retries: 0,
        started_at_us: 500,
        mount_path: FixedString256::default(),
        ir_hash: FixedString64::default(),
    };

    let max_retries = 3;

    // Retry 1: Synthesizing -> Certifying -> fail -> Synthesizing (retries = 1)
    let task1 = AssimilationTask::<Synthesizing> {
        record,
        _marker: PhantomData,
    };
    let certifying1 = task1.finalize_synthesis();
    let retry1 = certifying1
        .certify(false, max_retries)
        .expect_err("certify must fail")
        .expect("retry 1");
    assert_eq!(retry1.record.retries, 1);
    assert_eq!(retry1.record.phase, AssimilationPhase::Synthesizing as u32);

    // Retry 2: Synthesizing -> Certifying -> fail -> Synthesizing (retries = 2)
    let certifying2 = retry1.finalize_synthesis();
    let retry2 = certifying2
        .certify(false, max_retries)
        .expect_err("certify must fail")
        .expect("retry 2");
    assert_eq!(retry2.record.retries, 2);
    assert_eq!(retry2.record.phase, AssimilationPhase::Synthesizing as u32);

    // Retry 3: Synthesizing -> Certifying -> fail -> Synthesizing (retries = 3)
    let certifying3 = retry2.finalize_synthesis();
    let retry3 = certifying3
        .certify(false, max_retries)
        .expect_err("certify must fail")
        .expect("retry 3");
    assert_eq!(retry3.record.retries, 3);
    assert_eq!(retry3.record.phase, AssimilationPhase::Synthesizing as u32);

    // Retry exhaustion: Synthesizing -> Certifying -> fail -> Rejected
    let certifying4 = retry3.finalize_synthesis();
    let rejected = certifying4
        .certify(false, max_retries)
        .expect_err("certify must fail")
        .expect_err("exhausted");
    assert_eq!(rejected.record.phase, AssimilationPhase::Rejected as u32);
    assert_eq!(rejected.record.retries, 3);

    // Verify rejection broadcast
    let broadcast = rejected.record.to_broadcast(99, 10);
    assert_eq!(
        broadcast.broadcast_type,
        UcpBroadcastType::AssimilationState as u32
    );
    assert_eq!(broadcast.boolean_flag, 0);
    let msg = broadcast.message.as_str().expect("valid utf8 message");
    assert_eq!(msg, "Assimilation rejected during certification/audit");
}

#[test]
fn test_e2e_event_bus_zero_copy_dispatch() {
    let bus = UniversalEventBus::<Vec<u8>>::new();
    let subscriber = bus.subscribe("assimilation.events", "orchestrator_daemon_sub");

    let initial = AssimilationRecord::new([0xBB; 16], 999_999);
    let raw_bytes = bytemuck::bytes_of(&initial).to_vec();

    // Drop raw binary onto the event bus
    let seq = bus.publish("assimilation.events", raw_bytes, 999_999);
    assert_eq!(seq, 0);

    // Receive envelope from subscriber
    let envelope = subscriber
        .try_recv()
        .expect("Subscriber must receive envelope");
    assert_eq!(envelope.topic, "assimilation.events");
    assert_eq!(envelope.sequence_id, 0);

    // Invoke reactive reducer without heap allocation in reduction
    let transitioned =
        handle_assimilation_event(&envelope.payload).expect("Reducer must process event");
    assert_eq!(transitioned.phase, AssimilationPhase::Quarantined as u32);
    assert_eq!(transitioned.source_id, [0xBB; 16]);
}
