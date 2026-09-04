//! # Industrial OT Bridge & Protocol Translator
//! Ingests byte streams over serial/USB, decodes COBS framed `aaroneous_wire` packets,
//! maintains industrial register states, and exposes telemetry.

use aaroneous_wire::{decode_frame, CommandPacket, TelemetryPacket, WireMessage};
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Simulated or physical transport interface.
#[derive(Debug, Clone)]
pub struct OtBridgeConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub heartbeat_interval_ms: u64,
}

impl Default for OtBridgeConfig {
    fn default() -> Self {
        Self {
            port_name: "VIRTUAL_COM1".to_string(),
            baud_rate: 115_200,
            heartbeat_interval_ms: 250,
        }
    }
}

/// Thread-safe industrial register state store.
#[derive(Debug)]
pub struct IndustrialRegisterBank {
    pub holding_registers: [u16; 64],
    pub discrete_inputs: [bool; 32],
    pub last_telemetry: Option<TelemetryPacket>,
}

impl Default for IndustrialRegisterBank {
    fn default() -> Self {
        Self {
            holding_registers: [0u16; 64],
            discrete_inputs: [false; 32],
            last_telemetry: None,
        }
    }
}

/// IT/OT Gateway Host Bridge managing communication with edge MCUs/PLCs.
pub struct OtEdgeGateway {
    pub config: OtBridgeConfig,
    registers: Arc<RwLock<IndustrialRegisterBank>>,
    cmd_tx: mpsc::Sender<CommandPacket>,
}

impl OtEdgeGateway {
    pub fn new(config: OtBridgeConfig) -> (Self, mpsc::Receiver<CommandPacket>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let registers = Arc::new(RwLock::new(IndustrialRegisterBank::default()));

        (
            Self {
                config,
                registers,
                cmd_tx,
            },
            cmd_rx,
        )
    }

    /// Access snapshot of current industrial registers.
    pub fn read_registers(&self) -> IndustrialRegisterBank {
        let guard = self.registers.read();
        IndustrialRegisterBank {
            holding_registers: guard.holding_registers,
            discrete_inputs: guard.discrete_inputs,
            last_telemetry: guard.last_telemetry.clone(),
        }
    }

    /// Update internal state from an ingested `TelemetryPacket`.
    pub fn ingest_telemetry(&self, packet: TelemetryPacket) {
        let mut guard = self.registers.write();
        
        // Map channels to Modbus holding registers & discrete flags
        for ch_opt in packet.channels.iter().flatten() {
            let idx = ch_opt.channel_id as usize;
            if idx < guard.holding_registers.len() {
                guard.holding_registers[idx] = (ch_opt.raw_value & 0xFFFF) as u16;
            }
            if idx < guard.discrete_inputs.len() {
                guard.discrete_inputs[idx] = ch_opt.raw_value > 0;
            }
        }
        
        guard.last_telemetry = Some(packet);
    }

    /// Ingest a raw framed slice coming from serial or socket.
    pub fn ingest_raw_frame(&self, raw_frame: &[u8]) -> Result<WireMessage> {
        let message = decode_frame(raw_frame)
            .map_err(|e| anyhow!("Failed to decode wire frame: {:?}", e))?;

        if let WireMessage::Telemetry(ref telem) = message {
            self.ingest_telemetry(telem.clone());
        }

        Ok(message)
    }

    /// Queue a command packet for transmission to the edge node.
    pub async fn send_command(&self, cmd: CommandPacket) -> Result<()> {
        self.cmd_tx.send(cmd).await.map_err(|e| anyhow!("Failed to queue command: {}", e))
    }

    /// Directly update a holding register value
    pub fn set_holding_register(&self, index: usize, value: u16) -> Result<()> {
        let mut guard = self.registers.write();
        if index < guard.holding_registers.len() {
            guard.holding_registers[index] = value;
            Ok(())
        } else {
            Err(anyhow!("Holding register index {} out of bounds", index))
        }
    }

    /// Directly update a discrete input flag
    pub fn set_discrete_input(&self, index: usize, value: bool) -> Result<()> {
        let mut guard = self.registers.write();
        if index < guard.discrete_inputs.len() {
            guard.discrete_inputs[index] = value;
            Ok(())
        } else {
            Err(anyhow!("Discrete input index {} out of bounds", index))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aaroneous_wire::{encode_frame, ChannelKind, ChannelValue, MAX_FRAMED_SIZE};

    #[test]
    fn test_ot_gateway_telemetry_ingestion() {
        let (gateway, _rx) = OtEdgeGateway::new(OtBridgeConfig::default());

        let mut pkt = TelemetryPacket::default();
        pkt.sequence = 1;
        pkt.channels[0] = Some(ChannelValue {
            channel_id: 0,
            kind: ChannelKind::AnalogInput,
            raw_value: 512,
            calibrated_f32: 2.5,
        });
        pkt.channels[1] = Some(ChannelValue {
            channel_id: 1,
            kind: ChannelKind::DigitalInput,
            raw_value: 1,
            calibrated_f32: 1.0,
        });

        let mut frame_buf = [0u8; MAX_FRAMED_SIZE];
        let frame = encode_frame(&WireMessage::Telemetry(pkt), &mut frame_buf).unwrap();

        let decoded = gateway.ingest_raw_frame(frame).expect("Decodes cleanly");
        assert!(matches!(decoded, WireMessage::Telemetry(_)));

        let state = gateway.read_registers();
        assert_eq!(state.holding_registers[0], 512);
        assert_eq!(state.holding_registers[1], 1);
        assert_eq!(state.discrete_inputs[1], true);
    }

    #[test]
    fn test_ot_gateway_register_mutation() {
        let (gateway, _rx) = OtEdgeGateway::new(OtBridgeConfig::default());

        assert!(gateway.set_holding_register(5, 1234).is_ok());
        assert!(gateway.set_discrete_input(10, true).is_ok());

        let state = gateway.read_registers();
        assert_eq!(state.holding_registers[5], 1234);
        assert_eq!(state.discrete_inputs[10], true);

        // Bounds check errors
        assert!(gateway.set_holding_register(100, 1).is_err());
        assert!(gateway.set_discrete_input(100, false).is_err());
    }
}
