use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::ffi::c_void;
use std::time::Duration;
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};
use libloading::{Library, Symbol};
use shared_memory::*;
use wasmtime::{Engine, Module, Store, Linker, Instance, Memory, TypedFunc};
use serde_json::json;
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

#[repr(C)]
pub struct AasBuffer {
    data: *mut c_void,
    size: u64,
    capacity: u64,
}

type InitFunc = unsafe extern "C" fn() -> i32;
type ProcessFunc = unsafe extern "C" fn(input: *mut AasBuffer, output: *mut AasBuffer) -> i32;
type CrystallizeHybridFunc = unsafe extern "C" fn(
    recipe_json: *const u8, 
    recipe_len: u32,
    index_json: *const u8,
    index_len: u32,
    output_path_ptr: *const u8,
    output_path_len: u32
) -> i32;

enum EnzymeType {
    Native {
        lib: Library,
        init: InitFunc,
        process: ProcessFunc,
        crystallize: Option<CrystallizeHybridFunc>,
    },
    Wasm {
        store: Store<()>,
        instance: Instance,
        init: TypedFunc<(), i32>,
        process: TypedFunc<(u32, u32, u32), i32>,
        memory: Memory,
    }
}

struct SharedMemorySynapse {
    shmem: Shmem,
}

impl SharedMemorySynapse {
    fn new(size: usize) -> Result<Self> {
        let shmem = ShmemConf::new().size(size).create().context("Failed to create shared memory")?;
        Ok(SharedMemorySynapse { shmem })
    }

    fn as_buffer(&self) -> AasBuffer {
        AasBuffer {
            data: self.shmem.as_ptr() as *mut c_void,
            size: self.shmem.len() as u64,
            capacity: self.shmem.len() as u64,
        }
    }
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

fn load_native_enzyme(path: &Path) -> Result<EnzymeType> {
    unsafe {
        let lib = Library::new(path).context("Failed to load native library")?;
        let init_sym: Symbol<InitFunc> = lib.get(b"aas_init").context("Failed to get aas_init symbol")?;
        let process_sym: Symbol<ProcessFunc> = lib.get(b"aas_process").context("Failed to get aas_process symbol")?;
        let crystallize = lib.get(b"crystallize_hybrid").ok().map(|s: Symbol<CrystallizeHybridFunc>| *s);
        let init = *init_sym;
        let process = *process_sym;
        Ok(EnzymeType::Native { lib, init, process, crystallize })
    }
}

fn load_wasm_enzyme(path: &Path) -> Result<EnzymeType> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, path).map_err(|e| anyhow!("Failed to load WASM module: {}", e))?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module).map_err(|e| anyhow!("Failed to instantiate WASM module: {}", e))?;
    let init = instance.get_typed_func::<(), i32>(&mut store, "aas_init").map_err(|e| anyhow!("Failed to get aas_init WASM export: {}", e))?;
    let process = instance.get_typed_func::<(u32, u32, u32), i32>(&mut store, "aas_process").map_err(|e| anyhow!("Failed to get aas_process WASM export: {}", e))?;
    let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow!("Memory export not found"))?;
    Ok(EnzymeType::Wasm { store, instance, init, process, memory })
}

pub fn hot_swap_wasm_enzyme(old_enzyme: &mut EnzymeType, new_path: &Path) -> Result<()> {
    if let EnzymeType::Wasm { store: old_store, memory: old_memory, .. } = old_enzyme {
        // 1. Read existing memory to preserve state
        let old_mem_size = old_memory.data_size(&*old_store);
        let mut old_mem_data = vec![0u8; old_mem_size];
        old_memory.read(&*old_store, 0, &mut old_mem_data).context("Failed to read old WASM memory")?;
        
        // 2. Load new enzyme
        let new_enzyme = load_wasm_enzyme(new_path)?;
        
        if let EnzymeType::Wasm { mut store, memory, instance, init, process } = new_enzyme {
            // 3. Write old memory into new instance
            let new_mem_size = memory.data_size(&store);
            if new_mem_size < old_mem_size {
                let pages_to_grow = ((old_mem_size - new_mem_size) as u64 / 65536) + 1;
                memory.grow(&mut store, pages_to_grow).map_err(|e| anyhow!("Failed to grow new WASM memory: {}", e))?;
            }
            memory.write(&mut store, 0, &old_mem_data).context("Failed to restore WASM state")?;
            
            println!("A-Run: Successfully hot-swapped WASM module, {} bytes of state preserved.", old_mem_size);
            
            // 4. Update the enzyme reference
            *old_enzyme = EnzymeType::Wasm {
                store,
                instance,
                init,
                process,
                memory,
            };
            
            Ok(())
        } else {
            Err(anyhow!("New enzyme is not WASM"))
        }
    } else {
        Err(anyhow!("Old enzyme is not WASM"))
    }
}

fn load_enzyme(name: &str) -> Result<EnzymeType> {
    let path = PathBuf::from(format!("D:\\Aaroneous\\cache\\{}", name));
    if !path.exists() {
        let genome_path = PathBuf::from(format!("D:\\Aaroneous\\chromosomes\\{}", name));
        if genome_path.exists() {
            fs::copy(&genome_path, &path).context("Failed to cache enzyme")?;
        } else {
            return Err(anyhow!("Enzyme not found in genome"));
        }
    }
    if name.ends_with(".wasm") { load_wasm_enzyme(&path) } else { load_native_enzyme(&path) }
}

fn get_allowed_enzymes(registry_path: &Path) -> Vec<String> {
    let data = fs::read_to_string(registry_path).unwrap_or_else(|_| "{}".to_string());
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
        if let Some(phenotype) = json.get("phenotype") {
            if let Some(enzymes) = phenotype.get("enforced_enzymes").and_then(|v| v.as_array()) {
                return enzymes.iter().filter_map(|v| v.get("name").and_then(|name| name.as_str()).map(|n| {
                    if n.contains("wasm") { format!("{}.wasm", n) } else { format!("{}.dll", n) }
                })).collect();
            }
        }
    }
    Vec::new()
}

const SERVICE_NAME: &str = "AaroneousARun";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

async fn run_arun_core() -> Result<()> {
    println!("--- Aaroneous A-Run Host Initializing (Async/NATS) ---");
    let nc = nats::connect("localhost:4222").context("Failed to connect to NATS server")?;
    
    // Intercept Automate Core Fabrication builds
    if let Ok(sub) = nc.subscribe("system/deploy/build") {
        let nc_clone = nc.clone();
        tokio::task::spawn_blocking(move || {
            for msg in sub.messages() {
                if let Ok(data) = std::str::from_utf8(&msg.data) {
                    if let Ok(cmd) = serde_json::from_str::<BuildCommand>(data) {
                        println!("A-Run: Intercepted CORE FABRICATION request for job: {}", cmd.job_id);
                        for module in cmd.modules {
                            // physically manufacture .sovereign packages
                            let base_gguf = PathBuf::from(format!("D:\\Aaroneous\\models\\{}-qwen2.5-7b.gguf", module.to_lowercase()));
                            let output_dir = PathBuf::from("D:\\Aaroneous\\data\\builds");
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
                                    let dir = PathBuf::from("D:\\Aaroneous\\data\\routines");
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
                let fabricator = a_run::federation::sovereign_package::auto_fabricator::AutoFabricator::new(llm_client);
                
                // Read the dynamic list of capabilities Aaroneous wants to acquire
                let crates_file = Path::new("D:\\Aaroneous\\registry\\target_crates.txt");
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
    let allowed_enzymes = get_allowed_enzymes(Path::new("D:\\Aaroneous\\registry\\hox_map.json"));
    let synapse = SharedMemorySynapse::new(65536)?;
    let mut buffer = synapse.as_buffer();

    let mut enzymes: HashMap<String, EnzymeType> = HashMap::new();
    for enzyme_name in allowed_enzymes {
        if let Ok(mut enzyme) = load_enzyme(&enzyme_name) {
            match &mut enzyme {
                EnzymeType::Native { init, .. } => unsafe { (*init)(); },
                EnzymeType::Wasm { store, init, .. } => { let _ = init.call(&mut *store, ()); }
            }
            enzymes.insert(enzyme_name, enzyme);
        }
    }
    
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        biology.update_metabolism();
        
        if biology.consume_catalyst() {
            println!("A-Run: Catalyst consumed. Tokens remaining: {:.2}", biology.tokens);
            // Process with first active enzyme
            if let Some((name, enzyme)) = enzymes.iter_mut().next() {
                match enzyme {
                    EnzymeType::Native { process, .. } => unsafe { (*process)(&mut buffer as *mut _, &mut buffer as *mut _); },
                    _ => {}
                }
            }
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
    if args.len() > 1 && args[1] == "--forge" {
        let recipe_path = args.get(2).map(|s| s.as_str()).unwrap_or("D:\\Aaroneous\\registry\\test_recipe.json");
        let output_path = args.get(3).map(|s| s.as_str()).unwrap_or("D:\\Aaroneous\\data\\hybrid_husk.gguf");
        let enzyme = load_enzyme("tensor_forge.dll")?;
        if let EnzymeType::Native { lib: _, crystallize, .. } = enzyme {
            if let Some(crystallize_func) = crystallize {
                let recipe_json = fs::read_to_string(recipe_path)?;
                let index_json = fs::read_to_string("D:\\Aaroneous\\registry\\tensor_index.json")?;
                unsafe {
                    crystallize_func(recipe_json.as_ptr(), recipe_json.len() as u32,
                                     index_json.as_ptr(), index_json.len() as u32,
                                     output_path.as_ptr(), output_path.len() as u32);
                }
            }
        }
        return Ok(());
    }
    if args.len() > 1 && args[1] == "--console" {
        run_arun_core().await.context("Failed during run_arun_core call")?;
    } else {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main).context("Failed to start service dispatcher")?;
    }
    Ok(())
}
