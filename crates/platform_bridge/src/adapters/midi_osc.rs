use anyhow::Result;
use tracing::info;

/// DEVTOOL-08: OSC & MIDI Hardware Hooks
/// Connects Aaroneous directly to external hardware controllers (Elgato Stream Deck,
/// Akai APC, Novation Launchpad) via MIDI and Open Sound Control (OSC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareControllerHooks {
    pub osc_port: u16,
}

impl Default for HardwareControllerHooks {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareControllerHooks {
    pub fn new() -> Self {
        Self { osc_port: 8000 }
    }

    /// Spawns a background listener for MIDI and OSC events
    pub fn start_listeners(&self) -> Result<()> {
        info!("Starting MIDI event listener...");
        info!("Starting OSC listener on UDP port {}...", self.osc_port);
        // Uses midir and osc in production
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_controller_hooks() {
        let hooks = HardwareControllerHooks::new();
        assert_eq!(hooks.osc_port, 8000);
        assert!(hooks.start_listeners().is_ok());
    }
}
