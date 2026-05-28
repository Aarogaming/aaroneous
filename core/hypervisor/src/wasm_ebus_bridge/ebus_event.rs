/// Hypervisor event types and structures
/// 
/// These events flow from the Hypervisor → WASM ringbuffer → WASM agent perception

use serde::{Deserialize, Serialize};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Event types that the Hypervisor can emit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EbusEventType {
    /// Input event (keyboard, mouse)
    InputEvent = 0x01,
    
    /// Visual state changed (UI element appeared/disappeared)
    VisualStateChange = 0x02,
    
    /// Entity movement/spawn/despawn
    EntityUpdate = 0x03,
    
    /// Game state change (pause, loading, etc.)
    GameStateChange = 0x04,
    
    /// Combat event (hit, miss, damage)
    CombatEvent = 0x05,
    
    /// Audio event (sound played)
    AudioEvent = 0x06,
    
    /// System event (performance warning, etc.)
    SystemEvent = 0x07,
    
    /// AAS Shard Signal (Communication from Python Shards)
    ShardSignal = 0x08,
}

/// Serialized EBus event (fits in fixed 256-byte payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbusEvent {
    /// Event type discriminator
    pub event_type: EbusEventType,
    
    /// Timestamp (nanoseconds since agent birth)
    pub timestamp_ns: u64,
    
    /// Serialized event payload (binary)
    pub payload: Vec<u8>,
}

impl EbusEvent {
    /// Create new EBus event
    pub fn new(event_type: EbusEventType, payload: Vec<u8>) -> Self {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        
        Self {
            event_type,
            timestamp_ns,
            payload,
        }
    }
    
    /// Parse payload as input event
    pub fn as_input_event(&self) -> Option<InputEvent> {
        if self.event_type != EbusEventType::InputEvent {
            return None;
        }
        
        serde_json::from_slice::<InputEvent>(&self.payload).ok()
    }
    
    /// Parse payload as visual state change
    pub fn as_visual_change(&self) -> Option<VisualStateChange> {
        if self.event_type != EbusEventType::VisualStateChange {
            return None;
        }
        
        serde_json::from_slice::<VisualStateChange>(&self.payload).ok()
    }
    
    /// Serialize to fixed-size buffer (max 256 bytes)
    pub fn to_fixed_buffer(&self) -> [u8; 256] {
        let mut buf = [0u8; 256];
        
        // Header (8 bytes)
        buf[0] = self.event_type as u8;
        let ts_bytes = self.timestamp_ns.to_le_bytes();
        buf[1..9].copy_from_slice(&ts_bytes);
        
        // Payload (up to 247 bytes)
        let payload_len = std::cmp::min(self.payload.len(), 247);
        buf[9..9 + payload_len].copy_from_slice(&self.payload[..payload_len]);
        
        buf
    }
    
    /// Deserialize from fixed-size buffer
    pub fn from_fixed_buffer(buf: &[u8; 256]) -> Option<Self> {
        if buf[0] == 0 {
            return None; // Empty slot
        }
        
        let event_type = match buf[0] {
            0x01 => EbusEventType::InputEvent,
            0x02 => EbusEventType::VisualStateChange,
            0x03 => EbusEventType::EntityUpdate,
            0x04 => EbusEventType::GameStateChange,
            0x05 => EbusEventType::CombatEvent,
            0x06 => EbusEventType::AudioEvent,
            0x07 => EbusEventType::SystemEvent,
            0x08 => EbusEventType::ShardSignal,
            _ => return None,
        };
        
        let timestamp_ns = u64::from_le_bytes([
            buf[1], buf[2], buf[3], buf[4],
            buf[5], buf[6], buf[7], buf[8],
        ]);
        
        // Find actual payload length
        let payload_len = buf[9..].iter().position(|&b| b == 0).unwrap_or(247);
        let payload = buf[9..9 + payload_len].to_vec();
        
        Some(Self {
            event_type,
            timestamp_ns,
            payload,
        })
    }
}

/// Input event (keyboard, mouse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseClick {
        button: MouseButton,
        x: i32,
        y: i32,
    },
    MouseRelease {
        button: MouseButton,
    },
    KeyPress {
        key: u32,  // Virtual key code
    },
    KeyRelease {
        key: u32,
    },
    Scroll {
        delta: i32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

/// Visual state change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualStateChange {
    /// UI element ID
    pub element_id: String,
    
    /// Change type
    pub change_type: VisualChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisualChangeType {
    Appeared,
    Disappeared,
    PropertyChanged { property: String, value: String },
}

/// Entity update event (position, state, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityUpdate {
    pub entity_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub state_change: Option<String>,
}

/// Engine state change (pause, loading, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameStateChange {
    Paused,
    Resumed,
    Loading,
    LoadingComplete,
    SceneChanged { new_scene: String },
}

/// Combat event (hit, miss, damage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEvent {
    pub attacker_id: u32,
    pub target_id: u32,
    pub damage: f32,
    pub hit: bool,
}

/// Signal from an AAS Shard (Python) to the WASM Runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardSignal {
    pub shard_name: String,
    pub signal_type: String,
    pub payload_json: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ebus_event_creation() {
        let event = EbusEvent::new(EbusEventType::InputEvent, vec![1, 2, 3]);
        
        assert_eq!(event.event_type, EbusEventType::InputEvent);
        assert_eq!(event.payload, vec![1, 2, 3]);
        assert!(event.timestamp_ns > 0);
    }
    
    #[test]
    fn test_ebus_fixed_buffer_roundtrip() {
        let original = EbusEvent::new(EbusEventType::InputEvent, vec![42, 43, 44]);
        let buf = original.to_fixed_buffer();
        let restored = EbusEvent::from_fixed_buffer(&buf);
        
        assert!(restored.is_some());
        let restored = restored.unwrap();
        assert_eq!(restored.event_type, original.event_type);
        assert_eq!(restored.payload, original.payload);
    }
}
