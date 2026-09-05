//! core/hypervisor/bin/watchdog.rs
//! Headless Watchdog Daemon (CONSUMER-11)
//! Spawns and monitors the Aaroneous UI. If the UI panics or crashes, it automatically restarts it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    let mut crash_count = 0;
    
    loop {
        // We use the same executable directory to find the aaroneous UI
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe_path.parent().expect("Failed to find executable directory");
        let target = exe_dir.join("aaroneous.exe");
        
        // Fallback to "cargo run --bin aaroneous" if running in dev mode
        let mut command = if target.exists() {
            Command::new(&target)
        } else {
            let mut c = Command::new("cargo");
            c.arg("run").arg("--bin").arg("aaroneous");
            c
        };

        let mut child = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|_| panic!("Failed to launch Aaroneous UI"));

        // Wait for the UI process to exit
        let status = child.wait().expect("Failed to wait on child process");

        if status.success() {
            // Normal exit, user closed the app intentionally
            break;
        } else {
            // Panic or crash
            crash_count += 1;
            // Sleep briefly to prevent tight infinite crash loops
            thread::sleep(Duration::from_millis(1000));
            
            if crash_count > 10 {
                // If it crashes 10 times, give up to prevent burning CPU
                break;
            }
        }
    }
}