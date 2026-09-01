//! crates/platform_bridge/tests/observability_test.rs
//! End-to-end integration tests for UIA Tree Walker and WASAPI Audio Loopback Ingestion.

use platform_bridge::observability::{
    UiaElementNode, UiaTreeWalker, WasapiCaptureConfig, WasapiLoopbackCapture,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_uia_tree_walker_integration() {
    let mut custom_window = UiaElementNode::new(
        "Code Editor Pro",
        "Window",
        (100.0, 100.0, 1200.0, 800.0),
        true,
        true,
    );

    let mut sidebar = UiaElementNode::new(
        "Project Explorer",
        "Pane",
        (100.0, 100.0, 300.0, 800.0),
        false,
        true,
    );
    sidebar.add_child(UiaElementNode::new(
        "main.rs File Item",
        "ListItem",
        (110.0, 140.0, 280.0, 24.0),
        false,
        true,
    ));

    custom_window.add_child(sidebar);

    let walker = UiaTreeWalker::new_mock(Some(custom_window));
    let root = walker.walk_window_tree(0x5678).expect("Walk tree failed");

    assert_eq!(root.name, "Code Editor Pro");
    assert_eq!(root.children.len(), 1);

    // Hit test on main.rs
    let hit = root.find_element_at_point(150.0, 150.0);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().name, "main.rs File Item");
    assert_eq!(hit.unwrap().control_type, "ListItem");

    // Query elements
    let file_items = root.find_elements_by_name("main.rs");
    assert_eq!(file_items.len(), 1);
}

#[test]
fn test_wasapi_loopback_capture_streaming_integration() {
    let mut capture = WasapiLoopbackCapture::new(WasapiCaptureConfig {
        sample_rate: 44100,
        channels: 2,
        buffer_size_frames: 441,
        capture_interval_ms: 10,
    });

    capture.start().expect("Capture start failed");
    assert!(capture.is_active());

    thread::sleep(Duration::from_millis(50));

    let samples = capture.drain_samples();
    assert!(!samples.is_empty(), "Should capture streaming PCM samples");

    let _event = capture.poll_latest_event();
    let latent = capture.poll_latest_latent();
    assert!(latent.is_some(), "Should extract 256-D acoustic latent vector");

    capture.stop().expect("Capture stop failed");
    assert!(!capture.is_active());
}

#[test]
fn test_acoustic_latent_to_reflex_matcher_closed_loop() {
    use compute::episodic_memory::{AcousticReflexMatcher, EpisodicMemoryFabric, TrajectoryMetadata};
    use platform_bridge::observability::{AcousticFeatureExtractor, FFT_SIZE};
    use std::sync::Arc;

    let mut extractor = AcousticFeatureExtractor::new(48000);

    // 1. Generate 440Hz tone and extract 256-D latent
    let mut pcm = Vec::with_capacity(FFT_SIZE);
    for i in 0..FFT_SIZE {
        let t = i as f32 / 48000.0;
        let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.4;
        pcm.push(sample);
    }
    let (target_latent, _) = extractor.process_frame(&pcm).unwrap();

    // 2. Index motor reflex trajectory in EpisodicMemoryFabric
    let fabric = Arc::new(EpisodicMemoryFabric::default());
    fabric
        .insert_trajectory(
            9001,
            target_latent.as_slice(),
            TrajectoryMetadata {
                skill_id: 0x05,
                trajectory_id: 9001,
                action_summary: "Trigger Sound Cue Action [F]".to_string(),
                thermodynamic_free_energy: 0.005,
                crystallized_handle_idx: Some(1),
                timestamp_ms: 123456,
            },
        )
        .unwrap();

    // 3. Match incoming tone
    let matcher = AcousticReflexMatcher::new(fabric, 0.90);
    let matched_reflex = matcher.match_acoustic_reflex(target_latent.as_slice());
    assert!(matched_reflex.is_some());
    let reflex = matched_reflex.unwrap();
    assert_eq!(reflex.id, 9001);
    assert_eq!(reflex.metadata.action_summary, "Trigger Sound Cue Action [F]");
    assert!(reflex.similarity > 0.95);
}

#[test]
fn test_etw_kernel_consumer_integration() {
    use platform_bridge::observability::{EtwKernelConsumer, KernelTraceEvent};

    let mut etw = EtwKernelConsumer::new_mock();
    etw.start().expect("Failed to start ETW consumer");
    assert!(etw.is_active());

    // Ingest simulated kernel events
    etw.push_event(KernelTraceEvent::ProcessStart {
        pid: 7777,
        ppid: 1000,
        image_name: "devenv.exe".to_string(),
        command_line: "devenv /debug".to_string(),
        timestamp_ms: 2000,
    });
    etw.push_event(KernelTraceEvent::WindowFocusChanged {
        hwnd: 0x9999,
        title: "Visual Studio 2026".to_string(),
        pid: 7777,
        timestamp_ms: 2050,
    });
    etw.push_event(KernelTraceEvent::FileCreate {
        path: "d:/Aaroneous/src/lib.rs".to_string(),
        pid: 7777,
        timestamp_ms: 2060,
    });

    assert_eq!(etw.len(), 3);

    // Filter by PID
    let vs_events = etw.filter_events_by_pid(7777);
    assert_eq!(vs_events.len(), 3);
    assert_eq!(vs_events[0].timestamp_ms(), 2000);

    // Poll recent 2
    let recent = etw.poll_recent_events(2);
    assert_eq!(recent.len(), 2);
    assert!(matches!(recent[0], KernelTraceEvent::WindowFocusChanged { .. }));

    // Drain
    let all = etw.drain_events();
    assert_eq!(all.len(), 3);
    assert!(etw.is_empty());

    etw.stop().expect("Failed to stop ETW consumer");
    assert!(!etw.is_active());
}
