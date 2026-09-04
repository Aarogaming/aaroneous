//! protocol_bridge.rs
//! Machine-Native Linking Protocol (MNLP) adapter for Marionette.
//! Marshals sensory frames and motor intents directly to binary packets with zero unsafe code.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::traits::{HidCommand, VisualObservation};

/// Binary packet for sensory frame broadcast over the Machine-Native Linking Protocol
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MnlpPerceptionPacket {
    pub magic: u32,
    pub timestamp_us: u64,
    pub frame_id: u64,
    pub width: u16,
    pub height: u16,
    pub payload_size: u32,
}

impl MnlpPerceptionPacket {
    pub const HEADER_SIZE: usize = 32;
    pub const MAGIC: u32 = 0x4141524F; // 'AARO'

    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let mut buf = [0u8; Self::HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        // 4..8 reserved / 64-bit alignment padding
        buf[8..16].copy_from_slice(&self.timestamp_us.to_le_bytes());
        buf[16..24].copy_from_slice(&self.frame_id.to_le_bytes());
        buf[24..26].copy_from_slice(&self.width.to_le_bytes());
        buf[26..28].copy_from_slice(&self.height.to_le_bytes());
        buf[28..32].copy_from_slice(&self.payload_size.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err(anyhow!(
                "Buffer too small for MnlpPerceptionPacket header: {} < {}",
                bytes.len(),
                Self::HEADER_SIZE
            ));
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into()?);
        if magic != Self::MAGIC {
            return Err(anyhow!("Invalid MnlpPerceptionPacket magic: 0x{:08X}", magic));
        }
        let timestamp_us = u64::from_le_bytes(bytes[8..16].try_into()?);
        let frame_id = u64::from_le_bytes(bytes[16..24].try_into()?);
        let width = u16::from_le_bytes(bytes[24..26].try_into()?);
        let height = u16::from_le_bytes(bytes[26..28].try_into()?);
        let payload_size = u32::from_le_bytes(bytes[28..32].try_into()?);

        Ok(Self {
            magic,
            timestamp_us,
            frame_id,
            width,
            height,
            payload_size,
        })
    }
}

impl Default for MnlpPerceptionPacket {
    fn default() -> Self {
        Self {
            magic: Self::MAGIC,
            timestamp_us: 0,
            frame_id: 0,
            width: 128,
            height: 128,
            payload_size: (128 * 128 * 4) as u32,
        }
    }
}

/// Bridges Marionette visual observations into machine-native byte slices
pub struct MarionetteProtocolBridge;

impl MarionetteProtocolBridge {
    pub fn encode_perception(observation: &VisualObservation, frame_id: u64) -> Result<Vec<u8>> {
        let header = MnlpPerceptionPacket {
            magic: MnlpPerceptionPacket::MAGIC,
            timestamp_us: observation.timestamp_us,
            frame_id,
            width: observation.width as u16,
            height: observation.height as u16,
            payload_size: (observation.grid.len() * 4) as u32,
        };

        let mut bytes = Vec::with_capacity(MnlpPerceptionPacket::HEADER_SIZE + observation.grid.len() * 4);
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
        for chunk in float_bytes[..expected_floats * 4].chunks_exact(4) {
            grid.push(f32::from_le_bytes(chunk.try_into()?));
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
            vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6],
            4,
            4,
            42000,
        );

        let encoded = MarionetteProtocolBridge::encode_perception(&observation, 99).unwrap();
        assert_eq!(encoded.len(), MnlpPerceptionPacket::HEADER_SIZE + 16 * 4);

        let (header, grid) = MarionetteProtocolBridge::decode_perception(&encoded).unwrap();
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
            actions: vec![HidAction::MouseMove { delta_x: 12, delta_y: -3 }],
            sequence_id: 1,
            timestamp_us: 1000,
        };
        let encoded = MarionetteProtocolBridge::encode_hid_command(&cmd).unwrap();
        let decoded = MarionetteProtocolBridge::decode_hid_command(&encoded).unwrap();
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
