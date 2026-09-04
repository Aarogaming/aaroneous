//! crates/adaptation_engine/src/protocol_bridge.rs
//! Machine-Native Linking Protocol (MNLP) patch serialization and dispatch for Adaptation Engine.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::mutation::PatchProposal;

/// Binary header for code mutation patch proposal packets (28 bytes)
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MnlpPatchPacket {
    pub magic: u32,
    pub timestamp_us: u64,
    pub patch_id: u64,
    pub confidence_q16: u32, // Fixed-point 16.16 confidence representation
    pub payload_size: u32,
}

impl MnlpPatchPacket {
    pub const MAGIC: u32 = 0x4348494D; // 'CHIM'
    pub const SIZE: usize = 4 + 8 + 8 + 4 + 4; // 28 bytes

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..12].copy_from_slice(&self.timestamp_us.to_le_bytes());
        buf[12..20].copy_from_slice(&self.patch_id.to_le_bytes());
        buf[20..24].copy_from_slice(&self.confidence_q16.to_le_bytes());
        buf[24..28].copy_from_slice(&self.payload_size.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            bail!(
                "Packet buffer too small: required {} bytes, got {}",
                Self::SIZE,
                bytes.len()
            );
        }

        let magic = u32::from_le_bytes(bytes[0..4].try_into()?);
        if magic != Self::MAGIC {
            bail!(
                "Invalid magic in MnlpPatchPacket: expected 0x{:08X}, got 0x{:08X}",
                Self::MAGIC,
                magic
            );
        }

        let timestamp_us = u64::from_le_bytes(bytes[4..12].try_into()?);
        let patch_id = u64::from_le_bytes(bytes[12..20].try_into()?);
        let confidence_q16 = u32::from_le_bytes(bytes[20..24].try_into()?);
        let payload_size = u32::from_le_bytes(bytes[24..28].try_into()?);

        Ok(Self {
            magic,
            timestamp_us,
            patch_id,
            confidence_q16,
            payload_size,
        })
    }
}

impl Default for MnlpPatchPacket {
    fn default() -> Self {
        Self {
            magic: Self::MAGIC,
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
            magic: MnlpPatchPacket::MAGIC,
            timestamp_us: 0,
            patch_id,
            confidence_q16: (patch.confidence_score * 65536.0) as u32,
            payload_size: json_payload.len() as u32,
        };

        let mut bytes = Vec::with_capacity(MnlpPatchPacket::SIZE + json_payload.len());
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&json_payload);

        Ok(bytes)
    }

    pub fn decode_patch(bytes: &[u8]) -> Result<PatchProposal> {
        let header = MnlpPatchPacket::from_bytes(bytes)?;
        let payload_start = MnlpPatchPacket::SIZE;
        let payload_end = payload_start + header.payload_size as usize;

        if bytes.len() < payload_end {
            bail!(
                "Truncated patch payload: expected {} bytes, have {}",
                payload_end,
                bytes.len()
            );
        }

        let payload = &bytes[payload_start..payload_end];
        let patch: PatchProposal = serde_json::from_slice(payload)?;
        Ok(patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_encoding_and_decoding_roundtrip() {
        let original_patch = PatchProposal {
            target_file: "crates/core/src/runtime.rs".to_string(),
            original_checksum: "0123456789abcdef".to_string(),
            patch_content: "pub fn patched() -> bool { true }".to_string(),
            confidence_score: 0.98,
            mutation_type: "safe_repair".to_string(),
        };

        let encoded = ChimeraProtocolBridge::encode_patch(&original_patch, 42).unwrap();
        assert!(encoded.len() > MnlpPatchPacket::SIZE);

        let decoded = ChimeraProtocolBridge::decode_patch(&encoded).unwrap();
        assert_eq!(decoded.target_file, original_patch.target_file);
        assert_eq!(decoded.patch_content, original_patch.patch_content);
        assert!((decoded.confidence_score - original_patch.confidence_score).abs() < 1e-4);
    }
}
