// dev/tools/afc/src/config.rs
use clap::Parser;
use std::path::PathBuf;

/// Aaroneous Autonomous Flight Controller (AFC)
/// Out-of-tree Sovereign CI/CD Daemon & GUI Hypervisor
#[derive(Parser, Debug, Clone, Default)]
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

    // ── Phase Selection Toggles ─────────────────────────────────────────────
    #[arg(long, default_value_t = true)]
    pub phase_plan: bool,

    #[arg(long, default_value_t = true)]
    pub phase_audit: bool,

    #[arg(long, default_value_t = true)]
    pub phase_fix: bool,

    #[arg(long, default_value_t = true)]
    pub phase_sweep: bool,

    #[arg(long, default_value_t = true)]
    pub phase_verify: bool,

    #[arg(long, default_value_t = true)]
    pub phase_commit: bool,

    #[arg(long, default_value_t = true)]
    pub phase_deliver: bool,

    // ── Audit Types Multiselect ─────────────────────────────────────────────
    #[arg(long, default_value_t = true)]
    pub audit_security: bool,

    #[arg(long, default_value_t = true)]
    pub audit_panics: bool,

    #[arg(long, default_value_t = true)]
    pub audit_concurrency: bool,

    #[arg(long, default_value_t = true)]
    pub audit_dead_code: bool,

    #[arg(long, default_value_t = true)]
    pub audit_health: bool,

    #[arg(long, default_value_t = true)]
    pub audit_resilience: bool,
}

impl FlightConfig {
    /// Resolve the target repository path
    pub fn resolve_repo_root(&self) -> PathBuf {
        if let Some(ref path) = self.repo_root {
            return path.clone();
        }

        let is_repo_root = |p: &std::path::Path| {
            (p.join(".git").exists() || p.join("core").join("hypervisor").exists())
                && (p.join("opencode.json").exists() || p.join("crates").exists())
        };

        // 1. Try current working directory or its parents
        if let Ok(cwd) = std::env::current_dir() {
            if is_repo_root(&cwd) {
                return cwd;
            }
            let mut parent = cwd.as_path();
            while let Some(p) = parent.parent() {
                if is_repo_root(p) {
                    return p.to_path_buf();
                }
                parent = p;
            }
        }

        // 2. Check parent directories of the running executable
        if let Ok(exe_path) = std::env::current_exe() {
            let mut parent = exe_path.as_path();
            while let Some(p) = parent.parent() {
                if is_repo_root(p) {
                    return p.to_path_buf();
                }
                parent = p;
            }
        }

        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}
