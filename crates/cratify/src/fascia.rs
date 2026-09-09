// src/fascia.rs
//! Fascia continuity module – optional UI routing via a broker proxy.
//! When `retain_ui` is enabled, this stub would generate a `BrokerProxy`
//! that forwards UI‑related messages onto the `@hypervisor` bus.
//! For now it simply returns Ok(()) and can be expanded later.

use anyhow::Result;

/// Enable or disable UI routing through the fascia broker.
pub fn retain_ui(enable: bool) -> Result<()> {
    if enable {
        println!("[fascia] UI routing retained via broker proxy.");
    } else {
        println!("[fascia] UI routing disabled.");
    }
    Ok(())
}
