//! core/hypervisor/bin/aaroneous.rs
//! Aaroneous Desktop Studio & Spatial Window Manager Executable Launcher.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), eframe::Error> {
    a_run::hud::launch()
}
