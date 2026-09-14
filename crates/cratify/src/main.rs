//! Cratify CLI — Universal Sovereign Cratification CLI & Orchestration Dispatcher.
//!
//! Thin CLI bridge routing commands to modular engine crates:
//! - `audit`     -> delegates to `ast_auditor::run_workspace_audit`
//! - `scaffold`  -> delegates to `transpiler::create_crate_scaffold_at`
//! - `generate`  -> alias for `scaffold`
//! - `harvest`   -> delegates to `adaptation_engine::harvest`

#![deny(unsafe_code)]

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "cratify")]
#[command(about = "Universal Sovereign Cratification CLI & Orchestration Dispatcher")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Audit workspace source files and manifests for architectural invariants
    Audit {
        /// Target files or directories to audit (defaults to core/ and crates/)
        #[arg(default_values = ["core/", "crates/"])]
        targets: Vec<PathBuf>,
    },
    /// Scaffold a new Aaroneous Crate Component (ACC)
    Scaffold {
        /// Name of the crate to scaffold
        name: String,
        /// Optional workspace root directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate a new Aaroneous Crate Component (alias for scaffold)
    Generate {
        /// Name of the crate to generate
        name: String,
        /// Optional workspace root directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Harvest an existing source tree into ACC crate specifications
    Harvest {
        /// Source directory to harvest
        #[arg(default_value = ".")]
        src: PathBuf,
        /// File extensions to treat as source
        #[arg(short, long, default_values = ["rs"])]
        extensions: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Audit { targets } => {
            println!("[cratify] Delegating audit to `ast_auditor` for {} targets...", targets.len());
            match ast_auditor::run_workspace_audit(targets) {
                Ok(()) => ExitCode::SUCCESS,
                Err(code) => code,
            }
        }
        Commands::Scaffold { name, output } | Commands::Generate { name, output } => {
            println!("[cratify] Delegating scaffolding to `transpiler` for crate `{}`...", name);
            let result = if let Some(out) = output {
                transpiler::create_crate_scaffold_at(&name, &out)
            } else {
                let root = paths::WorkspacePaths::default().root().to_path_buf();
                transpiler::create_crate_scaffold_at(&name, &root)
            };

            match result {
                Ok(path) => {
                    println!("[cratify] Crate `{}` successfully scaffolded at {}", name, path.display());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("[cratify ERROR] Failed to scaffold crate `{}`: {:#}", name, err);
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Harvest { src, extensions } => {
            println!(
                "[cratify] Delegating harvest to `adaptation_engine` on {} (extensions: {:?})...",
                src.display(),
                extensions
            );
            let config = adaptation_engine::HarvestConfig {
                extensions,
                strategy: adaptation_engine::ParseStrategy::Inspect,
            };

            match adaptation_engine::harvest(&src, &config) {
                Ok(specs) => {
                    println!(
                        "[cratify] Harvest complete: {} crate specification(s) discovered.",
                        specs.len()
                    );
                    for spec in &specs {
                        println!(" - Crate `{}` (source: {})", spec.name, spec.source_path.display());
                        println!(
                            "   Functions: {}, Structs: {}, Enums: {}",
                            spec.code_info.functions.len(),
                            spec.code_info.structs.len(),
                            spec.code_info.enums.len(),
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("[cratify ERROR] Failed to harvest source tree: {:#}", err);
                    ExitCode::FAILURE
                }
            }
        }
    }
}
