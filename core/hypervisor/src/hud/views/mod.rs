// core/hypervisor/src/hud/views/mod.rs
//! Viewports and `HudView` trait implementation.

pub mod agents_hub;
pub mod galaxy_map_3d;
pub mod screen_automation;
pub mod settings;
pub mod si_forge;
pub mod signal_analyzer;
pub mod spatial_sensory;
pub mod system_thermo;
pub mod workbench;

pub use agents_hub::AgentsHubView;
pub use galaxy_map_3d::Galaxy3DView;
pub use screen_automation::ScreenAutomationView;
pub use settings::SettingsView;
pub use si_forge::SiForgeView;
pub use signal_analyzer::SignalAnalyzerView;
pub use spatial_sensory::SpatialSensoryView;
pub use system_thermo::SystemThermoView;
pub use workbench::WorkbenchView;

use crate::hud::state::SharedHudState;
use eframe::egui;

/// Trait implemented by all isolated viewports in the HUD
pub trait HudView: Send + Sync {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState);
}
