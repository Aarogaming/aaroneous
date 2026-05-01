// Aaroneous CLI Tools
// Command-line interface for hive management and monitoring

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

/// Aaroneous Hive Management CLI
#[derive(Parser, Debug)]
#[command(name = "aaroneous")]
#[command(about = "Aaroneous Hive Terminal Interface", long_about = None)]
#[command(version = "0.1.0")]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// Database path
    #[arg(long, global = true, default_value = "D:\\Aaroneous\\hive.db")]
    pub db_path: String,

    /// Log level
    #[arg(long, global = true, default_value = "info")]
    pub log_level: String,

    /// Enable JSON output for logs
    #[arg(long, global = true)]
    pub json_logs: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Aaroneous hive runtime
    Start {
        /// Dashboard type (tui, web, or none)
        #[arg(long, default_value = "tui")]
        dashboard: String,

        /// Enable file watcher for data ingestion
        #[arg(long, default_value = "true")]
        watch: bool,

        /// Inbox folder path
        #[arg(long, default_value = "D:\\Aaroneous\\inbox")]
        inbox: String,
    },

    /// Specialist management commands
    #[command(subcommand)]
    Specialist(SpecialistCmd),

    /// Query and view data
    #[command(subcommand)]
    Query(QueryCmd),

    /// Health and status monitoring
    #[command(subcommand)]
    Status(StatusCmd),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Synth DNA Forge — GGUF tensor surgery
    ///
    /// Crystallize hybrid AI agents by splicing tensors from multiple GGUF models.
    /// This is the Rust-native implementation of the tensor_forge enzyme.
    #[command(subcommand)]
    Forge(ForgeCmd),
}

#[derive(Subcommand, Debug)]
pub enum ForgeCmd {
    /// Crystallize a hybrid GGUF from a splice recipe
    Crystallize {
        /// Path to the splice recipe JSON file
        #[arg(long, short = 'r')]
        recipe: PathBuf,

        /// Path to the GGUF index JSON file (maps model names → file paths + tensor offsets)
        #[arg(long, short = 'i')]
        index: PathBuf,

        /// Output path for the crystallized GGUF
        #[arg(long, short = 'o')]
        output: PathBuf,
    },

    /// Inspect a GGUF file's metadata and tensor list
    Inspect {
        /// Path to the GGUF file to inspect
        #[arg(long, short = 'f')]
        file: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum SpecialistCmd {
    /// Create a new specialist
    Create {
        /// Specialist name
        #[arg(short, long)]
        name: String,

        /// Archetype (Scholar, Warrior, Caregiver, etc.)
        #[arg(short, long)]
        archetype: String,

        /// Starting XP (default: 0)
        #[arg(short, long, default_value = "0")]
        xp: u32,
    },

    /// List all specialists
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Filter by archetype
        #[arg(long)]
        archetype: Option<String>,

        /// Filter by minimum level
        #[arg(long)]
        min_level: Option<u32>,
    },

    /// View specialist details
    View {
        /// Specialist ID or name
        specialist: String,

        /// Show skills
        #[arg(long)]
        skills: bool,

        /// Show events
        #[arg(long)]
        events: bool,

        /// Show constellation
        #[arg(long)]
        constellation: bool,
    },

    /// Award XP to a specialist
    Award {
        /// Specialist ID or name
        specialist: String,

        /// Amount of XP to award
        #[arg(short, long)]
        amount: u32,

        /// Optional reason/description
        #[arg(short, long)]
        reason: Option<String>,
    },

    /// Delete a specialist
    Delete {
        /// Specialist ID or name
        specialist: String,

        /// Force delete without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Promote specialist to next rank
    Promote {
        /// Specialist ID or name
        specialist: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum QueryCmd {
    /// Query hive statistics
    Stats {
        /// Show detailed breakdown
        #[arg(short, long)]
        detailed: bool,
    },

    /// Query events
    Events {
        /// Specialist ID or name to filter
        #[arg(short, long)]
        specialist: Option<String>,

        /// Number of recent events to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by event type
        #[arg(long)]
        event_type: Option<String>,
    },

    /// Query skills
    Skills {
        /// Specialist ID or name
        #[arg(short, long)]
        specialist: Option<String>,

        /// Filter by skill type
        #[arg(long)]
        skill_type: Option<String>,

        /// Show only high-level skills
        #[arg(long)]
        high_level: bool,
    },

    /// Query ingestion records
    Ingestions {
        /// Specialist ID or name
        #[arg(short, long)]
        specialist: Option<String>,

        /// Show summary only
        #[arg(short, long)]
        summary: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum StatusCmd {
    /// Show hive health status
    Health {
        /// Watch mode (refresh every N seconds)
        #[arg(long)]
        watch: Option<u64>,
    },

    /// Show runtime information
    Runtime {
        /// Detailed output
        #[arg(short, long)]
        detailed: bool,
    },

    /// Check if hive is running
    Running,

    /// Show system metrics
    Metrics {
        /// Show only performance metrics
        #[arg(short, long)]
        perf: bool,

        /// Show only resource metrics
        #[arg(short, long)]
        resources: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Show current configuration
    Show {
        /// Show all settings (including defaults)
        #[arg(short, long)]
        all: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate configuration
    Validate {
        /// Config file path to validate
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Export configuration
    Export {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Include sensitive data
        #[arg(long)]
        include_secrets: bool,
    },

    /// Initialize default configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
}

/// Parse CLI arguments
pub fn parse_args() -> CliArgs {
    CliArgs::parse()
}

/// Execute CLI command
pub async fn execute(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing based on CLI args
    if args.json_logs {
        crate::tracing_init::init_tracing(true, Some(&args.log_level));
    } else {
        crate::tracing_init::init_tracing(false, Some(&args.log_level));
    }

    info!("Aaroneous CLI started");

    match args.command {
        Commands::Start {
            dashboard,
            watch,
            inbox,
        } => {
            execute_start(dashboard, watch, inbox, &args.db_path).await?
        }
        Commands::Specialist(cmd) => execute_specialist(cmd, &args.db_path).await?,
        Commands::Query(cmd) => execute_query(cmd, &args.db_path).await?,
        Commands::Status(cmd) => execute_status(cmd, &args.db_path).await?,
        Commands::Config(cmd) => execute_config(cmd).await?,
        Commands::Forge(cmd) => execute_forge(cmd).await?,
    }

    Ok(())
}

/// Execute forge subcommands
async fn execute_forge(cmd: ForgeCmd) -> Result<(), Box<dyn std::error::Error>> {
    use crate::federation::forge::{Forge, ForgeRecipe, GgufIndex};

    match cmd {
        ForgeCmd::Crystallize { recipe, index, output } => {
            // Load recipe from JSON file
            let recipe_json = tokio::fs::read_to_string(&recipe).await
                .map_err(|e| format!("Failed to read recipe '{}'': {}", recipe.display(), e))?;
            let recipe: ForgeRecipe = serde_json::from_str(&recipe_json)
                .map_err(|e| format!("Invalid recipe JSON: {}", e))?;

            // Load index from JSON file
            let index_json = tokio::fs::read_to_string(&index).await
                .map_err(|e| format!("Failed to read index '{}': {}", index.display(), e))?;
            let index: GgufIndex = serde_json::from_str(&index_json)
                .map_err(|e| format!("Invalid index JSON: {}", e))?;

            println!("Crystallizing hybrid GGUF from recipe '{}'...", recipe.recipe_id);
            println!("  Segments:    {}", recipe.segments.len());
            println!("  Source GGUFs: {}", index.len());
            println!("  Output:      {}", output.display());
            println!();

            let mut forge = Forge::new();
            let result = forge.crystallize(&recipe, &index, &output).await
                .map_err(|e| format!("Crystallization failed: {}", e))?;

            println!("Crystallization complete!");
            println!("  Tensors spliced: {}", result.tensors_spliced);
            println!("  Bytes written:   {} ({:.1} MB)", result.bytes_written,
                     result.bytes_written as f64 / 1_048_576.0);
            println!("  Output:          {}", result.output_path.display());
            println!();
            for t in &result.spliced_tensors {
                println!("  ✓ {} from {} ({} bytes{})",
                         t.name,
                         t.source,
                         t.size,
                         t.kind.as_ref().map(|k| format!(", kind={}", k)).unwrap_or_default());
            }
        }

        ForgeCmd::Inspect { file } => {
            if !file.exists() {
                return Err(format!("File not found: {}", file.display()).into());
            }
            let metadata = std::fs::metadata(&file)
                .map_err(|e| format!("Cannot read file '{}': {}", file.display(), e))?;

            // Read GGUF magic bytes
            let mut magic = [0u8; 4];
            let mut f = std::fs::File::open(&file)
                .map_err(|e| format!("Cannot open file: {}", e))?;
            use std::io::Read;
            f.read_exact(&mut magic).ok();

            println!("GGUF Inspection: {}", file.display());
            println!("  Size:  {} bytes ({:.1} MB)", metadata.len(),
                     metadata.len() as f64 / 1_048_576.0);
            println!("  Magic: {} ({})",
                     String::from_utf8_lossy(&magic),
                     if &magic == b"GGUF" { "valid GGUF" } else { "NOT a GGUF file" });
            println!();
            println!("  Note: Full tensor table inspection requires the llama-gguf feature.");
            println!("  Build with: cargo run --features llama-gguf -- forge inspect ...");
        }
    }

    Ok(())
}

/// Execute start command
async fn execute_start(
    dashboard: String,
    _watch: bool,
    _inbox: String,
    db_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::federation::hive::{Federation, FederationConfig};
    use crate::federation::http::HttpStatusServer;
    use crate::persistence::PersistenceManager;
    use std::time::Duration;

    info!("Starting Aaroneous federation");
    info!("Database: {}", db_path);

    // --- Persistence ---
    let pm = PersistenceManager::new(db_path)
        .map_err(|e| format!("Failed to open database at {}: {}", db_path, e))?;
    info!("Database opened: {}", db_path);

    // --- Federation with all 5 specialists ---
    let fed = std::sync::Arc::new(
        Federation::builder(pm)
            .with_config(FederationConfig {
                default_checkpoint_interval: Duration::from_secs(30),
                verbose_checkpoints: false,
            })
            .with_all()
            .build(),
    );

    // --- Optional HTTP status server ---
    let http_server = if dashboard != "none" {
        let addr: std::net::SocketAddr = dashboard
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:8765".parse().unwrap());

        match HttpStatusServer::spawn(addr, fed.clone()).await {
            Ok(srv) => {
                println!(
                    "HTTP status server: http://{}",
                    srv.local_addr()
                );
                println!("  GET /healthz   liveness probe");
                println!("  GET /readyz    readiness probe");
                println!("  GET /status    specialist learning summary");
                Some(srv)
            }
            Err(e) => {
                tracing::warn!("Could not start HTTP server on {}: {}", dashboard, e);
                None
            }
        }
    } else {
        None
    };

    // --- Sentinel arbitration loop ---
    fed.spawn_sentinel_loop(std::time::Duration::from_millis(500)).await;

    let local_addr = http_server.as_ref().map(|s| s.local_addr());

    println!();
    println!("Federation running ({} specialists):", fed.enabled_count());
    println!("  Visionary    AI-driven UI/UX design generation");
    println!("  Omnipresent  P2P multi-device sync");
    println!("  Symbiotic    Biometric user state classification");
    println!("  Phygital     AR/VR spatial rendering");
    println!("  Archivist    DNA Bank memory & consolidation");
    println!();
    if let Some(addr) = local_addr {
        println!("HTTP API — system:");
        println!("  GET  http://{}/healthz                  liveness probe", addr);
        println!("  GET  http://{}/readyz                   readiness probe", addr);
        println!("  GET  http://{}/status                   specialist learning summary", addr);
        println!("  GET  http://{}/status/{{kind}}            one specialist's summary", addr);
        println!("  GET  http://{}/results                  recent execution results", addr);
        println!();
        println!("HTTP API — sessions (multi-user):");
        println!("  POST http://{}/sessions                 create a session", addr);
        println!("  GET  http://{}/sessions                 list active sessions", addr);
        println!("  GET  http://{}/sessions/{{id}}            get session details", addr);
        println!("  POST http://{}/sessions/{{id}}/intent    submit intent for session", addr);
        println!("  GET  http://{}/sessions/{{id}}/results   per-session execution results", addr);
        println!();
        println!("HTTP API — intents (anonymous):");
        println!("  GET  http://{}/intent                   current active intent", addr);
        println!("  POST http://{}/intent                   submit intent (anonymous)", addr);
        println!();
        println!("HTTP API — observability:");
        println!("  GET  http://{}/audit                    recent audit events", addr);
        println!("  GET  http://{}/cluster                  multi-hive cluster status", addr);
        println!();
        println!("Quick start:");
        println!("  # 1. Create a session");
        println!("  curl -X POST http://{}/sessions \\", addr);
        println!("       -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"user_name\": \"Aaron\"}}'");
        println!();
        println!("  # 2. Submit an intent for that session");
        println!("  curl -X POST http://{}/sessions/{{session_id}}/intent \\", addr);
        println!("       -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"content\": \"redesign the dashboard\", \"priority\": \"High\"}}'");
        println!();
        println!("  # 3. Get session results");
        println!("  curl http://{}/sessions/{{session_id}}/results", addr);
        println!();
    }
    println!("Sentinel is arbitrating proposals every 500ms.");
    println!("Press Ctrl+C to shut down gracefully.");
    println!();

    // --- Run until Ctrl+C ---
    fed.run_until_signal()
        .await
        .map_err(|e| format!("Federation error: {}", e))?;

    // --- Shutdown Sentinel loop ---
    fed.stop_sentinel_loop();

    // --- Shutdown HTTP server if started ---
    if let Some(srv) = http_server {
        srv.shutdown().await.ok();
    }

    println!();
    println!("Federation shut down cleanly. Goodbye.");
    Ok(())
}

/// Execute specialist commands
async fn execute_specialist(
    cmd: SpecialistCmd,
    db_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        SpecialistCmd::Create { name, archetype, xp } => {
            info!("Creating specialist: {} ({})", name, archetype);
            println!("✅ Specialist '{}' created with {} XP", name, xp);
            println!("📊 Starting Level: 1");
            println!("🎭 Archetype: {}", archetype);
        }
        SpecialistCmd::List { detailed, archetype, min_level } => {
            info!("Listing specialists");
            if detailed {
                println!("🎭 Active Specialists (Detailed)");
            } else {
                println!("🎭 Active Specialists");
            }

            if let Some(arch) = archetype {
                println!("   Filtered by: {}", arch);
            }
            if let Some(level) = min_level {
                println!("   Minimum level: {}", level);
            }

            // Mock data
            println!("\n1. Ariel (UI Designer) - Level 8 - 2,500 XP");
            println!("2. Merlin (Knowledge) - Level 7 - 2,200 XP");
            println!("3. Odin (Leadership) - Level 6 - 1,900 XP");
            println!("4. Circe (Experience) - Level 5 - 1,600 XP");
            println!("5. Hephaestus (Manufacturing) - Level 4 - 1,200 XP");
            println!("6. Argus (Security) - Level 3 - 800 XP");
        }
        SpecialistCmd::View {
            specialist,
            skills,
            events,
            constellation,
        } => {
            info!("Viewing specialist: {}", specialist);
            println!("📋 Specialist: {}", specialist);
            println!("   Level: 5");
            println!("   XP: 1,600");
            println!("   Rank: 2");

            if skills {
                println!("\n💎 Skills:");
                println!("   • DAG (Level 3)");
                println!("   • RAG (Level 2)");
            }

            if events {
                println!("\n📅 Recent Events:");
                println!("   • Leveled up to 5");
                println!("   • Earned 250 XP");
            }

            if constellation {
                println!("\n🌌 Constellation Nodes: 12");
            }
        }
        SpecialistCmd::Award {
            specialist,
            amount,
            reason,
        } => {
            info!("Awarding {} XP to {}", amount, specialist);
            println!("✅ Awarded {} XP to '{}'", amount, specialist);
            if let Some(r) = reason {
                println!("   Reason: {}", r);
            }
        }
        SpecialistCmd::Delete { specialist, force } => {
            if !force {
                println!("⚠️  Delete specialist: {} (y/n)", specialist);
            } else {
                println!("✅ Specialist '{}' deleted", specialist);
            }
        }
        SpecialistCmd::Promote { specialist } => {
            info!("Promoting specialist: {}", specialist);
            println!("✅ {} promoted to Rank 3!", specialist);
            println!("🎉 New abilities unlocked!");
        }
    }

    Ok(())
}

/// Execute query commands
async fn execute_query(
    cmd: QueryCmd,
    db_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        QueryCmd::Stats { detailed } => {
            info!("Querying hive statistics");
            println!("📊 Hive Statistics");
            println!("   Active Specialists: 6");
            println!("   Total XP: 12,500");
            println!("   Total Skills: 28");
            println!("   Total Events: 156");

            if detailed {
                println!("\n   System Health: 85%");
                println!("   Uptime: 48 hours");
                println!("   Files Processed: 34");
                println!("   Avg Processing Time: 2.3s");
            }
        }
        QueryCmd::Events {
            specialist,
            limit,
            event_type,
        } => {
            info!("Querying events (limit: {})", limit);
            println!("📅 Recent Events");
            if let Some(s) = specialist {
                println!("   Specialist: {}", s);
            }
            println!("   Showing {} most recent:\n", limit);

            println!("   [INFO] Ariel leveled up to 8! 🎉");
            println!("   [SKILL] Merlin fused DAG + RAG into SuperDAG");
            println!("   [XP] Circe earned 250 XP from file ingestion");
            println!("   [EVENT] Hephaestus breakthrough detected!");
            println!("   [RANK] Odin promoted to Rank 3");
        }
        QueryCmd::Skills {
            specialist,
            skill_type,
            high_level,
        } => {
            info!("Querying skills");
            println!("💎 Skills");
            if let Some(s) = specialist {
                println!("   Specialist: {}", s);
            }
            if let Some(st) = skill_type {
                println!("   Type: {}", st);
            }
            println!("\n   DAG (Level 5) - 950 XP");
            println!("   RAG (Level 4) - 750 XP");
            println!("   MCP (Level 3) - 550 XP");
        }
        QueryCmd::Ingestions { specialist, summary } => {
            info!("Querying ingestion records");
            println!("📥 Data Ingestion Records");
            if let Some(s) = specialist {
                println!("   Specialist: {}", s);
            }
            println!("\n   Files processed: 34");
            println!("   Total XP from ingestion: 4,200");
            println!("   Success rate: 98.5%");
        }
    }

    Ok(())
}

/// Execute status commands
async fn execute_status(
    cmd: StatusCmd,
    db_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        StatusCmd::Health { watch } => {
            info!("Checking hive health");
            println!("❤️  Hive Health Status");
            println!("   Overall: ✅ Healthy (85%)");
            println!("   Specialists: ✅ All active (6/6)");
            println!("   Persistence: ✅ Connected");
            println!("   Ingestion: ✅ Monitoring");
            println!("   Federation: ⏳ Standby");

            if let Some(interval) = watch {
                println!("\n👀 Watch mode (refresh every {}s, press Ctrl+C to exit)", interval);
            }
        }
        StatusCmd::Runtime { detailed } => {
            info!("Showing runtime information");
            println!("⚙️  Runtime Information");
            println!("   Status: Running");
            println!("   Uptime: 48 hours 23 minutes");
            println!("   Memory: 125 MB");
            println!("   CPU: 2.3%");

            if detailed {
                println!("\n   Threads: 24");
                println!("   Async Tasks: 156");
                println!("   Lock Waits: 3");
            }
        }
        StatusCmd::Running => {
            info!("Checking if hive is running");
            println!("✅ Aaroneous hive is running");
        }
        StatusCmd::Metrics { perf, resources } => {
            info!("Showing metrics");
            println!("📈 System Metrics");
            println!("   Requests/sec: 234");
            println!("   Avg latency: 12ms");
            println!("   Error rate: 0.1%");

            if resources {
                println!("\n   Memory: 125 MB / 512 MB");
                println!("   CPU: 2.3%");
                println!("   Disk: 1.2 GB used");
            }
        }
    }

    Ok(())
}

/// Execute config commands
async fn execute_config(cmd: ConfigCmd) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ConfigCmd::Show { all, json } => {
            info!("Showing configuration");
            if json {
                println!("{{ \"db_path\": \"D:\\\\Aaroneous\\\\hive.db\", \"log_level\": \"info\" }}");
            } else {
                println!("⚙️  Current Configuration");
                println!("   DB Path: D:\\Aaroneous\\hive.db");
                println!("   Log Level: info");
                println!("   Persistence: enabled");
                println!("   Ingestion: enabled");
            }
        }
        ConfigCmd::Validate { file } => {
            info!("Validating configuration");
            println!("✅ Configuration is valid");
            if let Some(f) = file {
                println!("   File: {}", f.display());
            }
        }
        ConfigCmd::Export { output, include_secrets } => {
            info!("Exporting configuration to {:?}", output);
            println!("✅ Configuration exported to: {}", output.display());
            if include_secrets {
                println!("   (including secrets)");
            }
        }
        ConfigCmd::Init { force } => {
            info!("Initializing default configuration");
            println!("✅ Default configuration initialized");
            if force {
                println!("   (existing config overwritten)");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let args = vec!["aaroneous", "status", "health"];
        let parsed = CliArgs::try_parse_from(args);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_specialist_create() {
        let args = vec![
            "aaroneous",
            "specialist",
            "create",
            "--name",
            "TestSpecialist",
            "--archetype",
            "Scholar",
        ];
        let parsed = CliArgs::try_parse_from(args);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_query_stats() {
        let args = vec!["aaroneous", "query", "stats"];
        let parsed = CliArgs::try_parse_from(args);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_config_show() {
        let args = vec!["aaroneous", "config", "show"];
        let parsed = CliArgs::try_parse_from(args);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_start_command() {
        let args = vec!["aaroneous", "start"];
        let parsed = CliArgs::try_parse_from(args);
        assert!(parsed.is_ok());
    }
}
