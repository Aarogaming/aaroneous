// src/lib.rs
use anyhow::Result;
use libloading::Library;
use std::path::Path;

/// Loads a dynamic library and returns a handle.
pub fn load_module<P: AsRef<Path>>(path: P) -> Result<Library> {
    let lib = unsafe { Library::new(path.as_ref()) }?;
    Ok(lib)
}
