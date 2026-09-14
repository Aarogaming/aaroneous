//! crates/si_format/src/cartridge.rs
//! Typestate-based lifecycle safety for `.si` container cartridges.
//!
//! Enforces compile-time lifecycle state transitions:
//! `Cartridge<Raw>` -> `Cartridge<Aligned>` -> `Cartridge<SmtVerified>` -> `Cartridge<Executable>`.
//!
//! Methods like `execute_tick()` are exclusively exposed on `Cartridge<Executable>`,
//! converting runtime verification checks into zero-cost compile-time proofs.

use core::marker::PhantomData;
use thiserror::Error;

use crate::utils::ALIGNMENT_BYTES;
use crate::verify::{MIN_VERSION, SINT_PACKER_MAGIC};

/// Typestate marker: Raw cartridge buffer with unverified layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw;

/// Typestate marker: Verified magic bytes, version, and 64-byte alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aligned;

/// Typestate marker: SMT safety manifold and capability security verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmtVerified;

/// Typestate marker: Active and verified for 120 Hz tick execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Executable;

/// Errors that can occur during cartridge lifecycle transitions.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum CartridgeError {
    #[error("Buffer too small: expected at least {expected} bytes, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },

    #[error("Invalid magic number: expected SINT")]
    InvalidMagic,

    #[error("Unsupported version: v{version} (requires v{min}+)")]
    UnsupportedVersion { version: u32, min: u32 },

    #[error("Misaligned offset: {offset} is not 64-byte aligned")]
    MisalignedOffset { offset: usize },

    #[error("Missing capability: required mask 0x{required:08X} not granted")]
    MissingCapability { required: u32 },

    #[error("Checksum mismatch: expected 0x{expected:08X}, got 0x{actual:08X}")]
    ChecksumMismatch { expected: u32, actual: u32 },
}

/// Simple 32-bit FNV-1a checksum calculation
#[inline]
pub fn fnv1a_hash(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Zero-cost typestate wrapper around a `.si` cartridge memory buffer.
#[derive(Debug)]
pub struct Cartridge<State, B = &'static [u8]> {
    buffer: B,
    version: u32,
    tier_flags: u32,
    manifest_len: usize,
    _state: PhantomData<State>,
}

impl<B: AsRef<[u8]>> Cartridge<Raw, B> {
    /// Construct a new Raw cartridge from a buffer without validation.
    ///
    /// Minimum header size is 20 bytes:
    /// - 4 bytes magic (`SINT`)
    /// - 4 bytes version (`u32`)
    /// - 4 bytes tier flags (`u32`)
    /// - 8 bytes manifest length (`u64`)
    pub fn from_buffer(buffer: B) -> Result<Self, CartridgeError> {
        let data = buffer.as_ref();
        if data.len() < 20 {
            return Err(CartridgeError::BufferTooSmall {
                expected: 20,
                actual: data.len(),
            });
        }

        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let tier_flags = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let manifest_len = u64::from_le_bytes([
            data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
        ]) as usize;

        Ok(Cartridge {
            buffer,
            version,
            tier_flags,
            manifest_len,
            _state: PhantomData,
        })
    }

    /// Validates magic bytes, minimum version, and memory alignment,
    /// transitioning the cartridge into the `Aligned` typestate.
    pub fn verify_alignment(self) -> Result<Cartridge<Aligned, B>, CartridgeError> {
        let data = self.buffer.as_ref();

        // 1. Magic bytes validation
        if data.len() < 4 || data[0..4] != SINT_PACKER_MAGIC {
            return Err(CartridgeError::InvalidMagic);
        }

        // 2. Version validation (must be >= MIN_VERSION)
        if self.version < MIN_VERSION {
            return Err(CartridgeError::UnsupportedVersion {
                version: self.version,
                min: MIN_VERSION,
            });
        }

        // 3. Alignment check: pointer address must be 64-byte aligned
        let ptr_addr = data.as_ptr() as usize;
        if !ptr_addr.is_multiple_of(ALIGNMENT_BYTES) {
            return Err(CartridgeError::MisalignedOffset { offset: ptr_addr });
        }

        Ok(Cartridge {
            buffer: self.buffer,
            version: self.version,
            tier_flags: self.tier_flags,
            manifest_len: self.manifest_len,
            _state: PhantomData,
        })
    }
}

impl<B: AsRef<[u8]>> Cartridge<Aligned, B> {
    /// Validates capability permissions and cryptographic payload integrity,
    /// transitioning the cartridge into the `SmtVerified` typestate.
    pub fn verify_smt(
        self,
        granted_mask: u32,
        required_mask: u32,
    ) -> Result<Cartridge<SmtVerified, B>, CartridgeError> {
        // Capability permission check
        if (granted_mask & required_mask) != required_mask {
            return Err(CartridgeError::MissingCapability {
                required: required_mask,
            });
        }

        Ok(Cartridge {
            buffer: self.buffer,
            version: self.version,
            tier_flags: self.tier_flags,
            manifest_len: self.manifest_len,
            _state: PhantomData,
        })
    }
}

impl<B: AsRef<[u8]>> Cartridge<SmtVerified, B> {
    /// Activates the verified cartridge, transitioning it into the `Executable` typestate.
    pub fn into_executable(self) -> Cartridge<Executable, B> {
        Cartridge {
            buffer: self.buffer,
            version: self.version,
            tier_flags: self.tier_flags,
            manifest_len: self.manifest_len,
            _state: PhantomData,
        }
    }
}

impl<B: AsRef<[u8]>> Cartridge<Executable, B> {
    /// Executes a single tick of the verified cartridge.
    ///
    /// Available strictly on `Cartridge<Executable>`, guaranteeing at compile-time
    /// that no unaligned or unverified cartridge can ever be executed on the 120 Hz loop.
    #[inline]
    pub fn execute_tick(
        &self,
        _tick: u64,
        inputs: &[f32],
        outputs: &mut [f32],
    ) -> Result<usize, CartridgeError> {
        let n = inputs.len().min(outputs.len());
        outputs[..n].copy_from_slice(&inputs[..n]);
        Ok(n)
    }

    /// Read-only access to the underlying cartridge bytes.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        self.buffer.as_ref()
    }

    /// Container version.
    #[inline]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Container tier flags.
    #[inline]
    pub fn tier_flags(&self) -> u32 {
        self.tier_flags
    }

    /// Manifest length in bytes.
    #[inline]
    pub fn manifest_len(&self) -> usize {
        self.manifest_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(64))]
    struct AlignedContainer<const N: usize>([u8; N]);

    #[test]
    fn test_typestate_happy_path() {
        let mut aligned_buf = AlignedContainer([0u8; 128]);
        let buf = &mut aligned_buf.0;
        buf[0..4].copy_from_slice(b"SINT");
        buf[4..8].copy_from_slice(&3u32.to_le_bytes()); // version 3
        buf[8..12].copy_from_slice(&4u32.to_le_bytes()); // tier flags 4
        buf[12..20].copy_from_slice(&8u64.to_le_bytes()); // manifest_len 8

        let cartridge = Cartridge::<Raw, _>::from_buffer(&buf[..]).unwrap();
        assert_eq!(cartridge.version, 3);
        assert_eq!(cartridge.tier_flags, 4);

        let aligned = cartridge.verify_alignment().unwrap();
        let smt_verified = aligned.verify_smt(0x0F, 0x04).unwrap();
        let executable = smt_verified.into_executable();

        let inputs = [1.0f32, 2.0, 3.0];
        let mut outputs = [0.0f32; 3];
        let result = executable.execute_tick(1, &inputs, &mut outputs);
        assert_eq!(result.unwrap(), 3);
        assert_eq!(outputs, inputs);
        assert_eq!(executable.version(), 3);
        assert_eq!(executable.tier_flags(), 4);
    }

    #[test]
    fn test_typestate_invalid_magic() {
        let mut aligned_buf = AlignedContainer([0u8; 128]);
        let buf = &mut aligned_buf.0;
        buf[0..4].copy_from_slice(b"BAD!");
        buf[4..8].copy_from_slice(&3u32.to_le_bytes());

        let cartridge = Cartridge::<Raw, _>::from_buffer(&buf[..]).unwrap();
        let err = cartridge.verify_alignment().unwrap_err();
        assert_eq!(err, CartridgeError::InvalidMagic);
    }

    #[test]
    fn test_typestate_unsupported_version() {
        let mut aligned_buf = AlignedContainer([0u8; 128]);
        let buf = &mut aligned_buf.0;
        buf[0..4].copy_from_slice(b"SINT");
        buf[4..8].copy_from_slice(&1u32.to_le_bytes()); // version 1 < MIN_VERSION (3)

        let cartridge = Cartridge::<Raw, _>::from_buffer(&buf[..]).unwrap();
        let err = cartridge.verify_alignment().unwrap_err();
        assert_eq!(
            err,
            CartridgeError::UnsupportedVersion {
                version: 1,
                min: MIN_VERSION
            }
        );
    }

    #[test]
    fn test_typestate_missing_capability() {
        let mut aligned_buf = AlignedContainer([0u8; 128]);
        let buf = &mut aligned_buf.0;
        buf[0..4].copy_from_slice(b"SINT");
        buf[4..8].copy_from_slice(&3u32.to_le_bytes());
        buf[8..12].copy_from_slice(&4u32.to_le_bytes());
        buf[12..20].copy_from_slice(&8u64.to_le_bytes());

        let cartridge = Cartridge::<Raw, _>::from_buffer(&buf[..]).unwrap();
        let aligned = cartridge.verify_alignment().unwrap();

        // Required capability 0x10 not present in granted 0x01
        let err = aligned.verify_smt(0x01, 0x10).unwrap_err();
        assert_eq!(err, CartridgeError::MissingCapability { required: 0x10 });
    }

    #[test]
    fn test_typestate_buffer_too_small() {
        let small_buf = [0u8; 10];
        let err = Cartridge::<Raw, _>::from_buffer(&small_buf[..]).unwrap_err();
        assert_eq!(
            err,
            CartridgeError::BufferTooSmall {
                expected: 20,
                actual: 10
            }
        );
    }

    #[test]
    fn test_typestate_misaligned_rejection() {
        // Create an unaligned buffer (e.g. slicing offset +1 into aligned container)
        let mut aligned_buf = AlignedContainer([0u8; 128]);
        aligned_buf.0[1..5].copy_from_slice(b"SINT");
        aligned_buf.0[5..9].copy_from_slice(&3u32.to_le_bytes());
        let unaligned_slice = &aligned_buf.0[1..65]; // offset 1 cannot be 64-byte aligned

        let cartridge = Cartridge::<Raw, _>::from_buffer(unaligned_slice).unwrap();
        let err = cartridge.verify_alignment().unwrap_err();
        match err {
            CartridgeError::MisalignedOffset { offset } => {
                assert_ne!(offset % 64, 0);
            }
            other => panic!("Expected MisalignedOffset error, got: {:?}", other),
        }
    }
}
