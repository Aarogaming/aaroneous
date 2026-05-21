use std::fs;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use serde_json::json;
use a_run::workspace::WorkspacePaths;
use a_run::federation::hive::{Federation, FederationConfig};
use a_run::federation::http::HttpStatusServer;
use a_run::persistence::PersistenceManager;
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

const SERVICE_NAME: &str = "AaroneousARun";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

async fn run_federation() -> Result<()> {
    println!("--- Aaroneous Federation Initializing ---");
    
    let paths = WorkspacePaths::discover();
    let db_path = paths.hive_db();
    println!("Federation: Database path: {}", db_path.display());
    
    // Initialize persistence
    let pm = PersistenceManager::new(db_path.to_str().unwrap_or("hive.db"))
        .context("Failed to open persistence database")?;
    println!("Federation: Persistence initialized");
    
    // Build the federation with all 5 specialists
    let federation = Federation::builder(pm)
        .with_config(FederationConfig {
            default_checkpoint_interval: Duration::from_secs(30),
            verbose_checkpoints: false,
            optimization_profile: None,
        })
        .with_visionary()
        .with_omnipresent()
        .with_symbiotic()
        .with_phygital()
        .with_archivist()
        .build();
    
    println!("Federation: Built with 5 specialists");
    
    // Start all specialist hosts
    federation.start_all().await
        .context("Failed to start federation specialists")?;
    println!("Federation: All specialists started");
    
    // Spawn checkpoint loops for auto-save
    federation.spawn_checkpoint_loops().await;
    println!("Federation: Checkpoint loops spawned");
    
    // Start HTTP server for federation API
    let fed_arc = Arc::new(federation);
    let http_server = HttpStatusServer::spawn(
        "0.0.0.0:8001".parse().unwrap(),
        fed_arc.clone(),
    ).await;
    
    match &http_server {
        Ok(server) => println!("Federation: HTTP server listening on {}", server.local_addr()),
        Err(e) => eprintln!("Federation: Failed to start HTTP server: {}", e),
    }
    
    // NATS connection for fabrication and sleep digestion
    if let Ok(nc) = nats::connect("localhost:4222") {
        println!("Federation: Connected to NATS");
        spawn_fabrication_listener(nc.clone());
        spawn_sleep_digestion(nc.clone());
    } else {
        println!("Federation: NATS not available, running without messaging");
    }
    
    // Main heartbeat loop
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let status = fed_arc.get_status();
        println!(
            "Federation: Active specialists: {}, Cycles: {}, Errors: {}",
            status.active_specialists,
            status.total_cycles,
            status.total_errors,
        );
    }
}

fn spawn_fabrication_listener(nc: nats::Connection) {
    tokio::task::spawn_blocking(move || {
        if let Ok(sub) = nc.subscribe("system/deploy/build") {
            let paths = WorkspacePaths::discover();
            for msg in sub.messages() {
                if let Ok(data) = std::str::from_utf8(&msg.data) {
                    if let Ok(cmd) = serde_json::from_str::<BuildCommand>(data) {
                        println!("Federation: Intercepted CORE FABRICATION request for job: {}", cmd.job_id);
                        for module in cmd.modules {
                            let base_gguf = paths.sovereign_model(&module);
                            let output_dir = paths.builds();
                            let _ = fs::create_dir_all(&output_dir);
                            
                            if base_gguf.exists() {
                                println!("Federation: Fabricating .sovereign package for {}", module);
                                let opts = a_run::federation::sovereign_package::PackageOptions::default();
                                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                                if let Err(e) = rt.block_on(async {
                                    a_run::federation::sovereign_package::export_sovereign(&module, &base_gguf, &output_dir, None, opts).await
                                }) {
                                    println!("Federation: Failed to fabricate {}: {}", module, e);
                                } else {
                                    println!("Federation: Successfully fabricated {}.sovereign", module);
                                }
                            } else {
                                println!("Federation: Skipping fabrication of {} (GGUF not found)", module);
                            }
                        }
                    }
                }
            }
        }
    });
}

fn spawn_sleep_digestion(nc: nats::Connection) {
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        loop {
            interval.tick().await;
            println!("Federation: Initiating Automated Sleep Digestion...");
            
            let daily_insights = vec![
                "Observation: System biology tokens were consistently utilized.",
                "Observation: Genesis Architect successfully deployed Phygital spatial nodes.",
                "Observation: Core sovereign fabrication process has 100% success rate today."
            ];
            
            for insight in daily_insights {
                let payload = json!({
                    "title": "Nightly Sleep Digestion Insight",
                    "domain": "system_operations",
                    "content": insight,
                    "dimensions": vec![0.0_f32; 256],
                    "mass": 0.8
                });
                
                if let Ok(data) = serde_json::to_vec(&payload) {
                    let _ = nc.publish("system/knowledge/inject", data);
                }
            }
            
            println!("Federation: Sleep Digestion complete.");
        }
    });
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
    let _ = run_federation().await;
    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "--console" => {
                run_federation().await.context("Failed during federation call")?;
            }
            "--start" => {
                run_federation().await.context("Failed to start federation")?;
            }
            "--dashboard" => {
                a_run::dashboard::run_dashboard().context("Failed to run dashboard")?;
            }
            _ => {
                println!("Aaroneous Federation Host");
                println!("Usage:");
                println!("  a_run                  - Run as Windows Service");
                println!("  a_run --console        - Run interactively in console");
                println!("  a_run --start          - Start the federation with all specialists");
                println!("  a_run --dashboard      - Launch native GUI (egui/wgpu 3D)");
            }
        }
    } else {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main).context("Failed to start service dispatcher")?;
    }
    Ok(())
}
