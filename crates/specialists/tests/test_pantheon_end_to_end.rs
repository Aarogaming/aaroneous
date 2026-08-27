//! crates/specialists/tests/test_pantheon_end_to_end.rs
//! End-to-End Orchestration Integration Test across the 9 Federated Specialists and Relic Substrates.

use specialists::{MnlpPacket, SpecialistFederation};

#[tokio::test]
async fn test_full_specialist_federation_orchestration_cycle() {
    let mut federation = SpecialistFederation::new();

    // Verify all 9 specialists are online and healthy
    let health_map = federation.collect_health_reports();
    assert_eq!(health_map.len(), 9);
    for (name, health) in &health_map {
        assert!(!health.is_dormant, "Specialist {} is dormant", name);
        assert!(health.tokens > 0.0, "Specialist {} has no metabolic tokens", name);
    }

    // Step 1: ORCHESTRATOR & DRAUPNIR (0x0100) - Intent Decomposition
    let pkt_orchestrator = MnlpPacket {
        opcode: 0x0100,
        source: "orchestrator".to_string(),
        target: "orchestrator".to_string(),
        correlation_id: 101,
        payload: b"Deconstruct and modernize entire target binary".to_vec(),
    };
    let res_orchestrator = federation.dispatch_packet(pkt_orchestrator).await.unwrap();
    assert!(res_orchestrator.success);

    // Step 2: SYNTHESIZER & GRIMOIRE (0x0200) - Conceptual & Mathematical Synthesis
    let pkt_synthesizer = MnlpPacket {
        opcode: 0x0200,
        source: "orchestrator".to_string(),
        target: "synthesizer".to_string(),
        correlation_id: 102,
        payload: b"Synthesize memory layout and algebraic graph".to_vec(),
    };
    let res_synthesizer = federation.dispatch_packet(pkt_synthesizer).await.unwrap();
    assert!(res_synthesizer.success);

    // Step 3: PRESENTER & GLASS (0x0300) - UI Telemetry & Visual State Composition
    let pkt_presenter = MnlpPacket {
        opcode: 0x0300,
        source: "synthesizer".to_string(),
        target: "presenter".to_string(),
        correlation_id: 103,
        payload: b"Compose telemetry HUD for active operation".to_vec(),
    };
    let res_presenter = federation.dispatch_packet(pkt_presenter).await.unwrap();
    assert!(res_presenter.success);

    // Step 4: FABRICATOR & FORGE (0x0400) - Native AST Mutation & Tool Forging
    let pkt_fabricator = MnlpPacket {
        opcode: 0x0400,
        source: "orchestrator".to_string(),
        target: "fabricator".to_string(),
        correlation_id: 104,
        payload: b"Forge zero-copy SWMR adapter".to_vec(),
    };
    let res_fabricator = federation.dispatch_packet(pkt_fabricator).await.unwrap();
    assert!(res_fabricator.success);

    // Step 5: SENTINEL & SENTINEL (0x0500) - Zero-Trust Safety & Constraint Audit
    let pkt_sentinel = MnlpPacket {
        opcode: 0x0500,
        source: "fabricator".to_string(),
        target: "sentinel".to_string(),
        correlation_id: 105,
        payload: b"Audit forged adapter against sandbox security boundaries".to_vec(),
    };
    let res_sentinel = federation.dispatch_packet(pkt_sentinel).await.unwrap();
    assert!(res_sentinel.success);

    // Step 6: ARCHIVIST & OMNI (0x0600) - Semantic Memory & 3D Star-Node Consolidation
    let pkt_archivist = MnlpPacket {
        opcode: 0x0600,
        source: "sentinel".to_string(),
        target: "archivist".to_string(),
        correlation_id: 106,
        payload: b"Consolidate memory node into 3D Galaxy constellation".to_vec(),
    };
    let res_archivist = federation.dispatch_packet(pkt_archivist).await.unwrap();
    assert!(res_archivist.success);

    // Step 7: ROUTER & CADUCEUS (0x0700) - High-Speed NATS Distributed Message Dispatch
    let pkt_router = MnlpPacket {
        opcode: 0x0700,
        source: "archivist".to_string(),
        target: "router".to_string(),
        correlation_id: 107,
        payload: b"Broadcast state sync packet across mesh peers".to_vec(),
    };
    let res_router = federation.dispatch_packet(pkt_router).await.unwrap();
    assert!(res_router.success);

    // Step 8: ALIGNER & RESONANCE (0x0800) - Cognitive Balance & Resonance Alignment
    let pkt_aligner = MnlpPacket {
        opcode: 0x0800,
        source: "router".to_string(),
        target: "aligner".to_string(),
        correlation_id: 108,
        payload: b"Assess cognitive load and thermodynamic resonance".to_vec(),
    };
    let res_aligner = federation.dispatch_packet(pkt_aligner).await.unwrap();
    assert!(res_aligner.success);

    // Step 9: PERCEIVER & THRESHOLD (0x0900) - Autonomous Perception & Threshold Gating
    let pkt_perceiver = MnlpPacket {
        opcode: 0x0900,
        source: "aligner".to_string(),
        target: "perceiver".to_string(),
        correlation_id: 109,
        payload: b"Evaluate autonomous perception threshold".to_vec(),
    };
    let res_perceiver = federation.dispatch_packet(pkt_perceiver).await.unwrap();
    assert!(res_perceiver.success);

    println!("[ORCHESTRATION TEST] 9/9 Sovereign Specialists fully coordinated in sequence.");
}

#[tokio::test]
async fn test_hephaestus_auto_wrap_tool_integration() {
    let mut federation = SpecialistFederation::new();

    // Send MNLP packet with "wrap:" directive to Fabricator (0x0400)
    let pkt_wrap = MnlpPacket {
        opcode: 0x0400,
        source: "orchestrator".to_string(),
        target: "fabricator".to_string(),
        correlation_id: 201,
        payload: b"wrap: target_util.exe".to_vec(),
    };

    let res_wrap = federation.dispatch_packet(pkt_wrap).await.unwrap();
    assert!(res_wrap.success);
    assert!(res_wrap.message.contains("Fabricator successfully forged organ"));
    assert_eq!(res_wrap.opcode, 0x0400);
}

#[tokio::test]
async fn test_hermes_multi_node_swarm_mesh_cluster() {
    let mut node_prime = SpecialistFederation::new();
    let mut node_alpha = SpecialistFederation::new();
    let mut node_beta = SpecialistFederation::new();

    // 1. Prime initiates gossip broadcast across swarm
    let peers = vec!["node_alpha", "node_beta"];
    let gossip_results = node_prime.router.broadcast_gossip_pulse(&peers);
    assert_eq!(gossip_results.len(), 2);
    assert!(gossip_results.iter().all(|p| p.is_connected));
    assert!(gossip_results.iter().all(|p| p.latency_ms > 0.0 && p.latency_ms < 5.0));

    // 2. Nodes exchange domain capabilities
    let prime_synced = node_alpha.router.sync_swarm_manifest("node_prime", &["orchestrator", "fabricator", "perceiver"]);
    let alpha_synced = node_beta.router.sync_swarm_manifest("node_alpha", &["synthesizer", "presenter", "sentinel"]);
    assert!(prime_synced);
    assert!(alpha_synced);

    // 3. Dispatch cross-node offload request from Prime to Beta via Router
    let offload_state = node_prime.router.route_task_offload(0x0500, "node_beta");
    assert!(offload_state.is_connected);
    assert_eq!(offload_state.peer_node_id, "node_beta");
    assert!(offload_state.latency_ms < 1.0);
}
