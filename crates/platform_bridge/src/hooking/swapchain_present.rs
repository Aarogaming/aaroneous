// crates/platform_bridge/src/hooking/swapchain_present.rs
//! DirectX 11 / 12 SwapChain Present Hook & Zero-Latency Overlay Submission Layer.
//!
//! Provides in-process frame interception for DirectX swapchains, allowing
//! sub-frame action feedback overlays to be composited directly into the rendering pipeline.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{bail, Result};
use parking_lot::RwLock;

/// Configuration for the DXGI Present hooking layer.
#[derive(Debug, Clone)]
pub struct PresentHookConfig {
    pub enable_dx11: bool,
    pub enable_dx12: bool,
    pub enable_vulkan: bool,
}

impl Default for PresentHookConfig {
    fn default() -> Self {
        Self {
            enable_dx11: true,
            enable_dx12: true,
            enable_vulkan: false,
        }
    }
}

/// Trait for submitting zero-latency overlay frames during present.
pub trait OverlaySubmitter: Send + Sync {
    /// Submits overlay graphics primitives for the current frame.
    fn submit_overlay(&self, frame_id: u64, backbuffer_handle: usize) -> Result<()>;
}

/// Handle to active Present hooks with telemetry counters.
#[derive(Debug, Clone)]
pub struct PresentHookHandle {
    frame_counter: Arc<AtomicU64>,
    is_active: Arc<AtomicBool>,
}

impl PresentHookHandle {
    pub fn new() -> Self {
        Self {
            frame_counter: Arc::new(AtomicU64::new(0)),
            is_active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Total intercepted frames submitted through the swapchain.
    pub fn frame_count(&self) -> u64 {
        self.frame_counter.load(Ordering::Relaxed)
    }

    /// Whether the hook is actively intercepting Present calls.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Acquire)
    }
}

impl Default for PresentHookHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe manager for SwapChain Present interception.
pub struct SwapChainHookManager {
    handle: PresentHookHandle,
    submitter: Arc<RwLock<Option<Box<dyn OverlaySubmitter>>>>,
}

impl SwapChainHookManager {
    pub fn new() -> Self {
        Self {
            handle: PresentHookHandle::new(),
            submitter: Arc::new(RwLock::new(None)),
        }
    }

    /// Registers an overlay submitter.
    pub fn register_submitter(&self, submitter: Box<dyn OverlaySubmitter>) {
        let mut lock = self.submitter.write();
        *lock = Some(submitter);
    }

    /// Intercepts a Present call, dispatches the overlay, and increments the frame counter.
    pub fn on_present(&self, backbuffer_handle: usize) -> Result<u64> {
        if !self.handle.is_active() {
            bail!("Present hook is currently disengaged");
        }

        let frame_id = self.handle.frame_counter.fetch_add(1, Ordering::Relaxed);
        let submitter_lock = self.submitter.read();
        if let Some(ref sub) = *submitter_lock {
            sub.submit_overlay(frame_id, backbuffer_handle)?;
        }

        Ok(frame_id)
    }

    /// Returns a lightweight handle for telemetry and lifecycle monitoring.
    pub fn handle(&self) -> PresentHookHandle {
        self.handle.clone()
    }

    /// Disengages the hook.
    pub fn disengage(&self) {
        self.handle.is_active.store(false, Ordering::Release);
    }

    /// Re-engages the hook.
    pub fn engage(&self) {
        self.handle.is_active.store(true, Ordering::Release);
    }
}

impl Default for SwapChainHookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockOverlaySubmitter {
        submitted_frames: Arc<AtomicU64>,
    }

    impl OverlaySubmitter for MockOverlaySubmitter {
        fn submit_overlay(&self, _frame_id: u64, _backbuffer_handle: usize) -> Result<()> {
            self.submitted_frames.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn test_swapchain_hook_lifecycle_and_dispatch() {
        let manager = SwapChainHookManager::new();
        let submitted = Arc::new(AtomicU64::new(0));

        manager.register_submitter(Box::new(MockOverlaySubmitter {
            submitted_frames: Arc::clone(&submitted),
        }));

        assert!(manager.handle().is_active());
        assert_eq!(manager.handle().frame_count(), 0);

        for _ in 0..10 {
            let res = manager.on_present(0x1234);
            assert!(res.is_ok());
        }

        assert_eq!(manager.handle().frame_count(), 10);
        assert_eq!(submitted.load(Ordering::Relaxed), 10);

        // Disengage
        manager.disengage();
        assert!(!manager.handle().is_active());
        assert!(manager.on_present(0x1234).is_err());

        // Re-engage
        manager.engage();
        assert!(manager.handle().is_active());
        assert!(manager.on_present(0x1234).is_ok());
        assert_eq!(manager.handle().frame_count(), 11);
    }
}
