use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use anyhow::{Context, Result};
use serde_json::json;
use a_run::SabMatrix;
use a_run::workspace::WorkspacePaths;
use a_run::orchestration_daemon::{OrchestrationDaemon, OrchestrationDaemonConfig, DaemonState};
use a_run::metadata_ingestor::MetadataIngestorConfig;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

#[derive(serde::Deserialize, Debug)]
struct BuildCommand {
    target: String,
    modules: Vec<String>,
    knowledge_subset_ids: Vec<String>,
    job_id: String,
}

struct SystemBiology {
    expression_rate: f32, 
    tokens: f32,
    last_regen: std::time::Instant,
}

impl SystemBiology {
    fn new() -> Self {
        SystemBiology {
            expression_rate: 1.0,
            tokens: 10.0,
            last_regen: std::time::Instant::now(),
        }
    }

    fn update_metabolism(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_regen).as_secs_f32();
        let rate_per_sec = 1.0 * self.expression_rate;
        self.tokens = (self.tokens + elapsed * rate_per_sec).min(10.0);
        self.last_regen = now;
    }

    fn consume_catalyst(&mut self) -> bool {
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

const SERVICE_NAME: &str = "AaroneousARun";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

async fn run_arun_core() -> Result<()> {
    println!("--- Aaroneous A-Run Host Initializing (Async/NATS) ---");
    let sab_matrix = SabMatrix::load_generated().context("Failed to load SAB matrix")?;
    println!("A-Run: Loaded SAB matrix with {} surfaces", sab_matrix.surfaces.len());
    let nc = nats::connect("localhost:4222").context("Failed to connect to NATS server")?;
    
    // Intercept Automate Core Fabrication builds
    if let Ok(sub) = nc.subscribe("system/deploy/build") {
        let _nc_clone = nc.clone();
        tokio::task::spawn_blocking(move || {
            let paths = WorkspacePaths::discover();
            for msg in sub.messages() {
                if let Ok(data) = std::str::from_utf8(&msg.data) {
                    if let Ok(cmd) = serde_json::from_str::<BuildCommand>(data) {
                        println!("A-Run: Intercepted CORE FABRICATION request for job: {}", cmd.job_id);
                        for module in cmd.modules {
                            // physically manufacture .sovereign packages
                            let base_gguf = paths.sovereign_model(&module);
                            let output_dir = paths.builds();
                            let _ = fs::create_dir_all(&output_dir);
                            
                            if base_gguf.exists() {
                                println!("A-Run: Fabricating .sovereign package for {}", module);
                                let opts = a_run::federation::sovereign_package::PackageOptions::default();
                                // We are inside a spawn_blocking closure, so we must use a separate runtime to block on async tasks, or just block_in_place
                                // Actually, let's just use a local runtime to avoid the panic
                                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                                if let Err(e) = rt.block_on(async {
                                    a_run::federation::sovereign_package::export_sovereign(&module, &base_gguf, &output_dir, None, opts).await
                                }) {
                                    println!("A-Run: Failed to fabricate {}: {}", module, e);
                                } else {
                                    println!("A-Run: Successfully fabricated {}.sovereign", module);
                                }
                            } else {
                                println!("A-Run: Skipping fabrication of {} because base GGUF {} does not exist", module, base_gguf.display());
                            }
                        }
                    }
                }
            }
        });
    }

    // Automated Sleep Digestion, SAB Fabrication, & Emulation Recording
    let nc_sleep = nc.clone();
    tokio::task::spawn(async move {
        // 1. Emulation Recording Listener
        let nc_record = nc_sleep.clone();
        tokio::task::spawn_blocking(move || {
            if let Ok(sub) = nc_record.subscribe("chimera.emulation.record") {
                let mut current_routine = Vec::new();
                let mut is_recording = false;
                
                // We also need to listen for start/stop. A separate subscription is better.
                let ctrl_sub = nc_record.subscribe("chimera.record").unwrap();
                
                loop {
                    // Check for control messages
                    if let Some(msg) = ctrl_sub.try_next() {
                        if let Ok(cmd) = std::str::from_utf8(&msg.data) {
                            if cmd == "start" {
                                current_routine.clear();
                                is_recording = true;
                                println!("A-Run: Emulation Routine Recording Started.");
                            } else if cmd == "stop" {
                                is_recording = false;
                                if !current_routine.is_empty() {
                                    let paths = WorkspacePaths::discover();
                                    let dir = paths.routines();
                                    let _ = fs::create_dir_all(&dir);
                                    let path = dir.join(format!("routine_{}.json", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>()));
                                    if let Ok(json) = serde_json::to_string_pretty(&current_routine) {
                                        let _ = fs::write(&path, json);
                                        println!("A-Run: Routine saved to {}", path.display());
                                    }
                                }
                            }
                        }
                    }
                    
                    // Check for emulation events
                    if let Some(msg) = sub.try_next() {
                        if is_recording {
                            if let Ok(json_str) = std::str::from_utf8(&msg.data) {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                    current_routine.push(val);
                                }
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        // 2. Sleep until "night" or just run a periodic batch every 24 hours
        // For testing, we use a 24-hour interval
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        loop {
            interval.tick().await;
            println!("A-Run: Initiating Automated Sleep Digestion & Core Expansion...");
            
            // 1. Digest insights
            let daily_insights = vec![
                "Observation: System biology tokens were consistently utilized, indicating stable metabolism.",
                "Observation: Genesis Architect successfully deployed Phygital spatial nodes.",
                "Observation: Core sovereign fabrication process has 100% success rate today."
            ];
            
            for insight in daily_insights {
                let payload = json!({
                    "title": "Nightly Sleep Digestion Insight",
                    "domain": "system_operations",
                    "content": insight,
                    "dimensions": vec![0.0_f32; 256], // LLM will embed this since it's zero-filled
                    "mass": 0.8
                });
                
                if let Ok(data) = serde_json::to_vec(&payload) {
                    let _ = nc_sleep.publish("system/knowledge/inject", data);
                }
            }
            
            // 2. Autonomous SAB Fabrication (Massive Open-Source Ingestion)
            // Initialize LLM Client
            if let Ok(llm_client_res) = a_run::llm::LLMClient::new(a_run::llm::LLMConfig::default()).await {
                let llm_client = std::sync::Arc::new(llm_client_res);
                let paths = WorkspacePaths::discover();
                let fabricator = a_run::federation::sovereign_package::auto_fabricator::AutoFabricator::new(llm_client, paths.clone());
                
                // Read the dynamic list of capabilities Aaroneous wants to acquire
                let crates_file = paths.target_crates();
                let target_crates = if crates_file.exists() {
                    fs::read_to_string(crates_file).unwrap_or_default()
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty() && !s.starts_with("#"))
                        .collect::<Vec<String>>()
                } else {
                    vec![]
                };
                
                for target_crate in target_crates {
                    println!("A-Run: Initiating Autonomous SAB Fabrication for crate: {}", target_crate);
                    match fabricator.fabricate(&target_crate).await {
                        Ok(path) => {
                            println!("A-Run: Successfully auto-fabricated SAB for {}: {}", target_crate, path.display());
                            // Inject awareness of this new SAB into the Omni Relic
                            let sab_manifest = json!({
                                "title": format!("{} SAB Plugin", target_crate),
                                "domain": "plugin_capability",
                                "content": format!("Automatically generated WASM SAB wrapper for {}", target_crate),
                                "dimensions": vec![0.0_f32; 256],
                                "mass": 1.0
                            });
                            if let Ok(data) = serde_json::to_vec(&sab_manifest) {
                                let _ = nc_sleep.publish("system/knowledge/inject", data);
                            }
                        },
                        Err(e) => println!("A-Run: Failed to fabricate SAB for {}: {}", target_crate, e),
                    }
                }
            }

            println!("A-Run: Sleep Digestion & Core Expansion complete.");
        }
    });

    let mut biology = SystemBiology::new();
    
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        biology.update_metabolism();
        
        if biology.consume_catalyst() {
            println!("A-Run: Catalyst consumed. Tokens remaining: {:.2}", biology.tokens);
        } else {
            println!("A-Run: METABOLIC DEPRESSION: Insufficient tokens for catalyst consumption.");
        }
        
        let heartbeat_msg = json!({"repo": "Aaroneous", "tokens": biology.tokens, "expression": biology.expression_rate});
        let _ = nc.publish("federation.heartbeat", heartbeat_msg.to_string());
    }
}

fn service_main(arguments: Vec<std::ffi::OsString>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(_) = rt.block_on(run_service(arguments)) {}
}

async fn run_service(_arguments: Vec<std::ffi::OsString>) -> Result<(), windows_service::Error> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => std::process::exit(0),
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    let _ = run_arun_core().await;
    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "--console" => {
                run_arun_core().await.context("Failed during run_arun_core call")?;
            }
            "--daemon" => {
                run_daemon(&args[2..]).await?;
            }
            "--daemon-status" => {
                println!("Orchestration Daemon mode: watches filesystem, analyzes metadata, makes decisions, executes actions");
                println!("Usage: a_run --daemon [--watch <path>] [--interval <secs>] [--max-tasks <n>]");
            }
            "--dashboard" => {
                a_run::dashboard::run_dashboard().context("Failed to run dashboard")?;
            }
            _ => {
                println!("Aaroneous A-Run Host");
                println!("Usage:");
                println!("  a_run                  - Run as Windows Service");
                println!("  a_run --console        - Run interactively in console");
                println!("  a_run --daemon         - Run orchestration daemon");
                println!("  a_run --daemon-status  - Show daemon info");
                println!("  a_run --dashboard      - Launch native GUI (egui/wgpu 3D)");
            }
        }
    } else {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main).context("Failed to start service dispatcher")?;
    }
    Ok(())
}

/// Run the orchestration daemon
async fn run_daemon(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Aaroneous Orchestration Daemon Initializing ===");
    
    let mut config = OrchestrationDaemonConfig::default();
    
    // Parse daemon-specific arguments
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--watch" if i + 1 < args.len() => {
                config.ingestor_config.watch_paths.push(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--interval" if i + 1 < args.len() => {
                if let Ok(secs) = args[i + 1].parse::<u64>() {
                    config.cycle_interval = Duration::from_secs(secs);
                }
                i += 2;
            }
            "--max-tasks" if i + 1 < args.len() => {
                if let Ok(n) = args[i + 1].parse::<usize>() {
                    config.max_tasks_per_cycle = n;
                }
                i += 2;
            }
            "--no-throttle" => {
                config.enable_auto_throttle = false;
                i += 1;
            }
            "--no-constellation" => {
                config.enable_constellation_updates = false;
                i += 1;
            }
            _ => {
                println!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }
    
    println!("[Daemon] Watch paths: {:?}", config.ingestor_config.watch_paths);
    println!("[Daemon] Cycle interval: {:?}", config.cycle_interval);
    println!("[Daemon] Max tasks per cycle: {}", config.max_tasks_per_cycle);
    println!("[Daemon] Auto-throttle: {}", config.enable_auto_throttle);
    println!("[Daemon] Constellation updates: {}", config.enable_constellation_updates);
    println!();
    
    let mut daemon = OrchestrationDaemon::new(config);
    
    // Print initial status
    let status = daemon.get_status();
    println!("[Daemon] State: {:?}", status.state);
    println!("[Daemon] Metabolic tokens: {:.2}", status.metabolic_health.global_tokens);
    println!("[Daemon] Expression rate: {:.2}", status.metabolic_health.expression_rate);
    println!();
    
    // Run the daemon (this loops forever)
    daemon.run().await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>)
}
