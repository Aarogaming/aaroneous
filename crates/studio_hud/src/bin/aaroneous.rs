//! crates/studio_hud/src/bin/aaroneous.rs
//! Aaroneous Desktop Studio & Spatial Window Manager Executable Launcher.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), eframe::Error> {
    studio_hud::launch()
}
