// crates/flight_controller/src/delivery.rs
use anyhow::{Context, Result};
use chrono::Local;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use tracing::{info, warn};

pub struct DeliveryEngine;

impl DeliveryEngine {
    /// Package compiled release binaries into timestamped archive under releases/.
    pub async fn package_artifacts(repo_path: &Path) -> Result<()> {
        let release_dir = repo_path.join("releases");
        if !release_dir.exists() {
            fs::create_dir_all(&release_dir)
                .await
                .context("Failed to create releases directory")?;
        }

        let date_tag = Local::now().format("%Y%m%d-%H%M").to_string();
        let zip_filename = format!("Aaroneous_Flight_{date_tag}.zip");
        let zip_path = release_dir.join(&zip_filename);

        let target_release = repo_path.join("target").join("release");
        if !target_release.exists() {
            warn!("target/release directory does not exist. Skipping artifact packaging.");
            return Ok(());
        }

        // Collect all .exe files in target/release
        let mut exe_files = Vec::new();
        if let Ok(mut entries) = fs::read_dir(&target_release).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "exe") {
                    exe_files.push(path);
                }
            }
        }

        if exe_files.is_empty() {
            warn!("No executable binaries found in target/release to package.");
            return Ok(());
        }

        // Use native tar.exe (available on Windows 10/11) to create zip
        let mut tar_cmd = Command::new("tar");
        tar_cmd.arg("-a").arg("-c").arg("-f").arg(&zip_path);

        for exe in &exe_files {
            tar_cmd.arg(exe);
        }

        let output = tar_cmd.output().await;
        match output {
            Ok(out) if out.status.success() => {
                info!("Release archive successfully packaged: {:?}", zip_path);
            }
            _ => {
                // Fallback: copy exes directly to release_dir
                for exe in &exe_files {
                    if let Some(file_name) = exe.file_name() {
                        let dest = release_dir.join(file_name);
                        let _ = fs::copy(exe, dest).await;
                    }
                }
                info!("Release binaries copied directly to {:?}", release_dir);
            }
        }

        Ok(())
    }
}
