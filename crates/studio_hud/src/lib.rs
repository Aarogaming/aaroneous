#![recursion_limit = "512"]

//! Aaroneous Desktop Studio & Spatial Window Manager HUD.

pub use crate as hud;
pub extern crate adaptation_plane as evolution;
pub extern crate ipc_bus as nervous_system;

pub use hive_mind::capability_broker;
pub use hypervisor::util;
pub use omni::{ConstellationNode, NodeType, SpatialCoord, StarNode, StarNodeType};

pub mod achievements;
pub mod app;
pub mod auto_pilot;
pub mod command_registry;
pub mod companion_overlay;
pub mod fascia;
pub mod modes;
pub mod navigation;
pub mod onboarding;
pub mod plugin_api;
pub mod state;
pub mod state_snapshot;
pub mod theme;
pub mod transformer_bridge;
pub mod views;

pub mod bus_visualizer;
pub mod constellation_3d;
pub mod constellation_ui;
pub mod hypervisor_hud;
pub mod skill_constellation;
pub mod studio_ui;

pub use bus_visualizer::BusVisualizer;
pub use constellation_3d::Constellation3D;
pub use constellation_ui::{ConstellationCanvas, NodeMetrics};
pub use hypervisor_hud::{HudTab, HypervisorHudApp};
pub use skill_constellation::{SkillConstellationCanvas, VisualStarNode};
pub use studio_ui::{DistillStatus, DistillStudio};

pub use app::StudioApp;
pub use auto_pilot::{AutoPilotController, AutoPilotState, AutoPilotTelemetry};
pub use command_registry::{CapabilityCommand, CommandRegistry, TypedCommand};
pub use companion_overlay::{CompanionTelemetryOverlay, EquilibriumState};
pub use fascia::ProcessFasciaWatcher;
pub use navigation::{CommandAction, CommandPalette, NavSection, ToastLevel, ToastNotification};
pub use state::{CustomAgent, SharedHudState, UserSettings};
pub use state_snapshot::{
    ConsoleProjection, EngineSnapshot, EngineStatePublisher, HudProjection, StudioProjection,
};
pub use theme::HudTheme;
pub use transformer_bridge::{
    BackendTelemetryFrame, FrontendCommandSignal, FrontendTransformerBridge,
};
pub use views::HudView;

use eframe::egui;

/// Launch the Aaroneous Studio HUD native desktop window
pub fn launch() -> Result<(), eframe::Error> {
    let settings = UserSettings::load_from_disk();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Aaroneous")
        .with_inner_size([1240.0, 840.0])
        .with_min_inner_size([340.0, 60.0])
        .with_transparent(true);

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
