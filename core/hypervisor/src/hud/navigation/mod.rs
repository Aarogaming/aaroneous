// core/hypervisor/src/hud/navigation/mod.rs
//! Navigation rail, command palette, toast alerts, and modal dialogs.

pub mod palette;
pub mod shortcuts;
pub mod toast;

pub use palette::{CommandAction, CommandPalette};
pub use shortcuts::ShortcutsModal;
pub use toast::{ToastLevel, ToastNotification, ToastNotificationManager};

use serde::{Deserialize, Serialize};

/// Navigation Categories in the Left Sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NavSection {
    // Cognitive Hypervisor Main Deck
    #[serde(alias = "Pantheon")]
    Specialists,          // 👥 9 Domain Specialists & Hive Intent
    GalaxyMap3D,          // 🌌 3D Omni Knowledge Galaxy
    LearningAndSelfPlay,  // 🧬 Neurochemistry & Self-Play Learning
    SiForge,              // ⚡ Solid-State SI Model Forge & Compiler
    ScreenAutomation,     // 👁️ Epigenetic Vision & Sandboxed Motor Engine
    SwarmMesh,            // 🌐 FederationBus Multi-Hive P2P Swarm Mesh
    Agents,               // 🤖 Autonomous SI Agents & Workflows
    Settings,             // ⚙️ Preferences, Model Hub & Shaders

    // Developer Mode Views (Unlocked in Settings)
    DevStudio,
    InterconnectMonitor,
    Console,

    // Legacy / Backwards Compatibility Aliases
    Cosmos3D,
    LivingMind,
    GhostStation,
    GameStudio,
    CustomTools,
    ScreenCapture,
    Galaxy3D,
    #[serde(other)]
    Home,
}

impl NavSection {
    pub fn display_label(&self) -> &'static str {
        match self {
            NavSection::Specialists => "👥 Specialists",
            NavSection::GalaxyMap3D | NavSection::Galaxy3D | NavSection::Cosmos3D => "🌌 3D Galaxy",
            NavSection::LearningAndSelfPlay | NavSection::LivingMind => "🧬 Learning & Self-Play",
            NavSection::SiForge => "⚡ SI Forge",
            NavSection::ScreenAutomation | NavSection::ScreenCapture => "👁️ Screen & Motor",
            NavSection::SwarmMesh | NavSection::GhostStation => "🌐 Swarm Mesh",
            NavSection::Agents => "🤖 Agents Hub",
            NavSection::Settings => "⚙️ Settings",
            NavSection::DevStudio | NavSection::GameStudio | NavSection::CustomTools => "🛠️ Dev Studio",
            NavSection::InterconnectMonitor => "⚡ Bus Monitor",
            NavSection::Console => "💬 Chat Console",
            NavSection::Home => "🏠 Home Hub",
        }
    }
}
