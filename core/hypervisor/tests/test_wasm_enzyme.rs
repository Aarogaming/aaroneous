#[cfg(test)]
mod wasm_tests {
    use a_run::wasm_loader::WasmEnzymeLoader;
    use std::path::PathBuf;

    #[test]
    fn test_load_and_run_enzyme() {
        let loader = WasmEnzymeLoader::new().expect("Failed to create loader");
        
        // Path to the compiled test enzyme
        let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../extensions/wasm/test_enzyme/target/wasm32-unknown-unknown/release/test_enzyme.wasm");
            
        if !wasm_path.exists() {
            println!("[Test] WASM file not found at {:?}, skipping test.", wasm_path);
            return;
        }

        let result = loader.load_and_run(wasm_path.to_str().unwrap());
        assert!(result.is_ok(), "Failed to run enzyme: {:?}", result);
    }
}
