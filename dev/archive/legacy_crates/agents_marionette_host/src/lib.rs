pub mod marionette_host;
pub use marionette_host::{MarionetteHost, PermissionLevel};
use anyhow::Result;

/// Universal host() function
pub fn host() -> MarionetteHost {
    MarionetteHost::new(PermissionLevel::Untrusted)
}

/// Universal host_with_gate() function
pub fn host_with_gate(gate: bool) -> MarionetteHost {
    let level = if gate { PermissionLevel::Trusted } else { PermissionLevel::Untrusted };
    MarionetteHost::new(level)
}
