//! protocol_bridge.rs
//! Machine-Native Linking Protocol (MNLP) patch serialization and dispatch for Adaptation Engine.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::mutation::PatchProposal;

/// Binary header for code mutation patch proposal packets
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MnlpPatchPacket {
    pub magic: u32,
    pub timestamp_us: u64,
    pub patch_id: u64,
    pub confidence_q16: u32, // Fixed-point 16.16 confidence representation
    pub payload_size: u32,
}

impl Default for MnlpPatchPacket {
    fn default() -> Self {
        Self {
            magic: 0x4348494D, // 'CHIM'
            timestamp_us: 0,
            patch_id: 0,
            confidence_q16: (0.95 * 65536.0) as u32,
            payload_size: 0,
        }
    }
}

/// Machine-Native Linking Protocol bridge for Adaptation Engine patch proposals
pub struct ChimeraProtocolBridge;

pub type MnlpProtocolBridge = ChimeraProtocolBridge;

impl ChimeraProtocolBridge {
    pub fn encode_patch(patch: &PatchProposal, patch_id: u64) -> Result<Vec<u8>> {
        let json_payload = serde_json::to_vec(patch)?;
        let header = MnlpPatchPacket {
            magic: 0x4348494D,
            timestamp_us: 0,
            patch_id,
            confidence_q16: (patch.confidence_score * 65536.0) as u32,
            payload_size: json_payload.len() as u32,
        };

        let mut bytes = Vec::with_capacity(std::mem::size_of::<MnlpPatchPacket>() + json_payload.len());
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const MnlpPatchPacket as *const u8,
                std::mem::size_of::<MnlpPatchPacket>(),
            )
        };
        bytes.extend_from_slice(header_bytes);
        bytes.extend_from_slice(&json_payload);

        Ok(bytes)
    }

    pub fn decode_patch(bytes: &[u8]) -> Result<PatchProposal> {
        let header_size = std::mem::size_of::<MnlpPatchPacket>();
        if bytes.len() < header_size {
            anyhow::bail!("Packet too small for Adaptation Engine MNLP header");
        }

        let payload = &bytes[header_size..];
        let patch: PatchProposal = serde_json::from_slice(payload)?;
        Ok(patch)
    }
}
