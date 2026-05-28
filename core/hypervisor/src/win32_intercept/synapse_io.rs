// Synapse I/O - Shared memory channel for frame data and motor intents

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::win32_intercept::hid_bridge::MotorIntent;

pub const SYNAPSE_MAGIC: [u8; 4] = *b"AAS1";
pub const SYNAPSE_SIZE: usize = 1024 * 1024; // 1MB
pub const GRID_SIZE: usize = 128 * 128;

/// Shared memory channel for the spatial-kinetic pipeline
pub struct SynapseChannel {
    name: String,
    path: PathBuf,
    file: Option<std::fs::File>,
}

impl std::fmt::Debug for SynapseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SynapseChannel")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("is_open", &self.file.is_some())
            .finish()
    }
}

impl SynapseChannel {
    pub fn new(name: &str) -> Self {
        let path = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
            .join("Temp")
            .join(format!("{}.synapse", name));
        
        Self {
            name: name.to_string(),
            path,
            file: None,
        }
    }

    /// Public getter for name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn open(&mut self) -> std::io::Result<()> {
        self.file = Some(OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.path)?);
        Ok(())
    }
}