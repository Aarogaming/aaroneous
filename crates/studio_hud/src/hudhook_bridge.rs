// core/hypervisor/src/hud/hudhook_bridge.rs
use std::sync::atomic::{AtomicBool, Ordering};

pub static IS_OVERLAY_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Initializes the hudhook rendering pipeline for in-game injection
pub fn initialize_hudhook() {
    // In a real implementation, this would build the `hudhook` RenderLoop 
    // and inject into the target process.
    IS_OVERLAY_ACTIVE.store(true, Ordering::SeqCst);
}
