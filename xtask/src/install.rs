use anyhow::{Result, bail, Context};
use std::process::Command;
use std::env;
use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<()> {
    if env::consts::OS != "windows" {
        bail!("The install subcommand is only supported on Windows.");
    }
    
    let mut install_dir = String::from(r"C:\Programs\Aaroneous");
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--dir" && i + 1 < args.len() {
            install_dir = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }
    
    println!("Building release binaries...");
    let status = Command::new("cargo")
        .args(["build", "--release", "--bin", "aaroneous", "--bin", "a_run"])
        .status()?;
    if !status.success() { bail!("Failed to build release binaries"); }
    
    let base_path = PathBuf::from(&install_dir);
    let bin_dir = base_path.join("bin");
    let data_dir = base_path.join("data");
    let config_dir = base_path.join("config");
    
    std::fs::create_dir_all(&bin_dir).context("Failed to create bin dir")?;
    std::fs::create_dir_all(&data_dir).context("Failed to create data dir")?;
    std::fs::create_dir_all(&config_dir).context("Failed to create config dir")?;
    
    // Copy binaries - ignore errors if they don't exist yet (in case aaroneous crate name differs)
    let _ = std::fs::copy("target/release/aaroneous.exe", bin_dir.join("aaroneous.exe"));
    let _ = std::fs::copy("target/release/a_run.exe", bin_dir.join("a_run.exe"));
        
    let bin_dir_str = bin_dir.to_str().unwrap();
    println!("Adding {} to PATH via setx...", bin_dir_str);
    
    let status = Command::new("setx")
        .args(["PATH", &format!("%PATH%;{}", bin_dir_str)])
        .status()?;
    if !status.success() {
        bail!("Failed to update PATH via setx");
    }
    
    println!("Install complete to {}", install_dir);
    Ok(())
}
