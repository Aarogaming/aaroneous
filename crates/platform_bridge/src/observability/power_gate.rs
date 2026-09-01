//! crates/platform_bridge/src/observability/power_gate.rs
//! Tri-State Sensor Power Gating Engine (Mechanical Sympathy & Zero-Polling Idling).
//! Dynamically gates sensory pipeline power across:
//! - Level 0: Idle Listen (WASAPI low-power audio monitor only, <0.1% CPU; DXGI & UIA threads parked)
//! - Level 1: Ambient Awareness (Triggered by sound/focus; 16x16 downscaled delta gate active)
//! - Level 2: Active Foveation (Triggered by motion/task; full 60fps DXGI, UIA hierarchy, HID dispatch)

use serde::{Deserialize, Serialize};

/// Tri-State Sensor Power Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorPowerMode {
    /// Level 0: WASAPI low-power background acoustic monitor only (<0.1% CPU)
    Level0IdleListen,
    /// Level 1: Screen spatial delta gate active at 16x16 resolution
    Level1AmbientAwareness,
    /// Level 2: High-resolution DXGI capture, UIA tree walk, and HID reflex dispatch
    Level2ActiveFoveation,
}

impl Default for SensorPowerMode {
    fn default() -> Self {
        Self::Level0IdleListen
    }
}

/// Adaptive Sensor Power Gate Manager
#[derive(Debug, Clone)]
pub struct SensorPowerGate {
    current_mode: SensorPowerMode,
    consecutive_static_ticks: u32,
    ambient_timeout_ticks: u32,
    foveation_timeout_ticks: u32,
}

impl Default for SensorPowerGate {
    fn default() -> Self {
        Self::new(60, 300) // 1s at 60Hz to drop from L2->L1, 5s to drop from L1->L0
    }
}

impl SensorPowerGate {
    /// Instantiates a new SensorPowerGate with specified decay tick thresholds.
    pub fn new(ambient_timeout_ticks: u32, foveation_timeout_ticks: u32) -> Self {
        Self {
            current_mode: SensorPowerMode::Level0IdleListen,
            consecutive_static_ticks: 0,
            ambient_timeout_ticks,
            foveation_timeout_ticks,
        }
    }

    /// Evaluates sensory triggers and updates the active power mode.
    pub fn evaluate_sensory_pulse(
        &mut self,
        has_acoustic_spike: bool,
        has_window_focus_change: bool,
        has_visual_motion: bool,
    ) -> SensorPowerMode {
        if has_visual_motion {
            // Motion directly escalates to Active Foveation
            self.current_mode = SensorPowerMode::Level2ActiveFoveation;
            self.consecutive_static_ticks = 0;
        } else if has_acoustic_spike || has_window_focus_change {
            // Sound or Focus change escalates at least to Ambient Awareness
            if self.current_mode == SensorPowerMode::Level0IdleListen {
                self.current_mode = SensorPowerMode::Level1AmbientAwareness;
            }
            self.consecutive_static_ticks = 0;
        } else {
            // No stimulus: increment dormancy counter and decay power level
            self.consecutive_static_ticks += 1;
            match self.current_mode {
                SensorPowerMode::Level2ActiveFoveation => {
                    if self.consecutive_static_ticks >= self.ambient_timeout_ticks {
                        self.current_mode = SensorPowerMode::Level1AmbientAwareness;
                    }
                }
                SensorPowerMode::Level1AmbientAwareness => {
                    if self.consecutive_static_ticks >= self.foveation_timeout_ticks {
                        self.current_mode = SensorPowerMode::Level0IdleListen;
                    }
                }
                SensorPowerMode::Level0IdleListen => {}
            }
        }

        self.current_mode
    }

    /// Forcefully sets the power mode (e.g. on manual user override).
    pub fn force_mode(&mut self, mode: SensorPowerMode) {
        self.current_mode = mode;
        self.consecutive_static_ticks = 0;
    }

    /// Returns the current active sensor power mode.
    pub fn current_mode(&self) -> SensorPowerMode {
        self.current_mode
    }

    /// Whether full DXGI and UIA tree walkers should be actively running.
    pub fn is_foveation_active(&self) -> bool {
        self.current_mode == SensorPowerMode::Level2ActiveFoveation
    }

    /// Whether the system is in deep low-power acoustic idle listening.
    pub fn is_idle_listen(&self) -> bool {
        self.current_mode == SensorPowerMode::Level0IdleListen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_power_gate_transitions_and_decay() {
        let mut gate = SensorPowerGate::new(5, 10);
        assert_eq!(gate.current_mode(), SensorPowerMode::Level0IdleListen);

        // Acoustic spike promotes to Level 1
        let mode = gate.evaluate_sensory_pulse(true, false, false);
        assert_eq!(mode, SensorPowerMode::Level1AmbientAwareness);

        // Motion promotes to Level 2
        let mode2 = gate.evaluate_sensory_pulse(false, false, true);
        assert_eq!(mode2, SensorPowerMode::Level2ActiveFoveation);
        assert!(gate.is_foveation_active());

        // 5 static ticks decay L2 -> L1
        for _ in 0..5 {
            gate.evaluate_sensory_pulse(false, false, false);
        }
        assert_eq!(gate.current_mode(), SensorPowerMode::Level1AmbientAwareness);

        // 10 static ticks decay L1 -> L0
        for _ in 0..5 {
            gate.evaluate_sensory_pulse(false, false, false);
        }
        assert_eq!(gate.current_mode(), SensorPowerMode::Level0IdleListen);
        assert!(gate.is_idle_listen());
    }
}
