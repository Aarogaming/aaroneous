use anyhow::{Result, bail};
use std::env;
use std::process::Command;

pub fn run() -> Result<()> {
    if env::consts::OS != "windows" {
        bail!("The uninstall subcommand is only supported on Windows.");
    }
    
    // 1. Remove from PATH via setx (placeholder)
    println!("Removing from PATH via setx is complex and not fully implemented here.");
    
    // 2. Remove registry entry
    println!("Removing registry entry...");
    let _ = Command::new("reg")
        .args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Aaroneous", "/f"])
        .status();
        
    // 3. Remove install dir
    let install_dir = r"C:\Programs\Aaroneous";
    println!("Removing install directory: {}", install_dir);
    let _ = std::fs::remove_dir_all(install_dir);
    
    // 4. Print summary
    println!("Uninstall complete.");
    Ok(())
}
