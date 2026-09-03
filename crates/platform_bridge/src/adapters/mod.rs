// crates/platform_bridge/src/adapters/mod.rs
//! Universal Hardware & Sensory-Actuator Adapter Ecosystem.
//!
//! Decouples core intelligence from physical operating systems and hardware:
//! 1. `SensoryFeedAdapter`: Generic multi-modal perception feeds (DXGI, USB Cam, CAN, Audio).
//! 2. `PhysicalActuatorAdapter`: Generic action dispatchers (Win32 Mouse, BOE-Bot Serial, CANbus, Virtual Sim).
//! 3. `AdapterRegistry`: Dynamic runtime registry managing active plug-and-play adapters.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic normalized observation emitted by any sensory adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedObservation {
    pub source_id: String,
    pub timestamp_us: u64,
    pub latent_feature_vector: Vec<f32>,
    pub metadata_tag: String,
}

/// Generic action command dispatched to any physical actuator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalActuatorCommand {
    MoveCursorRelative { dx: i32, dy: i32 },
    MoveCursorAbsolute { x: i32, y: i32 },
    MouseButton { button_code: u8, is_down: bool },
    KeyPress { key_code: u16, is_down: bool },
    RobotLocomotion { left_speed: f32, right_speed: f32 },
    VehicleActiveAero { wing_angle_deg: f32, brake_duct_percent: f32 },
    RawCanPacket { arbitration_id: u32, payload: Vec<u8> },
    EmergencyStop,
}

/// The Universal Sensory Feed Trait (Perception Inputs)
pub trait SensoryFeedAdapter: Send + Sync {
    /// Identifier for the feed (e.g. "DXGI-Screen-4K", "BoeBot-Ocular-Cam", "OBDII-CAN-Feed")
    fn feed_name(&self) -> &str;

    /// Samples a fresh observation, normalized into latent vector representation
    fn sample_observation(&mut self) -> Result<NormalizedObservation>;

    /// Health check to confirm hardware sensor is alive
    fn is_healthy(&self) -> bool;
}

/// The Universal Physical Actuator Trait (Motor & Peripheral Outputs)
pub trait PhysicalActuatorAdapter: Send + Sync {
    /// Identifier for the actuator (e.g. "Win32-SendInput", "BoeBot-Serial-Servos", "CAN-FD-Actuator")
    fn actuator_name(&self) -> &str;

    /// Validates safety boundaries before physical dispatch
    fn verify_safety_bounds(&self, cmd: &UniversalActuatorCommand) -> bool;

    /// Dispatches physical command to real-world hardware or virtual simulator
    fn dispatch(&mut self, cmd: UniversalActuatorCommand) -> Result<()>;
}

/// Virtual Simulator Actuator Adapter (Safe for Testing / The Crucible)
pub struct VirtualSimActuator {
    name: String,
    pub dispatched_commands: Vec<UniversalActuatorCommand>,
}

impl Default for VirtualSimActuator {
    fn default() -> Self {
        Self::new("Virtual-Sandbox-Actuator")
    }
}

impl VirtualSimActuator {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dispatched_commands: Vec::new(),
        }
    }
}

impl PhysicalActuatorAdapter for VirtualSimActuator {
    fn actuator_name(&self) -> &str {
        &self.name
    }

    fn verify_safety_bounds(&self, _cmd: &UniversalActuatorCommand) -> bool {
        // Virtual sandbox is safe by definition
        true
    }

    fn dispatch(&mut self, cmd: UniversalActuatorCommand) -> Result<()> {
        self.dispatched_commands.push(cmd);
        Ok(())
    }
}

/// Dynamic Runtime Adapter Registry
#[derive(Default)]
pub struct UniversalAdapterRegistry {
    sensory_feeds: HashMap<String, Box<dyn SensoryFeedAdapter>>,
    actuators: HashMap<String, Box<dyn PhysicalActuatorAdapter>>,
}

impl UniversalAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_sensory_feed(&mut self, feed: Box<dyn SensoryFeedAdapter>) {
        self.sensory_feeds.insert(feed.feed_name().to_string(), feed);
    }

    pub fn register_actuator(&mut self, actuator: Box<dyn PhysicalActuatorAdapter>) {
        self.actuators.insert(actuator.actuator_name().to_string(), actuator);
    }

    pub fn sensory_feed_count(&self) -> usize {
        self.sensory_feeds.len()
    }

    pub fn actuator_count(&self) -> usize {
        self.actuators.len()
    }

    /// Dispatches a command to a named actuator, validating safety bounds first
    pub fn dispatch_to_actuator(&mut self, actuator_name: &str, cmd: UniversalActuatorCommand) -> Result<()> {
        if let Some(actuator) = self.actuators.get_mut(actuator_name) {
            if !actuator.verify_safety_bounds(&cmd) {
                bail!("Safety boundary violation: Command rejected by actuator [{}]", actuator_name);
            }
            actuator.dispatch(cmd)
        } else {
            bail!("Actuator [{}] is not registered in the adapter ecosystem", actuator_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSensoryFeed {
        name: String,
        tick: u64,
    }

    impl SensoryFeedAdapter for MockSensoryFeed {
        fn feed_name(&self) -> &str {
            &self.name
        }

        fn sample_observation(&mut self) -> Result<NormalizedObservation> {
            self.tick += 1;
            Ok(NormalizedObservation {
                source_id: self.name.clone(),
                timestamp_us: self.tick * 1000,
                latent_feature_vector: vec![0.1, 0.5, 0.9],
                metadata_tag: "mock_data".to_string(),
            })
        }

        fn is_healthy(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_universal_adapter_registry_lifecycle() {
        let mut registry = UniversalAdapterRegistry::new();

        let feed = MockSensoryFeed {
            name: "Mock-Ocular-Feed".to_string(),
            tick: 0,
        };
        registry.register_sensory_feed(Box::new(feed));
        assert_eq!(registry.sensory_feed_count(), 1);

        let sim_actuator = VirtualSimActuator::new("Test-Sim");
        registry.register_actuator(Box::new(sim_actuator));
        assert_eq!(registry.actuator_count(), 1);

        // Dispatch command to virtual actuator
        let cmd = UniversalActuatorCommand::RobotLocomotion {
            left_speed: 0.8,
            right_speed: 0.8,
        };

        assert!(registry.dispatch_to_actuator("Test-Sim", cmd).is_ok());
    }
}
