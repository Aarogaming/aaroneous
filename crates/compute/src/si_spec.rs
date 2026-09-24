//! crates/compute/src/si_spec.rs
//! Canonical Sovereign `.si` Cartridge Binary Standard (SINT v3.0).
//!
//! Specifications & Memory Layout:
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │ HEADER (64 Bytes Aligned)                                              │
//! │ 0..4    Magic 'SINT' [0x53, 0x49, 0x4E, 0x54]                          │
//! │ 4..6    Version = 3 (u16 LE)                                           │
//! │ 6..8    Header Size = 64 (u16 LE)                                      │
//! │ 8..12   Flags (u32 LE: Tier 1 Cortex / Tier 2 Router / Tier 3 Reflex)  │
//! │ 12..16  CRC32 Payload Checksum (u32 LE)                                │
//! │ 16..24  Block 1 Offset (u64 LE) - Frozen Core SSM Weights              │
//! │ 24..32  Block 1 Length (u64 LE)                                        │
//! │ 32..40  Block 2 Offset (u64 LE) - Dynamic Adaptation Matrix            │
//! │ 40..48  Block 2 Length (u64 LE)                                        │
//! │ 48..56  Block 3 Offset (u64 LE) - Episodic Skill Stack & Habits        │
//! │ 56..64  Block 3 Length (u64 LE)                                        │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │ [BLOCK 1: FROZEN CORE SSM WEIGHTS] (64-byte aligned, zero-copy mmap)   │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │ [BLOCK 2: DYNAMIC ADAPTATION MATRIX] (Streaming LoRA delta / TD(λ))    │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │ [BLOCK 3: EPISODIC SKILL STACK] (Mined AST DAGs, habits, fast-reflex)  │
//! └────────────────────────────────────────────────────────────────────────┘

use anyhow::{Result, bail};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub use si_format::header::{
    SI_CANONICAL_MAGIC, SI_CANONICAL_VERSION, SI_FLAG_COMPRESSED, SI_FLAG_ENCRYPTED,
    SI_FLAG_TIER_1_CORTEX, SI_FLAG_TIER_2_ROUTER, SI_FLAG_TIER_3_REFLEX, SI_HEADER_SIZE,
    SiCartridgeHeader, SiTierFlags,
};

/// Simple CRC32 computation for payload integrity verification
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = -(crc as i32 & 1) as u32;
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Comprehensive Verification and Inspection Report for `.si` Cartridges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiCartridgeReport {
    pub file_path: String,
    pub is_valid: bool,
    pub version: u16,
    pub flags: u32,
    pub is_cortex: bool,
    pub is_router: bool,
    pub is_reflex: bool,
    pub total_bytes: usize,
    pub block1_bytes: usize,
    pub block2_bytes: usize,
    pub block3_bytes: usize,
    pub crc32_match: bool,
    pub mount_time_us: f64,
    pub issues: Vec<String>,
}

/// Deconstructed Representation of a `.si` Cartridge for Inspection & Packing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiCartridgeDeconstructed {
    pub header: SiCartridgeHeader,
    pub block1_core_weights: Vec<u8>,
    pub block2_dynamic_adapter: Vec<u8>,
    pub block3_skill_stack: Vec<u8>,
}

/// Cartridge Comparison / Diff Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiCartridgeDiffReport {
    pub path_a: String,
    pub path_b: String,
    pub version_match: bool,
    pub flags_match: bool,
    pub core_weights_identical: bool,
    pub adapter_drift_bytes: usize,
    pub skill_count_delta: i64,
}

/// Canonical `.si` Cartridge Tooling & Verification Engine
pub struct SiCartridgeEngine;

impl SiCartridgeEngine {
    /// Builds and writes a canonical `.si` cartridge with strict 64-byte alignment and CRC32 verification
    pub fn pack_cartridge(
        core_weights: &[u8],
        dynamic_adapter: &[u8],
        skill_stack: &[u8],
        tier_flags: u32,
        out_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let path = out_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Align each block boundary to 64 bytes
        let b1_offset = 64u64;
        let b1_len = core_weights.len() as u64;

        let b1_padding = (64 - (b1_len as usize % 64)) % 64;
        let b2_offset = b1_offset + b1_len + (b1_padding as u64);
        let b2_len = dynamic_adapter.len() as u64;

        let b2_padding = (64 - (b2_len as usize % 64)) % 64;
        let b3_offset = b2_offset + b2_len + (b2_padding as u64);
        let b3_len = skill_stack.len() as u64;

        // Build payload buffer to compute CRC32
        let mut payload = Vec::new();
        payload.extend_from_slice(core_weights);
        payload.extend_from_slice(&vec![0u8; b1_padding]);
        payload.extend_from_slice(dynamic_adapter);
        payload.extend_from_slice(&vec![0u8; b2_padding]);
        payload.extend_from_slice(skill_stack);

        let crc = compute_crc32(&payload);

        let header = SiCartridgeHeader {
            magic: SI_CANONICAL_MAGIC,
            version: SI_CANONICAL_VERSION,
            header_size: SI_HEADER_SIZE as u16,
            flags: tier_flags,
            crc32_checksum: crc,
            block1_offset: b1_offset,
            block1_len: b1_len,
            block2_offset: b2_offset,
            block2_len: b2_len,
            block3_offset: b3_offset,
            block3_len: b3_len,
        };

        let mut file = File::create(path)?;
        file.write_all(&header.to_bytes())?;
        file.write_all(&payload)?;

        Ok(path.to_path_buf())
    }

    /// Verifies, lints, and benchmarks a `.si` cartridge via zero-copy memory mapping
    pub fn verify_cartridge(path: impl AsRef<Path>) -> Result<SiCartridgeReport> {
        let path = path.as_ref();
        let start = std::time::Instant::now();

        if !path.exists() {
            bail!("Cartridge file not found: {:?}", path);
        }

        let file = File::open(path)?;
        // SAFETY: `file` is a fresh handle this call just opened; `mmap2`'s
        // precondition is that it isn't concurrently modified, and `mmap`
        // is only read from for the rest of this function.
        let mmap = unsafe { Mmap::map(&file)? };
        let mount_time_us = start.elapsed().as_secs_f64() * 1_000_000.0;

        let mut issues = Vec::new();
        let total_bytes = mmap.len();

        if total_bytes < SI_HEADER_SIZE {
            issues.push(format!(
                "File smaller than 64-byte header: {} bytes",
                total_bytes
            ));
            return Ok(SiCartridgeReport {
                file_path: path.display().to_string(),
                is_valid: false,
                version: 0,
                flags: 0,
                is_cortex: false,
                is_router: false,
                is_reflex: false,
                total_bytes,
                block1_bytes: 0,
                block2_bytes: 0,
                block3_bytes: 0,
                crc32_match: false,
                mount_time_us,
                issues,
            });
        }

        let header = match SiCartridgeHeader::from_bytes(&mmap[0..SI_HEADER_SIZE]) {
            Ok(h) => h,
            Err(e) => {
                issues.push(format!("Header decode error: {}", e));
                return Ok(SiCartridgeReport {
                    file_path: path.display().to_string(),
                    is_valid: false,
                    version: 0,
                    flags: 0,
                    is_cortex: false,
                    is_router: false,
                    is_reflex: false,
                    total_bytes,
                    block1_bytes: 0,
                    block2_bytes: 0,
                    block3_bytes: 0,
                    crc32_match: false,
                    mount_time_us,
                    issues,
                });
            }
        };

        // Check payload CRC32
        let payload = &mmap[SI_HEADER_SIZE..];
        let actual_crc = compute_crc32(payload);
        let crc_match = actual_crc == header.crc32_checksum;
        if !crc_match {
            issues.push(format!(
                "CRC32 mismatch: expected 0x{:08X}, computed 0x{:08X}",
                header.crc32_checksum, actual_crc
            ));
        }

        let b1_end = (header.block1_offset + header.block1_len) as usize;
        let b2_end = (header.block2_offset + header.block2_len) as usize;
        let b3_end = (header.block3_offset + header.block3_len) as usize;

        if b1_end > total_bytes || b2_end > total_bytes || b3_end > total_bytes {
            issues.push("One or more section block offsets exceed file boundaries".to_string());
        }

        let is_valid = issues.is_empty();

        Ok(SiCartridgeReport {
            file_path: path.display().to_string(),
            is_valid,
            version: header.version,
            flags: header.flags,
            is_cortex: (header.flags & SI_FLAG_TIER_1_CORTEX) != 0,
            is_router: (header.flags & SI_FLAG_TIER_2_ROUTER) != 0,
            is_reflex: (header.flags & SI_FLAG_TIER_3_REFLEX) != 0,
            total_bytes,
            block1_bytes: header.block1_len as usize,
            block2_bytes: header.block2_len as usize,
            block3_bytes: header.block3_len as usize,
            crc32_match: crc_match,
            mount_time_us,
            issues,
        })
    }

    /// Unpacks a `.si` cartridge into raw constituent slices
    pub fn unpack_cartridge(path: impl AsRef<Path>) -> Result<SiCartridgeDeconstructed> {
        let file = File::open(path)?;
        // SAFETY: `file` is a fresh handle this call just opened; `mmap2`'s
        // precondition is that it isn't concurrently modified, and `mmap`
        // is only read from for the rest of this function.
        let mmap = unsafe { Mmap::map(&file)? };

        let header = SiCartridgeHeader::from_bytes(&mmap[0..SI_HEADER_SIZE])?;

        let b1_start = header.block1_offset as usize;
        let b1_end = b1_start + (header.block1_len as usize);

        let b2_start = header.block2_offset as usize;
        let b2_end = b2_start + (header.block2_len as usize);

        let b3_start = header.block3_offset as usize;
        let b3_end = b3_start + (header.block3_len as usize);

        Ok(SiCartridgeDeconstructed {
            header,
            block1_core_weights: mmap[b1_start..b1_end].to_vec(),
            block2_dynamic_adapter: mmap[b2_start..b2_end].to_vec(),
            block3_skill_stack: mmap[b3_start..b3_end].to_vec(),
        })
    }

    /// Diffs two `.si` cartridges for adapter drift and skill evolution
    pub fn diff_cartridges(
        path_a: impl AsRef<Path>,
        path_b: impl AsRef<Path>,
    ) -> Result<SiCartridgeDiffReport> {
        let a = Self::unpack_cartridge(&path_a)?;
        let b = Self::unpack_cartridge(&path_b)?;

        let core_identical = a.block1_core_weights == b.block1_core_weights;
        let drift_bytes = a
            .block2_dynamic_adapter
            .iter()
            .zip(b.block2_dynamic_adapter.iter())
            .filter(|(x, y)| x != y)
            .count();

        Ok(SiCartridgeDiffReport {
            path_a: path_a.as_ref().display().to_string(),
            path_b: path_b.as_ref().display().to_string(),
            version_match: a.header.version == b.header.version,
            flags_match: a.header.flags == b.header.flags,
            core_weights_identical: core_identical,
            adapter_drift_bytes: drift_bytes,
            skill_count_delta: (b.block3_skill_stack.len() as i64)
                - (a.block3_skill_stack.len() as i64),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encode_decode_roundtrip() {
        let header = SiCartridgeHeader {
            magic: SI_CANONICAL_MAGIC,
            version: 3,
            header_size: 64,
            flags: SI_FLAG_TIER_3_REFLEX | SI_FLAG_TIER_2_ROUTER,
            crc32_checksum: 0xDEADBEEF,
            block1_offset: 64,
            block1_len: 1024,
            block2_offset: 1088,
            block2_len: 256,
            block3_offset: 1344,
            block3_len: 512,
        };

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 64);

        let decoded = SiCartridgeHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn test_pack_and_verify_cartridge_end_to_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cart_path = temp_dir.path().join("test_agent.si");

        let core_weights = vec![0x42u8; 512];
        let dynamic_adapter = vec![0x11u8; 128];
        let skill_stack = vec![0xAAu8; 256];

        let res = SiCartridgeEngine::pack_cartridge(
            &core_weights,
            &dynamic_adapter,
            &skill_stack,
            SI_FLAG_TIER_3_REFLEX,
            &cart_path,
        );
        assert!(res.is_ok());

        let report = SiCartridgeEngine::verify_cartridge(&cart_path).unwrap();
        assert!(report.is_valid);
        assert_eq!(report.version, 3);
        assert!(report.is_reflex);
        assert!(report.crc32_match);
        assert_eq!(report.block1_bytes, 512);
        assert_eq!(report.block2_bytes, 128);
        assert_eq!(report.block3_bytes, 256);

        let unpacked = SiCartridgeEngine::unpack_cartridge(&cart_path).unwrap();
        assert_eq!(unpacked.block1_core_weights, core_weights);
        assert_eq!(unpacked.block2_dynamic_adapter, dynamic_adapter);
        assert_eq!(unpacked.block3_skill_stack, skill_stack);
    }

    #[test]
    fn test_cartridge_diffing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cart_a = temp_dir.path().join("agent_v1.si");
        let cart_b = temp_dir.path().join("agent_v2.si");

        let core = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let adapt_a = vec![0; 64];
        let mut adapt_b = vec![0; 64];
        adapt_b[0] = 99;
        adapt_b[10] = 55;

        let skills_a = vec![10, 20];
        let skills_b = vec![10, 20, 30, 40];

        SiCartridgeEngine::pack_cartridge(
            &core,
            &adapt_a,
            &skills_a,
            SI_FLAG_TIER_3_REFLEX,
            &cart_a,
        )
        .unwrap();
        SiCartridgeEngine::pack_cartridge(
            &core,
            &adapt_b,
            &skills_b,
            SI_FLAG_TIER_3_REFLEX,
            &cart_b,
        )
        .unwrap();

        let diff = SiCartridgeEngine::diff_cartridges(&cart_a, &cart_b).unwrap();
        assert!(diff.core_weights_identical);
        assert_eq!(diff.adapter_drift_bytes, 2);
        assert_eq!(diff.skill_count_delta, 2);
    }
}
