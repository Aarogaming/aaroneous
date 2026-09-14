// Win32 Intercept Perimeter - Rust Implementation
// Captures workspace screen as 128x128 float grid, converts motor intents to HID events.

#[cfg(windows)]
pub mod capture;
pub mod hid_bridge;
pub mod shm_io;
pub use shm_io as synapse_io;

#[cfg(windows)]
pub use capture::Win32ScreenCapture;
#[cfg(windows)]
pub use hid_bridge::HIDOutputBridge;
pub use shm_io::SynapseChannel;
