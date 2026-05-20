use std::fs::OpenOptions;
use std::path::Path;
use memmap2::MmapMut;
use rkyv::{Archive, Serialize, Deserialize};
use anyhow::{Result, anyhow};
use nervous_system::shared_memory::{
    SynapseState as ZeroCopyState,
    McpToolCallFrame as ZeroCopyMcpFrame,
    SpecialistDialogue as ZeroCopyDialogue,
};

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct McpToolCallFrame {
    pub tool_name: String,
    pub arguments: String,
    pub call_id: String,
}

impl McpToolCallFrame {
    pub fn from_zero_copy(zc: &ZeroCopyMcpFrame, tool_name_registry: &[(u64, String)]) -> Self {
        let tool_name = tool_name_registry.iter()
            .find(|(hash, _)| *hash == zc.tool_name_hash)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| format!("hash_{}", zc.tool_name_hash));
        
        let arguments = String::from_utf8_lossy(&zc.arguments_payload[..zc.arguments_size as usize])
            .to_string();
        
        Self {
            tool_name,
            arguments,
            call_id: zc.call_id.to_string(),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct SpecialistDialogue {
    pub specialist_id: String,
    pub message: String,
    pub priority: u8,
}

impl SpecialistDialogue {
    pub fn from_zero_copy(zc: &ZeroCopyDialogue, speaker_registry: &[(u64, String)]) -> Self {
        let specialist_id = speaker_registry.iter()
            .find(|(hash, _)| *hash == zc.active_speaker_hash)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| format!("hash_{}", zc.active_speaker_hash));
        
        let message = String::from_utf8_lossy(&zc.message_payload[..zc.message_size as usize])
            .to_string();
        
        Self {
            specialist_id,
            message,
            priority: zc.consensus_score,
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct SynapseState {
    pub clock_tick: u64,
    pub latent_vector: [f32; 1024],
    pub mcp_frame: Option<McpToolCallFrame>,
    pub dialogue: Vec<SpecialistDialogue>,
}

impl SynapseState {
    pub fn from_zero_copy(zc: &ZeroCopyState, tool_registry: &[(u64, String)], speaker_registry: &[(u64, String)]) -> Self {
        let mcp_frame = if zc.mcp_tool_call.status > 0 {
            Some(McpToolCallFrame::from_zero_copy(&zc.mcp_tool_call, tool_registry))
        } else {
            None
        };
        
        Self {
            clock_tick: zc.clock_tick,
            latent_vector: zc.latent_vector,
            mcp_frame,
            dialogue: vec![SpecialistDialogue::from_zero_copy(&zc.dialogue, speaker_registry)],
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
#[archive(check_bytes)]
pub struct SynapsePayload {
    pub key: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

pub struct Synapse {
    mmap: MmapMut,
}

impl Synapse {
    pub fn new(path: &Path, size: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        file.set_len(size as u64)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        
        Ok(Self { mmap })
    }

    pub fn write_payload(&mut self, payload: &SynapsePayload) -> Result<()> {
        let bytes = rkyv::to_bytes::<_, 1024>(payload)?;
        if bytes.len() > self.mmap.len() {
            return Err(anyhow!("Payload too large for synapse"));
        }
        
        self.mmap[..bytes.len()].copy_from_slice(&bytes);
        self.mmap.flush()?;
        Ok(())
    }

    pub fn read_payload(&self) -> Result<SynapsePayload> {
        let archived = unsafe { rkyv::check_archived_root::<SynapsePayload>(&self.mmap[..]) }
            .map_err(|e| anyhow!("Failed to check archived root: {:?}", e))?;
        
        let payload: SynapsePayload = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(payload)
    }
}
