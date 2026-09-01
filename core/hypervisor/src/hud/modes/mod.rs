// core/hypervisor/src/hud/modes/mod.rs
//! HUD window modes (FullStudio, CompactRecorder, TransparentHud).

pub mod compact_recorder;
pub mod full_studio;
pub mod transparent_hud;

pub use compact_recorder::render_compact_recorder_overlay;
pub use full_studio::render_full_studio;
pub use transparent_hud::render_transparent_hud;
