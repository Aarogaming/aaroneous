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

/// Current Universal Client Protocol Version
pub const UCP_PROTOCOL_VERSION: u32 = 1;

/// Standard Named Pipe path on Windows
pub const UCP_NAMED_PIPE_PATH: &str = r"\\.\pipe\aaroneous_ucp_v1";

/// Standard WebSocket port for local and remote frontends
pub const UCP_DEFAULT_WS_PORT: u16 = 8765;

/// Requests emitted by ANY frontend to the Aaroneous Core Engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum UniversalClientRequest {
    /// Initial client connection handshake
    Handshake {
        client_name: String,
        protocol_version: u32,
    },
    /// Submits plain-text user intent for linguistic transduction & Opcode compilation
    SubmitIntent {
        prompt: String,
    },
    /// Mounts an `.si` or `.si-pack` cartridge into an active execution slot
    MountCartridge {
        slot_id: usize,
        file_path: String,
    },
    /// Adjusts active operational domain (e.g. InteractiveDesktop, RealTimeTelemetryControl)
    SetExecutionDomain {
        domain_id: u8,
    },
    /// Immediate hardware safety interlock cutoff
    EmergencyStop,
    /// Heartbeat ping to verify connection latency
    Ping {
        sequence: u64,
    },
}

/// Telemetry frames broadcasted by the Aaroneous Core Engine to ALL frontends
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum UniversalServerBroadcast {
    /// Response to successful handshake
    HandshakeAck {
        server_version: u32,
        session_id: String,
        accepted: bool,
    },
    /// High-frequency execution telemetry frame (10Hz to 120Hz)
    TelemetryFrame {
        timestamp_us: u64,
        free_energy_delta: f32,
        cycle_latency_us: u64,
        active_modules: Vec<String>,
        feedback_message: String,
    },
    /// Safety interlock notification or error trip
    SafetyAlert {
        message: String,
        is_tripped: bool,
    },
    /// Heartbeat pong response
    Pong {
        sequence: u64,
    },
}

impl UniversalClientRequest {
    /// Serializes request to JSON string for WebSockets / HTTP
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializes request from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl UniversalServerBroadcast {
    /// Serializes broadcast to JSON string for WebSockets / HTTP
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializes broadcast from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ucp_request_json_roundtrip() {
        let req = UniversalClientRequest::SubmitIntent {
            prompt: "Optimize aerodynamic stability".to_string(),
        };

        let json = req.to_json().unwrap();
        assert!(json.contains("SubmitIntent"));
        assert!(json.contains("Optimize aerodynamic stability"));

        let deserialized: UniversalClientRequest = UniversalClientRequest::from_json(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn test_ucp_broadcast_json_roundtrip() {
        let frame = UniversalServerBroadcast::TelemetryFrame {
            timestamp_us: 42000,
            free_energy_delta: 0.012,
            cycle_latency_us: 15,
            active_modules: vec!["OpticalPerception".to_string(), "KineticDispatch".to_string()],
            feedback_message: "Equilibrium nominal".to_string(),
        };

        let json = frame.to_json().unwrap();
        assert!(json.contains("TelemetryFrame"));
        assert!(json.contains("OpticalPerception"));

        let deserialized: UniversalServerBroadcast = UniversalServerBroadcast::from_json(&json).unwrap();
        assert_eq!(frame, deserialized);
    }
}
