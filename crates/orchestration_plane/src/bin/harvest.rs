//! Unified Harvester CLI Runner
//!
//! Autonomous codebase ingestion, signature extraction, domain classification,
//! ambient AST remediation, and domain module grafting.
//!
//! Workflow:
//! Walk Rust source files -> Parse AST (syn) -> DomainClassifier::classify() ->
//! AmbientAstRewriter::rewrite() -> Grafter::graft_module() [or notify Novel Domain] ->
//! Output Ingestion and Remediated Code Reports.

#![allow(ambient_authority)]

use anyhow::{Context, Result};
use orchestration_plane::ast_transformer::AmbientAstRewriter;
use orchestration_plane::domain_classifier::{Domain, DomainClassifier, IngestionReport};
use orchestration_plane::grafter::{graft_module, GraftReport};
use paths::{WorkspacePaths, WorkspacePathsConfig};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// CLI Options parsed from environment arguments.
#[derive(Debug, Clone)]
pub struct HarvestCliOptions {
    pub input_path: PathBuf,
    pub dry_run: bool,
    pub verbose: bool,
}

impl HarvestCliOptions {
    pub fn parse() -> Result<Self> {
        let args: Vec<String> = env::args().collect();
        let mut input_path = None;
        let mut dry_run = false;
        let mut verbose = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--dry-run" => dry_run = true,
                "-v" | "--verbose" => verbose = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                arg if !arg.starts_with('-') && input_path.is_none() => {
                    input_path = Some(PathBuf::from(arg));
                }
                other => {
                    eprintln!("Unknown argument: {}", other);
                    print_usage();
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        let input_path = input_path.unwrap_or_else(|| PathBuf::from("dev/legacy_staging"));

        Ok(Self {
            input_path,
            dry_run,
            verbose,
        })
    }
}

fn print_usage() {
    println!("Usage: harvest [OPTIONS] [INPUT_PATH]");
    println!();
    println!("Autonomous codebase ingestion and domain grafting engine.");
    println!();
    println!("Options:");
    println!("  --dry-run       Analyze, classify, and rewrite ASTs without writing files to disk");
    println!("  -v, --verbose   Show detailed per-file AST reports and AST diffs");
    println!("  -h, --help      Display this help message");
}

pub struct HarvestSummary {
    pub files_scanned: usize,
    pub files_ingested: usize,
    pub compute_count: usize,
    pub hypervisor_count: usize,
    pub ipc_bus_count: usize,
    pub orchestrator_count: usize,
    pub novel_count: usize,
    pub total_ambient_risks: usize,
    pub total_rewrites: usize,
    pub graft_reports: Vec<GraftReport>,
    pub novel_candidates: Vec<IngestionReport>,
}

impl Default for HarvestSummary {
    fn default() -> Self {
        Self {
            files_scanned: 0,
            files_ingested: 0,
            compute_count: 0,
            hypervisor_count: 0,
            ipc_bus_count: 0,
            orchestrator_count: 0,
            novel_count: 0,
            total_ambient_risks: 0,
            total_rewrites: 0,
            graft_reports: Vec::new(),
            novel_candidates: Vec::new(),
        }
    }
}

/// Executes the complete harvesting pipeline over a directory or file.
pub fn run_harvest(
    input_path: &Path,
    dry_run: bool,
    workspace_root: &Path,
) -> Result<HarvestSummary> {
    let classifier = DomainClassifier::default();
    let mut summary = HarvestSummary::default();

    let mut target_files = Vec::new();
    if input_path.is_file() {
        if input_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            target_files.push(input_path.to_path_buf());
        }
    } else if input_path.is_dir() {
        for entry in WalkDir::new(input_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                target_files.push(path.to_path_buf());
            }
        }
    } else {
        anyhow::bail!("Input path does not exist: {:?}", input_path);
    }

    summary.files_scanned = target_files.len();

    println!("Scanning {} Rust source files in {:?}", summary.files_scanned, input_path);
    println!("Mode: {}", if dry_run { "[DRY RUN] (Simulation only)" } else { "[LIVE] (Grafting to crates)" });
    println!("------------------------------------------------------------");

    for file_path in target_files {
        let source_code = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[SKIP] Could not read file {:?}: {}", file_path, e);
                continue;
            }
        };

        let file_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.rs");

        // 1. AST Domain Classification
        let report = match classifier.classify_source(file_name, &source_code) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[PARSE ERROR] Failed to parse {:?}: {}", file_path, e);
                continue;
            }
        };

        summary.files_ingested += 1;

        match &report.target_domain {
            Domain::Compute => summary.compute_count += 1,
            Domain::Hypervisor => summary.hypervisor_count += 1,
            Domain::IpcBus => summary.ipc_bus_count += 1,
            Domain::Orchestrator => summary.orchestrator_count += 1,
            Domain::Novel(_) => summary.novel_count += 1,
        }

        // 2. Ambient Authority AST Rewriting
        let mut rewriter = AmbientAstRewriter::new();
        let (remediated_code, rewrite_summary) = match rewriter.rewrite_source(&source_code) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("[REWRITE ERROR] Failed to rewrite {:?}: {}", file_path, e);
                continue;
            }
        };

        let total_file_rewrites = rewrite_summary.canonicalize_rewrites
            + rewrite_summary.env_temp_dir_rewrites
            + rewrite_summary.env_current_dir_rewrites
            + rewrite_summary.env_var_rewrites
            + rewrite_summary.stripped_banned_imports;
        summary.total_rewrites += total_file_rewrites;

        // 3. Post-Remediation Verification & Parity Check
        // Re-analyze remediated AST to verify remaining ambient risks
        let post_report = match classifier.classify_source(file_name, &remediated_code) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[POST-CHECK ERROR] Failed to parse remediated {:?}: {}", file_path, e);
                continue;
            }
        };

        summary.total_ambient_risks += post_report.ambient_risks.len();

        let unhandled_risks = post_report.ambient_risks.len();
        let status = if unhandled_risks == 0 {
            "REMEDIATED (0 unhandled)"
        } else {
            "QUARANTINED"
        };

        println!(
            "-> {:<25} | Domain: {:<16} | Items: {:<2} | Pre-Risks: {:<2} | Rewrites: {:<2} | Status: {}",
            file_name,
            report.target_domain.to_string(),
            report.public_items.len(),
            report.ambient_risks.len(),
            total_file_rewrites,
            status
        );

        if !report.ambient_risks.is_empty() {
            for risk in &report.ambient_risks {
                println!("   [PRE-REWRITE RISK] L{}:{} -> {}", risk.line, risk.column, risk.symbol);
            }
        }

        if unhandled_risks > 0 {
            for risk in &post_report.ambient_risks {
                println!("   [UNHANDLED RISK] L{}:{} -> {}", risk.line, risk.column, risk.symbol);
            }
        }

        // 4. Domain Grafting or Quarantine Isolation
        match &report.target_domain {
            Domain::Novel(candidate_name) => {
                println!("   [NOVEL DOMAIN] Candidate: `{}` (Score < 0.60 threshold). Tagged for staging.", candidate_name);
                summary.novel_candidates.push(report);
            }
            _ => {
                if unhandled_risks > 0 {
                    if dry_run {
                        println!(
                            "   [QUARANTINE SIMULATED] {} unhandled risks -> staging/quarantine/{}",
                            unhandled_risks,
                            orchestration_plane::grafter::sanitize_module_name(file_name)
                        );
                    } else {
                        println!(
                            "   [QUARANTINED] Module contains unhandled risks. Routing to staging/quarantine/..."
                        );
                        let q_report = orchestration_plane::grafter::quarantine_module(
                            &post_report,
                            &remediated_code,
                            workspace_root,
                        ).with_context(|| format!("Failed to quarantine {:?}", file_name))?;
                        println!("   [QUARANTINED] Isolated at: {:?}", q_report.destination_file);
                        summary.graft_reports.push(q_report);
                    }
                } else if dry_run {
                    let planned_dest = orchestration_plane::grafter::resolve_domain_crate_path(
                        &report.target_domain,
                        workspace_root,
                    );
                    let sanitized = orchestration_plane::grafter::sanitize_module_name(file_name);
                    println!(
                        "   [PLANNED GRAFT] {} -> {}/src/{}.rs",
                        file_name,
                        planned_dest.strip_prefix(workspace_root).unwrap_or(&planned_dest).display(),
                        sanitized
                    );
                } else {
                    let graft_result = graft_module(&post_report, &remediated_code, workspace_root)
                        .with_context(|| format!("Failed to graft {:?}", file_name))?;
                    println!(
                        "   [GRAFTED] Written to: {:?} (Updated: {:?})",
                        graft_result.destination_file, graft_result.updated_lib_rs
                    );
                    summary.graft_reports.push(graft_result);
                }
            }
        }
    }

    println!("------------------------------------------------------------");
    println!("=== HARVEST INGESTION SUMMARY ===");
    println!("Files Scanned:        {}", summary.files_scanned);
    println!("Files Ingested:       {}", summary.files_ingested);
    println!("Domain Distribution:");
    println!("  - Compute:          {}", summary.compute_count);
    println!("  - Hypervisor:       {}", summary.hypervisor_count);
    println!("  - IpcBus:           {}", summary.ipc_bus_count);
    println!("  - Orchestrator:     {}", summary.orchestrator_count);
    println!("  - Novel Domain:     {}", summary.novel_count);
    println!("Remaining Ambient Risks: {}", summary.total_ambient_risks);
    println!("AST Rewrites Applied:    {}", summary.total_rewrites);
    println!("=================================");

    Ok(summary)
}

fn main() -> Result<()> {
    let opts = HarvestCliOptions::parse()?;
    let workspace_paths = WorkspacePaths::discover(&WorkspacePathsConfig::new());

    run_harvest(&opts.input_path, opts.dry_run, workspace_paths.root())?;

    Ok(())
}
