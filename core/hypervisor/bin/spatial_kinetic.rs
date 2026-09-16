#![recursion_limit = "256"]
#![allow(ambient_authority)]

// Spatial-Kinetic Engine Binary
// Standalone executable that runs the universal spatial-kinetic reflex loop.
//
// Usage:
//   spatial_kinetic.exe                          # Run with defaults
//   spatial_kinetic.exe --profile path/to/genome  # Custom profile path
//   spatial_kinetic.exe --fps 60                 # Target 60 FPS
//   spatial_kinetic.exe --no-hid                 # Disable HID output (capture only)

#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
use hypervisor::spatial_kinetic_engine::{SpatialKineticConfig, SpatialKineticEngine};

#[cfg(windows)]
#[tokio::main]
#[allow(clippy::await_holding_lock)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install logging
    let (_init, _guard) = hypervisor::init_logging();

    let args: Vec<String> = std::env::args().collect();

    let mut config = SpatialKineticConfig::default();

    // Parse command line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--profile" | "-g" => {
                i += 1;
                if i < args.len() {
                    config.profile_path = PathBuf::from(&args[i]);
                }
            }
            "--reflex-shader" | "-r" => {
                i += 1;
                if i < args.len() {
                    config.reflex_shader_path = PathBuf::from(&args[i]);
                }
            }
            "--gate-shader" => {
                i += 1;
                if i < args.len() {
                    config.gate_shader_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--no-gate-shader" => {
                config.gate_shader_path = None;
            }
            "--fps" => {
                i += 1;
                if i < args.len() {
                    config.target_fps = args[i].parse().unwrap_or(30.0);
                }
            }
            "--sensitivity" | "-s" => {
                i += 1;
                if i < args.len() {
                    config.mouse_sensitivity = args[i].parse().unwrap_or(1.0);
                }
            }
            "--no-hid" => {
                config.enable_hid_output = false;
            }
            "--no-gating" => {
                config.enable_epigenetic_gating = false;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    tracing::info!("╔══════════════════════════════════════════════════════════╗");
    tracing::info!("║   Aaroneous Spatial-Kinetic Engine                      ║");
    tracing::info!("║   Universal Gaming Genome Reflex Loop                   ║");
    tracing::info!("╚══════════════════════════════════════════════════════════╝");
    tracing::info!(
        profile = %config.profile_path.display(),
        reflex = %config.reflex_shader_path.display(),
        fps = config.target_fps,
        sensitivity = config.mouse_sensitivity,
        hid = config.enable_hid_output,
        gating = config.enable_epigenetic_gating,
        "Engine configuration loaded"
    );

    let engine = SpatialKineticEngine::new(config);

    // Perform internal health checks
    if !hypervisor::run_health_checks() {
        tracing::error!("Internal health checks failed! Engine aborting startup.");
        std::process::exit(1);
    }
    tracing::info!("Internal health checks passed.");

    // Handle Ctrl+C for graceful shutdown
    let engine_handle = std::sync::Arc::new(parking_lot::Mutex::new(engine));
    let engine_clone = engine_handle.clone();

    ctrlc::set_handler(move || {
        tracing::warn!("[SpatialKineticEngine] Received shutdown signal...");
        engine_clone.lock().stop();
    })
    .expect("Failed to set Ctrl+C handler");

    let result = engine_handle.lock().run().await;
    result?;

    Ok(())
}

#[cfg(windows)]
fn print_help() {
    println!("Usage: spatial_kinetic [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -g, --profile <PATH>        Path to profile binary file");
    println!("                             (default: chromosomes/universal_gaming_core.bin)");
    println!("  -r, --reflex-shader <PATH> Path to reflex kernel WGSL shader");
    println!("                             (default: shaders/reflex_kernel.wgsl)");
    println!("      --gate-shader <PATH>   Path to delta gate WGSL shader");
    println!("      --no-gate-shader       Disable epigenetic gate shader");
    println!("      --fps <FPS>            Target frame rate (default: 30)");
    println!("  -s, --sensitivity <VAL>    Mouse sensitivity multiplier (default: 1.0)");
    println!("      --no-hid               Disable HID output (capture + compute only)");
    println!("      --no-gating            Disable epigenetic visual gating");
    println!("  -h, --help                 Show this help message");
}

#[cfg(not(windows))]
fn main() -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "spatial-kinetic requires Windows desktop capture and input",
    ))
}
