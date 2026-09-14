// crates/ipc_bus/src/universal_protocol.rs
//! Universal Client Protocol (UCP) Specification & Schema.
//!
//! Provides a single, canonical, serialization-agnostic wire protocol for ANY frontend:
//! - Native Desktop Studio HUD (DirectX 12 / Vulkan / egui)
//! - Compact Floating Toolbar Overlay Widget (F10)
//! - In-Game Swapchain Overlay (DirectX 11/12 / Vulkan via hudhook)
//! - Remote Web, Mobile, or Custom Hardware Head-Units (WebSocket / Named Pipe)
//!
//! Guarantees:
//! 1. Version Handshake: Semantic version negotiation ensures client-server compatibility.
//! 2. Loose Coupling: Frontends contain zero engine execution logic.
//! 3. Bounded Telemetry: Asynchronous broadcast protects real-time core loops from UI lag.

use serde::{Deserialize, Serialize};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct FixedString256 {
    pub len: u32,
    pub data: [u8; 256],
}

impl FixedString256 {
    pub fn new(s: &str) -> Result<Self, &'static str> {
        let bytes = s.as_bytes();
        if bytes.len() > 256 {
            return Err("String exceeds 256 bytes");
        }
        let mut data = [0u8; 256];
        data[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            len: bytes.len() as u32,
            data,
        })
    }

    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.data[..self.len as usize])
    }
}

impl Default for FixedString256 {
    fn default() -> Self {
        Self {
            len: 0,
            data: [0u8; 256],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct FixedString64 {
    pub len: u32,
    pub data: [u8; 64],
}

impl FixedString64 {
    pub fn new(s: &str) -> Result<Self, &'static str> {
        let bytes = s.as_bytes();
        if bytes.len() > 64 {
            return Err("String exceeds 64 bytes");
        }
        let mut data = [0u8; 64];
        data[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            len: bytes.len() as u32,
            data,
        })
    }

    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.data[..self.len as usize])
    }
}

impl Default for FixedString64 {
    fn default() -> Self {
        Self {
            len: 0,
            data: [0u8; 64],
        }
    }
}

/// Current Universal Client Protocol Version
pub const UCP_PROTOCOL_VERSION: u32 = 1;

/// Standard Named Pipe path on Windows
pub const UCP_NAMED_PIPE_PATH: &str = r"\\.\pipe\aaroneous_ucp_v1";

/// Standard WebSocket port for local and remote frontends
pub const UCP_DEFAULT_WS_PORT: u16 = 8765;

/// Requests emitted by ANY frontend to the Aaroneous Core Engine
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UcpRequestType {
    Handshake = 0,
    SubmitIntent = 1,
    MountCartridge = 2,
    SetExecutionDomain = 3,
    EmergencyStop = 4,
    Ping = 5,
    AssimilationEvent = 6,
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct UniversalClientRequest {
    pub req_type: u32,
    pub _pad0: u32,
    pub sequence: u64,
    pub slot_id: u32,
    pub domain_id: u8,
    pub _pad1: [u8; 3],
    pub payload_a: FixedString256,
    pub payload_b: FixedString256,
}

/// Telemetry frames broadcasted by the Aaroneous Core Engine to ALL frontends
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UcpBroadcastType {
    HandshakeAck = 0,
    TelemetryFrame = 1,
    SafetyAlert = 2,
    Pong = 3,
    AssimilationState = 4,
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct UniversalServerBroadcast {
    pub broadcast_type: u32,
    pub _pad0: u32,
    pub sequence: u64,
    pub timestamp_us: u64,
    pub cycle_latency_us: u64,
    pub free_energy_delta: f32,
    pub boolean_flag: u8,
    pub _pad1: [u8; 3],
    pub message: FixedString256,
    pub _pad2: u32,
}

/// Assimilation lifecycle phases
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssimilationPhase {
    Idle = 0,
    Quarantined = 1,
    Auditing = 2,
    Synthesizing = 3,
    Certifying = 4,
    Committed = 5,
    Rejected = 6,
}

impl AssimilationPhase {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(Self::Idle),
            1 => Some(Self::Quarantined),
            2 => Some(Self::Auditing),
            3 => Some(Self::Synthesizing),
            4 => Some(Self::Certifying),
            5 => Some(Self::Committed),
            6 => Some(Self::Rejected),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Quarantined => "Quarantined",
            Self::Auditing => "Auditing",
            Self::Synthesizing => "Synthesizing",
            Self::Certifying => "Certifying",
            Self::Committed => "Committed",
            Self::Rejected => "Rejected",
        }
    }
}

/// Zero-copy wire frame representing an asset undergoing assimilation
/// Exact size: 360 bytes. Perfectly 8-byte aligned (360 % 8 == 0).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct AssimilationRecord {
    pub source_id: [u8; 16],          // Offset: 0
    pub phase: u32,                   // Offset: 16
    pub retries: u32,                 // Offset: 20
    pub started_at_us: u64,           // Offset: 24
    pub mount_path: FixedString256,   // Offset: 32 (Size: 260)
    pub ir_hash: FixedString64,       // Offset: 292 (Size: 68)
}

impl AssimilationRecord {
    pub fn new(source_id: [u8; 16], timestamp_us: u64) -> Self {
        Self {
            source_id,
            phase: AssimilationPhase::Idle as u32,
            retries: 0,
            started_at_us: timestamp_us,
            mount_path: FixedString256::default(),
            ir_hash: FixedString64::default(),
        }
    }

    pub fn to_broadcast(&self, sequence: u64, cycle_latency_us: u64) -> UniversalServerBroadcast {
        let phase_str = AssimilationPhase::from_u32(self.phase)
            .map(|p| p.as_str())
            .unwrap_or("Unknown");
        let msg_str = match phase_str {
            "Committed" => "Assimilation completed successfully",
            "Rejected" => "Assimilation rejected during certification/audit",
            _ => "Assimilation in progress",
        };
        let message = FixedString256::new(msg_str).unwrap_or_default();
        UniversalServerBroadcast {
            broadcast_type: UcpBroadcastType::AssimilationState as u32,
            _pad0: 0,
            sequence,
            timestamp_us: self.started_at_us,
            cycle_latency_us,
            free_energy_delta: 0.0,
            boolean_flag: if self.phase == AssimilationPhase::Committed as u32 { 1 } else { 0 },
            _pad1: [0u8; 3],
            message,
            _pad2: 0,
        }
    }

    pub fn from_client_request(req: &UniversalClientRequest) -> Option<Self> {
        if req.req_type != UcpRequestType::AssimilationEvent as u32 {
            return None;
        }
        let mut source_id = [0u8; 16];
        source_id[..8].copy_from_slice(&req.sequence.to_le_bytes());
        source_id[8..12].copy_from_slice(&req.slot_id.to_le_bytes());
        source_id[12] = req.domain_id;

        let mut ir_hash = FixedString64::default();
        if let Ok(hash_str) = req.payload_b.as_str() {
            if let Ok(h) = FixedString64::new(hash_str) {
                ir_hash = h;
            }
        }

        Some(Self {
            source_id,
            phase: AssimilationPhase::Idle as u32,
            retries: 0,
            started_at_us: 0,
            mount_path: req.payload_a,
            ir_hash,
        })
    }

    pub fn to_client_request(&self, sequence: u64, slot_id: u32, domain_id: u8) -> UniversalClientRequest {
        let hash_str = self.ir_hash.as_str().unwrap_or("");
        let payload_b = FixedString256::new(hash_str).unwrap_or_default();
        UniversalClientRequest {
            req_type: UcpRequestType::AssimilationEvent as u32,
            _pad0: 0,
            sequence,
            slot_id,
            domain_id,
            _pad1: [0u8; 3],
            payload_a: self.mount_path,
            payload_b,
        }
    }
}

impl Default for AssimilationRecord {
    fn default() -> Self {
        Self {
            source_id: [0u8; 16],
            phase: 0,
            retries: 0,
            started_at_us: 0,
            mount_path: FixedString256::default(),
            ir_hash: FixedString64::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assimilation_record_geometry() {
        assert_eq!(core::mem::size_of::<AssimilationRecord>(), 360);
        assert_eq!(core::mem::align_of::<AssimilationRecord>(), 8);
        assert_eq!(core::mem::size_of::<UniversalClientRequest>(), 544);
        assert_eq!(core::mem::size_of::<UniversalServerBroadcast>(), 304);

        let record = AssimilationRecord::new([42u8; 16], 1000);
        let bytes = bytemuck::bytes_of(&record);
        assert_eq!(bytes.len(), 360);

        let recovered: &AssimilationRecord = bytemuck::try_from_bytes(bytes).expect("Pod conversion must succeed");
        assert_eq!(recovered.source_id, [42u8; 16]);
        assert_eq!(recovered.started_at_us, 1000);
        assert_eq!(recovered.phase, AssimilationPhase::Idle as u32);
    }
}

