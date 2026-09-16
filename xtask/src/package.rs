use anyhow::{Result, bail};
use std::process::Command;
use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<()> {
    let mut version = String::from("unknown");
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--version" && i + 1 < args.len() {
            version = args[i + 1].clone();
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
    
    let staging = format!("dist/Aaroneous-v{}-windows-x86_64", version);
    let staging_path = PathBuf::from(&staging);
    let bin_dir = staging_path.join("bin");
    
    std::fs::create_dir_all(&bin_dir)?;
    let _ = std::fs::copy("target/release/aaroneous.exe", bin_dir.join("aaroneous.exe"));
    let _ = std::fs::copy("target/release/a_run.exe", bin_dir.join("a_run.exe"));
    
    for dir in ["config", "shaders", "deploy/mcp_clients"] {
        let dest = format!("{}/{}", staging, dir);
        let _ = std::fs::create_dir_all(&dest);
        let _ = Command::new("powershell")
            .args(["-Command", &format!("Copy-Item -Path '{}' -Destination '{}' -Recurse -ErrorAction SilentlyContinue", dir, dest)])
            .status();
    }
    
    let zip_path = format!("dist/Aaroneous-v{}-windows-x86_64.zip", version);
    println!("Zipping staging dir to {}...", zip_path);
    let status = Command::new("powershell")
        .args(["-Command", &format!("Compress-Archive -Path '{}\\*' -DestinationPath '{}' -CompressionLevel Optimal", staging, zip_path)])
        .status()?;
    if !status.success() { bail!("Failed to zip staging dir"); }
    
    println!("Packaging complete: {}", zip_path);
    Ok(())
}
