// Spatial-Kinetic Engine Binary
// Standalone executable that runs the universal spatial-kinetic reflex loop.
//
// Usage:
//   spatial_kinetic.exe                          # Run with defaults
//   spatial_kinetic.exe --genome path/to/genome  # Custom genome path
//   spatial_kinetic.exe --fps 60                 # Target 60 FPS
//   spatial_kinetic.exe --no-hid                 # Disable HID output (capture only)

use std::path::PathBuf;

use a_run::spatial_kinetic_engine::{SpatialKineticConfig, SpatialKineticEngine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut config = SpatialKineticConfig::default();

    // Parse command line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--genome" | "-g" => {
                i += 1;
                if i < args.len() {
                    config.genome_path = PathBuf::from(&args[i]);
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

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   Aaroneous Spatial-Kinetic Engine                      ║");
    println!("║   Universal Gaming Genome Reflex Loop                   ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Genome:     {}", config.genome_path.display());
    println!("Reflex:     {}", config.reflex_shader_path.display());
    println!("Gate:       {:?}", config.gate_shader_path.as_ref().map(|p| p.display()));
    println!("FPS:        {}", config.target_fps);
    println!("Sensitivity: {}", config.mouse_sensitivity);
    println!("HID Output: {}", if config.enable_hid_output { "enabled" } else { "disabled" });
    println!("Gating:     {}", if config.enable_epigenetic_gating { "enabled" } else { "disabled" });
    println!();

    let engine = SpatialKineticEngine::new(config);

    // Handle Ctrl+C for graceful shutdown
    let engine_handle = std::sync::Arc::new(parking_lot::Mutex::new(engine));
    let engine_clone = engine_handle.clone();

    ctrlc::set_handler(move || {
        println!("\n[SpatialKineticEngine] Received shutdown signal...");
        engine_clone.lock().stop();
    })
    .expect("Failed to set Ctrl+C handler");

    engine_handle.lock().run().await?;

    Ok(())
}

fn print_help() {
    println!("Usage: spatial_kinetic [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -g, --genome <PATH>        Path to genome binary file");
    println!("                             (default: chromosomes/universal_gaming_core.bin)");
    println!("  -r, --reflex-shader <PATH> Path to reflex kernel WGSL shader");
    println!("                             (default: shaders/reflex_kernel.wgsl)");
    println!("      --gate-shader <PATH>   Path to epigenetic gate WGSL shader");
    println!("      --no-gate-shader       Disable epigenetic gate shader");
    println!("      --fps <FPS>            Target frame rate (default: 30)");
    println!("  -s, --sensitivity <VAL>    Mouse sensitivity multiplier (default: 1.0)");
    println!("      --no-hid               Disable HID output (capture + compute only)");
    println!("      --no-gating            Disable epigenetic visual gating");
    println!("  -h, --help                 Show this help message");
}
