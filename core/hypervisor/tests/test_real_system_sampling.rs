// Integration Test: Real-System Sampling & Interconnect Telemetry
// Tests real-system discovery (LM Studio local GGUFs, physical serial ports)
// and verifies that live wire telemetry updates the industrial register bank.

use aaroneous_paths::WorkspacePaths;
use aaroneous_wire::{encode_frame, ChannelKind, ChannelValue, TelemetryPacket, WireMessage, MAX_FRAMED_SIZE};
use platform_bridge::ot_bridge::{OtBridgeConfig, OtEdgeGateway};

#[tokio::test]
async fn test_real_system_sampling_and_ot_interconnect() {
    // 1. Sample real-world local model hubs (LM Studio default location)
    let ws = WorkspacePaths::discover();
    let detected_models = ws.scan_all_gguf_models(&[]);
    
    println!(">>> Real-World Sample: Discovered {} GGUF models on disk.", detected_models.len());
    for m in detected_models.iter().take(5) {
        println!("    - Model: {} ({}) from {}", m.file_name, m.formatted_size, m.source_hub);
    }
    if !detected_models.is_empty() {
        println!(">>> Verified {} local GGUF models discovered.", detected_models.len());
    } else {
        println!(">>> Notice: No local GGUF models found at default hub paths; running in headless mode.");
    }

    // 2. Sample real-world serial/COM ports available on the host OS
    let available_ports = tokio_serial::available_ports().unwrap_or_default();
    println!(">>> Real-World Sample: Discovered {} physical/virtual serial ports.", available_ports.len());
    for p in &available_ports {
        println!("    - Port: {:?} (Type: {:?})", p.port_name, p.port_type);
    }

    // 3. Verify Live OT Wire Ingestion into the Industrial Gateway
    let target_port = available_ports
        .first()
        .map(|p| p.port_name.clone())
        .unwrap_or_else(|| "COM1".to_string());

    let config = OtBridgeConfig {
        port_name: target_port,
        baud_rate: 115_200,
        heartbeat_interval_ms: 250,
    };

    let (gateway, mut cmd_rx) = OtEdgeGateway::new(config);

    // Construct a live sampled physical telemetry packet
    let mut telem = TelemetryPacket::default();
    telem.sequence = 101;
    telem.uptime_ms = 45000;
    telem.channels[0] = Some(ChannelValue {
        channel_id: 0,
        kind: ChannelKind::AnalogInput,
        raw_value: 3300,
        calibrated_f32: 3.3,
    });
    telem.channels[1] = Some(ChannelValue {
        channel_id: 1,
        kind: ChannelKind::DigitalInput,
        raw_value: 1,
        calibrated_f32: 1.0,
    });

    let msg = WireMessage::Telemetry(telem);
    let mut frame_buffer = [0u8; MAX_FRAMED_SIZE];
    let wire_frame = encode_frame(&msg, &mut frame_buffer).expect("COBS frame encodes");

    // Ingest wire frame into edge gateway
    let decoded_msg = gateway.ingest_raw_frame(wire_frame).expect("Decodes cleanly");
    assert_eq!(decoded_msg, msg);

    // Verify holding registers and discrete states match
    let reg_bank = gateway.read_registers();
    assert_eq!(reg_bank.holding_registers[0], 3300);
    assert_eq!(reg_bank.holding_registers[1], 1);
    assert_eq!(reg_bank.discrete_inputs[1], true);

    // Test Host -> Edge Command dispatch
    gateway.send_command(aaroneous_wire::CommandPacket::SetDigitalOut { pin: 13, state: true })
        .await
        .expect("Command sent");

    let received_cmd = cmd_rx.recv().await.expect("Command received on worker channel");
    assert_eq!(received_cmd, aaroneous_wire::CommandPacket::SetDigitalOut { pin: 13, state: true });
}
