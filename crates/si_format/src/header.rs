//! crates/si_format/src/header.rs
//! Canonical SINT (Synthetic Intelligence Native Topology) Header Specification.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Magic identifier for Canonical `.si` Cartridges: 'SINT'
pub const SI_CANONICAL_MAGIC: [u8; 4] = *b"SINT";

/// Canonical Specification Version 3.0
pub const SI_CANONICAL_VERSION: u16 = 3;

/// 64-byte Header and SIMD Cache-Line Alignment Constant
pub const SI_HEADER_SIZE: usize = 64;

/// Tier Execution Capability Flags
pub const SI_FLAG_TIER_1_CORTEX: u32 = 0x0001;
pub const SI_FLAG_TIER_2_ROUTER: u32 = 0x0002;
pub const SI_FLAG_TIER_3_REFLEX: u32 = 0x0004;
pub const SI_FLAG_ENCRYPTED: u32 = 0x0010;
pub const SI_FLAG_COMPRESSED: u32 = 0x0020;

/// Tier Designation Flags defining CPU/memory execution profiles and routing topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiTierFlags(pub u32);

impl SiTierFlags {
    /// Tier 1: Strategic Cortex (HD R^4096 representation, background OS thread)
    pub const TIER_1_CORTEX: Self = Self(SI_FLAG_TIER_1_CORTEX);
    /// Tier 2: Orchestration / Router (R^256, connects to central SPMC hub)
    pub const TIER_2_ROUTER: Self = Self(SI_FLAG_TIER_2_ROUTER);
    /// Tier 3: Kinetic Specialist / Reflex (R^256, L1 cache priority, thread pinning)
    pub const TIER_3_REFLEX: Self = Self(SI_FLAG_TIER_3_REFLEX);

    #[inline]
    pub fn bits(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    #[inline]
    pub fn is_cortex(&self) -> bool {
        self.0 & Self::TIER_1_CORTEX.0 != 0
    }

    #[inline]
    pub fn is_router(&self) -> bool {
        self.0 & Self::TIER_2_ROUTER.0 != 0
    }

    #[inline]
    pub fn is_reflex(&self) -> bool {
        self.0 & Self::TIER_3_REFLEX.0 != 0
    }

    pub fn label(&self) -> &'static str {
        if self.is_cortex() {
            "Tier 1: Strategic Cortex (R^4096)"
        } else if self.is_router() {
            "Tier 2: Router (R^256)"
        } else if self.is_reflex() {
            "Tier 3: Kinetic Reflex (R^256)"
        } else {
            "Tier 3: Kinetic Reflex (Default)"
        }
    }
}

impl Default for SiTierFlags {
    fn default() -> Self {
        Self::TIER_3_REFLEX
    }
}

/// Standard Canonical Cartridge Header (64 Bytes, Little-Endian)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiCartridgeHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub header_size: u16,
    pub flags: u32,
    pub crc32_checksum: u32,
    pub block1_offset: u64,
    pub block1_len: u64,
    pub block2_offset: u64,
    pub block2_len: u64,
    pub block3_offset: u64,
    pub block3_len: u64,
}

impl Default for SiCartridgeHeader {
    fn default() -> Self {
        Self {
            magic: SI_CANONICAL_MAGIC,
            version: SI_CANONICAL_VERSION,
            header_size: SI_HEADER_SIZE as u16,
            flags: SI_FLAG_TIER_3_REFLEX,
            crc32_checksum: 0,
            block1_offset: 64,
            block1_len: 0,
            block2_offset: 64,
            block2_len: 0,
            block3_offset: 64,
            block3_len: 0,
        }
    }
}

impl SiCartridgeHeader {
    /// Encodes header into exactly 64 little-endian bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        buf[6..8].copy_from_slice(&self.header_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.flags.to_le_bytes());
        buf[12..16].copy_from_slice(&self.crc32_checksum.to_le_bytes());
        buf[16..24].copy_from_slice(&self.block1_offset.to_le_bytes());
        buf[24..32].copy_from_slice(&self.block1_len.to_le_bytes());
        buf[32..40].copy_from_slice(&self.block2_offset.to_le_bytes());
        buf[40..48].copy_from_slice(&self.block2_len.to_le_bytes());
        buf[48..56].copy_from_slice(&self.block3_offset.to_le_bytes());
        buf[56..64].copy_from_slice(&self.block3_len.to_le_bytes());
        buf
    }

    /// Decodes header from a byte slice with strict bounds and magic validation
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < SI_HEADER_SIZE {
            bail!(
                "Buffer too small for .si header: {} bytes (required {})",
                bytes.len(),
                SI_HEADER_SIZE
            );
        }

        let magic: [u8; 4] = bytes[0..4].try_into()?;
        if magic != SI_CANONICAL_MAGIC {
            bail!("Invalid .si magic bytes: {:?}", magic);
        }

        let version = u16::from_le_bytes(bytes[4..6].try_into()?);
        let header_size = u16::from_le_bytes(bytes[6..8].try_into()?);
        let flags = u32::from_le_bytes(bytes[8..12].try_into()?);
        let crc32_checksum = u32::from_le_bytes(bytes[12..16].try_into()?);
        let block1_offset = u64::from_le_bytes(bytes[16..24].try_into()?);
        let block1_len = u64::from_le_bytes(bytes[24..32].try_into()?);
        let block2_offset = u64::from_le_bytes(bytes[32..40].try_into()?);
        let block2_len = u64::from_le_bytes(bytes[40..48].try_into()?);
        let block3_offset = u64::from_le_bytes(bytes[48..56].try_into()?);
        let block3_len = u64::from_le_bytes(bytes[56..64].try_into()?);

        Ok(Self {
            magic,
            version,
            header_size,
            flags,
            crc32_checksum,
            block1_offset,
            block1_len,
            block2_offset,
            block2_len,
            block3_offset,
            block3_len,
        })
    }
}
