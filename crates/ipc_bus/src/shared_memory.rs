use std::ptr;
use anyhow::{Result, Context};
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::fs;

/// The raw memory layout for the Autonomic Nervous System
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SynapseState {
    pub clock_tick: u64,
    pub energy_budget: u32,
    pub memory_pressure: u8,
    pub safety_lock: u8,        // 0: Neutral, 1: Violation Blocked
    pub approval_required: u8,  // 0: No, 1: Awaiting User
    pub approval_granted: u8,   // 0: No, 1: Yes
    pub hox_mutation_flag: u8,
    pub intent_vector_id: [u8; 16],
    pub intent_payload: [u8; 256],
    pub sovereignty_tier: u8,   // 0: Local, 1: Bounded Web, 2: Remote LLM
    
    // Homeostatic Metrics (The "Dopamine" System)
    pub curiosity_drive: u8,    // 0-100: Triggers autonomous research
    pub integrity_score: u8,    // 0-100: Drops on errors/inconsistencies
    pub understanding_score: u8, // 0-100: Confidence in current context
    pub concept_drift: f32,      // 0.0 - 1.0: Mathematical divergence score
    
    // Latent Space Injection Buffers (The "Mathematical Thought" pipeline)
    // Bypasses string parsing by passing raw embedding vectors (1024-dim, f32)
    pub latent_activation_id: [u8; 16],
    pub latent_vector: [f32; 1024],
    
    // MCP (Model Context Protocol) Shared-Memory IPC Partition
    pub mcp_tool_call: McpToolCallFrame,
    
    // Specialist Dialogue Partition (The "Debate" space)
    pub dialogue: SpecialistDialogue,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpecialistDialogue {
    pub active_speaker_hash: u64, // Pre-hashed specialist name
    pub turn_count: u32,
    pub consensus_score: u8,      // 0-100: Calculated by the Diplomat Enzyme
    pub message_size: u32,
    pub message_payload: [u8; 1024], // Current specialist thought (MessagePack/BSON)
}

impl Default for SpecialistDialogue {
    fn default() -> Self {
        Self {
            active_speaker_hash: 0,
            turn_count: 0,
            consensus_score: 50,
            message_size: 0,
            message_payload: [0; 1024],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct McpToolCallFrame {
    pub call_id: u64,             // Unique monotonic sequence identifier
    pub tool_name_hash: u64,      // Pre-hashed tool name
    pub status: u8,               // 0=Idle, 1=Pending, 2=Executing, 3=Success, 4=Failed
    pub arguments_size: u32,      // Length of packed binary arguments block
    pub arguments_payload: [u8; 2048], // Pre-allocated byte buffer for inputs (MessagePack / BSON)
}

impl Default for McpToolCallFrame {
    fn default() -> Self {
        Self {
            call_id: 0,
            tool_name_hash: 0,
            status: 0,
            arguments_size: 0,
            arguments_payload: [0; 2048],
        }
    }
}

impl Default for SynapseState {
    fn default() -> Self {
        Self {
            clock_tick: 0,
            energy_budget: 1000,
            memory_pressure: 0,
            safety_lock: 0,
            approval_required: 0,
            approval_granted: 0,
            hox_mutation_flag: 0,
            intent_vector_id: [0; 16],
            intent_payload: [0; 256],
            sovereignty_tier: 0,
            curiosity_drive: 50,
            integrity_score: 100,
            understanding_score: 100,
            concept_drift: 0.0,
            latent_activation_id: [0; 16],
            latent_vector: [0.0; 1024],
            mcp_tool_call: McpToolCallFrame::default(),
            dialogue: SpecialistDialogue::default(),
        }
    }
}

pub struct SharedMemorySynapse {
    mmap: MmapMut,
    path: PathBuf,
}

impl SharedMemorySynapse {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        let path = aaroneous_paths::resolve_synapse_path(name);
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .with_context(|| format!("Failed to open/create synapse file at {:?}", path))?;

        file.set_len(size as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self { mmap, path })
    }

    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.mmap.len() {
            anyhow::bail!("Synapse overflow: writing past allocated size");
        }
        self.mmap[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn read_at(&self, offset: usize, len: usize) -> Result<&[u8]> {
        if offset + len > self.mmap.len() {
            anyhow::bail!("Synapse overflow: reading past allocated size");
        }
        Ok(&self.mmap[offset..offset + len])
    }

    pub fn get_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }
}

impl Drop for SharedMemorySynapse {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
