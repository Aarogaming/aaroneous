//! Base36 validation, encoding, and decoding CLI surface for xtask.
//!
//! Enforces the Aaroneous canonical Base36 notation contract:
//! - Lowercase alphabet: `0123456789abcdefghijklmnopqrstuvwxyz`
//! - Strict case: Uppercase is non-canonical and rejected.
//! - Canonical big-endian digit weighting.
//! - Leading zeroes preservation.
//! - Zero ambient authority.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

pub const BASE36_ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
pub const LINEAGE_ID_LEN: usize = 24;
pub const LINEAGE_DIGEST_BYTES: usize = 15;
pub const MAX_INPUT_BYTES: usize = 4_096;
pub const MAX_ENCODED_CHARS: usize = 8_192;

/// Validates that a string slice strictly contains only canonical lowercase Base36 characters `[0-9a-z]`.
pub fn validate_base36(input: &str) -> Result<()> {
    if input.is_empty() {
        return Ok(());
    }
    if input.len() > MAX_ENCODED_CHARS {
        bail!(
            "Base36 string length {} exceeds maximum allowed length of {} characters",
            input.len(),
            MAX_ENCODED_CHARS
        );
    }
    for (idx, c) in input.chars().enumerate() {
        if !matches!(c, '0'..='9' | 'a'..='z') {
            if c.is_ascii_uppercase() {
                bail!(
                    "Non-canonical Base36 character '{}' at index {}: uppercase characters are strictly rejected; use lowercase [0-9a-z]",
                    c,
                    idx
                );
            }
            if !c.is_ascii() {
                bail!(
                    "Non-ASCII character '{}' at index {}: Base36 requires canonical lowercase alphanumeric [0-9a-z]",
                    c,
                    idx
                );
            }
            bail!(
                "Invalid Base36 character '{}' at index {}: must be lowercase alphanumeric [0-9a-z]",
                c,
                idx
            );
        }
    }
    Ok(())
}

/// Encodes an arbitrary byte slice into a canonical lowercase Base36 string.
/// Preserves leading zero bytes as leading '0' characters.
pub fn encode_base36(input: &[u8]) -> Result<String> {
    if input.len() > MAX_INPUT_BYTES {
        bail!(
            "input length {} bytes exceeds maximum allowed length of {} bytes",
            input.len(),
            MAX_INPUT_BYTES
        );
    }
    if input.is_empty() {
        return Ok(String::new());
    }

    let leading_zeros = input.iter().take_while(|&&b| b == 0).count();
    let payload = &input[leading_zeros..];

    if payload.is_empty() {
        return Ok("0".repeat(leading_zeros));
    }

    let mut num = payload.to_vec();
    let mut result = String::new();
    while !num.is_empty() {
        let mut remainder = 0usize;
        let mut next_num = Vec::new();
        let mut started = false;
        for &byte in num.iter() {
            let val = (remainder << 8) | (byte as usize);
            let div = val / 36;
            remainder = val % 36;
            if div > 0 || started {
                next_num.push(div as u8);
                started = true;
            }
        }
        result.push(BASE36_ALPHABET[remainder] as char);
        num = next_num;
    }

    let mut out = "0".repeat(leading_zeros);
    out.extend(result.chars().rev());
    Ok(out)
}

/// Decodes a canonical lowercase Base36 string into raw bytes.
/// Rejects non-canonical characters (including uppercase) and enforces length bounds.
pub fn decode_base36(input: &str) -> Result<Vec<u8>> {
    if input.len() > MAX_ENCODED_CHARS {
        bail!(
            "Base36 encoded input length {} exceeds maximum allowed length of {} characters",
            input.len(),
            MAX_ENCODED_CHARS
        );
    }
    if input.is_empty() {
        return Ok(Vec::new());
    }

    validate_base36(input)?;

    let leading_zeros = input.chars().take_while(|&c| c == '0').count();
    let payload = &input[leading_zeros..];

    if payload.is_empty() {
        return Ok(vec![0u8; leading_zeros]);
    }

    let mut num: Vec<u8> = Vec::new();
    for c in payload.chars() {
        let val = match c {
            '0'..='9' => (c as u8 - b'0') as usize,
            'a'..='z' => (c as u8 - b'a' + 10) as usize,
            _ => unreachable!("validated by validate_base36"),
        };
        let mut carry = val;
        let mut next_num = Vec::new();
        for &byte in num.iter().rev() {
            let v = (byte as usize) * 36 + carry;
            next_num.push((v % 256) as u8);
            carry = v / 256;
        }
        while carry > 0 {
            next_num.push((carry % 256) as u8);
            carry /= 256;
        }
        next_num.reverse();
        num = next_num;
    }

    let mut result = vec![0u8; leading_zeros];
    result.extend(num);
    Ok(result)
}

/// Computes a fixed-width 24-character Base36 lineage stamp from a source message.
pub fn lineage_id(message: &str) -> Result<String> {
    if message.is_empty() {
        bail!("lineage message must not be empty");
    }
    if message.len() > MAX_INPUT_BYTES {
        bail!("lineage message exceeds {MAX_INPUT_BYTES} bytes");
    }
    let digest = Sha256::digest(message.as_bytes());
    let mut encoded = encode_base36(&digest[..LINEAGE_DIGEST_BYTES])?;
    if encoded.len() > LINEAGE_ID_LEN {
        bail!("internal lineage encoding exceeded {LINEAGE_ID_LEN} characters");
    }
    encoded.insert_str(0, &"0".repeat(LINEAGE_ID_LEN - encoded.len()));
    Ok(encoded)
}

/// CLI entrypoint for `cargo xtask base36 <subcommand>`.
pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() {
        print_usage();
        bail!("Missing base36 subcommand");
    }

    match args[0].as_str() {
        "validate" => {
            if args.len() < 2 {
                bail!("Usage: cargo xtask base36 validate <string> [--width <N>]");
            }
            let input = &args[1];
            validate_base36(input)?;
            if args.len() >= 4 && args[2] == "--width" {
                let expected: usize = args[3]
                    .parse()
                    .context("Invalid --width argument: must be integer")?;
                if input.len() != expected {
                    bail!(
                        "Base36 string length mismatch: expected width {}, got {}",
                        expected,
                        input.len()
                    );
                }
            }
            println!("OK: valid canonical Base36 (length {})", input.len());
            Ok(())
        }
        "encode" => {
            if args.len() < 2 {
                bail!("Usage: cargo xtask base36 encode <text> [--hex]");
            }
            let input = &args[1];
            let bytes = if args.iter().any(|a| a == "--hex") {
                hex::decode(input).context("Failed to decode input as hex bytes")?
            } else {
                input.as_bytes().to_vec()
            };
            let encoded = encode_base36(&bytes)?;
            println!("{}", encoded);
            Ok(())
        }
        "decode" => {
            if args.len() < 2 {
                bail!("Usage: cargo xtask base36 decode <base36-string> [--hex]");
            }
            let input = &args[1];
            let decoded = decode_base36(input)?;
            if args.iter().any(|a| a == "--hex") {
                println!("{}", hex::encode(&decoded));
            } else {
                match String::from_utf8(decoded.clone()) {
                    Ok(s) => println!("{}", s),
                    Err(_) => {
                        println!("(hex) {}", hex::encode(&decoded));
                    }
                }
            }
            Ok(())
        }
        "lineage" => {
            if args.len() < 2 {
                bail!("Usage: cargo xtask base36 lineage <message>");
            }
            let message = &args[1];
            let stamp = lineage_id(message)?;
            println!("{}", stamp);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => {
            print_usage();
            bail!("Unknown base36 subcommand: '{}'", other);
        }
    }
}

fn print_usage() {
    eprintln!(
        r#"Usage: cargo xtask base36 <subcommand> [args...]

Subcommands:
  validate <string> [--width <N>]  Validate that string conforms strictly to canonical Base36 [0-9a-z]
  encode <text> [--hex]            Encode ASCII text or raw hex bytes to canonical Base36
  decode <base36-string> [--hex]   Decode canonical Base36 string to ASCII text or hex
  lineage <message>                Generate canonical 24-character Base36 lineage stamp from Sha256 digest
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_base36_canonical() {
        assert!(validate_base36("0123456789abcdefghijklmnopqrstuvwxyz").is_ok());
        assert!(validate_base36("000000000000000000000000").is_ok());
        assert!(validate_base36("").is_ok());
    }

    #[test]
    fn test_validate_base36_rejects_uppercase() {
        let err = validate_base36("0123456789ABC").unwrap_err();
        assert!(err.to_string().contains("uppercase"));
    }

    #[test]
    fn test_validate_base36_rejects_punctuation() {
        let err = validate_base36("abc!def").unwrap_err();
        assert!(err.to_string().contains("Invalid Base36 character"));
    }

    #[test]
    fn test_roundtrip_ascii() {
        let original = b"hello aaroneous world";
        let encoded = encode_base36(original).unwrap();
        assert!(validate_base36(&encoded).is_ok());
        let decoded = decode_base36(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_roundtrip_leading_zeroes() {
        let original = vec![0u8, 0u8, 1u8, 2u8, 0u8];
        let encoded = encode_base36(&original).unwrap();
        assert!(encoded.starts_with("00"));
        let decoded = decode_base36(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_roundtrip_all_zeros_and_variable_lengths() {
        let zero_cases: &[&[u8]] = &[
            &[0u8],
            &[0u8, 0u8],
            &[0u8, 0u8, 0u8],
            &[0u8, 1u8],
            &[1u8],
            &[0u8, 0xffu8],
            &[0u8, 0u8, 0xffu8],
        ];
        for &case in zero_cases {
            let encoded = encode_base36(case).unwrap();
            let decoded = decode_base36(&encoded).unwrap();
            assert_eq!(decoded, case, "Failed roundtrip for zero case {:?}", case);
        }
    }

    #[test]
    fn test_cross_crate_parity_with_core_contracts() {
        use core_contracts::base36::Base36Lineage;

        let test_cases: &[&[u8]] = &[
            b"",
            b"a",
            b"hello",
            &[0u8],
            &[0u8, 0u8],
            &[0u8, 1u8],
            &[1u8, 0u8],
            &[0u8, 0u8, 0xde, 0xad, 0xbe, 0xef],
            b"123456789012345",
        ];

        for &input in test_cases {
            let core_stamp = Base36Lineage::encode_from_digest(input).unwrap();
            let core_str = core_stamp.as_str().unwrap();

            let mut xtask_str = encode_base36(input).unwrap();
            if xtask_str.len() < LINEAGE_ID_LEN {
                xtask_str.insert_str(0, &"0".repeat(LINEAGE_ID_LEN - xtask_str.len()));
            }

            assert_eq!(
                core_str, xtask_str,
                "Mismatch for input {:?}: core-contracts got {}, xtask got {}",
                input, core_str, xtask_str
            );
        }
    }

    #[test]
    fn test_core_contracts_serialization_fails_on_invalid_bytes() {
        use core_contracts::base36::Base36Lineage;

        let nul_stamp = Base36Lineage { bytes: [0u8; 24] };
        assert!(serde_json::to_string(&nul_stamp).is_err());

        let ff_stamp = Base36Lineage { bytes: [0xff; 24] };
        assert!(serde_json::to_string(&ff_stamp).is_err());

        let uppercase_stamp = Base36Lineage { bytes: [b'A'; 24] };
        assert!(serde_json::to_string(&uppercase_stamp).is_err());

        let valid_stamp = Base36Lineage::ZERO;
        let serialized = serde_json::to_string(&valid_stamp).unwrap();
        assert_eq!(serialized, "\"000000000000000000000000\"");
    }

    #[test]
    fn test_lineage_fixed_width() {
        let stamp = lineage_id("initial commit test").unwrap();
        assert_eq!(stamp.len(), LINEAGE_ID_LEN);
        assert!(validate_base36(&stamp).is_ok());
    }
}
