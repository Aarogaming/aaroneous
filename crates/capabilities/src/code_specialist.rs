//! crates/specialists/src/code_specialist.rs
//! Concrete Dynamic Domain Specialist Plugin implementing the Aaroneous `SpecialistEngine` C-ABI.
//! Provides code manipulation, AST analysis dispatch, and 256-dimensional latent transformation.

use aaroneous_sdk::dynamic_plugin::{SpecialistEngine, SpecialistPluginManifest, SPECIALIST_ABI_VERSION};
use anyhow::Result;
use std::os::raw::c_char;

static PLUGIN_NAME: &[u8] = b"CodeSpecialist\0";
static PLUGIN_VERSION: &[u8] = b"0.1.0\0";

/// Reference dynamic code specialist engine
pub struct CodeSpecialist {
    name: String,
    version: String,
}

impl Default for CodeSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeSpecialist {
    pub fn new() -> Self {
        Self {
            name: "CodeSpecialist".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

impl SpecialistEngine for CodeSpecialist {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn execute_action(&self, input: &[u8]) -> Result<Vec<u8>> {
        // Echo and transform code payload
        let mut response = Vec::with_capacity(input.len() + 16);
        response.extend_from_slice(b"CODE_ACK:");
        response.extend_from_slice(input);
        Ok(response)
    }

    fn process_latent(&self, latent: &[f32; 256]) -> [f32; 256] {
        let mut out = *latent;
        // Non-linear projection on primary code latent axis
        out[0] = out[0].tanh();
        out[1] = (out[1] * 0.5) + 0.1;
        out
    }
}

/// Dynamic ABI Entrypoint: Instantiates the Code Specialist
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn aaroneous_create_specialist() -> *mut dyn SpecialistEngine {
    Box::into_raw(Box::new(CodeSpecialist::new()))
}

/// Dynamic ABI Entrypoint: Queries the Plugin Manifest
#[no_mangle]
pub extern "C" fn aaroneous_specialist_manifest() -> SpecialistPluginManifest {
    SpecialistPluginManifest {
        abi_version: SPECIALIST_ABI_VERSION,
        name: PLUGIN_NAME.as_ptr() as *const c_char,
        version: PLUGIN_VERSION.as_ptr() as *const c_char,
        capability_flags: 0x01, // CODE_ANALYSIS_CAPABILITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_specialist_direct_execution() {
        let specialist = CodeSpecialist::new();
        assert_eq!(specialist.name(), "CodeSpecialist");
        assert_eq!(specialist.version(), "0.1.0");

        let res = specialist.execute_action(b"fn main() {}").unwrap();
        assert_eq!(res, b"CODE_ACK:fn main() {}");

        let latent = [0.5f32; 256];
        let transformed = specialist.process_latent(&latent);
        assert!((transformed[0] - 0.5f32.tanh()).abs() < 1e-5);
    }
}
