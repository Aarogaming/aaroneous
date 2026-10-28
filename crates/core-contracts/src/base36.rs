//! Canonical Base36 notation contract for Aaroneous component and build-sheet lifecycle records.
//!
//! # Schema Version: v1
//!
//! # Invariants:
//! 1. **Alphabet**: Exactly the 36 lowercase alphanumeric ASCII symbols: `0123456789abcdefghijklmnopqrstuvwxyz`.
//! 2. **Strict Case**: Lowercase only. Uppercase characters `A-Z` are strictly non-canonical and rejected.
//! 3. **Big-Endian Weighting**: High-order digits precede low-order digits.
//! 4. **Fixed Width Geometry**: The canonical lifecycle stamp (`Base36Lineage`) has width 24 ASCII characters (`[u8; 24]`), left-padded with `'0'`.
//! 5. **Monotonic Ordering**: Because the ASCII byte values preserve numeric digit values (`'0' < '9' < 'a' < 'z'`), lexicographical comparison of equal-width zero-padded stamps strictly matches numerical magnitude comparison.
//! 6. **Reserved Sentinels**:
//!    - Zero / Uninitialized: all `'0'` characters (`"000000000000000000000000"`).
//!    - Max / Tombstone: all `'z'` characters (`"zzzzzzzzzzzzzzzzzzzzzzzz"`).
//! 7. **Zero Ambient Authority**: Pure `#![no_std]` state transformations without heap allocation or ambient system calls.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Canonical schema revision identifier.
pub const BASE36_SCHEMA_VERSION: &str = "v1";

/// The 36 canonical lowercase Base36 ASCII symbols.
pub const BASE36_ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Canonical width of fixed-size lifecycle lineage stamps (24 ASCII characters = 192 bits).
pub const LINEAGE_STAMP_WIDTH: usize = 24;

/// Maximum raw byte count that can be losslessly represented in 24 Base36 characters
/// (256^15 ≈ 1.329e36 < 36^24 ≈ 2.245e37).
pub const MAX_DIGEST_INPUT_BYTES: usize = 15;

/// Error variants encountered during Base36 validation, encoding, or decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base36Error {
    /// Character at index is not a canonical lowercase Base36 symbol `[0-9a-z]`.
    InvalidChar { byte: u8, index: usize },
    /// Input length does not match expected fixed length.
    InvalidLength { expected: usize, actual: usize },
    /// Input byte slice exceeds maximum capacity for fixed-width representation.
    InputTooLarge { max: usize, actual: usize },
    /// Output buffer is too small to receive the encoded or decoded bytes.
    BufferTooSmall { required: usize, provided: usize },
    /// Value overflowed numeric representation limits.
    Overflow,
}

impl core::fmt::Display for Base36Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidChar { byte, index } => {
                write!(
                    f,
                    "invalid Base36 character byte 0x{:02x} at index {}; must be lowercase [0-9a-z]",
                    byte, index
                )
            }
            Self::InvalidLength { expected, actual } => {
                write!(
                    f,
                    "invalid Base36 length: expected {} characters, got {}",
                    expected, actual
                )
            }
            Self::InputTooLarge { max, actual } => {
                write!(
                    f,
                    "input size exceeds maximum capacity: max {} bytes, got {}",
                    max, actual
                )
            }
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "buffer too small: required {} bytes, provided {}",
                    required, provided
                )
            }
            Self::Overflow => write!(f, "Base36 numeric value overflowed limits"),
        }
    }
}

/// Returns true if byte is a valid canonical lowercase Base36 ASCII character `[0-9a-z]`.
#[inline]
pub const fn is_valid_base36_byte(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z')
}

/// Returns true if character is a valid canonical lowercase Base36 ASCII character `[0-9a-z]`.
#[inline]
pub const fn is_valid_base36_char(c: char) -> bool {
    matches!(c, '0'..='9' | 'a'..='z')
}

/// Computes the numeric digit value (0..35) of a canonical Base36 ASCII byte.
#[inline]
pub const fn base36_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'z' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Validates that an ASCII byte slice contains only canonical lowercase Base36 characters.
pub fn validate_base36_bytes(bytes: &[u8]) -> Result<(), Base36Error> {
    for (i, &b) in bytes.iter().enumerate() {
        if !is_valid_base36_byte(b) {
            return Err(Base36Error::InvalidChar { byte: b, index: i });
        }
    }
    Ok(())
}

/// Validates that a string slice contains only canonical lowercase Base36 characters.
pub fn validate_base36_str(s: &str) -> Result<(), Base36Error> {
    validate_base36_bytes(s.as_bytes())
}

/// Canonical 24-character fixed-width Base36 lifecycle and lineage stamp.
///
/// Complies with `bytemuck::Pod` and zero-allocation hot-path rules.
/// Lexicographical ordering strictly matches monotonic numeric ordering.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Pod, Zeroable)]
pub struct Base36Lineage {
    pub bytes: [u8; LINEAGE_STAMP_WIDTH],
}

impl Base36Lineage {
    /// All-zero uninitialized sentinel (`"000000000000000000000000"`).
    pub const ZERO: Self = Self {
        bytes: [b'0'; LINEAGE_STAMP_WIDTH],
    };

    /// All-z maximum tombstone sentinel (`"zzzzzzzzzzzzzzzzzzzzzzzz"`).
    pub const MAX: Self = Self {
        bytes: [b'z'; LINEAGE_STAMP_WIDTH],
    };

    /// Creates a stamp from validated raw ASCII bytes.
    pub const fn from_ascii_bytes(bytes: [u8; LINEAGE_STAMP_WIDTH]) -> Result<Self, Base36Error> {
        let mut i = 0;
        while i < LINEAGE_STAMP_WIDTH {
            if !is_valid_base36_byte(bytes[i]) {
                return Err(Base36Error::InvalidChar {
                    byte: bytes[i],
                    index: i,
                });
            }
            i += 1;
        }
        Ok(Self { bytes })
    }

    /// Parses a 24-character string slice as a canonical Base36 stamp.
    pub fn from_str_exact(s: &str) -> Result<Self, Base36Error> {
        let bytes = s.as_bytes();
        if bytes.len() != LINEAGE_STAMP_WIDTH {
            return Err(Base36Error::InvalidLength {
                expected: LINEAGE_STAMP_WIDTH,
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; LINEAGE_STAMP_WIDTH];
        arr.copy_from_slice(bytes);
        Self::from_ascii_bytes(arr)
    }

    /// Returns the raw ASCII bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; LINEAGE_STAMP_WIDTH] {
        &self.bytes
    }

    /// Validates that the underlying byte array contains exactly canonical lowercase Base36 characters.
    #[inline]
    pub fn validate(&self) -> Result<(), Base36Error> {
        validate_base36_bytes(&self.bytes)
    }

    /// Returns the stamp as an ASCII string slice if valid, or an error if invalid.
    pub fn try_as_str(&self) -> Result<&str, Base36Error> {
        self.validate()?;
        // INFALLIBLE: validate() confirms all bytes are in [b'0'..=b'9' | b'a'..=b'z'], which is valid ASCII/UTF-8.
        match core::str::from_utf8(&self.bytes) {
            Ok(s) => Ok(s),
            Err(_) => Err(Base36Error::InvalidChar { byte: 0, index: 0 }),
        }
    }

    /// Returns the stamp as an ASCII string slice if valid, or None if invalid.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        self.try_as_str().ok()
    }

    /// Returns true if this stamp is the all-zero uninitialized sentinel.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.bytes == Self::ZERO.bytes
    }

    /// Returns true if this stamp is the maximum tombstone sentinel.
    #[inline]
    pub fn is_max(&self) -> bool {
        self.bytes == Self::MAX.bytes
    }

    /// Encodes a raw byte slice (up to 15 bytes) into a zero-padded 24-character Base36 stamp
    /// without heap allocation.
    pub fn encode_from_digest(input: &[u8]) -> Result<Self, Base36Error> {
        if input.len() > MAX_DIGEST_INPUT_BYTES {
            return Err(Base36Error::InputTooLarge {
                max: MAX_DIGEST_INPUT_BYTES,
                actual: input.len(),
            });
        }
        if input.is_empty() {
            return Ok(Self::ZERO);
        }

        let mut num = [0u8; MAX_DIGEST_INPUT_BYTES];
        let offset = MAX_DIGEST_INPUT_BYTES - input.len();
        num[offset..].copy_from_slice(input);

        let mut out = [b'0'; LINEAGE_STAMP_WIDTH];
        let mut out_idx = LINEAGE_STAMP_WIDTH;

        let mut start = offset;
        while start < MAX_DIGEST_INPUT_BYTES {
            let mut remainder = 0usize;
            let mut next_start = MAX_DIGEST_INPUT_BYTES;
            for (idx, item) in num.iter_mut().enumerate().skip(start) {
                let val = (remainder << 8) | (*item as usize);
                let div = val / 36;
                remainder = val % 36;
                *item = div as u8;
                if div > 0 && next_start == MAX_DIGEST_INPUT_BYTES {
                    next_start = idx;
                }
            }
            start = next_start;
            if out_idx == 0 {
                return Err(Base36Error::Overflow);
            }
            out_idx -= 1;
            out[out_idx] = BASE36_ALPHABET[remainder];
        }

        Ok(Self { bytes: out })
    }

    /// Decodes the 24-character Base36 stamp back into its 15 raw big-endian bytes
    /// without heap allocation.
    pub fn decode_to_digest(&self) -> Result<[u8; MAX_DIGEST_INPUT_BYTES], Base36Error> {
        let mut out = [0u8; MAX_DIGEST_INPUT_BYTES];
        for (i, &b) in self.bytes.iter().enumerate() {
            let digit = base36_digit_value(b)
                .ok_or(Base36Error::InvalidChar { byte: b, index: i })?
                as usize;
            let mut carry = digit;
            for byte in out.iter_mut().rev() {
                let v = (*byte as usize) * 36 + carry;
                *byte = (v % 256) as u8;
                carry = v / 256;
            }
            if carry > 0 {
                return Err(Base36Error::Overflow);
            }
        }
        Ok(out)
    }
}

impl Default for Base36Lineage {
    fn default() -> Self {
        Self::ZERO
    }
}

impl core::fmt::Debug for Base36Lineage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_as_str() {
            Ok(s) => write!(f, "Base36Lineage(\"{}\")", s),
            Err(_) => write!(f, "Base36Lineage(invalid: {:?})", self.bytes),
        }
    }
}

impl core::fmt::Display for Base36Lineage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_as_str() {
            Ok(s) => f.write_str(s),
            Err(_) => f.write_str("<invalid-base36>"),
        }
    }
}

impl Serialize for Base36Lineage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.try_as_str().map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Base36Lineage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Base36Visitor;

        impl<'de> serde::de::Visitor<'de> for Base36Visitor {
            type Value = Base36Lineage;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a 24-character canonical lowercase Base36 string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Base36Lineage::from_str_exact(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Base36Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_and_max_sentinels() {
        assert_eq!(
            Base36Lineage::ZERO.as_str().unwrap(),
            "000000000000000000000000"
        );
        assert_eq!(
            Base36Lineage::ZERO.try_as_str().unwrap(),
            "000000000000000000000000"
        );
        assert!(Base36Lineage::ZERO.is_zero());
        assert!(!Base36Lineage::ZERO.is_max());

        assert_eq!(
            Base36Lineage::MAX.as_str().unwrap(),
            "zzzzzzzzzzzzzzzzzzzzzzzz"
        );
        assert_eq!(
            Base36Lineage::MAX.try_as_str().unwrap(),
            "zzzzzzzzzzzzzzzzzzzzzzzz"
        );
        assert!(Base36Lineage::MAX.is_max());
        assert!(!Base36Lineage::MAX.is_zero());

        // MAX is a sentinel whose magnitude (36^24 - 1) exceeds the 15-byte digest capacity.
        assert_eq!(
            Base36Lineage::MAX.decode_to_digest(),
            Err(Base36Error::Overflow)
        );
    }

    #[test]
    fn test_rejects_uppercase_and_invalid() {
        assert!(Base36Lineage::from_str_exact("00000000000000000000000A").is_err());
        assert!(Base36Lineage::from_str_exact("00000000000000000000000!").is_err());
        assert!(Base36Lineage::from_str_exact("short").is_err());
        assert!(Base36Lineage::from_str_exact("toolong00000000000000000000000").is_err());
    }

    #[test]
    fn test_invalid_byte_states_fail_explicitly_without_substitution() {
        // Test zero-initialized memory (e.g. from Pod/Zeroable transmutation)
        let nul_stamp = Base36Lineage { bytes: [0u8; 24] };
        assert!(nul_stamp.validate().is_err());
        assert!(nul_stamp.try_as_str().is_err());
        assert_eq!(nul_stamp.as_str(), None);
        assert!(!nul_stamp.is_zero());

        // Test non-UTF-8 bytes (e.g. 0xff)
        let raw_bytes_0xff = Base36Lineage { bytes: [0xff; 24] };
        assert!(raw_bytes_0xff.validate().is_err());
        assert!(raw_bytes_0xff.try_as_str().is_err());
        assert_eq!(raw_bytes_0xff.as_str(), None);
        assert!(!raw_bytes_0xff.is_zero());

        // Test uppercase ASCII letters
        let uppercase_stamp = Base36Lineage { bytes: [b'A'; 24] };
        assert!(uppercase_stamp.validate().is_err());
        assert!(uppercase_stamp.try_as_str().is_err());
        assert_eq!(uppercase_stamp.as_str(), None);
        assert!(!uppercase_stamp.is_zero());
    }

    #[test]
    fn test_roundtrip_digest_encode_decode() {
        let digest = *b"123456789012345";
        let stamp = Base36Lineage::encode_from_digest(&digest).unwrap();
        assert_eq!(stamp.as_str().unwrap().len(), 24);
        let decoded = stamp.decode_to_digest().unwrap();
        assert_eq!(decoded, digest);
    }

    #[test]
    fn test_monotonic_ordering() {
        let stamp1 = Base36Lineage::from_str_exact("000000000000000000000001").unwrap();
        let stamp2 = Base36Lineage::from_str_exact("000000000000000000000002").unwrap();
        let stamp9 = Base36Lineage::from_str_exact("000000000000000000000009").unwrap();
        let stampa = Base36Lineage::from_str_exact("00000000000000000000000a").unwrap();
        let stampz = Base36Lineage::from_str_exact("00000000000000000000000z").unwrap();

        assert!(Base36Lineage::ZERO < stamp1);
        assert!(stamp1 < stamp2);
        assert!(stamp2 < stamp9);
        assert!(stamp9 < stampa);
        assert!(stampa < stampz);
        assert!(stampz < Base36Lineage::MAX);
    }

    #[test]
    fn test_pod_and_memory_geometry() {
        assert_eq!(core::mem::size_of::<Base36Lineage>(), 24);
        assert_eq!(core::mem::align_of::<Base36Lineage>(), 1);

        let stamp = Base36Lineage::from_str_exact("0123456789abcdefghijklmn").unwrap();
        let bytes = bytemuck::bytes_of(&stamp);
        assert_eq!(bytes.len(), 24);
        let decoded: &Base36Lineage = bytemuck::from_bytes(bytes);
        assert_eq!(*decoded, stamp);
    }
}
