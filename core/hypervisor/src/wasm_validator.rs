/// WASM Module Validator — validates WASM binary modules for integrity and conformance.
///
/// Checks magic bytes, version, section structure, imports/exports,
/// and basic sanity of WASM modules before loading them into the hypervisor.

use std::path::Path;
use anyhow::{Result, bail};
use tracing::{info, warn};

/// WASM magic bytes
const WASM_MAGIC: &[u8; 4] = b"\0asm";
/// Current WASM spec version
const WASM_VERSION: u32 = 1;

/// WASM section IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WasmSection {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
    DataCount = 12,
    Unknown(u8),
}

impl From<u8> for WasmSection {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Custom,
            1 => Self::Type,
            2 => Self::Import,
            3 => Self::Function,
            4 => Self::Table,
            5 => Self::Memory,
            6 => Self::Global,
            7 => Self::Export,
            8 => Self::Start,
            9 => Self::Element,
            10 => Self::Code,
            11 => Self::Data,
            12 => Self::DataCount,
            other => Self::Unknown(other),
        }
    }
}

/// Validation result for a WASM module.
#[derive(Debug, Clone)]
pub struct WasmValidation {
    pub path: String,
    pub valid: bool,
    pub version: u32,
    pub sections: Vec<(WasmSection, u64)>,
    pub import_count: usize,
    pub export_count: usize,
    pub function_count: usize,
    pub memory_count: usize,
    pub has_start: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validate a WASM binary file.
pub fn validate_wasm(path: &Path) -> Result<WasmValidation> {
    info!("Validating WASM module: {}", path.display());

    let data = std::fs::read(path)?;
    validate_wasm_bytes(&data, path.to_string_lossy().as_ref())
}

/// Validate WASM binary data.
pub fn validate_wasm_bytes(data: &[u8], name: &str) -> Result<WasmValidation> {
    let mut validation = WasmValidation {
        path: name.to_string(),
        valid: true,
        version: 0,
        sections: Vec::new(),
        import_count: 0,
        export_count: 0,
        function_count: 0,
        memory_count: 0,
        has_start: false,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    // Check minimum size
    if data.len() < 8 {
        validation.valid = false;
        validation.errors.push(format!("File too small: {} bytes (minimum 8)", data.len()));
        return Ok(validation);
    }

    // Check magic bytes
    if &data[..4] != WASM_MAGIC {
        validation.valid = false;
        validation.errors.push(format!(
            "Invalid magic bytes: {:02x} {:02x} {:02x} {:02x} (expected 00 61 73 6d)",
            data[0], data[1], data[2], data[3]
        ));
        return Ok(validation);
    }

    // Check version
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    validation.version = version;
    if version != WASM_VERSION {
        validation.valid = false;
        validation.errors.push(format!("Unsupported WASM version: {} (expected {})", version, WASM_VERSION));
        return Ok(validation);
    }

    // Parse sections
    let mut offset = 8;
    while offset < data.len() {
        if offset >= data.len() {
            break;
        }

        let section_id = data[offset];
        offset += 1;

        // Read LEB128 section size
        let (size, new_offset) = match read_leb128(data, offset) {
            Some(v) => v,
            None => {
                validation.warnings.push(format!("Truncated section at offset {}", offset));
                break;
            }
        };
        offset = new_offset;

        let section = WasmSection::from(section_id);
        validation.sections.push((section, size));

        match section {
            WasmSection::Import => {
                // Parse import count
                if let Some((count, _)) = read_leb128(data, offset) {
                    validation.import_count = count as usize;
                }
            }
            WasmSection::Export => {
                // Parse export count
                if let Some((count, _)) = read_leb128(data, offset) {
                    validation.export_count = count as usize;
                }
            }
            WasmSection::Function => {
                // Parse function count
                if let Some((count, _)) = read_leb128(data, offset) {
                    validation.function_count = count as usize;
                }
            }
            WasmSection::Memory => {
                validation.memory_count += 1;
            }
            WasmSection::Start => {
                validation.has_start = true;
            }
            _ => {}
        }

        // Skip section data
        offset += size as usize;
    }

    // Validate section structure
    if validation.sections.is_empty() {
        validation.warnings.push("No sections found".to_string());
    }

    // Check for required sections
    let has_type = validation.sections.iter().any(|(s, _)| *s == WasmSection::Type);
    let has_function = validation.sections.iter().any(|(s, _)| *s == WasmSection::Function);
    let has_code = validation.sections.iter().any(|(s, _)| *s == WasmSection::Code);

    if !has_type {
        validation.warnings.push("Missing Type section".to_string());
    }
    if !has_function {
        validation.warnings.push("Missing Function section".to_string());
    }
    if !has_code && validation.function_count > 0 {
        validation.warnings.push("Has functions but no Code section".to_string());
    }

    // Check export count
    if validation.export_count == 0 {
        validation.warnings.push("No exports — module may not be usable".to_string());
    }

    // Check for WASI imports
    let has_wasi = validation.import_count > 0; // Simplified check
    if has_wasi {
        info!("  Module has {} imports (may include WASI)", validation.import_count);
    }

    info!("  Validation complete: {} (sections={}, imports={}, exports={}, functions={})",
        if validation.valid { "VALID" } else { "INVALID" },
        validation.sections.len(),
        validation.import_count,
        validation.export_count,
        validation.function_count,
    );

    Ok(validation)
}

/// Read a LEB128-encoded unsigned integer.
fn read_leb128(data: &[u8], offset: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = offset;

    loop {
        if pos >= data.len() {
            return None;
        }
        let byte = data[pos];
        result |= ((byte & 0x7F) as u64) << shift;
        pos += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return None; // Overflow
        }
    }

    Some((result, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_leb128() {
        // Single byte: 0
        assert_eq!(read_leb128(&[0x00], 0), Some((0, 1)));
        // Single byte: 1
        assert_eq!(read_leb128(&[0x01], 0), Some((1, 1)));
        // Two bytes: 128
        assert_eq!(read_leb128(&[0x80, 0x01], 0), Some((128, 2)));
        // Three bytes: 300
        assert_eq!(read_leb128(&[0xac, 0x02], 0), Some((300, 2)));
    }

    #[test]
    fn test_invalid_magic() {
        let data = b"\x00\x00\x00\x00\x01\x00\x00\x00";
        let result = validate_wasm_bytes(data, "test").unwrap();
        assert!(!result.valid);
        assert!(result.errors[0].contains("Invalid magic"));
    }

    #[test]
    fn test_invalid_version() {
        let mut data = vec![0x00, 0x61, 0x73, 0x6d]; // magic
        data.extend_from_slice(&2u32.to_le_bytes()); // version 2
        let result = validate_wasm_bytes(&data, "test").unwrap();
        assert!(!result.valid);
        assert!(result.errors[0].contains("Unsupported WASM version"));
    }

    #[test]
    fn test_valid_minimal() {
        // Minimal valid WASM module: magic + version + empty
        let mut data = vec![0x00, 0x61, 0x73, 0x6d]; // magic
        data.extend_from_slice(&1u32.to_le_bytes()); // version 1
        let result = validate_wasm_bytes(&data, "test").unwrap();
        assert!(result.valid);
        assert_eq!(result.version, 1);
    }
}
