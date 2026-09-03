// core/hypervisor/src/hud/mod.rs
//! Aaroneous Desktop Studio & Spatial Window Manager HUD.

pub mod app;
pub mod auto_pilot;
pub mod companion_overlay;
pub mod fascia;
pub mod modes;
pub mod navigation;
pub mod state;
pub mod theme;
pub mod views;

pub use app::StudioApp;
pub use auto_pilot::{AutoPilotController, AutoPilotState, AutoPilotTelemetry};
pub use companion_overlay::{CompanionTelemetryOverlay, EquilibriumState};
pub use fascia::ProcessFasciaWatcher;
pub use navigation::{CommandAction, CommandPalette, NavSection, ToastLevel, ToastNotification};
pub use state::{CustomAgent, SharedHudState, UserSettings};
pub use theme::HudTheme;
pub use views::HudView;

use eframe::egui;

/// Launch the Aaroneous Studio HUD native desktop window
pub fn launch() -> Result<(), eframe::Error> {
    let settings = UserSettings::load_from_disk();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Aaroneous")
        .with_inner_size([1240.0, 840.0])
        .with_min_inner_size([340.0, 60.0]);

    if settings.always_on_top {
        viewport = viewport.with_always_on_top();
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Aaroneous",
        options,
        Box::new(|_cc| Ok(Box::new(StudioApp::new()))),
    )
}
