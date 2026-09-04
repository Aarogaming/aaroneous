//! crates/si_format/src/verify.rs
//! Centralized validation logic for `.si` container format.
//!
//! Ensures magic bytes, tier flags, and alignment invariants are enforced
//! consistently across packers, loaders, and pipelines.

use anyhow::{bail, Result};
use crate::utils::ALIGNMENT_BYTES;

/// Magic bytes for `.si` SINT containers (distinct from legacy v2 JSON containers)
pub const SINT_PACKER_MAGIC: [u8; 4] = *b"SINT";

/// Minimum supported version for this loader/packer suite.
/// Must be >= 3 to enforce tensor-descriptor manifest with explicit byte offsets.
pub const MIN_VERSION: u32 = 3;

/// Validates the magic bytes at the start of a `.si` container.
///
/// # Errors
/// Returns `Err` if the magic bytes are missing or incorrect.
pub fn validate_magic_bytes(mmap: &[u8]) -> Result<()> {
    if mmap.len() < 4 || mmap[0..4] != SINT_PACKER_MAGIC {
        bail!("Missing or invalid SINT magic bytes");
    }
    Ok(())
}

/// Validates the version field (offset 0x04) in a `.si` container.
///
/// # Errors
/// Returns `Err` if the version is below `MIN_VERSION`.
pub fn validate_version(version: u32) -> Result<()> {
    if version < MIN_VERSION {
        bail!(
            "Container version v{} is not supported (requires v{}+)",
            version,
            MIN_VERSION
        );
    }
    Ok(())
}

/// Validates that a tensor descriptor's byte offset and length are properly aligned.
///
/// # Errors
/// Returns `Err` if the offset is not 64-byte aligned or the length is not a multiple of 4.
pub fn validate_tensor_descriptor(offset: u64, length: u64) -> Result<()> {
    if !(offset as usize).is_multiple_of(ALIGNMENT_BYTES) {
        bail!(
            "Tensor byte_offset {} is not 64-byte aligned",
            offset
        );
    }
    if !(length as usize).is_multiple_of(4) {
        bail!(
            "Tensor byte_length {} is not a multiple of 4 (f32)",
            length
        );
    }
    Ok(())
}

/// Capability permissions for sandboxed execution
pub const CAPABILITY_READ_STORAGE: u32 = 1 << 0;
pub const CAPABILITY_WRITE_STORAGE: u32 = 1 << 1;
pub const CAPABILITY_NETWORK_MESH: u32 = 1 << 2;
pub const CAPABILITY_HARDWARE_ACCEL: u32 = 1 << 3;
pub const CAPABILITY_JIT_EXECUTION: u32 = 1 << 4;

/// Validates that required capabilities are permitted by the granted capability mask
pub fn validate_capability_mask(granted_mask: u32, required_mask: u32) -> Result<()> {
    if (granted_mask & required_mask) != required_mask {
        let missing = required_mask & !granted_mask;
        bail!("Capability violation: missing required permissions (mask 0x{:08X})", missing);
    }
    Ok(())
}

/// Simple 32-bit FNV-1a checksum verification for sovereign container payload integrity
pub fn validate_payload_checksum(data: &[u8], expected_fnv1a: u32) -> Result<()> {
    let mut hash: u32 = 0x811c9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    if hash != expected_fnv1a {
        bail!("Payload integrity check failed: expected 0x{:08X}, got 0x{:08X}", expected_fnv1a, hash);
    }
    Ok(())
}

/// Validates that a given offset and payload size fit within the mapped region.
pub fn validate_range(mmap_len: usize, start: usize, end: usize) -> Result<()> {
    if end > mmap_len {
        bail!(
            "Tensor slice exceeds mapped region (start={}, end={}, mmap_len={})",
            start,
            end,
            mmap_len
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_magic_bytes_ok() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&SINT_PACKER_MAGIC);
        assert!(validate_magic_bytes(&data).is_ok());
    }

    #[test]
    fn test_validate_magic_bytes_bad() {
        let data = vec![0u8; 20];
        assert!(validate_magic_bytes(&data).is_err());
    }

    #[test]
    fn test_validate_version_ok() {
        assert!(validate_version(MIN_VERSION).is_ok());
        assert!(validate_version(4).is_ok());
    }

    #[test]
    fn test_validate_version_bad() {
        assert!(validate_version(MIN_VERSION - 1).is_err());
    }

    #[test]
    fn test_validate_tensor_descriptor_aligned() {
        let offset = ALIGNMENT_BYTES as u64;
        let length = (ALIGNMENT_BYTES * 2) as u64;
        assert!(validate_tensor_descriptor(offset, length).is_ok());
    }

    #[test]
    fn test_validate_tensor_descriptor_unaligned() {
        let offset = 1u64;
        let length = ALIGNMENT_BYTES as u64;
        assert!(validate_tensor_descriptor(offset, length).is_err());
    }

    #[test]
    fn test_validate_range_ok() {
        let mmap_len = 256;
        let start = 0;
        let end = 128;
        assert!(validate_range(mmap_len, start, end).is_ok());
    }

    #[test]
    fn test_validate_range_oob() {
        let mmap_len = 64;
        let start = 0;
        let end = 128;
        assert!(validate_range(mmap_len, start, end).is_err());
    }

    #[test]
    fn test_validate_capability_mask() {
        let granted = CAPABILITY_READ_STORAGE | CAPABILITY_HARDWARE_ACCEL;
        assert!(validate_capability_mask(granted, CAPABILITY_READ_STORAGE).is_ok());
        assert!(validate_capability_mask(granted, CAPABILITY_JIT_EXECUTION).is_err());
    }

    #[test]
    fn test_validate_payload_checksum() {
        let payload = b"SOVEREIGN_CART_DATA";
        let mut hash: u32 = 0x811c9dc5;
        for &b in payload {
            hash ^= b as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        assert!(validate_payload_checksum(payload, hash).is_ok());
        assert!(validate_payload_checksum(payload, hash + 1).is_err());
    }
}
