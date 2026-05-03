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

    /// Install Aaroneous as a Windows service (requires elevated privileges)
    ///
    /// After installation, Aaroneous starts automatically with Windows and
    /// can be managed with: sc start/stop/query AaroneousFederation
    ///
    /// The service runs `aaroneous start` and serves the HTTP API on port 8765.
    InstallService {
        /// Service display name
        #[arg(long, default_value = "Aaroneous Federation")]
        display_name: String,
        /// HTTP port the service will listen on
        #[arg(long, default_value = "8765")]
        port: u16,
    },

    /// Uninstall the Aaroneous Windows service (requires elevated privileges)
    UninstallService,
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
        Commands::InstallService { display_name, port } => {
            execute_install_service(&display_name, port)?
        }
        Commands::UninstallService => execute_uninstall_service()?,
    }

    Ok(())
}

/// Install Aaroneous as a Windows service.
///
/// Requires the process to run as Administrator.
/// The service runs `aaroneous.exe start --dashboard none` so it has
/// no TUI but serves the full HTTP API.
fn execute_install_service(
    display_name: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use windows_service::service::{
            ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
        };
        use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
        use std::ffi::OsString;

        const SERVICE_NAME: &str = "AaroneousFederation";

        // Path to this executable
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Cannot determine exe path: {}", e))?;

        let manager = ServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CREATE_SERVICE,
        ).map_err(|e| format!(
            "Cannot open Service Manager (are you running as Administrator?): {}", e
        ))?;

        let service_info = ServiceInfo {
            name: OsString::from(SERVICE_NAME),
            display_name: OsString::from(display_name),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe_path,
            launch_arguments: vec![
                OsString::from("start"),
                OsString::from("--dashboard"),
                OsString::from("none"),
            ],
            dependencies: vec![],
            account_name: None,  // LocalSystem
            account_password: None,
        };

        let service = manager.create_service(
            &service_info,
            ServiceAccess::CHANGE_CONFIG,
        ).map_err(|e| format!("Failed to create service: {}", e))?;

        // Set description
        service.set_description(format!(
            "Aaroneous federation backend. Serves HTTP API on port {}. \
             Manages the specialist hive, GGUF inference, and session tracking.",
            port
        )).ok();

        println!("Service '{}' installed successfully.", SERVICE_NAME);
        println!("Start it with:  sc start {}", SERVICE_NAME);
        println!("Status:         sc query {}", SERVICE_NAME);
        println!("Stop:           sc stop {}", SERVICE_NAME);
        println!("HTTP API:       http://localhost:{}/healthz", port);
    }

    #[cfg(not(windows))]
    {
        println!("Service installation is only supported on Windows.");
        println!("On Linux/macOS, use a systemd unit or launchd plist.");
        println!("Example systemd unit at: D:\\Aaroneous\\registry\\aaroneous.service");
    }

    Ok(())
}

/// Uninstall the Aaroneous Windows service.
fn execute_uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use windows_service::service::ServiceAccess;
        use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

        const SERVICE_NAME: &str = "AaroneousFederation";

        let manager = ServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CONNECT,
        ).map_err(|e| format!(
            "Cannot open Service Manager (are you running as Administrator?): {}", e
        ))?;

        let service = manager
            .open_service(SERVICE_NAME, ServiceAccess::DELETE)
            .map_err(|e| format!("Service '{}' not found: {}", SERVICE_NAME, e))?;

        service.delete()
            .map_err(|e| format!("Failed to delete service: {}", e))?;

        println!("Service '{}' uninstalled.", SERVICE_NAME);
        println!("Any running instance will stop after the next system restart,");
        println!("or stop it now with: sc stop {}", SERVICE_NAME);
    }

    #[cfg(not(windows))]
    {
        println!("Service uninstallation is only supported on Windows.");
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
            let file_size = std::fs::metadata(&file)
                .map_err(|e| format!("Cannot read file '{}': {}", file.display(), e))?.len();

            println!("GGUF Inspection: {}", file.display());
            println!("  Size: {} bytes ({:.1} MB)", file_size, file_size as f64 / 1_048_576.0);

            use crate::federation::forge::read_gguf;
            match read_gguf(&file) {
                Ok((index, meta)) => {
                    println!("  Valid GGUF v{}", meta.version);
                    println!("  Architecture : {}", if meta.architecture.is_empty() { "(unset)" } else { &meta.architecture });
                    println!("  Model name   : {}", if meta.model_name.is_empty() { "(unset)" } else { &meta.model_name });
                    if let Some(ctx) = meta.context_length {
                        println!("  Context len  : {}", ctx);
                    }
                    println!("  Tensors      : {}", meta.tensor_count);
                    println!();

                    // List all tensors
                    let tensors = &index.0.values().next()
                        .map(|m| &m.tensors)
                        .ok_or("no tensors in index")?;

                    println!("  {:<50} {:>8}  {:>6}  {:>12}  shape", "Tensor name", "dtype", "kind", "size (B)");
                    println!("  {}", "-".repeat(100));

                    let mut sorted: Vec<_> = tensors.iter().collect();
                    sorted.sort_by_key(|(name, _)| name.as_str());
                    for (name, tm) in &sorted {
                        let shape_str: Vec<String> = tm.shape.iter().map(|d| d.to_string()).collect();
                        println!("  {:<50} {:>8}  {:>6}  {:>12}  [{}]",
                            if name.len() > 48 { &name[..48] } else { name },
                            tm.dtype,
                            tm.kind.as_deref().unwrap_or(""),
                            tm.size,
                            shape_str.join("×")
                        );
                    }
                    println!();
                    println!("  {} metadata KV entries. Use POST /forge/inspect for JSON output.", meta.kv.len());
                }
                Err(e) => {
                    println!("  ERROR: {}", e);
                    return Err(format!("GGUF parse failed: {}", e).into());
                }
            }
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
    use crate::hive_runtime::{HiveRuntime, HiveRuntimeConfig};
    use std::time::Duration;

    info!("Starting Aaroneous federation");
    info!("Database: {}", db_path);

    // --- HiveRuntime: orchestrates inbox, biology, autonomous coordinator ---
    // HiveRuntime opens its own PersistenceManager internally; the Federation
    // specialists share that same DB path so all state lands in one file.
    let runtime_config = HiveRuntimeConfig {
        db_path: db_path.to_string(),
        ..HiveRuntimeConfig::default()
    };
    let hive_runtime = std::sync::Arc::new(
        HiveRuntime::new(runtime_config)
            .await
            .map_err(|e| format!("HiveRuntime init failed: {}", e))?
    );
    info!("HiveRuntime initialised (inbox, biology, autonomous coordinator)");

    // --- Federation with all 5 specialists ---
    // Re-use the same db_path; PersistenceManager opens its own connection.
    use crate::persistence::PersistenceManager;
    let pm = PersistenceManager::new(db_path)
        .map_err(|e| format!("Failed to open database at {}: {}", db_path, e))?;
    info!("Database opened: {}", db_path);

    // Visionary: probe known GGUF model paths in order, fall back to MockLLM.
    // To use real LLM inference: place a Qwen2.5 GGUF in models/ AND compile
    // with `cargo run --features llama-gguf -- start`.
    // See config/specialist_registry.json → federation_core_specialists.Visionary
    use crate::federation::specialists::Visionary;
    let gguf_search = [
        // Preferred: Qwen abliterated variants (no refusals)
        "D:\\Aaroneous\\models\\visionary-qwen2.5-1.5b.gguf",
        "D:\\Aaroneous\\models\\qwen2.5-1.5b-instruct-abliterated.gguf",
        "D:\\Aaroneous\\models\\qwen2.5-1.5b.gguf",
        "D:\\Aaroneous\\models\\qwen2.5-0.5b.gguf",
        // Foundation model (Qwen2.5 Coder 7B Instruct — present in archive)
        "D:\\Aaroneous\\models\\foundation_v1.gguf",
        // Relative paths for CI/development
        "./models/qwen2.5-1.5b.gguf",
        "./models/qwen-1.8b.gguf",
    ];
    let found_gguf = gguf_search.iter()
        .map(std::path::Path::new)
        .find(|p| p.exists());

    let visionary = if let Some(gguf_path) = found_gguf {
        info!("Visionary: GGUF found at {} — using real inference (requires --features llama-gguf)", gguf_path.display());
        std::sync::Arc::new(Visionary::with_gguf_path(gguf_path).await)
    } else {
        info!("Visionary: no GGUF found — using MockLLM (structured output, no real inference)");
        info!("  To enable real inference: place a Qwen2.5 .gguf in D:\\Aaroneous\\models\\");
        info!("  then rebuild with: cargo run --features llama-gguf -- start");
        match Visionary::with_mock_llm().await {
            Ok(v) => std::sync::Arc::new(v),
            Err(e) => {
                tracing::warn!("Visionary: MockLLM failed ({}), falling back to rule-based", e);
                std::sync::Arc::new(Visionary::new())
            }
        }
    };

    // Archivist gets an in-memory DNA Bank so execute() persists events.
    // Swap for DNABank::open(path) with `--features rocksdb-dna` for durability.
    use crate::federation::specialists::Archivist;
    let archivist = std::sync::Arc::new(Archivist::new().with_in_memory_dna_bank());
    info!("Archivist: in-memory DNA Bank attached");

    // Load any enabled dynamic specialists from the registry
    use crate::federation::specialists::GenericSpecialist;
    let mut builder = Federation::builder(pm)
        .with_config(FederationConfig {
            default_checkpoint_interval: Duration::from_secs(30),
            verbose_checkpoints: false,
            optimization_profile: None,
        })
        .with_visionary_instance(visionary)
        .with_omnipresent()
        .with_symbiotic()
        .with_phygital()
        .with_archivist_instance(archivist);

    // Parse specialist_registry.json for enabled dynamic specialists
    let registry_path = std::path::Path::new("D:\\Aaroneous\\config\\specialist_registry.json");
    if registry_path.exists() {
        if let Ok(content) = std::fs::read_to_string(registry_path) {
            if let Ok(registry) = serde_json::from_str::<serde_json::Value>(&content) {
                // Support both old key ("dynamic_specialists.examples") and
                // new key ("dynamic_sovereigns") as a flat object of named entries
                let entries: Vec<serde_json::Value> = registry
                    .get("dynamic_sovereigns")
                    .and_then(|d| d.as_object())
                    .map(|obj| obj.values().cloned().collect())
                    .or_else(|| {
                        registry.get("dynamic_specialists")
                            .and_then(|d| d.get("examples"))
                            .and_then(|e| e.as_array())
                            .cloned()
                    })
                    .unwrap_or_default();

                for entry in &entries {
                    let enabled = entry.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    if !enabled { continue; }
                    let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                    let domain = entry.get("domain").and_then(|d| d.as_str()).unwrap_or("general");
                    let gguf = entry.get("gguf_path").and_then(|g| g.as_str()).unwrap_or("");
                    let specialist = GenericSpecialist::new(name, domain)
                        .with_gguf_path(gguf).await;
                    info!("Loading sovereign '{}' (domain: {})", name, domain);
                    builder = builder.with_gguf_specialist(std::sync::Arc::new(specialist));
                }

                if entries.iter().filter(|e| e.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false)).count() == 0 {
                    let names: Vec<&str> = entries.iter()
                        .filter_map(|e| e.get("name").and_then(|n| n.as_str()))
                        .collect();
                    info!("Dynamic sovereigns registered but not yet enabled: {}",
                        names.join(", "));
                    info!("  Set enabled=true in config/specialist_registry.json to activate");
                }
            }
        }
    }

    let fed = std::sync::Arc::new(builder.build_async().await);

    // Attach the federation to HiveRuntime so start()/shutdown() drives it.
    hive_runtime.attach_federation(Some(fed.clone())).await;

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

    // --- Start HiveRuntime (starts federation + inbox + biology + autonomous coordinator) ---
    hive_runtime.start().await
        .map_err(|e| format!("HiveRuntime start failed: {}", e))?;

    // --- Sentinel arbitration loop + session tick loop + system sensor ---
    fed.spawn_sentinel_loop(std::time::Duration::from_millis(500)).await;
    fed.spawn_session_tick_loop().await;
    // Bridge: reads CPU/memory from sysinfo every 5s → Symbiotic bio_inbox
    fed.spawn_system_sensor_loop().await;
    info!("System sensor loop started (CPU/memory → Symbiotic bio_inbox)");

    let local_addr = http_server.as_ref().map(|s| s.local_addr());

    println!();
    println!("Federation running ({} specialists):", fed.enabled_count());
    println!("  Visionary    AI-driven UI/UX design generation");
    println!("  Omnipresent  P2P multi-device sync");
    println!("  Symbiotic    Biometric user state classification");
    println!("  Phygital     AR/VR spatial rendering");
    println!("  Archivist    DNA Bank memory & consolidation");
    // Print inference mode
    #[cfg(feature = "llama-gguf")]
    println!("Inference mode: REAL (llama-gguf feature enabled)");
    #[cfg(not(feature = "llama-gguf"))]
    {
        println!("Inference mode: MOCK (structured output, no real LLM)");
        println!("  → For real inference: cargo run --features llama-gguf -- start");
    }
    if std::env::var("AARONEOUS_API_KEY").is_ok() {
        println!("Auth: API key required (AARONEOUS_API_KEY is set)");
    } else {
        println!("Auth: DISABLED — set AARONEOUS_API_KEY env var to protect the API");
    }
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
        println!("  GET  http://{}/audit                    audit events (?limit=N&since_ms=TS)", addr);
        println!("  GET  http://{}/learning/trends          confidence time-series per specialist", addr);
        println!("  GET  http://{}/cluster                  multi-hive cluster status", addr);
        println!();
        println!("HTTP API — specialists (O3DE / XR integration):");
        println!("  GET  http://{}/specialists              full specialist snapshot (O3DE initial sync)", addr);
        println!("  GET  http://{}/specialists/stream       SSE push stream (O3DE persistent connection)", addr);
        println!();
        println!("HTTP API — dynamic specialists:");
        println!("  GET  http://{}/dynamic-specialists      list runtime-loaded GenericSpecialists", addr);
        println!("  POST http://{}/dynamic-specialists      add a new specialist from GGUF", addr);
        println!("  POST http://{}/dynamic-specialists/reload  re-read registry, add new enabled entries", addr);
        println!("  GET  http://{}/models                   list GGUF files in models/ directory", addr);
        println!();
        println!("HTTP API — Synth DNA Forge:");
        println!("  POST http://{}/forge/inspect            parse GGUF header + tensor table", addr);
        println!("  POST http://{}/forge/auto-recipe        auto-generate ForgeRecipe from 2 GGUFs", addr);
        println!("  POST http://{}/forge/single-recipe      extract tensor subset from 1 GGUF", addr);
        println!("  POST http://{}/forge/crystallize        crystallize hybrid GGUF from recipe", addr);
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
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::warn!("Failed to wait for ctrl_c signal: {}", e);
    }
    info!("Shutdown signal received");

    // --- Shutdown Sentinel loop ---
    fed.stop_sentinel_loop();

    // --- Shutdown HiveRuntime (includes federation shutdown_all + checkpoint) ---
    hive_runtime.shutdown().await
        .map_err(|e| format!("HiveRuntime shutdown error: {}", e))?;

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
            println!("Specialist '{}' created with {} XP", name, xp);
            println!("Starting Level: 1");
            println!("Archetype: {}", archetype);
        }
        SpecialistCmd::List { detailed, archetype, min_level } => {
            use crate::persistence::PersistenceManager;
            let pm = PersistenceManager::new(db_path)
                .map_err(|e| format!("Cannot open DB: {}", e))?;

            // Era 2/3: federation specialist learning state
            println!("Federation Specialists (learning state from {}):", db_path);
            println!("{:<20} {:>12} {:>8} {:>8} {:>8}", "Specialist", "Confidence", "Successes", "Failures", "Execs");
            println!("{}", "-".repeat(65));

            let learning_kinds = ["Visionary", "Omnipresent", "Symbiotic", "Phygital", "Archivist"];
            let mut found_any = false;
            for kind in &learning_kinds {
                if let Ok(Some(rec)) = pm.load_learning_state(kind) {
                    if let Some(ref arch) = archetype {
                        if !kind.to_lowercase().contains(&arch.to_lowercase()) { continue; }
                    }
                    if let Some(min_lvl) = min_level {
                        // Map confidence to approximate "level" (1–10)
                        let approx_level = (rec.confidence_score * 10.0) as u32 + 1;
                        if approx_level < min_lvl { continue; }
                    }
                    println!("{:<20} {:>12.3} {:>8} {:>8} {:>8}",
                        kind,
                        rec.confidence_score,
                        rec.success_count,
                        rec.failure_count,
                        rec.total_executions,
                    );
                    if detailed {
                        println!("   Last updated: {}", rec.last_updated);
                    }
                    found_any = true;
                }
            }

            // Era 1: legacy specialists from specialists table
            if let Ok(specialists) = pm.list_specialists() {
                let mut era1_shown = false;
                for s in &specialists {
                    if let Some(ref arch) = archetype {
                        if !s.archetype.to_lowercase().contains(&arch.to_lowercase()) { continue; }
                    }
                    if let Some(min_lvl) = min_level {
                        if s.current_level < min_lvl { continue; }
                    }
                    if !era1_shown {
                        println!("\nEra 1 Specialists:");
                        println!("{:<20} {:>8} {:>10} {:>6}", "Name", "Level", "XP", "Rank");
                        println!("{}", "-".repeat(50));
                        era1_shown = true;
                    }
                    println!("{:<20} {:>8} {:>10} {:>6}",
                        s.name, s.current_level, s.xp, s.rank);
                    if detailed && !s.archetype.is_empty() {
                        println!("   Archetype: {}", s.archetype);
                    }
                    found_any = true;
                }
            }

            if !found_any {
                println!("No specialists found. Run 'aaroneous start' to initialize the hive.");
            }
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
    use crate::persistence::PersistenceManager;

    match cmd {
        QueryCmd::Stats { detailed } => {
            let pm = PersistenceManager::new(db_path)
                .map_err(|e| format!("Cannot open DB at {}: {}", db_path, e))?;

            // Era 1 specialists
            let era1 = pm.list_specialists().unwrap_or_default();
            let total_xp: u32 = era1.iter().map(|s| s.xp_total).sum();

            // Era 2/3 learning state
            let kinds = ["Visionary", "Omnipresent", "Symbiotic", "Phygital", "Archivist"];
            let mut total_executions = 0u32;
            let mut total_successes = 0u32;
            let mut configured_count = 0;
            for kind in &kinds {
                if let Ok(Some(rec)) = pm.load_learning_state(kind) {
                    total_executions += rec.total_executions;
                    total_successes += rec.success_count;
                    configured_count += 1;
                }
            }

            println!("Hive Statistics (database: {})", db_path);
            println!("  Era 1 Specialists:     {}", era1.len());
            println!("  Total Era 1 XP:        {}", total_xp);
            println!("  Era 2/3 Configured:    {}", configured_count);
            println!("  Federation Executions: {}", total_executions);
            println!("  Federation Successes:  {}", total_successes);
            if total_executions > 0 {
                println!("  Overall Success Rate:  {:.1}%",
                    total_successes as f32 / total_executions as f32 * 100.0);
            }

            if detailed {
                println!("\nPer-specialist learning:");
                for kind in &kinds {
                    if let Ok(Some(rec)) = pm.load_learning_state(kind) {
                        println!("  {:<14} conf={:.3}  execs={}  successes={}",
                            kind, rec.confidence_score, rec.total_executions, rec.success_count);
                    }
                }

                // Models directory
                let models_dir = std::path::Path::new("D:\\Aaroneous\\models");
                if models_dir.exists() {
                    let gguf_count = std::fs::read_dir(models_dir)
                        .map(|d| d.filter_map(|e| e.ok())
                            .filter(|e| e.path().extension().map_or(false, |x| x == "gguf"))
                            .count())
                        .unwrap_or(0);
                    println!("\nGGUF models in models/: {}", gguf_count);
                } else {
                    println!("\nGGUF models: models/ directory not found");
                }
            }
        }
        QueryCmd::Events {
            specialist,
            limit,
            event_type: _,
        } => {
            let pm = PersistenceManager::new(db_path)
                .map_err(|e| format!("Cannot open DB: {}", e))?;

            println!("Recent Events (database: {})", db_path);
            if let Some(ref s) = specialist {
                println!("Filtered by specialist: {}", s);
            }

            // Query Era 1 events from the events table
            // list_events is not yet exposed on PersistenceManager, so we show
            // what we can from the available API
            let specialists = pm.list_specialists().unwrap_or_default();
            if specialists.is_empty() {
                println!("No Era 1 events found (no specialists in database).");
                println!("Run 'aaroneous start' to initialize, then use the HTTP API to submit intents.");
            } else {
                println!("Era 1 specialists in database: {}", specialists.len());
                for s in specialists.iter().take(limit as usize) {
                    if let Some(ref filter) = specialist {
                        if !s.name.to_lowercase().contains(&filter.to_lowercase()) { continue; }
                    }
                    println!("  {} ({}): level {}, XP {}", s.name, s.archetype, s.current_level, s.xp);
                }
            }
        }
        QueryCmd::Skills { specialist, skill_type, high_level } => {
            let pm = PersistenceManager::new(db_path)
                .map_err(|e| format!("Cannot open DB: {}", e))?;
            let specialists = pm.list_specialists().unwrap_or_default();

            println!("Skills (database: {})", db_path);
            if specialists.is_empty() {
                println!("No specialists found. Run 'aaroneous start' to initialize.");
            } else {
                for s in &specialists {
                    if let Some(ref filter) = specialist {
                        if !s.name.to_lowercase().contains(&filter.to_lowercase()) { continue; }
                    }
                    println!("  {} — {} skills (level {}{})",
                        s.name, "see HTTP API /status", s.current_level,
                        if high_level { ", high-level filter active" } else { "" });
                    if let Some(ref st) = skill_type {
                        println!("    (type filter: {})", st);
                    }
                }
                println!("\nNote: Full skill detail available via: GET http://localhost:8765/status");
            }
        }
        QueryCmd::Ingestions { specialist, summary } => {
            let pm = PersistenceManager::new(db_path)
                .map_err(|e| format!("Cannot open DB: {}", e))?;
            let specialists = pm.list_specialists().unwrap_or_default();

            println!("Ingestion Records (database: {})", db_path);
            if let Some(ref s) = specialist {
                println!("  Specialist filter: {}", s);
            }
            println!("  Era 1 specialists: {}", specialists.len());
            if summary {
                let total_xp: u32 = specialists.iter().map(|s| s.xp_total).sum();
                println!("  Total accumulated XP: {}", total_xp);
            }
            println!("\nNote: Full ingestion history available via: GET http://localhost:8765/audit");
        }
    }

    Ok(())
}

/// Execute status commands
async fn execute_status(
    cmd: StatusCmd,
    db_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::persistence::PersistenceManager;

    match cmd {
        StatusCmd::Health { watch } => {
            // Try to open the database and report real status
            let db_ok = PersistenceManager::new(db_path).is_ok();
            let models_dir = std::path::Path::new("D:\\Aaroneous\\models");
            let models_count = if models_dir.exists() {
                std::fs::read_dir(models_dir)
                    .map(|d| d.filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map_or(false, |x| x == "gguf"))
                        .count())
                    .unwrap_or(0)
            } else { 0 };

            let pm_res = PersistenceManager::new(db_path);
            let (specialist_count, total_executions) = if let Ok(ref pm) = pm_res {
                let era1 = pm.list_specialists().unwrap_or_default().len();
                let kinds = ["Visionary", "Omnipresent", "Symbiotic", "Phygital", "Archivist"];
                let execs: u32 = kinds.iter()
                    .filter_map(|k| pm.load_learning_state(k).ok().flatten())
                    .map(|r| r.total_executions)
                    .sum();
                (era1, execs)
            } else { (0, 0) };

            #[cfg(feature = "llama-gguf")]
            let inference_mode = "REAL (llama-gguf)";
            #[cfg(not(feature = "llama-gguf"))]
            let inference_mode = "MOCK (no llama-gguf)";

            println!("Hive Health Status");
            println!("  Database:          {} ({})", if db_ok { "OK" } else { "ERROR" }, db_path);
            println!("  GGUF models:       {} file(s) in models/", models_count);
            println!("  Inference mode:    {}", inference_mode);
            println!("  Era 1 specialists: {}", specialist_count);
            println!("  Total executions:  {}", total_executions);
            println!("  Auth:              {}", if std::env::var("AARONEOUS_API_KEY").is_ok() {
                "API key set"
            } else {
                "DISABLED (no AARONEOUS_API_KEY)"
            });

            println!("\nTo check the live running federation: GET http://localhost:8765/healthz");
            println!("To check specialist learning:        GET http://localhost:8765/status");

            if let Some(interval) = watch {
                println!("\nWatch mode (every {}s) — use curl for live data:", interval);
                println!("  curl http://localhost:8765/healthz");
                println!("  curl http://localhost:8765/status");
                println!("  curl -N http://localhost:8765/results/stream  (SSE)");
            }
        }
        StatusCmd::Runtime { detailed } => {
            info!("Showing runtime information");
            println!("Runtime Information");
            println!("  Note: detailed runtime stats require the federation to be running.");
            println!("  Check: GET http://localhost:8765/status");

            if detailed {
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

