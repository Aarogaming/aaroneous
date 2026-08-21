use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ShadowSandbox {
    shadow_dir: PathBuf,
}

impl ShadowSandbox {
    pub fn new() -> Result<Self> {
        let shadow_dir = std::env::current_dir()?.join(".sab").join("shadow");
        if !shadow_dir.exists() {
            fs::create_dir_all(&shadow_dir).context("Failed to create shadow sandbox directory")?;
        }
        Ok(Self { shadow_dir })
    }

    pub fn shadow_dir(&self) -> &Path {
        &self.shadow_dir
    }

    /// Write file strictly inside the shadow sandbox, preventing any escape to live root
    pub fn write_shadow_file(&self, file_name: &str, content: &[u8]) -> Result<PathBuf> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow write"))?;
        let target_path = self.shadow_dir.join(safe_name);
        fs::write(&target_path, content).context("Failed to write file inside shadow sandbox")?;
        Ok(target_path)
    }

    /// Execute syntax check or compiler toolchain strictly within shadow directory using relative filename
    pub fn execute_syntax_check(&self, file_name: &str) -> Result<(bool, String)> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow execution"))?;
        let target_path = self.shadow_dir.join(safe_name);

        if !target_path.exists() {
            anyhow::bail!(
                "Target file does not exist in shadow sandbox: {:?}",
                target_path
            );
        }

        tracing::info!(target: "shadow_sandbox", path = ?target_path, "Executing sandboxed toolchain check");

        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let mut cmd = match ext {
            "rs" => {
                let mut c = Command::new("rustc");
                c.arg("--crate-type=lib")
                    .arg("--emit=metadata")
                    .arg(safe_name);
                c
            }
            "py" => {
                let mut c = Command::new("python");
                c.arg("-m").arg("py_compile").arg(safe_name);
                c
            }
            _ => {
                let mut c = Command::new("rustc");
                c.arg("--crate-type=lib")
                    .arg("--emit=metadata")
                    .arg(safe_name);
                c
            }
        };

        cmd.current_dir(&self.shadow_dir);

        let output = cmd
            .output()
            .context("Failed to execute sandboxed compiler toolchain")?;
        let success = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);

        Ok((success, combined))
    }
}
