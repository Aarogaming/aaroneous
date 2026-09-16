use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// AST Auditor — Invariant and Architectural Compliance CLI
#[derive(Parser)]
#[command(
    name = "ast_auditor",
    author,
    version,
    about = "Static analysis AST auditor enforcing architectural invariants"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Fallback positional targets if no subcommand given
    #[arg(trailing_var_arg = true)]
    targets: Vec<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect target directories or files for invariant violations
    Audit {
        /// Paths to the source directories or files
        paths: Vec<PathBuf>,
    },
    /// Review target code against declarative architectural patterns
    Review {
        /// Paths to the source directories or files
        paths: Vec<PathBuf>,
        /// Optional path to the pattern definitions registry
        #[arg(long)]
        registry: Option<PathBuf>,
        /// Output findings in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Verify repository-level hardening policy and print its static inventory
    Hardening {
        /// Fail when the baseline policy is incomplete
        #[arg(long)]
        check: bool,
        /// Output the report as JSON
        #[arg(long)]
        json: bool,
        /// Repository root to inspect
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Audit { paths }) => {
            let target_paths = if paths.is_empty() {
                default_targets()
            } else {
                paths
            };
            match ast_auditor::run_workspace_audit(target_paths) {
                Ok(()) => ExitCode::SUCCESS,
                Err(code) => code,
            }
        }
        Some(Commands::Review {
            paths,
            registry,
            json,
        }) => {
            let targets = if paths.is_empty() {
                default_targets()
            } else {
                paths
            };
            match ast_auditor::run_pattern_review(&targets, registry) {
                Ok(report) => {
                    if json {
                        match serde_json::to_string_pretty(&report) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("Failed to serialize report: {e}");
                                return ExitCode::FAILURE;
                            }
                        }
                    } else {
                        print!("{report}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Pattern review failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some(Commands::Hardening { check, json, root }) => {
            let report = if check {
                ast_auditor::hardening::enforce_hardening_audit(&root)
            } else {
                ast_auditor::hardening::run_hardening_audit(&root)
            };
            match report {
                Ok(report) => {
                    if json {
                        match serde_json::to_string_pretty(&report) {
                            Ok(output) => println!("{output}"),
                            Err(error) => {
                                eprintln!("Failed to serialize hardening report: {error}");
                                return ExitCode::FAILURE;
                            }
                        }
                    } else {
                        println!("Hardening audit baseline");
                        for (name, passed) in &report.checks {
                            println!("{} {name}", if *passed { "PASS" } else { "FAIL" });
                        }
                        println!("Inventory: {:?}", report.source_inventory);
                        println!(
                            "cargo audit available: {}",
                            report.tooling.cargo_audit_available
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Hardening audit failed: {error:#}");
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            let target_paths = if cli.targets.is_empty() {
                default_targets()
            } else {
                cli.targets
            };
            match ast_auditor::run_workspace_audit(target_paths) {
                Ok(()) => ExitCode::SUCCESS,
                Err(code) => code,
            }
        }
    }
}

fn default_targets() -> Vec<PathBuf> {
    vec![
        PathBuf::from("core/hypervisor"),
        PathBuf::from("crates/compute"),
        PathBuf::from("crates/orchestration_plane"),
        PathBuf::from("crates/llm_gateway"),
    ]
}
