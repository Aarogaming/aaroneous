use anyhow::{Context, Result};
use shared_memory::{Shmem, ShmemConf};
use std::ffi::c_void;

#[repr(C)]
pub struct AasBuffer {
    pub data: *mut c_void,
    pub size: u64,
    pub capacity: u64,
}

pub struct SharedMemorySynapse {
    shmem: Shmem,
}

impl SharedMemorySynapse {
    pub fn new(size: usize) -> Result<Self> {
        let shmem = ShmemConf::new().size(size).create().context("Failed to create shared memory")?;
        Ok(SharedMemorySynapse { shmem })
    }

    pub fn as_buffer(&self) -> AasBuffer {
        AasBuffer {
            data: self.shmem.as_ptr() as *mut c_void,
            size: self.shmem.len() as u64,
            capacity: self.shmem.len() as u64,
        }
    }
}
