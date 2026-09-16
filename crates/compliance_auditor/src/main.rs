//! Aaroneous compliance auditor CLI.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use compliance_auditor::ReviewConfig;
use llm_gateway::{LLMClient, LLMConfig, ProviderType};

#[derive(Parser)]
#[command(
    name = "compliance_auditor",
    author,
    version,
    about = "Aaroneous compliance auditor: multi-angle diff review with adversarial verification"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Review the diff between two revisions.
    Review {
        /// Repo root (defaults to the current directory).
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Base revision (a ref, SHA, or branch name).
        #[arg(long, default_value = "origin/main")]
        base: String,
        /// Head revision.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Which LLM provider backs the review passes.
        #[arg(long, value_enum, default_value = "mock")]
        provider: ProviderArg,
        /// Model name (provider-specific; ignored for `mock`).
        #[arg(long, default_value = "")]
        model: String,
        /// Local/OpenAI endpoint override.
        #[arg(long)]
        endpoint: Option<String>,
        /// Max highest-churn files pulled into the reviewed diff.
        #[arg(long, default_value_t = 40)]
        max_files: usize,
        /// Max findings kept in the final report after ranking.
        #[arg(long, default_value_t = 10)]
        findings_cap: usize,
        /// Emit the report as JSON instead of the human-readable summary.
        #[arg(long)]
        json: bool,
        /// Write the report to this path in addition to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ProviderArg {
    Mock,
    Gguf,
    Local,
    Openai,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("failed to start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    rt.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> ExitCode {
    let Commands::Review {
        repo,
        base,
        head,
        provider,
        model,
        endpoint,
        max_files,
        findings_cap,
        json,
        output,
    } = cli.command;

    let mut llm_config = LLMConfig {
        provider_type: match provider {
            ProviderArg::Mock => ProviderType::Mock,
            ProviderArg::Gguf => ProviderType::GGUF,
            ProviderArg::Local => ProviderType::Local,
            ProviderArg::Openai => ProviderType::OpenAI,
        },
        ..LLMConfig::default()
    };
    if !model.is_empty() {
        llm_config.model_name = model.clone();
        llm_config.local_model = Some(model);
    }
    if let Some(endpoint) = endpoint {
        llm_config.local_endpoint = Some(endpoint.clone());
        llm_config.base_url = Some(endpoint);
    }

    let client = match LLMClient::new(llm_config).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to initialize LLM client: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    let config = ReviewConfig {
        base,
        head,
        max_files,
        findings_cap,
        ..ReviewConfig::default()
    };

    let report = match compliance_auditor::run_review(&client, &repo, &config).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("compliance review failed: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    if json {
        match report.to_json_pretty() {
            Ok(text) => {
                println!("{text}");
                if let Some(path) = &output {
                    let _ = std::fs::write(path, &text);
                }
            }
            Err(e) => eprintln!("failed to serialize report: {e}"),
        }
    } else {
        report.print_human();
        if let Some(path) = &output {
            if let Ok(text) = report.to_json_pretty() {
                let _ = std::fs::write(path, text);
            }
        }
    }

    if report.has_blocking_findings() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
