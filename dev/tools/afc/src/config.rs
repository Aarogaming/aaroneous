// dev/tools/afc/src/config.rs
use clap::Parser;
use std::path::PathBuf;

/// Aaroneous Autonomous Flight Controller (AFC)
/// Out-of-tree Sovereign CI/CD Daemon & GUI Hypervisor
#[derive(Parser, Debug, Clone)]
#[command(name = "afc", author, version, about)]
pub struct FlightConfig {
    /// Number of autonomous cycles to run
    #[arg(long, default_value_t = 5)]
    pub auto_cycles: usize,

    /// Enforce zero Clippy warnings gate
    #[arg(long, default_value_t = true)]
    pub clippy_gate: bool,

    /// Automatically rollback modified files if validation fails
    #[arg(long, default_value_t = true)]
    pub auto_rollback: bool,

    /// Automatically branch off protected branches (main/master)
    #[arg(long, default_value_t = true)]
    pub auto_branch: bool,

    /// Run non-interactively without prompt
    #[arg(long, default_value_t = false)]
    pub non_interactive: bool,

    /// Maximum GPU temperature (in Celsius) before cooling throttle
    #[arg(long, default_value_t = 80)]
    pub max_gpu_temp: u32,

    /// Run workspace unit test suite during verification
    #[arg(long, default_value_t = true)]
    pub run_tests: bool,

    /// Enforce cargo fmt across workspace before commit
    #[arg(long, default_value_t = true)]
    pub enforce_format: bool,

    /// Run cargo-audit supply chain vulnerability check
    #[arg(long, default_value_t = true)]
    pub run_security: bool,

    /// Build release binaries and package release artifacts at cycle completion
    #[arg(long, default_value_t = true)]
    pub build_artifacts: bool,

    /// Watchdog timeout in seconds for agent tasks
    #[arg(long, default_value_t = 300)]
    pub watchdog_timeout_secs: u64,

    /// Target Aaroneous repository root directory (auto-detects if omitted)
    #[arg(long)]
    pub repo_root: Option<PathBuf>,

    /// Launch interactive Desktop GUI HUD
    #[arg(long, default_value_t = false)]
    pub gui: bool,
}

impl FlightConfig {
    /// Resolve the target repository path
    pub fn resolve_repo_root(&self) -> PathBuf {
        if let Some(ref path) = self.repo_root {
            return path.clone();
        }

        // Try current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if cwd.join(".git").exists() || cwd.join("Cargo.toml").exists() {
            return cwd;
        }

        // Fallback: check parent directories (e.g. if run from dev/tools/afc)
        let mut parent = cwd.as_path();
        while let Some(p) = parent.parent() {
            if p.join(".git").exists() {
                return p.to_path_buf();
            }
            parent = p;
        }

        PathBuf::from("d:\\Aaroneous")
    }
}
