// Win32 Intercept Perimeter - Rust Implementation
// Captures workspace screen as 128x128 float grid, converts motor intents to HID events.

pub mod capture;
pub mod hid_bridge;
pub mod shm_io;
pub use shm_io as synapse_io;

pub use capture::Win32ScreenCapture;
pub use hid_bridge::HIDOutputBridge;
pub use shm_io::SynapseChannel;
