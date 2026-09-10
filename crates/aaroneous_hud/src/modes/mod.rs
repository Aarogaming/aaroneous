// core/hypervisor/src/hud/modes/mod.rs
//! HUD window modes (FullStudio, CompactRecorder, TransparentHud).

pub mod compact_recorder;
pub mod console_os;
pub mod full_studio;
pub mod transparent_hud;
pub mod utility_dashboard;

pub use compact_recorder::render_compact_recorder_overlay;
pub use console_os::ConsoleOsLauncher;
pub use full_studio::render_full_studio;
pub use transparent_hud::render_transparent_hud;
pub use utility_dashboard::render_utility_dashboard;
