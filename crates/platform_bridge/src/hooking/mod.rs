// crates/platform_bridge/src/hooking/mod.rs
//! In-Process Graphics Hooking & Sub-Frame Action Overlays.

#[cfg(feature = "hooking-injector")]
pub mod injector;
pub mod overlay_primitives;
pub mod swapchain_present;

#[cfg(feature = "hooking-injector")]
pub use injector::HudhookInjector;
pub use overlay_primitives::{OverlayPrimitive, Rgba8, SubFrameOverlayBatch};
pub use swapchain_present::{
    OverlaySubmitter, PresentHookConfig, PresentHookHandle, SwapChainHookManager,
};
