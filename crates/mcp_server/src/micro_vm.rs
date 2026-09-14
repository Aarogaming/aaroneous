// src/micro_vm.rs

use serde::{Serialize, Deserialize};

/// Minimal stub for VmProgram used by mcp_server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmProgram;

impl Default for VmProgram {
    fn default() -> Self {
        Self
    }
}
