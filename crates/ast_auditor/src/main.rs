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
    /// Validate UTF-8 encoding, BOM absence, and LF/CRLF line endings across Git-tracked files
    CheckEncoding {
        /// Optional repository root directory (defaults to current directory or Git root)
        #[arg(long)]
        root: Option<PathBuf>,
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
            match cratify_core::run_workspace_audit(target_paths) {
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
            match cratify_core::run_pattern_review(&targets, registry) {
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
        Some(Commands::CheckEncoding { root }) => {
            let repo_root = root.unwrap_or_else(|| PathBuf::from("."));
            match cratify_core::audit_tracked_encodings(&repo_root) {
                Ok(violations) => {
                    if violations.is_empty() {
                        println!("All text files pass encoding and line ending checks.");
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("Encoding and line ending violations found:");
                        for v in &violations {
                            eprintln!("  {v}");
                        }
                        eprintln!("Total violations: {}", violations.len());
                        ExitCode::FAILURE
                    }
                }
                Err(err) => {
                    eprintln!("Failed to perform encoding audit: {err}");
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
            match cratify_core::run_workspace_audit(target_paths) {
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
