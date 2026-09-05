// crates/flight_controller/src/config.rs
use clap::Parser;
use std::path::PathBuf;

/// Aaroneous Autonomous Flight Controller (AFC)
/// Native Rust daemon orchestrating autonomous code auditing, remediation,
/// validation gates, and delivery.
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

    /// Workspace repository root directory
    #[arg(long)]
    pub repo_root: Option<PathBuf>,
}
