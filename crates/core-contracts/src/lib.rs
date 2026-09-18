// Core contracts for Aaroneous microkernel

pub mod metadata;
pub use metadata::*;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// Hierarchy tier of a component.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HierarchyTier {
    Core = 0,
    System = 1,
    SubordinateModule = 2,
    Extension = 3,
}

/// Execution methodology flags.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMethodology {
    LocalQuantizedModel = 0x01,
    RemoteApiProxy = 0x02,
    HardwareAcceleratedDirect = 0x03,
    DeterministicSimulation = 0x04,
}

bitflags! {
    /// Operational intent bitflags.
    #[derive(Serialize, Deserialize)]
    pub struct OperationalIntent: u16 {
        const RealtimeLowLatency = 0b0000_0000_0000_0001;
        const BatchProcessing = 0b0000_0000_0000_0010;
        const HighThroughput = 0b0000_0000_0000_0100;
        const SecureEnclave = 0b0000_0000_0000_1000;
    }
}

bitflags! {
    /// Capability bitmask for components.
    #[derive(Serialize, Deserialize)]
    pub struct Capability: u64 {
        const INFERENCE_OUT = 0b0000_0001;
        const AUDIO_OUT = 0b0000_0010;
        const VIDEO_OUT = 0b0000_0100;
        const SENSOR_INPUT = 0b0000_1000;
    }
}

/// Zero‑copy IPC header used by transport frames.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct IpcHeader {
    pub dst: u32,
    pub src: u32,
    pub len: u32,
    pub flags: u32,
}

/// Zero-copy point-in-time snapshot of the hypervisor core engine state for out-of-process shell isolates.
/// Complies with `bytemuck::Pod` and zero-allocation hot-path rules.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct EngineSnapshotPod {
    pub timestamp_ms: u64,
    pub bus_generation: u64,
    pub user_xp: u64,
    pub measured_fps: f32,
    pub bus_integrity: f32,
    pub bus_understanding: f32,
    pub flow_score: f32,
    pub user_level: u32,
    pub active_companions_count: u32,
    pub running_macros_count: u32,
    pub pacing: u32,
    pub active_specialist: [u8; 32],
    pub active_profile_name: [u8; 32],
    pub last_event_desc: [u8; 128],
}

impl Default for EngineSnapshotPod {
    fn default() -> Self {
        let mut pod = Self {
            timestamp_ms: 0,
            bus_generation: 1,
            user_xp: 0,
            measured_fps: 120.0,
            bus_integrity: 99.4,
            bus_understanding: 98.6,
            flow_score: 0.85,
            user_level: 1,
            active_companions_count: 0,
            running_macros_count: 0,
            pacing: 0,
            active_specialist: [0; 32],
            active_profile_name: [0; 32],
            last_event_desc: [0; 128],
        };
        pod.set_active_specialist("Orchestrator");
        pod.set_active_profile_name("Default Operator");
        pod.set_last_event_desc("Core initialized and nominal");
        pod
    }
}

impl EngineSnapshotPod {
    #[inline]
    pub fn active_specialist_str(&self) -> &str {
        decode_fixed_str(&self.active_specialist)
    }

    #[inline]
    pub fn set_active_specialist(&mut self, s: &str) {
        encode_fixed_str(&mut self.active_specialist, s);
    }

    #[inline]
    pub fn active_profile_name_str(&self) -> &str {
        decode_fixed_str(&self.active_profile_name)
    }

    #[inline]
    pub fn set_active_profile_name(&mut self, s: &str) {
        encode_fixed_str(&mut self.active_profile_name, s);
    }

    #[inline]
    pub fn last_event_desc_str(&self) -> &str {
        decode_fixed_str(&self.last_event_desc)
    }

    #[inline]
    pub fn set_last_event_desc(&mut self, s: &str) {
        encode_fixed_str(&mut self.last_event_desc, s);
    }
}

#[inline]
fn encode_fixed_str(dst: &mut [u8], src: &str) {
    dst.fill(0);
    let bytes = src.as_bytes();
    let copy_len = bytes.len().min(dst.len());
    dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
}

#[inline]
fn decode_fixed_str(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..len]).unwrap_or("")
}

/// Component manifest exchanged during bootstrap.
#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ComponentManifest {
    pub name: [u8; 32],
    pub version: u32,
    pub tier: u8,
    pub capabilities: u64,
    pub methodology: u8,
    pub supported_intents: u16,
    pub priority_weight: u8,
    pub address: u32,
}

/// Query descriptor used for capability routing.
#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct QueryDescriptor {
    pub required_capabilities: u64,
    pub methodology: u8,
    pub intent: u16,
    pub priority: u8,
}

/// Helper to pack version numbers into a single u32 (major.minor.patch).
pub const fn pack_version(major: u8, minor: u8, patch: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (patch as u32)
}

/// Event category / opcode discriminator for flight recorder events
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlightEventKind {
    Unknown = 0,
    CommandInput = 1,
    StateTransition = 2,
    TelemetryTick = 3,
    AnomalyTrigger = 4,
    IntentDispatched = 5,
    MutationCommitted = 6,
    P2POffload = 7,
    PanicInterception = 8,
    Checkpoint = 9,
}

/// Zero-copy 128-byte flight event record for black-box crash recording and deterministic replay.
/// Derives `bytemuck::Pod` and `bytemuck::Zeroable` for zero-allocation circular disk/memory logging.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct FlightEventPod {
    /// Hardware CPU timestamp counter (`_rdtsc`) at moment of capture
    pub timestamp_rdtsc: u64,
    /// Standard wall-clock timestamp in nanoseconds since UNIX epoch
    pub wall_clock_ns: u64,
    /// Strictly monotonic causal sequence number (1, 2, 3...)
    pub sequence: u64,
    /// Input payload / command digest hash
    pub input_hash: u64,
    /// Hash of engine state immediately before transition
    pub pre_state_hash: u64,
    /// Hash of engine state immediately after transition
    pub post_state_hash: u64,
    /// Event category / opcode (`FlightEventKind`)
    pub event_kind: u16,
    /// Source subsystem identifier (0x01: Hypervisor, 0x02: PlatformBridge, etc.)
    pub source_id: u16,
    /// Effective byte length of `data_payload` (<= 64)
    pub payload_len: u32,
    /// Operational status and context flags
    pub flags: u32,
    /// Checksum (additive sum over scalar fields) for corruption detection
    pub checksum: u32,
    /// Fixed-size inline data payload (opcodes, arguments, metrics)
    pub data_payload: [u8; 64],
}

impl Default for FlightEventPod {
    fn default() -> Self {
        Self {
            timestamp_rdtsc: 0,
            wall_clock_ns: 0,
            sequence: 0,
            input_hash: 0,
            pre_state_hash: 0,
            post_state_hash: 0,
            event_kind: FlightEventKind::Unknown as u16,
            source_id: 0,
            payload_len: 0,
            flags: 0,
            checksum: 0,
            data_payload: [0u8; 64],
        }
    }
}

impl FlightEventPod {
    pub fn calculate_checksum(&self) -> u32 {
        let mut acc = 0u32;
        acc = acc.wrapping_add(self.timestamp_rdtsc as u32);
        acc = acc.wrapping_add((self.timestamp_rdtsc >> 32) as u32);
        acc = acc.wrapping_add(self.wall_clock_ns as u32);
        acc = acc.wrapping_add((self.wall_clock_ns >> 32) as u32);
        acc = acc.wrapping_add(self.sequence as u32);
        acc = acc.wrapping_add((self.sequence >> 32) as u32);
        acc = acc.wrapping_add(self.input_hash as u32);
        acc = acc.wrapping_add((self.input_hash >> 32) as u32);
        acc = acc.wrapping_add(self.pre_state_hash as u32);
        acc = acc.wrapping_add((self.pre_state_hash >> 32) as u32);
        acc = acc.wrapping_add(self.post_state_hash as u32);
        acc = acc.wrapping_add((self.post_state_hash >> 32) as u32);
        acc = acc.wrapping_add(self.event_kind as u32);
        acc = acc.wrapping_add((self.source_id as u32) << 16);
        acc = acc.wrapping_add(self.payload_len);
        acc = acc.wrapping_add(self.flags);
        for chunk in self.data_payload.chunks_exact(4) {
            let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            acc = acc.wrapping_add(val);
        }
        acc
    }

    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }

    pub fn set_payload(&mut self, payload: &[u8]) {
        self.data_payload.fill(0);
        let len = payload.len().min(self.data_payload.len());
        self.data_payload[..len].copy_from_slice(&payload[..len]);
        self.payload_len = len as u32;
    }

    pub fn payload(&self) -> &[u8] {
        let len = (self.payload_len as usize).min(self.data_payload.len());
        &self.data_payload[..len]
    }
}

/// Zero-copy 64-byte file header for `.flight` binary flight recorder logs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct FlightFileHeaderPod {
    /// Magic signature: b"AAROFLGT" (0x54474C464F524141)
    pub magic: [u8; 8],
    /// Binary format version (currently 1)
    pub version: u32,
    /// Maximum capacity in event slots
    pub max_events: u32,
    /// Byte size of each slot (128)
    pub slot_size: u32,
    /// Header size in bytes (4096)
    pub header_size: u32,
    /// Strictly monotonic write sequence of the latest recorded event
    pub write_sequence: u64,
    /// Total number of times the circular ring buffer has wrapped
    pub wrap_count: u64,
    /// Reserved/padding fields for 64-byte alignment
    pub _reserved: [u8; 24],
}

impl Default for FlightFileHeaderPod {
    fn default() -> Self {
        Self {
            magic: *b"AAROFLGT",
            version: 1,
            max_events: 131_040,
            slot_size: 128,
            header_size: 4096,
            write_sequence: 0,
            wrap_count: 0,
            _reserved: [0u8; 24],
        }
    }
}

/// Zero-copy 1088-byte machine observation frame for passive shadow ingestion,
/// dual-rail model verification, and continuous reinforcement learning.
/// 64-byte aligned, complies with `bytemuck::Pod` and zero-allocation hot-path rules.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ObservationFramePod {
    /// Monotonic frame sequence counter (1, 2, 3...)
    pub sequence: u64,
    /// Wall-clock or monotonic timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Hypervisor tick execution duration in microseconds
    pub tick_duration_us: u32,
    /// Dispatched action/decision opcode (ground truth from supervisor/rules)
    pub actual_opcode: u16,
    /// Predicted action opcode from shadow .si inference
    pub predicted_opcode: u16,
    /// Actor tier: 0 = Supervisor/Rule, 1 = Cortex, 2 = Router, 3 = Reflex
    pub actor_tier: u8,
    /// Status flags: bit 0: shadow mode, bit 1: concurrence, bit 2: anomaly
    pub flags: u8,
    /// Concurrence indicator: 1 = match (actual == predicted), 0 = divergence
    pub concurrence: u8,
    /// Explicit padding byte for natural 4-byte scalar alignment
    pub _pad0: u8,
    /// Bus state integrity [0.0..100.0]
    pub bus_integrity: f32,
    /// Bus understanding score [0.0..100.0]
    pub bus_understanding: f32,
    /// Continuous flow score [0.0..1.0]
    pub flow_score: f32,
    /// Autonomic thermal pacing factor [0.0..1.0]
    pub thermal_factor: f32,
    /// Immediate reward / reinforcement signal for TD(λ) credit assignment
    pub reward: f32,
    /// Model prediction confidence [0.0..1.0]
    pub confidence: f32,
    /// Thermodynamic free energy / entropy delta
    pub free_energy: f32,
    /// Reserved for future telemetry expansion; aligns header to 64 bytes exact
    pub _reserved: [u32; 2],
    /// Continuous machine state feature vector (256 dimensions = 1024 bytes)
    pub state_features: [f32; 256],
}

impl Default for ObservationFramePod {
    fn default() -> Self {
        Self {
            sequence: 0,
            timestamp_ns: 0,
            tick_duration_us: 0,
            actual_opcode: 0,
            predicted_opcode: 0,
            actor_tier: 0,
            flags: 0,
            concurrence: 0,
            _pad0: 0,
            bus_integrity: 100.0,
            bus_understanding: 100.0,
            flow_score: 1.0,
            thermal_factor: 1.0,
            reward: 0.0,
            confidence: 0.0,
            free_energy: 0.0,
            _reserved: [0u32; 2],
            state_features: [0.0f32; 256],
        }
    }
}

// Unit tests verify struct sizes are pod‑compatible.
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn manifest_size() {
        assert_eq!(size_of::<ComponentManifest>(), 64);
    }

    #[test]
    fn query_size() {
        assert_eq!(size_of::<QueryDescriptor>(), 16);
    }

    #[test]
    fn engine_snapshot_pod_layout_and_roundtrip() {
        assert_eq!(size_of::<EngineSnapshotPod>(), 248);

        let pod = EngineSnapshotPod::default();
        assert_eq!(pod.active_specialist_str(), "Orchestrator");
        assert_eq!(pod.active_profile_name_str(), "Default Operator");
        assert_eq!(pod.last_event_desc_str(), "Core initialized and nominal");

        // Zero-copy Pod round-trip
        let bytes = bytemuck::bytes_of(&pod);
        assert_eq!(bytes.len(), 248);
        let decoded: &EngineSnapshotPod = bytemuck::from_bytes(bytes);
        assert_eq!(*decoded, pod);
    }

    #[test]
    fn flight_record_geometry_and_roundtrip() {
        assert_eq!(size_of::<FlightEventPod>(), 128);
        assert_eq!(size_of::<FlightFileHeaderPod>(), 64);

        let mut event = FlightEventPod {
            timestamp_rdtsc: 123456789,
            wall_clock_ns: 987654321,
            sequence: 42,
            input_hash: 0xDEADBEEFCAFEBABE,
            pre_state_hash: 0x1111222233334444,
            post_state_hash: 0x5555666677778888,
            event_kind: FlightEventKind::StateTransition as u16,
            source_id: 1,
            flags: 0x01,
            ..Default::default()
        };
        event.set_payload(b"flight_test_payload_123");
        event.checksum = event.calculate_checksum();
        assert!(event.verify_checksum());

        let bytes = bytemuck::bytes_of(&event);
        assert_eq!(bytes.len(), 128);
        let decoded: &FlightEventPod = bytemuck::from_bytes(bytes);
        assert_eq!(*decoded, event);
        assert_eq!(decoded.payload(), b"flight_test_payload_123");
        assert!(decoded.verify_checksum());

        let header = FlightFileHeaderPod::default();
        let h_bytes = bytemuck::bytes_of(&header);
        assert_eq!(h_bytes.len(), 64);
        let decoded_h: &FlightFileHeaderPod = bytemuck::from_bytes(h_bytes);
        assert_eq!(*decoded_h, header);
        assert_eq!(&decoded_h.magic, b"AAROFLGT");
    }

    #[test]
    fn observation_frame_geometry_and_roundtrip() {
        assert_eq!(size_of::<ObservationFramePod>(), 1088);
        assert_eq!(std::mem::align_of::<ObservationFramePod>(), 64);

        let mut frame = ObservationFramePod {
            sequence: 101,
            timestamp_ns: 1234567890,
            tick_duration_us: 250,
            actual_opcode: 0x0100,
            predicted_opcode: 0x0100,
            actor_tier: 3,
            flags: 0x03,
            concurrence: 1,
            _pad0: 0,
            bus_integrity: 99.8,
            bus_understanding: 97.5,
            flow_score: 0.92,
            thermal_factor: 1.0,
            reward: 1.5,
            confidence: 0.95,
            free_energy: 0.04,
            _reserved: [0u32; 2],
            state_features: [0.5f32; 256],
        };
        frame.state_features[0] = 1.0;
        frame.state_features[255] = -1.0;

        let bytes = bytemuck::bytes_of(&frame);
        assert_eq!(bytes.len(), 1088);
        let decoded: &ObservationFramePod = bytemuck::from_bytes(bytes);
        assert_eq!(*decoded, frame);
        assert_eq!(decoded.state_features[0], 1.0);
        assert_eq!(decoded.state_features[255], -1.0);
    }
}
