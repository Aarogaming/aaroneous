// crates/platform_bridge/src/hooking/mod.rs
//! In-Process Graphics Hooking & Sub-Frame Action Overlays.

pub mod overlay_primitives;
pub mod swapchain_present;

pub use overlay_primitives::{OverlayPrimitive, Rgba8, SubFrameOverlayBatch};
pub use swapchain_present::{
    OverlaySubmitter, PresentHookConfig, PresentHookHandle, SwapChainHookManager,
};
