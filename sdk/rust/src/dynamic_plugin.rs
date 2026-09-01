//! sdk/rust/src/dynamic_plugin.rs
//! Dynamic Specialist Plugin ABI & Hot-Reload Loader for Aaroneous.
//! Enables live patching, dynamic shared library (`.dll` / `.so`) loading,
//! and hot-swapping specialist engines at runtime without hypervisor downtime.

use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

pub const SPECIALIST_ABI_VERSION: u32 = 1;

/// C-compatible manifest exported by dynamic specialist plugins
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpecialistPluginManifest {
    pub abi_version: u32,
    pub name: *const c_char,
    pub version: *const c_char,
    pub capability_flags: u32,
}

/// Standard trait implemented by dynamic specialist plugins
pub trait SpecialistEngine: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn execute_action(&self, input: &[u8]) -> Result<Vec<u8>>;
    fn process_latent(&self, latent: &[f32; 256]) -> [f32; 256];
}

/// C ABI entrypoint types
#[allow(improper_ctypes_definitions)]
pub type CreateSpecialistFn = unsafe extern "C" fn() -> *mut dyn SpecialistEngine;
pub type GetManifestFn = unsafe extern "C" fn() -> SpecialistPluginManifest;

struct LoadedPluginEntry {
    _lib: Library,
    engine: Box<dyn SpecialistEngine>,
    #[allow(dead_code)]
    path: PathBuf,
    version: String,
}

/// Dynamic Specialist Plugin Loader managing live library instances and safe hot-swapping
pub struct DynamicSpecialistLoader {
    plugins: RwLock<HashMap<String, LoadedPluginEntry>>,
}

impl Default for DynamicSpecialistLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicSpecialistLoader {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Loads a dynamic specialist shared library (`.dll` / `.so`)
    pub fn load_specialist_library(&self, path: &Path) -> Result<String> {
        if !path.exists() {
            return Err(anyhow!("Shared library does not exist at {:?}", path));
        }

        unsafe {
            let lib = Library::new(path)
                .map_err(|e| anyhow!("Failed to load dynamic library at {:?}: {e}", path))?;

            // 1. Verify ABI Manifest
            let manifest_sym: Symbol<GetManifestFn> = lib
                .get(b"aaroneous_specialist_manifest\0")
                .map_err(|e| anyhow!("Missing manifest symbol 'aaroneous_specialist_manifest': {e}"))?;

            let manifest = manifest_sym();
            if manifest.abi_version != SPECIALIST_ABI_VERSION {
                return Err(anyhow!(
                    "ABI version mismatch: plugin has v{}, runtime expects v{}",
                    manifest.abi_version,
                    SPECIALIST_ABI_VERSION
                ));
            }

            let name_cstr = CStr::from_ptr(manifest.name);
            let name = name_cstr.to_str()?.to_string();

            let ver_cstr = CStr::from_ptr(manifest.version);
            let version = ver_cstr.to_str()?.to_string();

            // 2. Instantiate Specialist Engine
            let create_sym: Symbol<CreateSpecialistFn> = lib
                .get(b"aaroneous_create_specialist\0")
                .map_err(|e| anyhow!("Missing entrypoint symbol 'aaroneous_create_specialist': {e}"))?;

            let raw_ptr = create_sym();
            if raw_ptr.is_null() {
                return Err(anyhow!("Plugin create entrypoint returned null pointer"));
            }

            let engine = Box::from_raw(raw_ptr);

            self.plugins.write().insert(
                name.clone(),
                LoadedPluginEntry {
                    _lib: lib,
                    engine,
                    path: path.to_path_buf(),
                    version,
                },
            );

            Ok(name)
        }
    }

    /// Atomically hot-swaps an existing specialist plugin with an updated shared library version
    pub fn hot_swap_specialist(&self, name: &str, new_path: &Path) -> Result<()> {
        if !new_path.exists() {
            return Err(anyhow!("New shared library does not exist at {:?}", new_path));
        }

        unsafe {
            let lib = Library::new(new_path)
                .map_err(|e| anyhow!("Failed to load new dynamic library at {:?}: {e}", new_path))?;

            let manifest_sym: Symbol<GetManifestFn> = lib
                .get(b"aaroneous_specialist_manifest\0")
                .map_err(|e| anyhow!("Missing manifest symbol: {e}"))?;

            let manifest = manifest_sym();
            if manifest.abi_version != SPECIALIST_ABI_VERSION {
                return Err(anyhow!("ABI mismatch on hot-swap"));
            }

            let create_sym: Symbol<CreateSpecialistFn> = lib
                .get(b"aaroneous_create_specialist\0")
                .map_err(|e| anyhow!("Missing create symbol: {e}"))?;

            let raw_ptr = create_sym();
            if raw_ptr.is_null() {
                return Err(anyhow!("Hot-swapped plugin returned null pointer"));
            }

            let new_engine = Box::from_raw(raw_ptr);
            let ver_cstr = CStr::from_ptr(manifest.version);
            let version = ver_cstr.to_str()?.to_string();

            // Atomically replace old plugin entry
            let mut guard = self.plugins.write();
            guard.insert(
                name.to_string(),
                LoadedPluginEntry {
                    _lib: lib,
                    engine: new_engine,
                    path: new_path.to_path_buf(),
                    version,
                },
            );

            Ok(())
        }
    }

    /// Registers an in-process mock / statically linked specialist engine directly
    pub fn register_in_process(&self, engine: Box<dyn SpecialistEngine>) {
        let name = engine.name().to_string();
        let version = engine.version().to_string();
        // Create an empty dummy library reference by using own executable
        if let Ok(lib) = unsafe { Library::new(std::env::current_exe().unwrap_or_default()) } {
            self.plugins.write().insert(
                name,
                LoadedPluginEntry {
                    _lib: lib,
                    engine,
                    path: PathBuf::new(),
                    version,
                },
            );
        }
    }

    /// Queries an action execution from a live specialist plugin
    pub fn execute_specialist_action(&self, name: &str, input: &[u8]) -> Result<Vec<u8>> {
        let guard = self.plugins.read();
        let plugin = guard
            .get(name)
            .ok_or_else(|| anyhow!("Specialist plugin '{}' is not loaded", name))?;
        plugin.engine.execute_action(input)
    }

    /// Returns list of loaded specialist plugin names and versions
    pub fn list_plugins(&self) -> Vec<(String, String)> {
        self.plugins
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.version.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSpecialist {
        name: String,
        version: String,
    }

    impl SpecialistEngine for MockSpecialist {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> &str {
            &self.version
        }
        fn execute_action(&self, input: &[u8]) -> Result<Vec<u8>> {
            let mut out = input.to_vec();
            out.reverse();
            Ok(out)
        }
        fn process_latent(&self, latent: &[f32; 256]) -> [f32; 256] {
            let mut out = *latent;
            out[0] += 1.0;
            out
        }
    }

    #[test]
    fn test_dynamic_specialist_loader_in_process_registration() {
        let loader = DynamicSpecialistLoader::new();
        loader.register_in_process(Box::new(MockSpecialist {
            name: "Mock-Vision".to_string(),
            version: "1.0.0".to_string(),
        }));

        let plugins = loader.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].0, "Mock-Vision");

        let res = loader.execute_specialist_action("Mock-Vision", b"hello").unwrap();
        assert_eq!(res, b"olleh");
    }
}
