//! crates/specialists/tests/dynamic_loader_test.rs
//! Integration test for Dynamic Specialist Plugin Loader and Live Hot-Swapping.

use aaroneous_sdk::dynamic_plugin::{DynamicSpecialistLoader, SpecialistEngine};
use anyhow::Result;
use specialists::code_specialist::CodeSpecialist;

struct UpdatedCodeSpecialist {
    name: String,
    version: String,
}

impl SpecialistEngine for UpdatedCodeSpecialist {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn execute_action(&self, input: &[u8]) -> Result<Vec<u8>> {
        let mut response = Vec::with_capacity(input.len() + 16);
        response.extend_from_slice(b"CODE_V2_ACK:");
        response.extend_from_slice(input);
        Ok(response)
    }
    fn process_latent(&self, latent: &[f32; 256]) -> [f32; 256] {
        let mut out = *latent;
        out[0] *= 2.0;
        out
    }
}

#[test]
fn test_dynamic_specialist_loader_and_live_hotswap_lifecycle() {
    let loader = DynamicSpecialistLoader::new();

    // 1. Register V1 CodeSpecialist
    loader.register_in_process(Box::new(CodeSpecialist::new()));

    let plugins = loader.list_plugins();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].0, "CodeSpecialist");
    assert_eq!(plugins[0].1, "0.1.0");

    let res_v1 = loader
        .execute_specialist_action("CodeSpecialist", b"let x = 42;")
        .unwrap();
    assert_eq!(res_v1, b"CODE_ACK:let x = 42;");

    // 2. Hot-swap live in-process with V2 CodeSpecialist
    loader.register_in_process(Box::new(UpdatedCodeSpecialist {
        name: "CodeSpecialist".to_string(),
        version: "0.2.0".to_string(),
    }));

    let plugins_after = loader.list_plugins();
    assert_eq!(plugins_after.len(), 1);
    assert_eq!(plugins_after[0].0, "CodeSpecialist");
    assert_eq!(plugins_after[0].1, "0.2.0");

    let res_v2 = loader
        .execute_specialist_action("CodeSpecialist", b"let x = 42;")
        .unwrap();
    assert_eq!(res_v2, b"CODE_V2_ACK:let x = 42;");
}
