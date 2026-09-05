// dev/tools/afc/src/hardware.rs
use anyhow::Result;
use std::path::Path;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

pub struct HardwareMonitor;

impl HardwareMonitor {
    pub async fn check_gpu_thermals(max_temp: u32) -> Result<()> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu,memory.used",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .await;

        let Ok(out) = output else {
            return Ok(());
        };

        if !out.status.success() {
            return Ok(());
        }

        let raw_str = String::from_utf8_lossy(&out.stdout);
        let mut parts = raw_str.trim().split(',');
        if let Some(temp_str) = parts.next() {
            if let Ok(temp) = temp_str.trim().parse::<u32>() {
                let vram_str = parts.next().unwrap_or("0").trim();
                info!("Hardware State: GPU Temp: {temp}°C | VRAM: {vram_str} MB");

                if temp >= max_temp {
                    warn!(
                        "THERMAL THROTTLE: GPU reached {temp}°C (Limit: {max_temp}°C). Cooling for 30s..."
                    );
                    sleep(Duration::from_secs(30)).await;
                }
            }
        }

        Ok(())
    }

    pub async fn check_build_cache_size(repo_path: &Path, max_gb: u64) -> Result<()> {
        let target_dir = repo_path.join("target");
        if !target_dir.exists() {
            return Ok(());
        }

        let mut total_bytes: u64 = 0;
        let mut stack = vec![target_dir.clone()];

        while let Some(dir) = stack.pop() {
            if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(meta) = entry.metadata().await {
                        if meta.is_dir() {
                            stack.push(entry.path());
                        } else {
                            total_bytes = total_bytes.saturating_add(meta.len());
                        }
                    }
                }
            }
        }

        let total_gb = total_bytes / (1024 * 1024 * 1024);
        if total_gb >= max_gb {
            warn!(
                "Build cache size is {total_gb} GB (threshold: {max_gb} GB). Running cargo clean on target repo..."
            );
            let _ = Command::new("cargo")
                .current_dir(repo_path)
                .arg("clean")
                .output()
                .await;
            info!("Target repo build cache cleaned successfully.");
        }

        Ok(())
    }
}
