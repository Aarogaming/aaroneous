// crates/platform_bridge/src/adapters/mod.rs
//! Universal Hardware & Sensory-Actuator Adapter Ecosystem.
//!
//! Decouples core intelligence from physical operating systems and hardware:
//! 1. `SensoryFeedAdapter`: Generic multi-modal perception feeds (DXGI, USB Cam, CAN, Audio).
//! 2. `PhysicalActuatorAdapter`: Generic action dispatchers (Win32 Mouse, BOE-Bot Serial, CANbus, Virtual Sim).
//! 3. `AdapterRegistry`: Dynamic runtime registry managing active plug-and-play adapters.

pub mod synthesizer;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use synthesizer::{AdapterSynthesizer, DeviceHardwareSpec, SynthesizedActuatorAdapter};

/// Generic normalized observation emitted by any sensory adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedObservation {
    pub source_id: String,
    pub timestamp_us: u64,
    pub latent_feature_vector: Vec<f32>,
    pub metadata_tag: String,
}

/// Generic action command dispatched to any physical or virtual actuator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalActuatorCommand {
    MoveCursorRelative { dx: i32, dy: i32 },
    MoveCursorAbsolute { x: i32, y: i32 },
    MouseButton { button_code: u8, is_down: bool },
    KeyPress { key_code: u16, is_down: bool },
    AnalogChannel { channel_id: u16, normalized_value: f32 },
    DigitalState { pin_or_index: u16, is_high: bool },
    RawBusFrame { bus_address: u32, payload: Vec<u8> },
    EmergencyStop,
}

/// The Universal Sensory Feed Trait (Perception Inputs)
pub trait SensoryFeedAdapter: Send + Sync {
    /// Identifier for the feed (e.g. "DXGI-Screen-4K", "Optical-Sensor-Array", "Raw-CAN-Bus")
    fn feed_name(&self) -> &str;

    /// Samples a fresh observation, normalized into latent vector representation
    fn sample_observation(&mut self) -> Result<NormalizedObservation>;

    /// Health check to confirm hardware sensor is alive
    fn is_healthy(&self) -> bool;
}

/// The Universal Physical Actuator Trait (Motor & Peripheral Outputs)
pub trait PhysicalActuatorAdapter: Send + Sync {
    /// Identifier for the actuator (e.g. "Win32-SendInput", "Serial-UART-Channel", "CAN-FD-Bus")
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

/// Adapter bridging any `MarionetteHost` visual perception into a `SensoryFeedAdapter`
pub struct MarionetteSensoryAdapter {
    name: String,
    host: std::sync::Arc<tokio::sync::Mutex<dyn crate::traits::MarionetteHost>>,
}

impl MarionetteSensoryAdapter {
    pub fn new(name: impl Into<String>, host: std::sync::Arc<tokio::sync::Mutex<dyn crate::traits::MarionetteHost>>) -> Self {
        Self {
            name: name.into(),
            host,
        }
    }
}

impl SensoryFeedAdapter for MarionetteSensoryAdapter {
    fn feed_name(&self) -> &str {
        &self.name
    }

    fn sample_observation(&mut self) -> Result<NormalizedObservation> {
        let host = self.host.clone();
        let obs = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut h = host.lock().await;
                h.pull_visual_perception().await
            })
        })?;

        Ok(NormalizedObservation {
            source_id: self.name.clone(),
            timestamp_us: obs.timestamp_us,
            latent_feature_vector: obs.grid,
            metadata_tag: format!("width={},height={}", obs.width, obs.height),
        })
    }

    fn is_healthy(&self) -> bool {
        true
    }
}

/// Adapter bridging `UniversalActuatorCommand` to a `MarionetteHost` peripheral input
pub struct MarionetteActuatorAdapter {
    name: String,
    host: std::sync::Arc<tokio::sync::Mutex<dyn crate::traits::MarionetteHost>>,
    sequence_id: u64,
}

impl MarionetteActuatorAdapter {
    pub fn new(name: impl Into<String>, host: std::sync::Arc<tokio::sync::Mutex<dyn crate::traits::MarionetteHost>>) -> Self {
        Self {
            name: name.into(),
            host,
            sequence_id: 0,
        }
    }
}

impl PhysicalActuatorAdapter for MarionetteActuatorAdapter {
    fn actuator_name(&self) -> &str {
        &self.name
    }

    fn verify_safety_bounds(&self, cmd: &UniversalActuatorCommand) -> bool {
        match cmd {
            UniversalActuatorCommand::MoveCursorRelative { dx, dy } => dx.abs() <= 10000 && dy.abs() <= 10000,
            UniversalActuatorCommand::MoveCursorAbsolute { x, y } => *x >= 0 && *y >= 0,
            _ => true,
        }
    }

    fn dispatch(&mut self, cmd: UniversalActuatorCommand) -> Result<()> {
        let action = match cmd {
            UniversalActuatorCommand::MoveCursorRelative { dx, dy } => {
                crate::traits::HidAction::MouseMove { delta_x: dx, delta_y: dy }
            }
            UniversalActuatorCommand::MouseButton { button_code, is_down: _ } => {
                if button_code == 1 {
                    crate::traits::HidAction::LeftClick
                } else {
                    crate::traits::HidAction::RightClick
                }
            }
            UniversalActuatorCommand::KeyPress { key_code, is_down } => {
                if is_down {
                    crate::traits::HidAction::KeyPress { key_code }
                } else {
                    crate::traits::HidAction::KeyRelease { key_code }
                }
            }
            _ => return Ok(()),
        };

        self.sequence_id += 1;
        let hid_cmd = crate::traits::HidCommand {
            actions: vec![action],
            sequence_id: self.sequence_id,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(0),
        };

        let host = self.host.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut h = host.lock().await;
                h.inject_hid_event(hid_cmd).await
            })
        })?;

        Ok(())
    }
}

/// Sensory feed adapter bridging an `OtEdgeGateway` into the universal adapter ecosystem
pub struct OtSensoryAdapter {
    name: String,
    gateway: std::sync::Arc<crate::ot_bridge::OtEdgeGateway>,
}

impl OtSensoryAdapter {
    pub fn new(name: impl Into<String>, gateway: std::sync::Arc<crate::ot_bridge::OtEdgeGateway>) -> Self {
        Self {
            name: name.into(),
            gateway,
        }
    }
}

impl SensoryFeedAdapter for OtSensoryAdapter {
    fn feed_name(&self) -> &str {
        &self.name
    }

    fn sample_observation(&mut self) -> Result<NormalizedObservation> {
        let state = self.gateway.read_registers();
        let mut latent_vec = vec![0.0f32; 16];
        for (i, &reg) in state.holding_registers.iter().take(16).enumerate() {
            latent_vec[i] = reg as f32;
        }

        let timestamp_us = state.last_telemetry
            .map(|t| t.uptime_ms * 1000)
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_micros() as u64)
                    .unwrap_or(0)
            });

        Ok(NormalizedObservation {
            source_id: self.name.clone(),
            timestamp_us,
            latent_feature_vector: latent_vec,
            metadata_tag: "OT_INDUSTRIAL_REGISTERS".to_string(),
        })
    }

    fn is_healthy(&self) -> bool {
        true
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

    pub fn sensory_feed_names(&self) -> Vec<String> {
        self.sensory_feeds.keys().cloned().collect()
    }

    pub fn actuator_names(&self) -> Vec<String> {
        self.actuators.keys().cloned().collect()
    }

    /// Initializes an environment with auto-detected physical and virtual adapters.
    ///
    /// - On Windows with interactive desktop: mounts live DXGI capture and SendInput actuator.
    /// - On systems with physical or virtual serial/OT ports: mounts live OT Edge Bridge.
    /// - Registers virtual simulation adapters as deterministic fallbacks with clear capability tags.
    pub fn live_environment() -> Self {
        let mut reg = Self::new();

        // 1. Ingest/Sensory Feeds
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            let emulator = std::sync::Arc::new(tokio::sync::Mutex::new(
                crate::native_win32::NativeWin32Marionette::new(false),
            ));
            reg.register_sensory_feed(Box::new(MarionetteSensoryAdapter::new(
                "Windows-Native-Desktop-DXGI",
                emulator,
            )));
        }

        // 2. OT / Hardware Serial Ingest
        if let Ok(ports) = tokio_serial::available_ports() {
            for port in ports {
                let name = format!("OT-Serial-Feed-{}", port.port_name);
                let (gateway, _rx) = crate::ot_bridge::OtEdgeGateway::new(crate::ot_bridge::OtBridgeConfig {
                    port_name: port.port_name.clone(),
                    baud_rate: 115_200,
                    heartbeat_interval_ms: 250,
                });
                reg.register_sensory_feed(Box::new(OtSensoryAdapter {
                    name,
                    gateway: std::sync::Arc::new(gateway),
                }));
            }
        }

        // 3. Actuator Outputs
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            let emulator = std::sync::Arc::new(tokio::sync::Mutex::new(
                crate::native_win32::NativeWin32Marionette::new(false),
            ));
            reg.register_actuator(Box::new(MarionetteActuatorAdapter::new(
                "Windows-SendInput-Actuator",
                emulator,
            )));
        }

        // Always register virtual simulator actuator for sandboxed validation
        reg.register_actuator(Box::new(VirtualSimActuator::new("Virtual-Sandbox-Actuator")));

        reg
    }

    /// Initializes a default desktop environment with native perception and action simulation
    pub fn default_desktop() -> Self {
        Self::live_environment()
    }

    /// Samples observations across all registered sensory feeds
    pub fn sample_all_feeds(&mut self) -> Vec<NormalizedObservation> {
        let mut observations = Vec::new();
        for feed in self.sensory_feeds.values_mut() {
            if feed.is_healthy() {
                if let Ok(obs) = feed.sample_observation() {
                    observations.push(obs);
                }
            }
        }
        observations
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
        let cmd = UniversalActuatorCommand::AnalogChannel {
            channel_id: 1,
            normalized_value: 0.8,
        };

        assert!(registry.dispatch_to_actuator("Test-Sim", cmd).is_ok());

        // Sample sensory feeds
        let observations = registry.sample_all_feeds();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].source_id, "Mock-Ocular-Feed");
        assert_eq!(observations[0].latent_feature_vector.len(), 3);
    }
}
