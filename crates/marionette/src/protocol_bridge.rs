//! protocol_bridge.rs
//! Machine-Native Linking Protocol (MNLP) adapter for Marionette.
//! Marshals sensory frames and motor intents directly to binary packets.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::traits::{HidCommand, VisualObservation};

/// Binary packet for sensory frame broadcast over the Machine-Native Linking Protocol
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MnlpPerceptionPacket {
    pub magic: u32,
    pub timestamp_us: u64,
    pub frame_id: u64,
    pub width: u16,
    pub height: u16,
    pub payload_size: u32,
}

impl Default for MnlpPerceptionPacket {
    fn default() -> Self {
        Self {
            magic: 0x4141524F, // 'AARO'
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
            magic: 0x4141524F,
            timestamp_us: observation.timestamp_us,
            frame_id,
            width: observation.width as u16,
            height: observation.height as u16,
            payload_size: (observation.grid.len() * 4) as u32,
        };

        let mut bytes = Vec::with_capacity(std::mem::size_of::<MnlpPerceptionPacket>() + observation.grid.len() * 4);
        
        // Append raw float bytes
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const MnlpPerceptionPacket as *const u8,
                std::mem::size_of::<MnlpPerceptionPacket>(),
            )
        };
        bytes.extend_from_slice(header_bytes);

        for &f in &observation.grid {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        Ok(bytes)
    }

    pub fn decode_hid_command(payload: &[u8]) -> Result<HidCommand> {
        let cmd: HidCommand = serde_json::from_slice(payload)?;
        Ok(cmd)
    }
}
