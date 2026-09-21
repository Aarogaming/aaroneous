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
        /// Optional absolute path to export the full `Vec<CrateSpec>` as JSON
        /// (M49). Written atomically: serialized to `<output>.partial`, then
        /// renamed into place, so a concurrent reader never observes a
        /// truncated file.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// M49: atomically write `specs` as pretty JSON to `output`.
///
/// Follows the same `.partial`-then-rename pattern established by M40
/// (`core/hypervisor/src/supervisory_loop.rs::write_concurrence_report`) and
/// M47 (`core/hypervisor/bin/hypervisor.rs::run_export_capabilities`): write
/// to a sibling `.partial` file first, then `fs::rename` into place, so a
/// concurrent reader never sees a partially-written file.
///
/// `output` must be an absolute path, consistent with this codebase's
/// explicit-path conventions (see `CONTRIBUTING.md` "No ambient reads").
fn export_crate_specs(
    specs: &[adaptation_engine::CrateSpec],
    output: &std::path::Path,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        output.is_absolute(),
        "harvest --output path must be absolute, got {:?}",
        output
    );

    let partial_path = output.with_extension(match output.extension() {
        Some(ext) => format!("{}.partial", ext.to_string_lossy()),
        None => "partial".to_string(),
    });

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let json_bytes = serde_json::to_vec_pretty(specs)?;
    std::fs::write(&partial_path, &json_bytes)?;
    std::fs::rename(&partial_path, output)?;

    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Audit { targets } => {
            println!(
                "[cratify] Delegating audit to `ast_auditor` for {} targets...",
                targets.len()
            );
            match ast_auditor::run_workspace_audit(targets) {
                Ok(()) => ExitCode::SUCCESS,
                Err(code) => code,
            }
        }
        Commands::Scaffold { name, output } | Commands::Generate { name, output } => {
            println!(
                "[cratify] Delegating scaffolding to `transpiler` for crate `{}`...",
                name
            );
            let result = if let Some(out) = output {
                transpiler::create_crate_scaffold_at(&name, &out)
            } else {
                let root = paths::WorkspacePaths::default().root().to_path_buf();
                transpiler::create_crate_scaffold_at(&name, &root)
            };

            match result {
                Ok(path) => {
                    println!(
                        "[cratify] Crate `{}` successfully scaffolded at {}",
                        name,
                        path.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!(
                        "[cratify ERROR] Failed to scaffold crate `{}`: {:#}",
                        name, err
                    );
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Harvest {
            src,
            extensions,
            output,
        } => {
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
                        println!(
                            " - Crate `{}` (source: {})",
                            spec.name,
                            spec.source_path.display()
                        );
                        println!(
                            "   Functions: {}, Structs: {}, Enums: {}",
                            spec.code_info.functions.len(),
                            spec.code_info.structs.len(),
                            spec.code_info.enums.len(),
                        );
                    }

                    if let Some(output_path) = output {
                        match export_crate_specs(&specs, &output_path) {
                            Ok(()) => {
                                println!(
                                    "[cratify] Exported {} crate specification(s) to {}",
                                    specs.len(),
                                    output_path.display()
                                );
                                ExitCode::SUCCESS
                            }
                            Err(err) => {
                                eprintln!(
                                    "[cratify ERROR] Failed to export crate specifications to {}: {:#}",
                                    output_path.display(),
                                    err
                                );
                                ExitCode::FAILURE
                            }
                        }
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(err) => {
                    eprintln!("[cratify ERROR] Failed to harvest source tree: {:#}", err);
                    ExitCode::FAILURE
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! M49: round-trip test for `harvest --output`'s JSON export.
    //!
    //! Lives here (not in `adaptation_engine::source_extraction`) because the
    //! export/atomic-write logic (`export_crate_specs`) is itself defined in
    //! this file, per the CLI-owns-its-I/O split established by M40/M47
    //! (the export function lives beside the CLI command that exposes it, not
    //! inside the read-only analysis engine it wraps).
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn harvest_export_round_trips_through_json() {
        // Harvest a trivial fixture source tree, all inside a tempdir per
        // CONTRIBUTING.md's mandatory-tempdir-in-tests rule.
        let src_dir = tempdir().expect("src tempdir");
        let fixture_path = src_dir.path().join("widget.rs");
        let mut fixture = File::create(&fixture_path).expect("create fixture");
        writeln!(fixture, "pub fn make_widget() -> u32 {{ 7 }}").expect("write fixture");
        drop(fixture);

        let config = adaptation_engine::HarvestConfig {
            extensions: vec!["rs".to_string()],
            strategy: adaptation_engine::ParseStrategy::Inspect,
        };
        let specs = adaptation_engine::harvest(src_dir.path(), &config).expect("harvest");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "widget");
        assert_eq!(specs[0].code_info.functions.len(), 1);

        // Export via the same atomic-write path the CLI's `--output` flag
        // uses, into a separate output tempdir (an absolute path, as
        // `export_crate_specs` requires).
        let out_dir = tempdir().expect("out tempdir");
        let output_path = out_dir.path().join("harvest.json");
        assert!(output_path.is_absolute());
        export_crate_specs(&specs, &output_path).expect("export");

        // No leftover `.partial` file after the atomic rename.
        let partial_path = output_path.with_extension("json.partial");
        assert!(!partial_path.exists());

        // Deserialize it back and assert the round-trip preserves crate name
        // and function count.
        let json_bytes = std::fs::read(&output_path).expect("read exported json");
        let round_tripped: Vec<adaptation_engine::CrateSpec> =
            serde_json::from_slice(&json_bytes).expect("deserialize exported json");
        assert_eq!(round_tripped.len(), 1);
        assert_eq!(round_tripped[0].name, "widget");
        assert_eq!(round_tripped[0].code_info.functions.len(), 1);
        assert_eq!(round_tripped[0].code_info.functions[0].name, "make_widget");
    }

    #[test]
    fn export_rejects_relative_output_path() {
        let specs: Vec<adaptation_engine::CrateSpec> = Vec::new();
        let relative = std::path::Path::new("relative/output.json");
        let err = export_crate_specs(&specs, relative).expect_err("relative path must be rejected");
        assert!(err.to_string().contains("absolute"));
    }
}
