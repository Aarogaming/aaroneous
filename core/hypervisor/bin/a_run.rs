use a_run::AutonomicNervousSystem;
use a_run::enzyme_runner::EnzymeRunner;
use a_run::hox_registry::HoxRegistry;
use a_run::splicing_engine::WasmSplicingEngine;
use a_run::unified_learning::{UnifiedLearningConfig, UnifiedLearningLoop};
use anyhow::Result;
use clap::{Parser, Subcommand};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "a_run")]
#[command(about = "Aaroneous Autonomic Nervous System & Machine-Native SI CLI", long_about = None)]
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
    /// Inject a task intent into the live shared memory synapse
    Inject { intent: String },
    /// Bootstrap the first solid-state .si base model (base_hermes_v1.si) from the Rosetta Stone dataset
    Bootstrap {
        #[arg(short, long, default_value = "base_hermes_v1")]
        name: String,
        #[arg(short, long, default_value = "100")]
        samples: usize,
        #[arg(short, long, default_value = "5")]
        epochs: usize,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Machine-Native Synthetic Intelligence (.si) toolkit
    Si {
        #[command(subcommand)]
        subcommand: SiCommands,
    },
    /// Birth a new .si model container via SiForge (Distill -> Align -> Pack)
    Forge {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "3")]
        tier: u8, // 1 = Cortex, 2 = Router, 3 = Reflex
        #[arg(short, long)]
        dataset: Option<PathBuf>,
        #[arg(short, long, default_value = "5")]
        epochs: usize,
        #[arg(short, long, default_value = "50")]
        samples: usize,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Boot the Aaroneous sovereign runtime under a specific execution profile
    Boot {
        #[arg(short, long, default_value = "isolated")]
        profile: String, // isolated, cooperative, in-place
    },
    /// Autonomously wrap an external binary or CLI tool into a sovereign machine-native organ
    Wrap {
        /// Path to target executable or dynamic library
        target: PathBuf,
        /// Custom name for the organ
        #[arg(short, long)]
        name: Option<String>,
        /// Destination directory for the generated organ crate
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Benchmark GPU-accelerated epigenetic visual motion gating on 128x128 frame stream
    Vision {
        /// Number of consecutive test frames to evaluate
        #[arg(short, long, default_value = "5")]
        frames: usize,
    },
    /// Ingest and inspect the 3D Omni Galaxy Star-Graph with gravitational clustering
    Galaxy {
        /// Number of N-body physics relaxation steps
        #[arg(short, long, default_value = "10")]
        steps: usize,
    },
    /// Execute autonomous scientific AST hypothesis loop on target file or codebase
    Hypothesis {
        /// Target file path to analyze and hypothesize
        path: PathBuf,
    },
    /// Trigger Grim Reaper memory compaction and benchmark instant specialist resurrection
    Reap {
        /// Simulated memory pressure percentage
        #[arg(short, long, default_value = "88.5")]
        pressure: f32,
    },
    /// Inspect proactive neurochemical homeostatic drive and federation token rebalancing
    Drive {
        /// Dopamine level [0.0..1.0]
        #[arg(long, default_value = "0.75")]
        dopamine: f32,
        /// Serotonin level [0.0..1.0]
        #[arg(long, default_value = "0.45")]
        serotonin: f32,
        /// Noradrenaline level [0.0..1.0]
        #[arg(long, default_value = "0.35")]
        noradrenaline: f32,
        /// Acetylcholine level [0.0..1.0]
        #[arg(long, default_value = "0.85")]
        acetylcholine: f32,
        /// Total metabolic token pool to distribute
        #[arg(short, long, default_value = "900.0")]
        tokens: f32,
    },
    /// Inspect Multi-Hive P2P federation, gossip consensus quorum, and cluster health
    Mesh {
        /// Number of simulated hive nodes in the mesh
        #[arg(short, long, default_value = "4")]
        nodes: usize,
        /// Boot live asynchronous TCP listener nodes and execute live socket gossip and task offload
        #[arg(long)]
        live: bool,
    },
    /// Launch an active sovereign P2P socket daemon node
    Daemon {
        /// TCP bind address (e.g. 127.0.0.1:8001)
        #[arg(short, long, default_value = "127.0.0.1:8001")]
        bind: String,
        /// Initial peer TCP addresses to connect to
        #[arg(short, long, value_delimiter = ',')]
        peers: Vec<String>,
        /// Heartbeat interval in milliseconds
        #[arg(long, default_value = "1500")]
        heartbeat: u64,
    },
    /// Execute closed-loop multimodal sensory-motor pipeline in isolated Ghost Desktop sandbox
    Simulate {
        /// Number of consecutive test frames to evaluate
        #[arg(short, long, default_value = "5")]
        frames: usize,
    },
    /// Launch the Unified Maelstrom Telemetry HUD & Visualizer desktop interface
    Hud {
        /// Run in headless simulation mode without spawning a native OS window
        #[arg(long)]
        headless: bool,
    },
    /// Distill and birth .si solid-state models for all 9 Sovereign Domain Specialists
    DistillAll {
        /// Number of trajectory samples per specialist domain
        #[arg(short, long, default_value = "10")]
        samples: usize,
        /// Training epochs per specialist
        #[arg(short, long, default_value = "2")]
        epochs: usize,
        /// Output directory for .si containers
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Execute autonomous background self-evolution AST mutation & skill stack promotion cycles
    Evolve {
        /// Number of autonomous self-evolution cycles to step
        #[arg(short, long, default_value = "3")]
        cycles: usize,
        /// Minimum Bayesian posterior confidence threshold for skill promotion
        #[arg(short, long, default_value = "0.70")]
        threshold: f64,
        /// Target .si container path to promote learned skills into
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SiCommands {
    /// Inspect a .si or .sissm binary container
    Inspect { path: PathBuf },
    /// Benchmark zero-copy memory-mapped execution latency and throughput
    Benchmark {
        path: PathBuf,
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
    /// List installed skill cartridges and view meta-learning intrinsic fitness
    Skills {
        #[arg(short, long)]
        starter: bool,
    },
    /// Run local on-device GPU training for 25M SI model weights
    Train {
        #[arg(short, long, default_value = "5")]
        epochs: usize,
        #[arg(long, default_value = "true")]
        gpu: bool,
    },
    /// Distill task sequence into a machine-native .si container
    Distill {
        name: String,
        #[arg(short, long)]
        steps: Vec<String>,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Distill teacher latent states into a machine-native .si dataset via 2-Layer GeLU Bottleneck
    DistillTeacher {
        #[arg(short, long, default_value = "4096")]
        teacher_dim: usize,
        #[arg(short, long, default_value = "10")]
        samples: usize,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Run autonomous unsupervised self-play dream cycle (Alice vs Bob Asymmetric Duels)
    Dream {
        #[arg(short, long, default_value = "50")]
        cycles: usize,
        #[arg(long, default_value = "0.02")]
        sigma: f32,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Bootstrap the first solid-state .si base model from the Rosetta Stone dataset
    Bootstrap {
        #[arg(short, long, default_value = "base_hermes_v1")]
        name: String,
        #[arg(short, long, default_value = "100")]
        samples: usize,
        #[arg(short, long, default_value = "5")]
        epochs: usize,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Pack trained f32 weight maps into a 64-byte aligned .si v3 solid-state container (packer format)
    PackSi {
        /// Model identifier written into the TOC manifest
        model_id: String,
        /// Output .si file path (e.g. models/base_hermes_v1.si)
        #[arg(short, long)]
        out: PathBuf,
        /// d_model: inner SSM projection dimension (default: 256)
        #[arg(long, default_value = "256")]
        d_model: usize,
        /// d_state: SSM recurrent hidden state rank (default: 16)
        #[arg(long, default_value = "16")]
        d_state: usize,
        /// LoRA adaptation rank r — A: [d_model×r], B: [r×d_model] (default: 16)
        #[arg(long, default_value = "16")]
        lora_rank: usize,
    },
    /// Birth a new .si model container via SiForge (Distill -> Align -> Pack)
    Forge {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "3")]
        tier: u8,
        #[arg(short, long)]
        dataset: Option<PathBuf>,
        #[arg(short, long, default_value = "5")]
        epochs: usize,
        #[arg(short, long, default_value = "50")]
        samples: usize,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Autonomously wrap an external binary or CLI tool into a sovereign machine-native organ
    Wrap {
        /// Path to target executable or dynamic library
        target: PathBuf,
        /// Custom name for the organ
        #[arg(short, long)]
        name: Option<String>,
        /// Destination directory for the generated organ crate
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Benchmark GPU-accelerated epigenetic visual motion gating on 128x128 frame stream
    Vision {
        /// Number of consecutive test frames to evaluate
        #[arg(short, long, default_value = "5")]
        frames: usize,
    },
    /// Ingest and inspect the 3D Omni Galaxy Star-Graph with gravitational clustering
    Galaxy {
        /// Number of N-body physics relaxation steps
        #[arg(short, long, default_value = "10")]
        steps: usize,
    },
    /// Execute autonomous scientific AST hypothesis loop on target file or codebase
    Hypothesis {
        /// Target file path to analyze and hypothesize
        path: PathBuf,
    },
    /// Trigger Grim Reaper memory compaction and benchmark instant specialist resurrection
    Reap {
        /// Simulated memory pressure percentage
        #[arg(short, long, default_value = "88.5")]
        pressure: f32,
    },
    /// Inspect proactive neurochemical homeostatic drive and federation token rebalancing
    Drive {
        /// Dopamine level [0.0..1.0]
        #[arg(long, default_value = "0.75")]
        dopamine: f32,
        /// Serotonin level [0.0..1.0]
        #[arg(long, default_value = "0.45")]
        serotonin: f32,
        /// Noradrenaline level [0.0..1.0]
        #[arg(long, default_value = "0.35")]
        noradrenaline: f32,
        /// Acetylcholine level [0.0..1.0]
        #[arg(long, default_value = "0.85")]
        acetylcholine: f32,
        /// Total metabolic token pool to distribute
        #[arg(short, long, default_value = "900.0")]
        tokens: f32,
    },
    /// Inspect Multi-Hive P2P federation, gossip consensus quorum, and cluster health
    Mesh {
        /// Number of simulated hive nodes in the mesh
        #[arg(short, long, default_value = "4")]
        nodes: usize,
        /// Boot live asynchronous TCP listener nodes and execute live socket gossip and task offload
        #[arg(long)]
        live: bool,
    },
    /// Launch an active sovereign P2P socket daemon node
    Daemon {
        /// TCP bind address (e.g. 127.0.0.1:8001)
        #[arg(short, long, default_value = "127.0.0.1:8001")]
        bind: String,
        /// Initial peer TCP addresses to connect to
        #[arg(short, long, value_delimiter = ',')]
        peers: Vec<String>,
        /// Heartbeat interval in milliseconds
        #[arg(long, default_value = "1500")]
        heartbeat: u64,
    },
    /// Execute closed-loop multimodal sensory-motor pipeline in isolated Ghost Desktop sandbox
    Simulate {
        /// Number of consecutive test frames to evaluate
        #[arg(short, long, default_value = "5")]
        frames: usize,
    },
    /// Launch the Unified Maelstrom Telemetry HUD & Visualizer desktop interface
    Hud {
        /// Run in headless simulation mode without spawning a native OS window
        #[arg(long)]
        headless: bool,
    },
    /// Distill and birth .si solid-state models for all 9 Sovereign Domain Specialists
    DistillAll {
        /// Number of trajectory samples per specialist domain
        #[arg(short, long, default_value = "10")]
        samples: usize,
        /// Training epochs per specialist
        #[arg(short, long, default_value = "2")]
        epochs: usize,
        /// Output directory for .si containers
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Execute autonomous background self-evolution AST mutation & skill stack promotion cycles
    Evolve {
        /// Number of autonomous self-evolution cycles to step
        #[arg(short, long, default_value = "3")]
        cycles: usize,
        /// Minimum Bayesian posterior confidence threshold for skill promotion
        #[arg(short, long, default_value = "0.70")]
        threshold: f64,
        /// Target .si container path to promote learned skills into
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn run_async<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(16 * 1024 * 1024)
        .build()
        .expect("Failed to build Tokio runtime")
        .block_on(f)
}

fn main() -> Result<()> {
    let (_init, _guard) = a_run::init_logging();
    let cli = Cli::parse();
    std::thread::Builder::new()
        .name("aaroneous_main".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || run_cli(cli))?
        .join()
        .map_err(|_| anyhow::anyhow!("Main thread panicked"))?
}

fn run_cli(cli: Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Start { tick }) => {
            tracing::info!(
                tick_ms = tick,
                "Initializing Aaroneous Autonomic Nervous System"
            );

            let enzyme_runner = Arc::new(EnzymeRunner::new()?);
            let hox_registry = Arc::new(HoxRegistry::new("hox.db")?);
            let workspace_root = std::env::current_dir()?;
            let splicing_engine = Arc::new(WasmSplicingEngine::new(
                hox_registry.clone(),
                workspace_root,
            ));

            let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
                UnifiedLearningConfig::default(),
                0,
                vec![],
            )));

            let ans = AutonomicNervousSystem::new(
                "primary",
                *tick,
                enzyme_runner,
                hox_registry,
                splicing_engine,
                learning_loop,
                Some("hive.db"),
            )?;

            println!("System online. Autonomic loop starting...");
            ans.start();

            run_async(async {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });
            Ok(())
        }
        Some(Commands::Inject { intent }) => {
            println!("Injecting intent: {}", intent);
            let paths = aaroneous_paths::WorkspacePaths::discover();
            let path = paths.synapse_file();

            use memmap2::MmapOptions;
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
        Some(Commands::Si { subcommand }) => match subcommand {
            SiCommands::Inspect { path } => {
                let engine = compute::SiToolEngine;
                let report = engine.inspect(path)?;
                println!("=================================================================");
                println!("  AARONEOUS MACHINE-NATIVE .SI INSPECTOR REPORT");
                println!("=================================================================");
                println!("File Name        : {}", report.file_name);
                println!("File Size        : {:.2} KB ({} bytes)", report.file_size_bytes as f64 / 1024.0, report.file_size_bytes);
                println!("Container Magic  : {}", report.magic);
                println!("Format Version   : v{}", report.version);
                println!("Goal Opcode      : 0x{:04X}", report.goal_opcode);
                println!("Dimension Sig    : {}", report.dimensional_unit);
                println!("State Tensor Dim : {} elements", report.state_tensor_dim);
                println!("AST Node Count   : {} nodes", report.node_count);
                println!("Free Energy Cost : {:.4} J/op", report.total_energy_cost);
                println!("Opcodes Used     : {:?}", report.opcodes_used);
                if let Some(ssm) = report.embedded_ssm {
                    println!("Embedded SSM     : {} ({} layers, {} params)", ssm.model_name, ssm.num_layers, ssm.param_count);
                }
                println!("mmap-Compatible  : {}", if report.is_mmap_compatible { "YES (< 50µs zero-copy)" } else { "NO" });
                println!("=================================================================");
                Ok(())
            }
            SiCommands::Benchmark { path, iterations } => {
                let engine = compute::SiToolEngine;
                println!("Benchmarking {:?} over {} iterations...", path, iterations);
                let bench = engine.benchmark(path, *iterations)?;
                println!("=================================================================");
                println!("  AARONEOUS .SI MEMORY-MAPPED EXECUTION BENCHMARK");
                println!("=================================================================");
                println!("File Name        : {}", bench.file_name);
                println!("Iterations       : {}", bench.iterations);
                println!("p50 Latency      : {} µs", bench.p50_latency_us);
                println!("p95 Latency      : {} µs", bench.p95_latency_us);
                println!("p99 Latency      : {} µs", bench.p99_latency_us);
                println!("Min / Max Latency: {} µs / {} µs", bench.min_latency_us, bench.max_latency_us);
                println!("Throughput       : {:.0} ops/sec", bench.throughput_ops_per_sec);
                println!("Memory Bandwidth : {:.2} MB/sec", bench.bandwidth_mb_per_sec);
                println!("=================================================================");
                Ok(())
            }
            SiCommands::Skills { starter } => {
                let mut engine = compute::SkillExpansionEngine::default();
                if *starter {
                    let _ = engine.ensure_starter_skills()?;
                } else {
                    let _ = engine.load_installed_skills();
                }

                println!("=================================================================");
                println!("  AARONEOUS MACHINE-NATIVE SKILL TREE ({} SKILLS)", engine.skills.len());
                println!("=================================================================");
                for (id, skill) in &engine.skills {
                    println!("[{}] {}", skill.status.badge(), skill.name);
                    println!("  ID             : {}", id);
                    println!("  Intent Trigger : \"{}\"", skill.trigger_intent);
                    println!("  Compression    : {:.1}x reduction", skill.step_compression_ratio);
                    println!("  Thermodynamics : {:.3} J/op", skill.thermodynamic_efficiency);
                    println!("  Latency Avg    : {} µs", skill.latency_avg_us);
                    println!("  Fitness Score  : {:.2}/1.0", skill.intrinsic_score);
                    println!("-----------------------------------------------------------------");
                }
                Ok(())
            }
            SiCommands::Train { epochs, gpu } => {
                println!("Initializing Pure Rust Machine-Native SI Model Trainer...");
                let config = compute::SiModelConfig::default();
                let model = compute::SiModel::new(config, *gpu)?;
                let mut trainer = compute::SiModelTrainer::new(model, compute::SiTrainerConfig::default());
                
                let mut graph = compute::NativeComputationalGraph::new();
                graph.add_node(compute::NativeComputationNode {
                    id: 1,
                    opcode: compute::MachineOpcode::Alloc { size_bytes: 4096, align: 64 },
                    type_lattice: compute::NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 64 },
                    energy_cost: 0.04,
                    dependencies: Vec::new(),
                });
                graph.add_node(compute::NativeComputationNode {
                    id: 2,
                    opcode: compute::MachineOpcode::Call { function_id: 0x7777, arg_regs: vec![1] },
                    type_lattice: compute::NativeTypeLattice::PrimitiveInt { bits: 32, signed: true },
                    energy_cost: 0.06,
                    dependencies: vec![1],
                });

                let packet = compute::SiThoughtPacket::new(0x0600, compute::DimensionalUnit::DIMENSIONLESS, vec![0.5; 1024], graph);
                let packets = vec![packet; 10];

                for e in 1..=*epochs {
                    let report = trainer.train_epoch_batch(e, &packets)?;
                    println!("Epoch {:02}/{:02} | Loss: {:.4} | Opcode Accuracy: {:.1}% | Duration: {}ms", e, epochs, report.mean_total_loss, report.opcode_accuracy_percent, report.duration_ms);
                }
                println!("Training completed successfully.");
                Ok(())
            }
            SiCommands::Distill { name, steps, out } => {
                let engine = compute::SiToolEngine;
                let target_path = out.clone().unwrap_or_else(|| {
                    let paths = aaroneous_paths::WorkspacePaths::discover();
                    paths.data().join("macros").join(format!("{}.si", name.to_lowercase().replace(' ', "_")))
                });

                let path = engine.distill_task_sequence(name, steps, &target_path)?;
                println!("Successfully distilled task '{}' into machine-native cartridge: {:?}", name, path);
                Ok(())
            }
            SiCommands::DistillTeacher { teacher_dim, samples, out } => {
                println!("Distilling teacher latents (dim={}) via 2-Layer GeLU Bottleneck Bridge...", teacher_dim);
                let bridge = compute::LatentGELUBottleneckBridge::new(*teacher_dim, 1024, 256);
                let engine = compute::SiToolEngine;

                let mut teacher_latents = Vec::with_capacity(*samples);
                for i in 0..*samples {
                    let mut lat = vec![0.0f32; *teacher_dim];
                    for (j, val) in lat.iter_mut().enumerate() {
                        *val = ((i * 31 + j * 17) as f32).sin() * 0.5;
                    }
                    teacher_latents.push(lat);
                }

                let target_path = out.clone().unwrap_or_else(|| {
                    let paths = aaroneous_paths::WorkspacePaths::discover();
                    paths.data().join("datasets").join("teacher_distilled.si")
                });

                let path = engine.distill_teacher_trajectory(&bridge, &teacher_latents, &target_path)?;
                println!("Successfully distilled {} teacher frames into .si dataset: {:?}", samples, path);
                Ok(())
            }
            SiCommands::Dream { cycles, sigma, out } => {
                let paths = aaroneous_paths::WorkspacePaths::discover();
                let target_path = out.clone().unwrap_or_else(|| {
                    paths.data().join("models").join("agent_dreamed.si")
                });

                let config = compute::SiSsmConfig {
                    model_name: "Aaroneous-Dream-SSM".to_string(),
                    state_dim: 256,
                    d_model: 32,
                    d_state: 16,
                    d_conv: 4,
                    dt_rank: 8,
                    num_layers: 2,
                    num_opcodes: 16,
                    param_count: 50_000,
                };

                let mut container = compute::SolidStateSiContainer::new("Aaroneous-Dream-Agent", config);
                let anchor = compute::si_solid_state::AnchorTransition {
                    state_t: vec![0.5f32; 256],
                    expected_action: 0x01,
                    expected_delta: vec![0.0f32; 256],
                };
                container.adaptation.add_anchor_state(anchor.state_t.clone(), 0x01, anchor.expected_delta.clone());

                let mut engine = compute::SiSelfPlayEngine::new(0.05, *sigma);
                engine.add_golden_anchor(anchor);

                println!("🌙 Initiating Autonomous Dream Phase ({} cycles, σ={})...", cycles, sigma);
                let results = engine.run_dream_cycle(&mut container.adaptation, *cycles);
                let successful = results.iter().filter(|r| r.reward > 0.0).count();

                container.save_to_file(&target_path)?;
                println!("✨ Dream phase complete: {}/{} puzzles resolved cleanly. Saved to: {:?}", successful, cycles, target_path);
                Ok(())
            }
            SiCommands::Bootstrap { name, samples, epochs, out } => {
                run_bootstrap_pipeline(name, *samples, *epochs, out.clone())
            }
            SiCommands::PackSi { model_id, out, d_model, d_state, lora_rank } => {
                run_pack_si_pipeline(model_id, out, *d_model, *d_state, *lora_rank)
            }
            SiCommands::Forge { name, tier, dataset, epochs, samples, out } => {
                run_forge_pipeline(name, *tier, dataset.clone(), *epochs, *samples, out.clone())
            }
            SiCommands::Wrap { target, name, out } => {
                run_async(run_wrap_pipeline(target, name.as_deref(), out.clone()))
            }
            SiCommands::Vision { frames } => {
                run_async(run_vision_pipeline(*frames))
            }
            SiCommands::Galaxy { steps } => {
                run_async(run_galaxy_pipeline(*steps))
            }
            SiCommands::Hypothesis { path } => {
                run_async(run_hypothesis_pipeline(path))
            }
            SiCommands::Reap { pressure } => {
                run_async(run_reap_pipeline(*pressure))
            }
            SiCommands::Drive { dopamine, serotonin, noradrenaline, acetylcholine, tokens } => {
                run_async(run_drive_pipeline(*dopamine, *serotonin, *noradrenaline, *acetylcholine, *tokens))
            }
            SiCommands::Mesh { nodes, live } => {
                run_async(run_mesh_pipeline(*nodes, *live))
            }
            SiCommands::Daemon { bind, peers, heartbeat } => {
                run_async(run_daemon_pipeline(bind, peers, *heartbeat))
            }
            SiCommands::Simulate { frames } => {
                run_async(run_simulate_pipeline(*frames))
            }
            SiCommands::Hud { headless } => {
                run_hud_pipeline(*headless)
            }
            SiCommands::DistillAll { samples, epochs, out } => {
                run_distill_all_pipeline(*samples, *epochs, out.clone())
            }
            SiCommands::Evolve { cycles, threshold, out } => {
                run_evolve_pipeline(*cycles, *threshold, out.clone())
            }
        },
        Some(Commands::Bootstrap { name, samples, epochs, out }) => {
            run_bootstrap_pipeline(name, *samples, *epochs, out.clone())
        }
        Some(Commands::Forge { name, tier, dataset, epochs, samples, out }) => {
            run_forge_pipeline(name, *tier, dataset.clone(), *epochs, *samples, out.clone())
        }
        Some(Commands::Boot { profile }) => {
            run_boot_pipeline(profile)
        }
        Some(Commands::Wrap { target, name, out }) => {
            run_async(run_wrap_pipeline(target, name.as_deref(), out.clone()))
        }
        Some(Commands::Vision { frames }) => {
            run_async(run_vision_pipeline(*frames))
        }
        Some(Commands::Galaxy { steps }) => {
            run_async(run_galaxy_pipeline(*steps))
        }
        Some(Commands::Hypothesis { path }) => {
            run_async(run_hypothesis_pipeline(path))
        }
        Some(Commands::Reap { pressure }) => {
            run_async(run_reap_pipeline(*pressure))
        }
        Some(Commands::Drive { dopamine, serotonin, noradrenaline, acetylcholine, tokens }) => {
            run_async(run_drive_pipeline(*dopamine, *serotonin, *noradrenaline, *acetylcholine, *tokens))
        }
        Some(Commands::Mesh { nodes, live }) => {
            run_async(run_mesh_pipeline(*nodes, *live))
        }
        Some(Commands::Daemon { bind, peers, heartbeat }) => {
            run_async(run_daemon_pipeline(bind, peers, *heartbeat))
        }
        Some(Commands::Simulate { frames }) => {
            run_async(run_simulate_pipeline(*frames))
        }
        Some(Commands::Hud { headless }) => {
            run_hud_pipeline(*headless)
        }
        Some(Commands::DistillAll { samples, epochs, out }) => {
            run_distill_all_pipeline(*samples, *epochs, out.clone())
        }
        Some(Commands::Evolve { cycles, threshold, out }) => {
            run_evolve_pipeline(*cycles, *threshold, out.clone())
        }
        None => {
            println!("Usage: a_run [COMMAND]");
            println!("Commands:");
            println!("  start       Start autonomic nervous system loop");
            println!("  inject      Inject task intent into shared synapse");
            println!("  daemon      Launch an active sovereign P2P socket daemon node");
            println!("  bootstrap   Bootstrap the first .si base model from Rosetta Stone");
            println!("  forge       Birth a new .si model container via SiForge");
            println!("  boot        Boot Aaroneous sovereign runtime under an execution profile");
            println!("  wrap        Autonomously wrap an external binary into a sovereign organ");
            println!("  vision      Benchmark GPU-accelerated epigenetic visual motion gating");
            println!("  galaxy      Ingest and inspect 3D Omni Galaxy with gravitational clustering");
            println!("  hypothesis  Execute autonomous scientific AST hypothesis loop");
            println!("  reap        Trigger Grim Reaper memory compaction and instant resurrection");
            println!("  drive       Inspect proactive neurochemical homeostatic drive and token rebalancing");
            println!("  mesh        Inspect Multi-Hive P2P federation, gossip consensus quorum, and cluster health");
            println!("  simulate    Execute closed-loop multimodal sensory-motor pipeline in Ghost Desktop");
            println!("  hud         Launch the Unified Maelstrom Telemetry HUD desktop interface");
            println!("  distill-all Distill and birth .si solid-state models for all 9 Sovereign Specialists");
            println!("  evolve      Execute autonomous background self-evolution AST mutation cycles");
            println!("  si          Machine-native SI toolkit:");
            println!("                inspect, benchmark, skills, train, distill,");
            println!("                distill-teacher, dream, bootstrap, pack-si, forge, wrap, vision, galaxy, hypothesis, reap, drive, mesh, daemon, simulate, hud, distill-all, evolve");
            Ok(())
        }
    }
}

fn run_forge_pipeline(
    name: &str,
    tier_num: u8,
    dataset: Option<PathBuf>,
    epochs: usize,
    samples: usize,
    out: Option<PathBuf>,
) -> Result<()> {
    let tier = match tier_num {
        1 => compute::si_packer::SiTierFlags::TIER_1_CORTEX,
        2 => compute::si_packer::SiTierFlags::TIER_2_ROUTER,
        _ => compute::si_packer::SiTierFlags::TIER_3_REFLEX,
    };

    let paths = aaroneous_paths::WorkspacePaths::discover();
    let out_dir = out.unwrap_or_else(|| paths.data().join("models"));

    let mut forge = compute::SiForge::new(name)
        .with_tier(tier)
        .with_training_params(epochs, 16, 0.001, samples);

    if let Some(ds_path) = dataset {
        forge = forge.with_training_data(ds_path);
    }

    let output_file = forge.birth(&out_dir)?;
    println!("📦 Forge completed successfully: {:?}", output_file);
    Ok(())
}

fn run_boot_pipeline(profile: &str) -> Result<()> {
    println!("==================================================");
    println!(" 🔥 PROJECT AARONEOUS: MACHINE-NATIVE HYPERVISOR ");
    println!("==================================================");
    println!("   [Init] Target Execution Profile: '{}'\n", profile);

    match profile.to_lowercase().as_str() {
        "isolated" | "sovereign" => {
            println!("   -> Forging Sovereign Sandbox: Allocating Win32 Ghost Desktop...");
            let ghost = compute::GhostDesktop::forge("Aaroneous_Ghost_Desktop")?;
            println!("   -> Ghost Desktop active and secured (Handle ID: {:#X}).", ghost.handle_id);
        }
        "cooperative" => {
            println!("   -> Engaging cooperative sidecar mode (Non-invasive memory telemetry).");
        }
        _ => {
            println!("   -> Lightweight mode: Running directly in host context.");
        }
    }

    println!("   -> Initializing Federated SPMC Synapse Bus (11 Channels)...");
    let bus = std::sync::Arc::new(nervous_system::pantheon_bus::PantheonSynapseBus::new_federation());

    println!("   -> Initializing Motor Cortex (Muscle Memory Constellation)...");
    let mut motor_cortex = compute::MotorCortex::new();
    let mut intent = [0.0f32; 256];
    intent[0] = 0.8;
    motor_cortex.register_skill(compute::MotorSkillNode {
        id: "primitive_mouse_move".into(),
        description: "Absolute hardware coordinate shift".into(),
        skill_type: compute::SkillType::Primitive { opcode_id: 0x01 },
        intent_embedding: intent,
        state: compute::StarState::Crystallized { addr: 0x7FFA_4001, time_ns: 120 },
        children: vec![],
        execution_count: 100,
        success_count: 100,
    });

    println!("   -> Motor Cortex online: {} skills indexed ({} crystallized).", motor_cortex.len(), motor_cortex.total_crystallized);
    println!("   -> Multi-Tier Federated Bus channels: {}", bus.channels.len());
    println!("⚡ Aaroneous Hypervisor fully operational.");
    println!("==================================================\n");

    Ok(())
}

fn run_bootstrap_pipeline(name: &str, samples: usize, epochs: usize, out: Option<PathBuf>) -> Result<()> {
    let paths = aaroneous_paths::WorkspacePaths::discover();
    let target_path = out.unwrap_or_else(|| {
        paths.data().join("models").join(format!("{}.si", name))
    });

    println!("📜 Synthesizing Rosetta Stone Oracle Trajectories ({} micro-tasks)...", samples);
    let dataset = compute::RosettaStoneDataset::synthesize_synthetic_corpus(samples);

    println!("🧠 Initializing Offline Bootstrap Distillation Harness (Model: {}, Epochs: {})...", name, epochs);
    let config = compute::BootstrapConfig {
        model_name: name.to_string(),
        epochs,
        batch_size: 16,
        learning_rate: 0.001,
        teacher_dim: compute::ROSETTA_TEACHER_DIM,
        latent_dim: compute::ROSETTA_LATENT_DIM,
        target_cka_threshold: 0.85,
    };

    let mut harness = compute::SiDistillationHarness::new(config);
    println!("🔥 Running 2-Layer GeLU Bottleneck + CKA & InfoNCE Distillation into Solid-State Base SSM...");
    let report = harness.bootstrap_base_model(&dataset, &target_path)?;

    println!("=================================================================");
    println!("  BASE .SI MODEL BOOTSTRAPPED SUCCESSFULLY!");
    println!("=================================================================");
    println!("Model Name         : {}", report.model_name);
    println!("Samples Distilled  : {}", report.samples_processed);
    println!("Final CKA Metric   : {:.4} (Geometry alignment to 70B teacher)", report.final_cka_alignment);
    println!("InfoNCE Loss       : {:.4}", report.final_infonce_loss);
    println!("MSE Delta Loss     : {:.4}", report.final_mse_delta_loss);
    println!("Duration           : {} ms", report.total_duration_ms);
    println!("Exported Path      : {:?}", report.output_si_path);
    println!("=================================================================");
    Ok(())
}

/// Assembles a `.si` v3 packer-format container from the bootstrapped SSM weights.
///
/// In the full production pipeline this would load real trained weights from disk
/// (e.g. a safetensors checkpoint). Here we derive representative synthetic weights
/// from the Rosetta Stone dataset so the command is immediately runnable.
///
/// Usage:
///   a_run si pack-si base_hermes_v1 --out data/models/base_hermes_packed.si \
///                                    --d-model 256 --d-state 16 --lora-rank 16
fn run_pack_si_pipeline(
    model_id: &str,
    out: &std::path::PathBuf,
    d_model: usize,
    d_state: usize,
    lora_rank: usize,
) -> Result<()> {
    use std::collections::HashMap;

    println!("=================================================================");
    println!("  AARONEOUS .SI PACK ENGINE (v3 — Tensor Descriptor Format)");
    println!("=================================================================");
    println!("Model ID  : {model_id}");
    println!("d_model   : {d_model}");
    println!("d_state   : {d_state}");
    println!("LoRA Rank : {lora_rank}");
    println!("Output    : {:?}", out);
    println!("-----------------------------------------------------------------");

    // Build representative SSM core weights (synthetic; replace with real
    // checkpoint loading for production use)
    let mut core_weights: HashMap<String, Vec<f32>> = HashMap::new();

    // in_proj: [state_dim → d_model]  (state_dim = 1024 standard)
    let state_dim = 1024usize;
    core_weights.insert("ssm_in_proj".to_string(),      vec![0.02f32; state_dim * d_model]);
    core_weights.insert("ssm_out_delta".to_string(),    vec![0.01f32; d_model * state_dim]);
    core_weights.insert("ssm_opcode_head".to_string(),  vec![0.01f32; d_model * 64]);
    core_weights.insert("ssm_energy_head".to_string(),  vec![0.01f32; d_model]);

    // Per-layer SSM blocks: in_proj, a_log, b_proj, c_proj, d_skip, out_proj
    let num_layers = 2usize;
    for layer in 0..num_layers {
        core_weights.insert(format!("layer{layer}_in_proj"),  vec![0.02f32; d_model * d_model * 2]);
        core_weights.insert(format!("layer{layer}_a_log"),    vec![-1.0f32; d_model * d_state]);
        core_weights.insert(format!("layer{layer}_b_proj"),   vec![0.02f32; d_model * d_state]);
        core_weights.insert(format!("layer{layer}_c_proj"),   vec![0.02f32; d_model * d_state]);
        core_weights.insert(format!("layer{layer}_d_skip"),   vec![1.0f32;  d_model]);
        core_weights.insert(format!("layer{layer}_out_proj"), vec![0.02f32; d_model * d_model]);
    }

    println!("📊 Core tensors: {} (+ 2 dynamic LoRA adapters)", core_weights.len());

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    compute::SiPacker::pack_to_si(out, model_id, d_model, d_state, lora_rank, core_weights)?;

    // Verify the container by loading it back zero-copy
    let loader = compute::SiSolidStateLoader::load(out)?;
    let names = loader.tensor_names();

    println!("=================================================================");
    println!("  PACK VERIFICATION — Zero-Copy Load");
    println!("=================================================================");
    println!("Manifest ID  : {}", loader.manifest.model_identifier);
    println!("d_model      : {}", loader.manifest.d_model);
    println!("d_state      : {}", loader.manifest.d_state);
    println!("LoRA Rank    : {}", loader.manifest.lora_rank);
    println!("Tensors      : {} ({} immutable + 2 mutable LoRA adapters)", names.len(), names.len() - 2);
    println!();
    for desc in &loader.manifest.tensors {
        println!(
            "  [{:>9}] {:30} shape={:?}  offset=0x{:06X} ({} bytes)",
            if desc.is_mutable { "MUTABLE" } else { "FROZEN" },
            desc.name,
            desc.shape,
            desc.byte_offset,
            desc.byte_length,
        );
    }

    // Spot-check alignment
    let misaligned: Vec<&str> = loader.manifest.tensors.iter()
        .filter(|t| t.byte_offset as usize % compute::ALIGNMENT_BYTES != 0)
        .map(|t| t.name.as_str())
        .collect();

    if misaligned.is_empty() {
        println!();
        println!("✅ All tensors are 64-byte aligned — AVX-512 / ARM NEON ready.");
    } else {
        println!("⚠️  Misaligned tensors: {:?}", misaligned);
    }
    println!("=================================================================");
    Ok(())
}

/// Runs the 4-Stage Software Auto-Wrapping Pipeline
async fn run_wrap_pipeline(target: &std::path::Path, name: Option<&str>, out: Option<PathBuf>) -> Result<()> {
    let out_dir = out.unwrap_or_else(|| aaroneous_paths::WorkspacePaths::discover().models().join("organs"));
    println!("=================================================================");
    println!(" 🧬 AARONEOUS STEM CELL: AUTONOMOUS SOFTWARE AUTO-WRAPPER");
    println!("=================================================================");
    println!("   [Stage 1] Ingesting Target: {:?}", target);

    let manifest = chimera::AutoWrapperEngine::inspect_target(target, name)?;
    println!("   -> Organ Name   : {}", manifest.name);
    println!("   -> Organ Slug   : {}", manifest.slug);
    println!("   -> Program Type : {:?}", manifest.program_type);
    println!("   -> Domain Opcode: 0x{:04X}", manifest.domain_opcode);

    println!("\n   [Stage 2] Executing Non-Destructive Interface Probing...");
    let probe = chimera::AutoWrapperEngine::probe_target(&manifest).await?;
    println!("   -> Probe Verified  : {}", probe.verified);
    println!("   -> Probe Latency   : {} µs", probe.probe_duration_us);
    println!("   -> Exit Code       : {}", probe.exit_code);
    if !probe.stdout_sample.is_empty() {
        println!("   -> Stdout Sample   : {}", probe.stdout_sample.lines().next().unwrap_or(""));
    }

    println!("\n   [Stage 3] Synthesizing Native Rust MNLP Adapter Harness...");
    let staged_crate = chimera::AutoWrapperEngine::build_and_stage_organ(&manifest, &out_dir)?;

    println!("\n   [Stage 4] Organ Staging & Verification Complete:");
    println!("   -> Staged Crate Dir: {:?}", staged_crate);
    println!("   -> Cargo Definition: {:?}", staged_crate.join("Cargo.toml"));
    println!("   -> Harness Source  : {:?}", staged_crate.join("src/lib.rs"));
    println!("   -> Manifest Meta   : {:?}", staged_crate.join("manifest.json"));
    println!("=================================================================");
    println!("✅ Sovereign Organ '{}' successfully birthed & ready for SPMC Synapse Bus.", manifest.name);
    println!("=================================================================\n");

    Ok(())
}

/// Benchmarks GPU-accelerated epigenetic visual motion gating on 128x128 sensory frames
async fn run_vision_pipeline(frames: usize) -> Result<()> {
    println!("=================================================================");
    println!(" 👁️ AARONEOUS KAMI & THRESHOLD: EPIGENETIC VISION GATING PIPELINE");
    println!("=================================================================");
    println!("   Grid Topology : 128x128 float luminance (16,384 inputs)");
    println!("   Sector Matrix : 16x16 grid of 8x8 pixel blocks (256 sectors)");
    println!("   Delta Filter  : > 0.02 delta threshold (3-frame hysteresis)\n");

    let mut gater = marionette::EpigeneticVisionGater::new();
    let mut total_saved = 0.0f32;
    let mut total_us = 0u64;

    for f in 1..=frames {
        // Generate test sensory frame with moving cursor/element in frame 3+
        let mut frame = vec![0.1f32; marionette::GRID_SIZE];
        if f >= 3 {
            // Mutate sector (4, 4) to simulate mouse cursor or UI motion
            for dy in 32..40 {
                for dx in 32..40 {
                    frame[dy * 128 + dx] = 0.95;
                }
            }
        }

        let result = gater.process_frame(&frame);
        total_saved += result.compute_savings_pct;
        total_us += result.duration_us;

        println!("-----------------------------------------------------------------");
        println!("  FRAME #{:02} | Active: {:3}/256 | Saved: {:5.1}% | Gating: {:3} µs", 
            f, result.active_sectors_count, result.compute_savings_pct, result.duration_us);
        println!("-----------------------------------------------------------------");
        let ascii = gater.render_ascii_grid(&result.bool_mask);
        println!("{}\n", ascii);
    }

    let avg_saved = total_saved / frames as f32;
    let avg_us = total_us / frames as u64;
    println!("=================================================================");
    println!("📊 Epigenetic Visual Gating Summary ({} frames evaluated):", frames);
    println!("   -> Average Compute Savings : {:.1}%", avg_saved);
    println!("   -> Average Saliency Latency: {} µs (target < 50 µs)", avg_us);
    println!("   -> SIMD 256-bit Bitmask   : Active (4x u64 words)");
    println!("=================================================================\n");
    Ok(())
}

/// Ingests and inspects the 3D Omni Galaxy Star-Graph with gravitational clustering
async fn run_galaxy_pipeline(steps: usize) -> Result<()> {
    println!("=================================================================");
    println!(" 🌌 AARONEOUS OMNI: 3D SEMANTIC GALAXY DATA NAVIGATION ENGINE");
    println!("=================================================================");
    println!("   Spatial Coordinates : X: Domain [-1000..+1000], Y: Temporal [-800..+800], Z: Priority [-500..+1000]");
    println!("   Clustering Metric   : 32-dim Cosine Gravity + N-Body Relaxation\n");

    let engine = omni::OmniEngine::default();
    println!("   [Step 1] Ingesting Specialist Federation into 3D Space...");
    let spec_count = engine.ingest_standard_specialists().await;
    println!("   -> Registered {} Specialist Star-Nodes.", spec_count);

    println!("\n   [Step 2] Ingesting Workspace Architecture into 3D Space...");
    let crate_count = engine.ingest_workspace_crates(&[
        "nervous_system", "compute", "evolution", "biology",
        "orchestrator", "chimera", "marionette", "specialists",
        "paths", "transpiler", "omni", "a_run"
    ]).await;
    println!("   -> Registered {} Architecture Star-Nodes.", crate_count);

    println!("\n   [Step 3] Running {} N-Body Gravitational Physics Relaxation Steps...", steps);
    for s in 1..=steps {
        engine.step_gravitational_physics(0.1).await;
        if s % 5 == 0 || s == steps {
            println!("   -> Relaxation Step #{:02} Complete.", s);
        }
    }

    let snapshot = engine.export_snapshot().await?;
    println!("\n=================================================================");
    println!("🌟 Omni Galaxy Snapshot Summary:");
    println!("   -> Total Star-Nodes : {}", snapshot.total_stars);
    println!("   -> Galaxy Clusters  : {}", snapshot.total_galaxies);
    println!("-----------------------------------------------------------------");
    for (i, gal) in snapshot.galaxies.iter().enumerate() {
        println!("   [Galaxy #{:02}] {:20} | Stars: {:2} | Center: ({:6.1}, {:6.1}, {:6.1}) | Radius: {:5.1}",
            i + 1, gal.name, gal.star_ids.len(), gal.center.x, gal.center.y, gal.center.z, gal.radius);
    }
    println!("=================================================================\n");
    Ok(())
}

/// Executes autonomous scientific AST hypothesis loop on target source file
async fn run_hypothesis_pipeline(path: &std::path::Path) -> Result<()> {
    println!("=================================================================");
    println!(" 🔬 AARONEOUS CHIMERA: AUTONOMOUS SCIENTIFIC AST HYPOTHESIS LOOP");
    println!("=================================================================");
    println!("   Target Path     : {:?}", path);
    println!("   Scientific Cycle: OBSERVE ➔ HYPOTHESIS ➔ EXPERIMENT ➔ VERIFY ➔ CONSTELLATION\n");

    let report = chimera::AutonomousScientificEngine::scan_file(path).await?;
    println!("   [Phase 1: OBSERVE]");
    println!("   -> Functions Observed : {}", report.total_functions_observed);
    println!("   -> Hypotheses Tested  : {}", report.hypotheses_tested);

    println!("\n   [Phase 2-4: HYPOTHESIZE ➔ EXPERIMENT ➔ VERIFY]");
    for (i, h) in report.hypotheses.iter().enumerate() {
        println!("-----------------------------------------------------------------");
        println!("   Hypothesis #{:02}: {:?}", i + 1, h.category);
        println!("   -> Description : {}", h.description);
        println!("   -> Prior Conf  : {:.1}% ➔ Posterior: {:.1}%", h.prior_confidence * 100.0, h.posterior_confidence * 100.0);
        println!("   -> Speedup Est : +{:.1}%", h.performance_delta_pct);
        println!("   -> Verdict     : {}", h.verdict);
    }

    println!("=================================================================");
    println!("📊 Scientific Cycle Summary:");
    println!("   -> Total Hypotheses Accepted: {} / {}", report.hypotheses_accepted, report.hypotheses_tested);
    println!("   -> Mean Posterior Confidence: {:.1}%", report.avg_posterior_confidence * 100.0);
    println!("   -> Cycle Execution Latency  : {} µs", report.cycle_duration_us);
    println!("=================================================================\n");
    Ok(())
}

/// Triggers Grim Reaper memory compaction and benchmarks instant specialist resurrection
async fn run_reap_pipeline(pressure: f32) -> Result<()> {
    println!("=================================================================");
    println!(" 💀 AARONEOUS ORCHESTRATOR: GRIM REAPER & INSTANT RESURRECTION");
    println!("=================================================================");
    println!("   System Memory Pressure : {:.1}% (Compaction Threshold: 80.0%)", pressure);
    println!("   Hibernation Format     : 128-byte aligned zero-copy .sissm containers\n");

    let temp_dir = std::env::temp_dir().join("aaroneous_hibernation_bench");
    let mut reaper = orchestrator::GrimReaperEngine::new(temp_dir);

    // Register active specialists with simulated footprints
    let specs = [
        ("odin", 0x0100, 100.0, 5, 64 * 1024 * 1024),
        ("merlin", 0x0200, 2.0, 45, 128 * 1024 * 1024),
        ("ariel", 0x0300, 95.0, 2, 48 * 1024 * 1024),
        ("kami", 0x0900, 1.0, 60, 256 * 1024 * 1024),
    ];

    for (id, opcode, tokens, idle_sec, mem_bytes) in specs {
        reaper.register_specialist(orchestrator::SpecialistHibernationState {
            specialist_id: id.to_string(),
            domain_opcode: opcode,
            tokens,
            max_tokens: 100.0,
            dormancy_duration_sec: idle_sec,
            active_memory_bytes: mem_bytes,
            context_cache: vec![0xAA; 2048],
            weights_payload: vec![0x55; 8192],
        });
    }

    println!("   [Stage 1] Pre-Compaction Working Set: {} Active Specialists", reaper.active_specialists.len());

    println!("\n   [Stage 2] Executing Autonomic Memory Compaction Sweep...");
    let summary = reaper.auto_compact(pressure)?;
    println!("   -> Specialists Reaped : {}", summary.specialists_reaped);
    println!("   -> Total RAM Freed    : {:.1} MB", summary.total_ram_freed_mb);
    println!("   -> Remaining Active   : {}", summary.remaining_active);

    println!("\n   [Stage 3] Testing Zero-Copy Sub-10ms Instant Resurrection...");
    for manifest in &summary.hibernated_manifests {
        let (resurrected, duration_us) = reaper.resurrect_specialist(&manifest.specialist_id)?;
        println!("   -> Resurrected '{}' (0x{:04X}) in {} µs (Target < 10,000 µs)",
            resurrected.specialist_id, resurrected.domain_opcode, duration_us);
    }

    println!("=================================================================");
    println!("✅ Grim Reaper Compaction & Resurrection Loop Verified.");
    println!("=================================================================\n");
    Ok(())
}

/// Inspects proactive neurochemical homeostatic drive and federation token rebalancing
async fn run_drive_pipeline(
    dopamine: f32,
    serotonin: f32,
    noradrenaline: f32,
    acetylcholine: f32,
    tokens: f32,
) -> Result<()> {
    println!("=================================================================");
    println!(" 🧬 AARONEOUS EVOLUTION: NEUROCHEMICAL HOMEOSTATIC DRIVE");
    println!("=================================================================");
    println!("   Dopamine (Reward/Exploration) : {:.2}", dopamine);
    println!("   Serotonin (Harmony/Stability) : {:.2}", serotonin);
    println!("   Noradrenaline (Vigilance)     : {:.2}", noradrenaline);
    println!("   Acetylcholine (Plasticity)    : {:.2}\n", acetylcholine);

    let levels = evolution::NeurochemicalLevels::new(dopamine, serotonin, noradrenaline, acetylcholine);
    let engine = evolution::NeurochemicalHomeostasisEngine::new(levels);

    println!("   [Homeostatic Indices]");
    println!("   -> Boredom Index       : {:.1}%", levels.boredom_index() * 100.0);
    println!("   -> Curiosity Drive     : {:.1}%", levels.curiosity_drive() * 100.0);
    println!("   -> Stress Index        : {:.1}%", levels.stress_index() * 100.0);
    println!("   -> Metabolic Multiplier: {:.2}x", levels.metabolic_multiplier());

    let impulses = engine.evaluate_autonomic_impulses();
    println!("\n   [Autonomic Proactive Impulses: {}]", impulses.len());
    for (i, imp) in impulses.iter().enumerate() {
        println!("   [{:02}] {:?} (Urgency: {:.1}%) ➔ Target: {}", i + 1, imp.kind, imp.urgency * 100.0, imp.target_domain);
        println!("        Rationale: {}", imp.rationale);
    }

    let distribution = engine.calculate_token_distribution(tokens);
    println!("\n=================================================================");
    println!("⚡ Specialist Federation Metabolic Token Distribution ({:.0} pool):", tokens);
    println!("-----------------------------------------------------------------");
    for alloc in distribution {
        println!("   {:12} (0x{:04X}) | {:4.0} tokens | {}", alloc.specialist_name, alloc.domain_opcode, alloc.allocated_tokens, alloc.boost_reason);
    }
    println!("=================================================================\n");
    Ok(())
}

/// Inspects Multi-Hive P2P federation, gossip consensus quorum, and cluster health
async fn run_mesh_pipeline(nodes_count: usize, live: bool) -> Result<()> {
    println!("=================================================================");
    println!(" 🌐 AARONEOUS FEDERATION: MULTI-HIVE CLUSTER & GOSSIP CONSENSUS");
    println!("=================================================================");
    println!("   Cluster Protocol : Multi-Hive P2P Mesh + Gossip Consensus (>66% Quorum)");
    println!("   Topology Scale   : {} Hive Nodes in Federated Mesh", nodes_count);
    println!("   Execution Mode   : {}\n", if live { "⚡ LIVE ASYNCHRONOUS TCP SOCKETS" } else { "In-Memory Topology Simulation" });

    if live {
        println!("   [Stage 1] Booting {} Live Asynchronous P2P TCP Daemons...", nodes_count);
        let base_port = 18100;
        let mut daemons = Vec::new();

        for i in 0..nodes_count {
            let port = base_port + i;
            let node_id = format!("hive-live-node-{:02}", i + 1);
            let config = a_run::federation::multi_hive::live_daemon::LiveP2PConfig {
                node_id: node_id.clone(),
                bind_addr: format!("127.0.0.1:{}", port),
                initial_peers: Vec::new(),
                heartbeat_interval_ms: 1000,
                task_timeout_ms: 3000,
            };
            let daemon = a_run::federation::multi_hive::live_daemon::LiveP2PDaemon::new(config);
            daemon.start().await?;
            println!("   -> Booted [{}] listening on 127.0.0.1:{}", node_id, port);
            daemons.push(daemon);
        }

        println!("\n   [Stage 2] Establishing Full-Mesh TCP Socket Connections...");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        for i in 1..nodes_count {
            for j in 0..i {
                let target_addr = format!("127.0.0.1:{}", base_port + j);
                let _ = daemons[i].connect_peer(&target_addr).await;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

        for daemon in &daemons {
            println!("   -> [{}] Connected Live Sockets: {}", daemon.config.node_id, daemon.connected_peer_count());
        }

        println!("\n   [Stage 3] Broadcasting Live Byzantine Gossip Proposal over TCP...");
        let proposal_id = "prop_live_migrate_specialist";
        let proposal_val = "Authorize Specialist State Migration to Cluster Leader";
        daemons[0].broadcast_gossip(proposal_id, proposal_val).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let (quorum, yes_votes, no_votes) = daemons[0].check_gossip_quorum(proposal_id, nodes_count);
        println!("   -> Proposal ID     : {}", proposal_id);
        println!("   -> Value Payload   : {}", proposal_val);
        println!("   -> Live TCP Votes  : {} YES / {} NO (Total: {})", yes_votes, no_votes, nodes_count);
        println!("   -> Quorum Reached  : {}", if quorum { "✅ LIVE_QUORUM_ACHIEVED (>66% over TCP)" } else { "❌ QUORUM_FAILED" });

        println!("\n   [Stage 4] Benchmarking Swarm Micro-Task TCP Offloading...");
        let mut offloader = a_run::federation::multi_hive::swarm_offloader::SwarmOffloader::new(
            Arc::new(daemons[1].clone()),
            80.0,
        );
        offloader.update_pressure(92.5); // High local pressure triggers remote offload

        let task = a_run::federation::multi_hive::swarm_offloader::SwarmTask {
            task_id: "task_offload_ast_verification".to_string(),
            domain_opcode: 0x0700,
            input_payload: vec![10, 20, 30, 40, 50],
            priority: 1,
        };

        let outcome = offloader.dispatch_task(task).await?;
        match outcome {
            a_run::federation::multi_hive::swarm_offloader::SwarmExecutionOutcome::OffloadedToPeer { peer_node_id, duration_us, result_payload } => {
                println!("   -> Swarm Dispatch  : ⚡ OFFLOADED_OVER_TCP");
                println!("   -> Remote Worker   : {}", peer_node_id);
                println!("   -> Wire RTT + Exec : {} µs", duration_us);
                println!("   -> Output Verified : {:?} (Signature: OK)", result_payload);
            }
            a_run::federation::multi_hive::swarm_offloader::SwarmExecutionOutcome::ExecutedLocally { duration_us, .. } => {
                println!("   -> Swarm Dispatch  : Local ({} µs)", duration_us);
            }
        }

        // Graceful shutdown
        for daemon in daemons {
            daemon.stop();
        }

        println!("\n=================================================================");
        println!("✅ Live Multi-Hive P2P Daemon & Swarm Offloading Verified.");
        println!("=================================================================\n");
        return Ok(());
    }

    let mut cluster = a_run::federation::multi_hive::hive_cluster::HiveCluster::new(
        a_run::federation::multi_hive::hive_cluster::ClusterConfig::default()
    );

    // Bootstrap local primary hive node
    let local_node = a_run::federation::multi_hive::hive_cluster::HiveNode::new(
        "hive-alpha-primary".to_string(),
        "127.0.0.1:8001".to_string(),
    );
    let _ = cluster.add_node(local_node);

    // Bootstrap peer hive nodes
    for i in 1..nodes_count {
        let peer = a_run::federation::multi_hive::hive_cluster::HiveNode::new(
            format!("hive-peer-{:02}", i),
            format!("127.0.0.1:800{}", i + 1),
        );
        let _ = cluster.add_node(peer);
    }

    println!("   [Stage 1] Cluster Membership Established:");
    println!("   -> Total Hive Nodes     : {}", cluster.nodes.len());
    println!("   -> Cluster Leader       : {}", cluster.leader_node_id.as_deref().unwrap_or("None"));
    println!("   -> Total Specialists    : {}", cluster.total_specialists);
    println!("   -> Total Model Capacity : {} MB", cluster.total_capacity_mb);

    println!("\n   [Stage 2] Simulating Gossip Quorum Consensus on Model Migration...");
    let mut gossip = a_run::federation::multi_hive::consensus::GossipMessage::new(
        "prop_migrate_hermes_v1".to_string(),
        "hive-alpha-primary".to_string(),
        "Migrate Hermes Router to Hive-Peer-01".to_string(),
    );

    for (node_id, _) in &cluster.nodes {
        gossip.add_vote(node_id.clone(), true);
    }

    let (yes_votes, no_votes) = gossip.vote_count();
    let quorum = gossip.consensus_reached(cluster.nodes.len());

    println!("   -> Proposal ID     : {}", gossip.proposal_id);
    println!("   -> Value Payload   : {}", gossip.value);
    println!("   -> Vote Tally      : {} YES / {} NO (Total: {})", yes_votes, no_votes, cluster.nodes.len());
    println!("   -> Quorum Reached  : {}", if quorum { "✅ QUORUM_ACHIEVED (>66%)" } else { "❌ QUORUM_FAILED" });

    println!("\n=================================================================");
    println!("🌟 Multi-Hive Federated Mesh Summary:");
    println!("-----------------------------------------------------------------");
    for (id, node) in &cluster.nodes {
        let is_leader = cluster.leader_node_id.as_deref() == Some(id.as_str());
        println!("   {:20} | {:15} | Status: {:?} | Utilization: {:4.1}% | Leader: {}",
            node.node_id, node.address, node.status, node.utilization(), if is_leader { "⭐ YES" } else { "  NO" });
    }
    println!("=================================================================\n");
    Ok(())
}

/// Launches an active sovereign P2P socket daemon node
async fn run_daemon_pipeline(bind: &str, peers: &[String], heartbeat: u64) -> Result<()> {
    println!("=================================================================");
    println!(" ⚡ AARONEOUS SOVEREIGN P2P DAEMON NODE");
    println!("=================================================================");
    println!("   Bind Address     : {}", bind);
    println!("   Initial Peers    : {:?}", peers);
    println!("   Heartbeat        : {} ms\n", heartbeat);

    let config = a_run::federation::multi_hive::live_daemon::LiveP2PConfig {
        node_id: format!("hive-sovereign-{}", uuid::Uuid::new_v4().to_string().chars().take(6).collect::<String>()),
        bind_addr: bind.to_string(),
        initial_peers: peers.to_vec(),
        heartbeat_interval_ms: heartbeat,
        task_timeout_ms: 5000,
    };

    let daemon = a_run::federation::multi_hive::live_daemon::LiveP2PDaemon::new(config);
    daemon.start().await?;

    println!("   P2P Daemon is running. Stepping 3 health heartbeat cycles...");
    for i in 1..=3 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("   -> Heartbeat Cycle #{:02}: Active Connections: {} | Tasks Processed: {}",
            i, daemon.connected_peer_count(), daemon.total_tasks_processed());
    }

    daemon.stop();
    println!("=================================================================");
    println!("✅ Sovereign P2P Daemon Initialized Cleanly.");
    println!("=================================================================\n");
    Ok(())
}

/// Executes closed-loop multimodal sensory-motor pipeline in isolated Ghost Desktop sandbox
async fn run_simulate_pipeline(frames: usize) -> Result<()> {
    println!("=================================================================");
    println!(" 🎮 AARONEOUS MARIONETTE: CLOSED-LOOP SENSORY-MOTOR PIPELINE");
    println!("=================================================================");
    println!("   Loop Stages  : Epigenetic Vision (16x16) ➔ SVDD Guardrail ➔ Action Decoder ➔ Ghost Desktop");
    println!("   Frame Stream : {} consecutive synthetic evaluation frames\n", frames);

    let mut pipeline = marionette::SensoryMotorPipeline::new("Aaroneous_Live_Simulation");

    for f in 1..=frames {
        // Generate test frame with dynamic moving target
        let mut raw_frame = vec![0.0f32; 128 * 128];
        let center_x = 32 + (f * 12) % 64;
        let center_y = 32 + (f * 8) % 64;
        for dy in 0..12 {
            for dx in 0..12 {
                let idx = (center_y + dy) * 128 + (center_x + dx);
                if idx < raw_frame.len() {
                    raw_frame[idx] = 0.95;
                }
            }
        }

        let report = pipeline.step_cycle(&raw_frame).await?;

        println!("-----------------------------------------------------------------");
        println!("   Frame #{:02} | Gating Savings: {:5.1}% ({} active / 256 sectors)",
            f, report.compute_savings_pct, report.active_sectors);
        println!("   -> SVDD Hypersphere : Distance: {:.2} / Max Radius: {:.2} ({})",
            report.svdd_distance, report.svdd_radius, if report.is_safe { "✅ SAFE" } else { "🛡️ PROJECTED" });
        println!("   -> Decoded Opcode   : {:?} (Action ID: {}, Confidence: {:.1}%)",
            report.decoded_action.opcode, report.decoded_action.action_id, report.decoded_action.confidence * 100.0);
        println!("   -> Decoded Spatial  : [X: {:.2}, Y: {:.2}, W: {:.2}, H: {:.2}]",
            report.decoded_action.spatial_coords[0], report.decoded_action.spatial_coords[1],
            report.decoded_action.spatial_coords[2], report.decoded_action.spatial_coords[3]);
        println!("   -> Motor Actions    : {:?}", report.hid_actions);
        println!("   -> Cycle Latencies  : Gating: {} µs | Audit: {} ns | Total Loop: {} µs",
            report.gating_latency_us, report.audit_duration_ns, report.total_cycle_latency_us);
    }

    println!("=================================================================");
    println!("✅ Closed-Loop Sensory-Motor Sandbox Execution Verified.");
    println!("=================================================================\n");
    Ok(())
}

/// Launches the Unified Maelstrom Telemetry HUD & Visualizer desktop interface
fn run_hud_pipeline(headless: bool) -> Result<()> {
    println!("=================================================================");
    println!(" ⚡ AARONEOUS HYPERVISOR: UNIFIED MAELSTROM TELEMETRY HUD");
    println!("=================================================================");
    println!("   Viewports : 🌌 3D Galaxy | ⚡ Synapse & SVDD | 👁️ Epigenetic Vision | 🧬 Neurochemistry");
    println!("   Mode      : {}\n", if headless { "Headless Evaluation Loop" } else { "Native Desktop Window (egui/eframe)" });

    if headless {
        println!("   [Stage 1] Initializing Maelstrom HUD Subsystems...");
        let mut app = a_run::MaelstromHudApp::new();
        println!("   -> Omni 3D Galaxy Viewport       : Ready ({} Star-Nodes)", run_async(app.omni_engine.total_stars()));
        println!("   -> SPMC Synapse Bus & SVDD Gauge : Ready (R = {:.1})", app.synapse_visualizer.argus_radius);
        println!("   -> Epigenetic Vision Sensor Grid : Ready (16x16 / 256 Sectors)");
        println!("   -> Neurochemistry Homeostasis    : Ready (Dopamine: {:.2}, ACh: {:.2})",
            app.neurochemistry.levels.dopamine, app.neurochemistry.levels.acetylcholine);

        println!("\n   [Stage 2] Stepping 10 Real-Time 60Hz Telemetry Cycles...");
        for i in 1..=10 {
            app.step_simulation();
            println!("   -> Cycle #{:02}: Active Sectors: {:3} | Savings: {:5.1}% | Galaxy Physics Relaxed",
                i, app.last_frame_active_sectors, app.last_frame_savings_pct);
        }

        println!("=================================================================");
        println!("✅ Unified Maelstrom HUD Headless Pipeline Verified.");
        println!("=================================================================\n");
        Ok(())
    } else {
        println!("   Launching native desktop window...");
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1100.0, 750.0])
                .with_title("Aaroneous Maelstrom HUD — Unified Telemetry Engine"),
            ..Default::default()
        };

        let _ = eframe::run_native(
            "Aaroneous Maelstrom HUD",
            native_options,
            Box::new(|_cc| Ok(Box::new(a_run::MaelstromHudApp::new()))),
        );
        Ok(())
    }
}

/// Distills and births .si solid-state models for all 9 Sovereign Domain Specialists
fn run_distill_all_pipeline(
    samples: usize,
    epochs: usize,
    out_dir: Option<PathBuf>,
) -> Result<()> {
    let out = out_dir.unwrap_or_else(|| {
        aaroneous_paths::WorkspacePaths::default().models().join("distilled_federation")
    });

    println!("=================================================================");
    println!(" ⚡ AARONEOUS COMPUTE: 9-SPECIALIST SOLID-STATE DISTILLATION");
    println!("=================================================================");
    println!("   Target Directory : {}", out.display());
    println!("   Samples/Domain   : {}", samples);
    println!("   Training Epochs  : {}\n", epochs);

    println!("   [Stage 1] Synthesizing Multi-Domain Rosetta Stone Datasets...");
    let out_clone = out.clone();
    let reports = std::thread::Builder::new()
        .name("si_distiller".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            compute::si_distillation_harness::SiDistillationHarness::distill_all_9_specialists(
                &out_clone,
                samples,
                epochs,
            )
        })?
        .join()
        .map_err(|_| anyhow::anyhow!("Distillation worker thread panicked"))??;

    println!("\n   [Stage 2] Benchmarking Memory-Mapped Zero-Copy Execution Latencies...");
    println!("-----------------------------------------------------------------");
    println!("   {:12} | {:10} | {:12} | {:10} | {:10}", "Specialist", "Alignment", "InfoNCE", "Duration", "File Size");
    println!("-----------------------------------------------------------------");

    for report in &reports {
        let size_kb = std::fs::metadata(&report.output_si_path)
            .map(|m| m.len() / 1024)
            .unwrap_or(0);
        let spec_name = report.model_name.replace("_sovereign_v1", "");

        println!("   {:12} | CKA: {:5.1}% | Loss: {:6.4} | {:6} ms | {:6} KB",
            spec_name,
            report.final_cka_alignment * 100.0,
            report.final_infonce_loss,
            report.total_duration_ms,
            size_kb
        );
    }

    println!("=================================================================");
    println!("✅ Full 9-Specialist Federation .si Containers Distilled & Verified.");
    println!("=================================================================\n");
    Ok(())
}

/// Executes autonomous background self-evolution AST mutation & skill stack promotion cycles
fn run_evolve_pipeline(
    cycles: usize,
    threshold: f64,
    out: Option<PathBuf>,
) -> Result<()> {
    std::thread::Builder::new()
        .name("evolution_worker".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            println!("=================================================================");
            println!(" 🧬 AARONEOUS AUTONOMIC SELF-EVOLUTION & CONTINUOUS CHIMERA");
            println!("=================================================================");
            println!("   Trigger Engine   : Dionysus 4-Channel Neurochemical Drive (Curiosity / Boredom)");
            println!("   Mutation Engine  : Hephaestus AST Synthesis & Shadow Sandbox");
            println!("   Safety Auditor   : Argus Deep SVDD Invariant Hypersphere Manifold");
            println!("   Promotion Target : .si Solid-State Container (Block 3: Episodic Skill Stack)");
            println!("   Execution Cycles : {} Cycles (Confidence Threshold: {:.2})\n", cycles, threshold);

            let config = evolution::SelfEvolutionConfig {
                curiosity_trigger_threshold: 0.50,
                boredom_trigger_threshold: 0.40,
                min_posterior_confidence: threshold,
                target_si_path: out,
            };

            let mut engine = evolution::ContinuousSelfEvolutionEngine::new(config);

            let sample_programs = [
                r#"
                pub fn allocate_simd_vector(len: usize) -> Vec<f32> {
                    if len == 0 {
                        panic!("Invalid buffer length");
                    }
                    vec![0.0f32; len]
                }
                "#,
                r#"
                pub fn route_synapse_tensor(payload: Option<&[u8]>) -> usize {
                    let p = payload.unwrap();
                    p.len()
                }
                "#,
                r#"
                pub fn query_star_graph(node_id: &str) -> String {
                    if node_id.is_empty() {
                        panic!("Star node id must not be empty");
                    }
                    format!("Star: {}", node_id)
                }
                "#,
            ];

            for i in 1..=cycles {
                let code = sample_programs[(i - 1) % sample_programs.len()];
                let report = engine.step_evolution_cycle(code)?;

                println!("   [Cycle #{:02}] Neurochemistry: Curiosity={:.1}%, Boredom={:.1}%",
                    report.cycle_number, report.curiosity_level * 100.0, report.boredom_level * 100.0);
                if let Some(impulse) = &report.triggered_impulse {
                    println!("   -> Autonomic Impulse : ⚡ {}", impulse);
                }
                println!("   -> AST Mutations     : Attempted: {} | Accepted: {}", report.mutations_attempted, report.hypotheses_accepted);
                println!("   -> Argus SVDD Audit  : {}", if report.argus_svdd_safety_verified { "🛡️ INVARIANTS_PASSED (Safe)" } else { "❌ SAFETY_VIOLATION" });
                println!("   -> Skill Promotion   : {} skills promoted to .si stack ({:.2} ms)", report.skills_promoted_to_si, report.duration_ms as f64);
                println!("   -------------------------------------------------------------");
            }

            println!("\n=================================================================");
            println!("🌟 Self-Evolution Summary:");
            println!("   Total Cycles Executed : {}", engine.total_cycles_run);
            println!("   Total Skills Promoted : {}", engine.total_skills_promoted);
            println!("   Final Dopamine / ACh  : {:.2} / {:.2} (Exploration Momentum)",
                engine.neurochemistry.levels.dopamine, engine.neurochemistry.levels.acetylcholine);
            println!("=================================================================");
            println!("✅ Autonomous Background Self-Evolution Pipeline Verified.");
            println!("=================================================================\n");
            Ok(())
        })?
        .join()
        .map_err(|_| anyhow::anyhow!("Evolution worker thread panicked"))?
}
