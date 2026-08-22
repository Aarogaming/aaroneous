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

    // Step 1: ODIN & DRAUPNIR (0x0100) - Intent Decomposition
    let pkt_odin = MnlpPacket {
        opcode: 0x0100,
        source: "orchestrator".to_string(),
        target: "odin".to_string(),
        correlation_id: 101,
        payload: b"Deconstruct and modernize entire target binary".to_vec(),
    };
    let res_odin = federation.dispatch_packet(pkt_odin).await.unwrap();
    assert!(res_odin.success);

    // Step 2: MERLIN & GRIMOIRE (0x0200) - Conceptual & Mathematical Synthesis
    let pkt_merlin = MnlpPacket {
        opcode: 0x0200,
        source: "odin".to_string(),
        target: "merlin".to_string(),
        correlation_id: 102,
        payload: b"Synthesize memory layout and algebraic graph".to_vec(),
    };
    let res_merlin = federation.dispatch_packet(pkt_merlin).await.unwrap();
    assert!(res_merlin.success);

    // Step 3: ARIEL & GLASS (0x0300) - UI Telemetry & Visual State Composition
    let pkt_ariel = MnlpPacket {
        opcode: 0x0300,
        source: "merlin".to_string(),
        target: "ariel".to_string(),
        correlation_id: 103,
        payload: b"Compose telemetry HUD for active operation".to_vec(),
    };
    let res_ariel = federation.dispatch_packet(pkt_ariel).await.unwrap();
    assert!(res_ariel.success);

    // Step 4: HEPHAESTUS & FORGE (0x0400) - Native AST Mutation & Tool Forging
    let pkt_hephaestus = MnlpPacket {
        opcode: 0x0400,
        source: "odin".to_string(),
        target: "hephaestus".to_string(),
        correlation_id: 104,
        payload: b"Forge zero-copy SWMR adapter".to_vec(),
    };
    let res_hephaestus = federation.dispatch_packet(pkt_hephaestus).await.unwrap();
    assert!(res_hephaestus.success);

    // Step 5: ARGUS & SENTINEL (0x0500) - Zero-Trust Safety & Constraint Audit
    let pkt_argus = MnlpPacket {
        opcode: 0x0500,
        source: "hephaestus".to_string(),
        target: "argus".to_string(),
        correlation_id: 105,
        payload: b"Audit forged adapter against sandbox security boundaries".to_vec(),
    };
    let res_argus = federation.dispatch_packet(pkt_argus).await.unwrap();
    assert!(res_argus.success);

    // Step 6: DIONYSUS & OMNI (0x0600) - Semantic Memory & 3D Star-Node Consolidation
    let pkt_dionysus = MnlpPacket {
        opcode: 0x0600,
        source: "argus".to_string(),
        target: "dionysus".to_string(),
        correlation_id: 106,
        payload: b"Consolidate memory node into 3D Galaxy constellation".to_vec(),
    };
    let res_dionysus = federation.dispatch_packet(pkt_dionysus).await.unwrap();
    assert!(res_dionysus.success);

    // Step 7: HERMES & CADUCEUS (0x0700) - High-Speed NATS Distributed Message Dispatch
    let pkt_hermes = MnlpPacket {
        opcode: 0x0700,
        source: "dionysus".to_string(),
        target: "hermes".to_string(),
        correlation_id: 107,
        payload: b"Broadcast state sync packet across mesh peers".to_vec(),
    };
    let res_hermes = federation.dispatch_packet(pkt_hermes).await.unwrap();
    assert!(res_hermes.success);

    // Step 8: WEN & RESONANCE (0x0800) - Cognitive Balance & Resonance Alignment
    let pkt_wen = MnlpPacket {
        opcode: 0x0800,
        source: "hermes".to_string(),
        target: "wen".to_string(),
        correlation_id: 108,
        payload: b"Assess cognitive load and thermodynamic resonance".to_vec(),
    };
    let res_wen = federation.dispatch_packet(pkt_wen).await.unwrap();
    assert!(res_wen.success);

    // Step 9: KAMI & THRESHOLD (0x0900) - Autonomous Perception & Threshold Gating
    let pkt_kami = MnlpPacket {
        opcode: 0x0900,
        source: "wen".to_string(),
        target: "kami".to_string(),
        correlation_id: 109,
        payload: b"Evaluate autonomous perception threshold".to_vec(),
    };
    let res_kami = federation.dispatch_packet(pkt_kami).await.unwrap();
    assert!(res_kami.success);

    println!("[ORCHESTRATION TEST] 9/9 Sovereign Specialists fully coordinated in sequence.");
}

#[tokio::test]
async fn test_hephaestus_auto_wrap_tool_integration() {
    let mut federation = SpecialistFederation::new();

    // Send MNLP packet with "wrap:" directive to Hephaestus (0x0400)
    let pkt_wrap = MnlpPacket {
        opcode: 0x0400,
        source: "odin".to_string(),
        target: "hephaestus".to_string(),
        correlation_id: 201,
        payload: b"wrap: target_util.exe".to_vec(),
    };

    let res_wrap = federation.dispatch_packet(pkt_wrap).await.unwrap();
    assert!(res_wrap.success);
    assert!(res_wrap.message.contains("Hephaestus successfully forged organ"));
    assert_eq!(res_wrap.opcode, 0x0400);
}

#[tokio::test]
async fn test_hermes_multi_node_swarm_mesh_cluster() {
    let mut node_prime = SpecialistFederation::new();
    let mut node_alpha = SpecialistFederation::new();
    let mut node_beta = SpecialistFederation::new();

    // 1. Prime initiates gossip broadcast across swarm
    let peers = vec!["node_alpha", "node_beta"];
    let gossip_results = node_prime.hermes.broadcast_gossip_pulse(&peers);
    assert_eq!(gossip_results.len(), 2);
    assert!(gossip_results.iter().all(|p| p.is_connected));
    assert!(gossip_results.iter().all(|p| p.latency_ms > 0.0 && p.latency_ms < 5.0));

    // 2. Nodes exchange domain capabilities
    let prime_synced = node_alpha.hermes.sync_swarm_manifest("node_prime", &["odin", "hephaestus", "kami"]);
    let alpha_synced = node_beta.hermes.sync_swarm_manifest("node_alpha", &["merlin", "ariel", "argus"]);
    assert!(prime_synced);
    assert!(alpha_synced);

    // 3. Dispatch cross-node offload request from Prime to Beta via Hermes
    let offload_state = node_prime.hermes.route_task_offload(0x0500, "node_beta");
    assert!(offload_state.is_connected);
    assert_eq!(offload_state.peer_node_id, "node_beta");
    assert!(offload_state.latency_ms < 1.0);
}
