use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use crate::llm::LLMClient;
use crate::workspace::WorkspacePaths;
use std::sync::Arc;

pub struct AutoFabricator {
    llm: Arc<LLMClient>,
    paths: WorkspacePaths,
}

impl AutoFabricator {
    pub fn new(llm: Arc<LLMClient>, paths: WorkspacePaths) -> Self {
        Self { llm, paths }
    }
    
    pub async fn fabricate(&self, crate_name: &str) -> Result<PathBuf> {
        let work_dir = self.paths.fabrication_workspace(crate_name);
        if work_dir.exists() {
            fs::remove_dir_all(&work_dir)?;
        }
        
        fs::create_dir_all(self.paths.sabs())?;
        
        // 1. Copy template
        self.copy_dir(&self.paths.sab_template(), &work_dir)?;
        
        // 2. Ask LLM for the Cargo.toml dependency string and the lib.rs code
        let prompt = format!(
            "You are an expert Rust programmer building an Aaroneous SAB plugin for the `wasm32-wasip1` target.\n\
            Write a SAB plugin wrapper for the open-source crate '{}'.\n\n\
            CRITICAL WASM CONSTRAINTS:\n\
            - WebAssembly (WASI) does NOT support raw OS threads, raw TCP/UDP sockets, or hardware terminal access.\n\
            - If this crate uses threads, networking, or OS features by default, you MUST output the dependency with `default-features = false` and only enable WASM-safe features.\n\
            - Example: {} = {{ version = \"*\", default-features = false }}\n\n\
            Provide the dependency line for Cargo.toml as [DEPENDENCY] ... [/DEPENDENCY]\n\
            Provide the Rust code for src/lib.rs as [RUST] ... [/RUST]\n\n\
            The Rust code must use C-ABI:\n\
            1. #[no_mangle] pub extern \"C\" fn execute_task(cmd_ptr: *const std::os::raw::c_char, payload_ptr: *const std::os::raw::c_char) -> *mut std::os::raw::c_char\n\
            2. Wrap basic features of {} and return the result as a CString pointer", crate_name, crate_name, crate_name);
        
        // Use domain "code_generation"
        let response = self.llm.generate_domain_response(
            "You write highly efficient Rust WASM plugins.",
            &prompt,
            "code_generation"
        ).await?;
        
        // 3. Extract [DEPENDENCY] and [RUST]
        let mut dep_str = self.extract_section(&response, "[DEPENDENCY]", "[/DEPENDENCY]")
            .unwrap_or_else(|| format!("{} = \"*\"", crate_name));
        
        if dep_str.contains("...") || !dep_str.contains("=") {
            dep_str = format!("{} = \"*\"", crate_name);
        }
            
        let rust_str = self.extract_section(&response, "[RUST]", "[/RUST]");
        
        // 4. Update Cargo.toml
        let cargo_path = work_dir.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_path)?;
        cargo_toml = cargo_toml.replace("# [TARGET_CRATE_DEPENDENCY]", &dep_str);
        cargo_toml = cargo_toml.replace("universal-sab-template", &format!("{}-sab", crate_name));
        fs::write(&cargo_path, cargo_toml)?;
        
        // 5. Update src/lib.rs if provided
        if let Some(rust_code) = rust_str {
            if rust_code.contains("execute_task") {
                let lib_path = work_dir.join("src").join("lib.rs");
                fs::write(&lib_path, rust_code)?;
            }
        }
        
        // 6. Build the WASM
        tracing::info!("AutoFabricator: Compiling SAB for {}", crate_name);
        let build_cmd = std::process::Command::new("cargo")
            .arg("build")
            .arg("--target")
            .arg("wasm32-wasip1")
            .arg("--release")
            .current_dir(&work_dir)
            .output()?;
            
        if !build_cmd.status.success() {
            let err = String::from_utf8_lossy(&build_cmd.stderr);
            tracing::warn!("AutoFabricator: WASM compilation failed for {}. Attempting native fallback. Error snippet: {}", crate_name, err.chars().take(200).collect::<String>());
            return self.fabricate_native(crate_name).await;
        }
        
        // 7. Extract the compiled WASM
        let wasm_name = format!("{}_sab.wasm", crate_name.replace("-", "_"));
        let wasm_path = work_dir.join("target").join("wasm32-wasip1").join("release").join(&wasm_name);
        
        if !wasm_path.exists() {
            return Err(anyhow!("WASM file not found at {}", wasm_path.display()));
        }
        
        let final_path = self.paths.sabs().join(format!("{}.wasm", crate_name));
        fs::copy(&wasm_path, &final_path)?;
        
        tracing::info!("AutoFabricator: Successfully fabricated SAB -> {}", final_path.display());
        
        Ok(final_path)
    }
    
    pub async fn fabricate_native(&self, crate_name: &str) -> Result<PathBuf> {
        let work_dir = self.paths.fabrication_native_workspace(crate_name);
        if work_dir.exists() {
            fs::remove_dir_all(&work_dir)?;
        }
        
        fs::create_dir_all(self.paths.sabs())?;
        
        self.copy_dir(&self.paths.native_template(), &work_dir)?;
        
        let prompt = format!(
            "You are an expert Rust programmer building an Aaroneous native enzyme plugin.\n\
            Write a native dynamic library wrapper for the open-source crate '{}'.\n\n\
            Provide the dependency line for Cargo.toml as [DEPENDENCY] ... [/DEPENDENCY]\n\
            Provide the Rust code for src/lib.rs as [RUST] ... [/RUST]\n\n\
            The Rust code must include the following struct EXACTLY:\n\
            #[repr(C)]\n\
            pub struct AasBuffer {{\n\
                pub data: *mut std::ffi::c_void,\n\
                pub size: u64,\n\
                pub capacity: u64,\n\
            }}\n\n\
            The Rust code must also implement:\n\
            1. #[no_mangle] pub extern \"C\" fn aas_init() -> i32\n\
            2. #[no_mangle] pub extern \"C\" fn aas_process(input: *mut AasBuffer, output: *mut AasBuffer) -> i32\n\
            3. Wrap basic features of {} inside aas_process", crate_name, crate_name);
            
        let response = self.llm.generate_domain_response(
            "You write highly efficient Rust native plugins.",
            &prompt,
            "code_generation"
        ).await?;
        
        let mut dep_str = self.extract_section(&response, "[DEPENDENCY]", "[/DEPENDENCY]")
            .unwrap_or_else(|| format!("{} = \"*\"", crate_name));
            
        if dep_str.contains("...") || !dep_str.contains("=") {
            dep_str = format!("{} = \"*\"", crate_name);
        }
            
        let rust_str = self.extract_section(&response, "[RUST]", "[/RUST]");
        
        let cargo_path = work_dir.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_path)?;
        cargo_toml = cargo_toml.replace("# [TARGET_CRATE_DEPENDENCY]", &dep_str);
        cargo_toml = cargo_toml.replace("universal-native-template", &format!("{}-native", crate_name));
        fs::write(&cargo_path, cargo_toml)?;
        
        if let Some(rust_code) = rust_str {
            if rust_code.contains("aas_process") {
                let lib_path = work_dir.join("src").join("lib.rs");
                fs::write(&lib_path, rust_code)?;
            }
        }
        
        tracing::info!("AutoFabricator: Compiling NATIVE enzyme for {}", crate_name);
        let build_cmd = std::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&work_dir)
            .output()?;
            
        if !build_cmd.status.success() {
            let err = String::from_utf8_lossy(&build_cmd.stderr);
            return Err(anyhow!("Native compilation failed for {}: {}", crate_name, err));
        }
        
        let dll_name = format!("{}_native.dll", crate_name.replace("-", "_"));
        let dll_path = work_dir.join("target").join("release").join(&dll_name);
        
        if !dll_path.exists() {
            return Err(anyhow!("Native library not found at {}", dll_path.display()));
        }
        
        let final_path = self.paths.sabs().join(format!("{}.dll", crate_name));
        fs::copy(&dll_path, &final_path)?;
        
        tracing::info!("AutoFabricator: Successfully fabricated NATIVE enzyme -> {}", final_path.display());
        
        Ok(final_path)
    }

    fn copy_dir(&self, src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());
            if ft.is_dir() {
                if entry.file_name() != "target" {
                    self.copy_dir(&entry.path(), &dest_path)?;
                }
            } else {
                fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }
    
    fn extract_section(&self, text: &str, start_tag: &str, end_tag: &str) -> Option<String> {
        if let Some(start) = text.find(start_tag) {
            if let Some(end) = text[start + start_tag.len()..].find(end_tag) {
                return Some(text[start + start_tag.len()..start + start_tag.len() + end].trim().to_string());
            }
        }
        None
    }
}