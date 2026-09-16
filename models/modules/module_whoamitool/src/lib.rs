//! Auto-generated Aaroneous Machine-Native Organ Wrapper for: WhoamiTool
//! Synthesized by the Adaptation Engine Stem Cell Auto-Wrapping Engine.

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// Sovereign Organ Wrapper for `WhoamiTool`
#[derive(Debug, Clone)]
pub struct WhoamiToolOrgan {
    pub executable_path: String,
    pub domain_opcode: u16,
    pub is_dry_run: bool,
}

impl Default for WhoamiToolOrgan {
    fn default() -> Self {
        Self {
            executable_path: "C:\\Windows\\System32\\whoami.exe".to_string(),
            domain_opcode: 0x0400,
            is_dry_run: false,
        }
    }
}

impl WhoamiToolOrgan {
    pub fn new(executable_path: &str) -> Self {
        Self {
            executable_path: executable_path.to_string(),
            ..Default::default()
        }
    }

    /// Invokes the underlying native tool and captures machine-native output
    pub async fn invoke(&self, args: &[&str], input_payload: Option<&[u8]>) -> Result<Vec<u8>> {
        info!(target: "whoamitool_organ", ?args, "Invoking sovereign wrapped organ");

        if self.is_dry_run {
            return Ok(b"MNLP_DRY_RUN_SUCCESS".to_vec());
        }

        let mut child = Command::new(&self.executable_path)
            .args(args)
            .stdin(if input_payload.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn {}", self.executable_path))?;

        if let Some(payload) = input_payload {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(payload).await?;
            }
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            warn!(target: "whoamitool_organ", exit_code = ?output.status.code(), %err_msg, "Organ execution returned warning/error");
        }

        Ok(output.stdout)
    }
}
