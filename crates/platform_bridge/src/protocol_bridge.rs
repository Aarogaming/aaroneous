//! protocol_bridge.rs
//! Machine-Native Linking Protocol (MNLP) adapter for Marionette.
//! Zero-copy serialization for high-throughput vision and telemetry.
//!
//! Dedupe-audit note (.audit/DEAD_CODE_ANALYSIS.md): two other crates carry a
//! file with this same name and a "Protocol Bridge" struct following the
//! same MNLP-branded naming convention —
//! `crates/adaptation_engine/src/protocol_bridge.rs` (binary patch-proposal
//! packets, `ChimeraProtocolBridge`) and `crates/omni/src/protocol_bridge.rs`
//! (JSON galaxy-snapshot serialization, `OmniProtocolBridge`). They were
//! reviewed together and are intentionally NOT merged here: each encodes a
//! wire format around a domain type owned by its own crate (this crate's
//! `VisualObservation`/`HidCommand` vs. adaptation_engine's `PatchProposal`
//! vs. omni's `GalaxyCluster`/`StarNode`), the three binary layouts are
//! mutually incompatible (different magics, field orders, and sizes), and
//! this crate is the OS/protocol-abstraction layer for the workspace — it
//! must not gain a dependency on adaptation_engine's or omni's domain types
//! just to host their unrelated wire formats. Only the shared name and
//! surface-level "static struct with encode/decode" shape are common; the
//! implementations are genuinely different and each stays in the crate that
//! owns its payload type.

#![deny(unsafe_code)]

use anyhow::{Result, anyhow};
use std::mem::size_of;

use crate::traits::{HidCommand, VisualObservation};

/// Fixed 32-byte header for binary perception packet
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MnlpPerceptionPacket {
    pub magic: [u8; 4],
    pub timestamp_us: u64,
    pub frame_id: u64,
    pub width: u16,
    pub height: u16,
    pub payload_size: u32,
    pub _reserved: [u8; 4],
}

impl MnlpPerceptionPacket {
    pub const MAGIC: [u8; 4] = *b"MNLP";
    pub const HEADER_SIZE: usize = size_of::<MnlpPerceptionPacket>();

    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let mut buf = [0u8; Self::HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..12].copy_from_slice(&self.timestamp_us.to_le_bytes());
        buf[12..20].copy_from_slice(&self.frame_id.to_le_bytes());
        buf[20..22].copy_from_slice(&self.width.to_le_bytes());
        buf[22..24].copy_from_slice(&self.height.to_le_bytes());
        buf[24..28].copy_from_slice(&self.payload_size.to_le_bytes());
        buf[28..32].copy_from_slice(&self._reserved);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err(anyhow!(
                "Packet too small: expected {}, got {}",
                Self::HEADER_SIZE,
                bytes.len()
            ));
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != Self::MAGIC {
            return Err(anyhow!("Invalid magic: {:?}", magic));
        }

        let mut ts_bytes = [0u8; 8];
        ts_bytes.copy_from_slice(&bytes[4..12]);
        let timestamp_us = u64::from_le_bytes(ts_bytes);

        let mut frame_bytes = [0u8; 8];
        frame_bytes.copy_from_slice(&bytes[12..20]);
        let frame_id = u64::from_le_bytes(frame_bytes);

        let mut w_bytes = [0u8; 2];
        w_bytes.copy_from_slice(&bytes[20..22]);
        let width = u16::from_le_bytes(w_bytes);

        let mut h_bytes = [0u8; 2];
        h_bytes.copy_from_slice(&bytes[22..24]);
        let height = u16::from_le_bytes(h_bytes);

        let mut size_bytes = [0u8; 4];
        size_bytes.copy_from_slice(&bytes[24..28]);
        let payload_size = u32::from_le_bytes(size_bytes);

        let mut res_bytes = [0u8; 4];
        res_bytes.copy_from_slice(&bytes[28..32]);

        Ok(Self {
            magic,
            timestamp_us,
            frame_id,
            width,
            height,
            payload_size,
            _reserved: res_bytes,
        })
    }
}

/// Bridges Marionette visual observations into machine-native byte slices
#[deprecated(note = "Use PlatformProtocolBridge instead")]
pub type MarionetteProtocolBridge = PlatformProtocolBridge;

/// Bridges Marionette visual observations into machine-native byte slices
pub struct PlatformProtocolBridge;

impl PlatformProtocolBridge {
    pub fn encode_perception(observation: &VisualObservation, frame_id: u64) -> Result<Vec<u8>> {
        let header = MnlpPerceptionPacket {
            magic: MnlpPerceptionPacket::MAGIC,
            timestamp_us: observation.timestamp_us,
            frame_id,
            width: observation.width as u16,
            height: observation.height as u16,
            payload_size: (observation.grid.len() * 4) as u32,
            _reserved: [0u8; 4],
        };

        let mut bytes =
            Vec::with_capacity(MnlpPerceptionPacket::HEADER_SIZE + observation.grid.len() * 4);
        bytes.extend_from_slice(&header.to_bytes());

        for &f in &observation.grid {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        Ok(bytes)
    }

    pub fn decode_perception(payload: &[u8]) -> Result<(MnlpPerceptionPacket, Vec<f32>)> {
        let header = MnlpPerceptionPacket::from_bytes(payload)?;
        let expected_floats = (header.payload_size / 4) as usize;
        let float_bytes = &payload[MnlpPerceptionPacket::HEADER_SIZE..];
        if float_bytes.len() < expected_floats * 4 {
            return Err(anyhow!(
                "Payload truncated: expected {} bytes, got {}",
                expected_floats * 4,
                float_bytes.len()
            ));
        }

        let mut grid = Vec::with_capacity(expected_floats);
        for chunk in float_bytes[..expected_floats * 4].as_chunks::<4>().0 {
            grid.push(f32::from_le_bytes(*chunk));
        }

        Ok((header, grid))
    }

    pub fn encode_hid_command(cmd: &HidCommand) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(cmd)?;
        Ok(bytes)
    }

    pub fn decode_hid_command(payload: &[u8]) -> Result<HidCommand> {
        let cmd: HidCommand = serde_json::from_slice(payload)?;
        Ok(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::HidAction;

    #[test]
    fn test_perception_packet_roundtrip() {
        let observation = VisualObservation::new(
            vec![
                0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6,
            ],
            4,
            4,
            42000,
        );

        let encoded = PlatformProtocolBridge::encode_perception(&observation, 99).unwrap();
        assert_eq!(encoded.len(), MnlpPerceptionPacket::HEADER_SIZE + 16 * 4);

        let (header, grid) = PlatformProtocolBridge::decode_perception(&encoded).unwrap();
        assert_eq!(header.frame_id, 99);
        assert_eq!(header.timestamp_us, 42000);
        assert_eq!(header.width, 4);
        assert_eq!(header.height, 4);
        assert_eq!(grid.len(), 16);
        assert_eq!(grid, observation.grid);
    }

    #[test]
    fn test_hid_command_json_roundtrip() {
        let cmd = HidCommand {
            actions: vec![HidAction::MouseMove {
                delta_x: 12,
                delta_y: -3,
            }],
            sequence_id: 1,
            timestamp_us: 1000,
        };
        let encoded = PlatformProtocolBridge::encode_hid_command(&cmd).unwrap();
        let decoded = PlatformProtocolBridge::decode_hid_command(&encoded).unwrap();
        assert_eq!(decoded.sequence_id, 1);
        assert_eq!(decoded.actions.len(), 1);
        match &decoded.actions[0] {
            HidAction::MouseMove { delta_x, delta_y } => {
                assert_eq!(*delta_x, 12);
                assert_eq!(*delta_y, -3);
            }
            _ => panic!("Expected MouseMove action"),
        }
    }
}
