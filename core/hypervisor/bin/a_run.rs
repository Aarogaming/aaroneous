use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Duration;
use anyhow::{Result, anyhow};
use a_run::AutonomicNervousSystem;
use a_run::enzyme_runner::EnzymeRunner;
use a_run::hox_registry::HoxRegistry;
use a_run::splicing_engine::WasmSplicingEngine;
use a_run::unified_learning::{UnifiedLearningLoop, UnifiedLearningConfig};
use parking_lot::RwLock;
use clap::{Parser, Subcommand};
use uuid::Uuid;
use a_run::mcp_service::{McpService, ServiceConfig, http_api::HttpServer};

#[derive(Parser)]
#[command(name = "a_run")]
#[command(about = "Aaroneous Autonomic Nervous System CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the autonomic nervous system
    Start {
        #[arg(short, long, default_value = "1000")]
        tick: u64,
    },
    /// Inject a task intent
    Inject {
        intent: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install the tracing-subscriber as the very first thing
    // so that subsequent init steps are observable. Idempotent;
    // safe to call from unit tests too.
    let _ = a_run::init_logging();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Start { tick }) => {
            tracing::info!(
                tick_ms = tick,
                "Initializing Aaroneous Autonomic Nervous System"
            );

            let enzyme_runner = Arc::new(EnzymeRunner::new()?);
            let hox_registry = Arc::new(HoxRegistry::new("hox.db")?);
            let workspace_root = std::env::current_dir()?;
            let splicing_engine = Arc::new(WasmSplicingEngine::new(hox_registry.clone(), workspace_root));
            
            let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
                UnifiedLearningConfig::default(),
                0,
                vec![]
            )));

            let ans = AutonomicNervousSystem::new(
                "primary",
                *tick,
                enzyme_runner,
                hox_registry,
                splicing_engine,
                learning_loop,
                Some("hive.db")
            )?;

            println!("System online. Autonomic loop starting...");
            ans.start();

            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
        Some(Commands::Inject { intent }) => {
            println!("Injecting intent: {}", intent);
            let path = PathBuf::from(format!(r"C:\Users\aarog\AppData\Local\Temp\{}.synapse", "primary"));
            
            use memmap2::{MmapOptions};
            use std::fs::OpenOptions;

            let file = OpenOptions::new().read(true).write(true).open(&path)?;
            let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

            let task_id = Uuid::new_v4();
            let id_bytes = task_id.as_bytes();
            
            mmap[16..32].copy_from_slice(id_bytes);
            
            let payload = intent.as_bytes();
            let payload_len = std::cmp::min(payload.len(), 4096);
            mmap[32..32 + payload_len].copy_from_slice(&payload[..payload_len]);
            mmap[32 + payload_len..4128].fill(0);
            
            println!("Intent injected with Task ID: {}", task_id);
            Ok(())
        }
        None => {
            println!("Usage: a_run [COMMAND]");
            Ok(())
        }
    }
}
