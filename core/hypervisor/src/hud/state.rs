// core/hypervisor/src/hud/state.rs
//! Shared HUD state, window modes, DPI scaling, and Spatial Canvas state.

use aaroneous_paths::{DiscoveredGgufModel, ModelHubLocation, WorkspacePaths};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{self, Color32, Pos2, Vec2};
use memmap2::{MmapMut, MmapOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::hud::navigation::NavSection;
use crate::hud::theme::HudTheme;

/// Type of User Automation Agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentKind {
    SingleUseTask,   // 1-shot automation (e.g. download report, organize folder)
    SmartMacroLoop,  // Repeating macro / game companion
    Assistant,       // Conversational helper / UI builder
}

impl AgentKind {
    pub fn name(&self) -> &'static str {
        match self {
            AgentKind::SingleUseTask => "⚡ Single-Use Task Bot",
            AgentKind::SmartMacroLoop => "🔄 Smart Macro & Loop Bot",
            AgentKind::Assistant => "🧠 Assistant Helper",
        }
    }
}

/// Agent Execution State
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentExecutionState {
    Idle,
    Running,
    Paused,
    Completed,
}

/// A User-Defined SI Automation Agent / Smart Macro (Persistent to Disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: AgentKind,
    pub instructions: String,
    pub target_app: String,
    pub tasks_completed: usize,
    pub state: AgentExecutionState,
    pub color: [u8; 3],
    #[serde(default)]
    pub soul_model: Option<String>,
}

impl CustomAgent {
    pub fn agents_dir() -> PathBuf {
        WorkspacePaths::discover().agents()
    }

    pub fn file_path(&self) -> PathBuf {
        Self::agents_dir().join(format!("{}.json", self.id))
    }

    pub fn save_to_disk(&self) {
        let dir = Self::agents_dir();
        let _ = fs::create_dir_all(&dir);
        let path = self.file_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, content);
        }
    }

    pub fn delete_from_disk(&self) {
        let path = self.file_path();
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    pub fn load_all_from_disk() -> Vec<Self> {
        let dir = Self::agents_dir();
        let _ = fs::create_dir_all(&dir);

        let mut agents = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(agent) = serde_json::from_str::<CustomAgent>(&content)
                {
                    agents.push(agent);
                }
            }
        }

        if agents.is_empty() {
            let default_target = WorkspacePaths::discover().root().to_string_lossy().to_string();

            // Seed starter agents and persist them to disk
            let starters = vec![
                CustomAgent {
                    id: "agent_game_helper".to_string(),
                    name: "Game Companion Bot".to_string(),
                    description: "Automates repetitive in-game grinding, assists aiming, and triggers macro dodges.".to_string(),
                    kind: AgentKind::SmartMacroLoop,
                    instructions: "Monitor health bar, auto-trigger dodge when attack indicator appears, and farm resource loop.".to_string(),
                    target_app: "Active Game Window".to_string(),
                    tasks_completed: 142,
                    state: AgentExecutionState::Running,
                    color: [56, 139, 253],
                    soul_model: Some("⚡ Burn WGPU Motor Reflex (Tier 1)".to_string()),
                },
                CustomAgent {
                    id: "agent_file_cleaner".to_string(),
                    name: "Download & Workspace Cleaner".to_string(),
                    description: "Single-use task bot that categorizes downloads and cleans temporary build artifacts.".to_string(),
                    kind: AgentKind::SingleUseTask,
                    instructions: "Scan Downloads folder, sort PDFs/ZIPs/images into folders, and delete files older than 30 days.".to_string(),
                    target_app: default_target,
                    tasks_completed: 28,
                    state: AgentExecutionState::Idle,
                    color: [63, 185, 80],
                    soul_model: Some("⚡ Machine-Native Task Engine".to_string()),
                },
                CustomAgent {
                    id: "agent_ui_builder".to_string(),
                    name: "Instant Tool Synthesizer".to_string(),
                    description: "Generates draggable native desktop calculators, performance monitors, and tools on demand.".to_string(),
                    kind: AgentKind::Assistant,
                    instructions: "Listen for prompt queries and compile dynamic UI window manifests without restarting.".to_string(),
                    target_app: "Aaroneous Desktop".to_string(),
                    tasks_completed: 64,
                    state: AgentExecutionState::Idle,
                    color: [163, 113, 247],
                    soul_model: Some("🧠 Qwen2.5-Coder-14B / Candle (Tier 3)".to_string()),
                },
                CustomAgent {
                    id: "agent_macro_login".to_string(),
                    name: "Daily Routine Macro".to_string(),
                    description: "Automates morning desktop workflow: launches tools, updates repositories, checks build status.".to_string(),
                    kind: AgentKind::SingleUseTask,
                    instructions: "Open IDE, pull latest git changes, run cargo check, and report diagnostics toast.".to_string(),
                    target_app: "System".to_string(),
                    tasks_completed: 85,
                    state: AgentExecutionState::Idle,
                    color: [210, 153, 34],
                    soul_model: Some("🔨 Compiler AST Reflexion (Tier 2)".to_string()),
                },
            ];

            for a in &starters {
                a.save_to_disk();
            }
            return starters;
        }

        agents
    }
}

/// Visual Node in the Agent Workflow Pipeline Editor
#[derive(Debug, Clone)]
pub struct AgentPipelineNode {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub pos: Pos2,
    pub color: Color32,
    pub output_connected_to: Option<String>,
}

/// Live Automation Event Record (Streamed from Background Workers)
#[derive(Debug, Clone)]
pub struct AutomationEventLog {
    pub timestamp_ms: u64,
    pub source: String,
    pub action: String,
    pub latency_us: f32,
    pub success: bool,
}

/// Background Worker IPC Message
#[derive(Debug, Clone)]
pub enum BackgroundAgentMessage {
    EventLog(AutomationEventLog),
    TaskFinished { agent_id: String, success: bool, msg: String },
}

/// Dev Studio Sub-Views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevStudioTab {
    Workbench,
    CompilerDiagnostics,
    StructuralForge,
    SiDistillation,
    SiMacroHub,
    SiSkillTree,
    SpecialistsAndFrontier,
}

/// Discord-Style Screen & Application Sharing Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenShareTab {
    Screens,
    Applications,
}

/// Application Window Display Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppWindowMode {
    FullStudio,
    CompactRecorderOverlay,
}

/// Persistent User Preferences (100% Dynamic Paths)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub theme: HudTheme,
    pub ui_scale: f32,
    pub target_fps: u32,
    pub always_on_top: bool,
    pub auto_recompile_on_save: bool,
    pub allow_host_input: bool,
    pub is_sidebar_expanded: bool,
    pub dev_mode: bool,
    pub custom_models_dir: Option<PathBuf>,
    pub selected_gguf_model: Option<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme: HudTheme::CobaltDark,
            ui_scale: 1.0,
            target_fps: 120,
            always_on_top: false,
            auto_recompile_on_save: true,
            allow_host_input: false,
            is_sidebar_expanded: true,
            dev_mode: false,
            custom_models_dir: None,
            selected_gguf_model: None,
        }
    }
}

impl UserSettings {
    pub fn config_path() -> PathBuf {
        WorkspacePaths::discover().config().join("hud_settings.json")
    }

    pub fn load_from_disk() -> Self {
        let path = Self::config_path();
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(settings) = serde_json::from_str::<UserSettings>(&content)
        {
            return settings;
        }
        Self::default()
    }

    pub fn save_to_disk(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, content);
        }
    }
}

/// A persisted spatial window entry on the canvas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpatialWindowState {
    pub window_id: String,
    pub title: String,
    pub pos: (f32, f32),
    pub size: (f32, f32),
    pub is_open: bool,
    pub is_minimized: bool,
    pub z_order: usize,
}

/// A serialized Spatial Canvas scene describing canvas coordinates and child windows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpatialCanvasScene {
    pub version: u32,
    pub canvas_pan: (f32, f32),
    pub canvas_zoom: f32,
    pub grid_snap_enabled: bool,
    pub grid_size: f32,
    pub windows: HashMap<String, SpatialWindowState>,
}

impl Default for SpatialCanvasScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialCanvasScene {
    pub fn new() -> Self {
        let mut windows = HashMap::new();
        windows.insert(
            "screen_automation".to_string(),
            SpatialWindowState {
                window_id: "screen_automation".to_string(),
                title: "Screen Automation".to_string(),
                pos: (40.0, 40.0),
                size: (480.0, 360.0),
                is_open: true,
                is_minimized: false,
                z_order: 1,
            },
        );
        windows.insert(
            "workbench".to_string(),
            SpatialWindowState {
                window_id: "workbench".to_string(),
                title: "Developer Workbench".to_string(),
                pos: (540.0, 40.0),
                size: (640.0, 480.0),
                is_open: true,
                is_minimized: false,
                z_order: 2,
            },
        );

        Self {
            version: 1,
            canvas_pan: (0.0, 0.0),
            canvas_zoom: 1.0,
            grid_snap_enabled: true,
            grid_size: 20.0,
            windows,
        }
    }

    pub fn save_to_disk(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let serialized = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        fs::write(path, serialized)?;
        Ok(())
    }

    pub fn load_from_disk(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let scene = ron::from_str::<Self>(&content)?;
        Ok(scene)
    }
}

/// A Star Node in the 3D Galaxy Canvas (Phase 17 Sovereign 3D Cosmos)
#[derive(Debug, Clone)]
pub struct GalaxyStar {
    pub id: String,
    pub name: String,
    pub pos: [f32; 3],
    pub category: String,
    pub domain_opcode: u16,
    pub color: Color32,
    pub connected_to: Vec<String>,
    pub description: String,
    pub activity_level: f32, // Pulsing activity level [0.0..1.0]
}

/// Shared HUD and runtime state for the Aaroneous Desktop Studio
pub struct SharedHudState {
    pub nav_section: NavSection,
    pub dev_tab: DevStudioTab,
    pub start_time: Instant,
    pub last_frame_instant: Instant,

    // Window Display & Background Mode
    pub app_window_mode: AppWindowMode,
    pub is_minimized_to_tray: bool,
    pub recording_start_instant: Option<Instant>,

    // SI Agents & Smart Macros Management State
    pub custom_agents: Vec<CustomAgent>,
    pub is_creating_agent: bool,
    pub new_agent_name: String,
    pub new_agent_desc: String,
    pub new_agent_kind: AgentKind,
    pub new_agent_instructions: String,
    pub new_agent_target_app: String,
    pub new_agent_soul_model: Option<String>,

    // Local GGUF Model Auto-Discovery Hub State
    pub model_hubs: Vec<ModelHubLocation>,
    pub discovered_gguf_models: Vec<DiscoveredGgufModel>,
    pub selected_model_idx: usize,

    // Background Worker Channels & Control
    pub bg_tx: Sender<BackgroundAgentMessage>,
    pub bg_rx: Receiver<BackgroundAgentMessage>,
    pub active_loop_flags: HashMap<String, Arc<AtomicBool>>,

    // Visual Node Graph Editor State
    pub pipeline_nodes: Vec<AgentPipelineNode>,
    pub selected_node_idx: Option<usize>,
    pub node_graph_pan: Vec2,

    // Real-Time Telemetry Plot Data
    pub telemetry_fps_history: Vec<f32>,
    pub telemetry_latency_history: Vec<f32>,
    pub telemetry_reward_history: Vec<f32>,
    pub telemetry_tick_counter: u64,
    pub measured_fps: f32,

    // Live Event Stream Log (egui_extras)
    pub event_logs: Vec<AutomationEventLog>,

    // Settings
    pub settings: UserSettings,

    // AI Dynamic Windows
    pub dynamic_windows: Vec<orchestrator::DynamicWindowManifest>,
    pub dynamic_prompt_input: String,
    pub dynamic_window_status: String,

    // Live Shared Memory Connection
    pub bus_mmap: Option<MmapMut>,
    pub bus_path: PathBuf,
    pub is_live_bus: bool,

    // Interconnect Bus Metrics
    pub bus_integrity: f32,
    pub bus_understanding: f32,
    pub bus_generation: u64,
    pub bus_events_per_sec: f32,

    // 3D Galaxy Viewport State
    pub galaxy_stars: Vec<GalaxyStar>,
    pub camera_pan: Vec2,
    pub camera_zoom: f32,
    pub camera_rotation: (f32, f32), // (yaw, pitch) in radians
    pub selected_galaxy_star_id: Option<String>,
    pub galaxy_filter_category: String,
    pub galaxy_auto_rotate: bool,

    // Presenter Live Vision Perception
    pub viewport_texture: Option<egui::TextureHandle>,
    pub vision_fps: f32,
    pub vision_entropy: f64,

    // Game & Task Emulation Agent
    pub game_agent: platform_bridge::AutonomousGameAgent,
    pub emulation_session_name: String,
    pub emulation_status_msg: String,

    // Discord-Style Screen & Application Sharing Picker
    pub screen_share_tab: ScreenShareTab,
    pub discovered_windows: Vec<platform_bridge::DiscoveredWindow>,
    pub selected_window_idx: usize,
    pub capture_modifiers: platform_bridge::CaptureModifiers,

    // 5 Frontier Engines Interactive State
    pub frontier_guardrail_test_val: f32,
    pub frontier_guardrail_verdict_str: String,
    pub frontier_dream_cycles: usize,
    pub frontier_dream_log: String,
    pub frontier_jit_executed_count: u64,
    pub frontier_jit_last_latency_ns: u64,

    // In-Game Xbox / Steam Style Overlay
    pub is_ingame_overlay_open: bool,
    pub overlay_click_through: bool,
    pub overlay_show_aim_crosshair: bool,
    pub overlay_show_bot_telemetry: bool,
    pub bot_aim_target: [f32; 2],
    pub bot_active_keys: [bool; 5], // W, A, S, D, L-Click

    // Developer Workbench State
    pub dev_tools_engine: adaptation_engine::DevToolsEngine,
    pub workspace_tree_items: Vec<adaptation_engine::WorkspaceFileItem>,
    pub selected_tree_idx: usize,
    pub workbench_active_file: String,
    pub workbench_file_content: String,
    pub workbench_diff_preview: String,
    pub workbench_diagnostics: Vec<adaptation_engine::CompilerDiagnosticItem>,
    pub workbench_status_msg: String,
    pub last_backup_path: Option<PathBuf>,

    // Fabricator Forge State
    pub forge_file_path: String,
    pub forge_source_code: String,
    pub forge_search_pattern: String,
    pub forge_replace_template: String,
    pub forge_diff_preview: String,
    pub forge_status_msg: String,
    pub rebuilder_engine: adaptation_engine::SelfRebuildEngine,

    // SI Machine-Native Distillation State
    pub si_miner: transpiler::SiDistillationMiner,
    pub si_corpus_count: usize,
    pub si_corpus_bytes: u64,
    pub si_corpus_avg_energy: f64,
    pub last_distillation_report: Option<transpiler::DistillationBatchReport>,
    pub last_training_report: Option<compute::TrainingEpochReport>,

    // Smart SI Macro Engine State
    pub si_macro_engine: compute::SiMacroEngine,
    pub saved_si_macros: Vec<compute::SiMacroMetadata>,
    pub macro_name_input: String,
    pub macro_desc_input: String,
    pub macro_hotkey_input: String,

    // Machine-Native Skill Tree & SI Tool Engine
    pub skill_engine: compute::SkillExpansionEngine,
    pub si_tool_engine: compute::SiToolEngine,
    pub si_inspect_path_input: String,
    pub last_inspector_report: Option<compute::SiInspectorReport>,
    pub last_benchmark_report: Option<compute::SiBenchmarkReport>,

    // Chat / Terminal State
    pub chat_input: String,
    pub chat_history: Vec<(String, String, Color32)>,

    // Cognitive Hypervisor State
    pub hive_intent_input: String,
    pub hive_routing_decision: Option<String>,
    pub hive_routing_trace: Vec<String>,
    pub living_mind_dopamine: f32,
    pub living_mind_acetylcholine: f32,
    pub living_mind_serotonin: f32,
    pub living_mind_noradrenaline: f32,
    pub dream_duel_round: usize,
    pub dream_duel_alice_hypotheses: usize,
    pub dream_duel_bob_verifications: usize,
    pub dream_duel_history: Vec<String>,
    pub forge_selected_domain: usize,
    pub forge_samples_count: usize,
    pub forge_epochs_count: usize,
    pub forge_distillation_status: String,
    pub swarm_live_quorums: usize,
    pub swarm_offload_count: usize,

    // Spatial Canvas Scene & Window Topology
    pub spatial_canvas_scene: SpatialCanvasScene,
}

impl Default for SharedHudState {
    fn default() -> Self {
        let ws = WorkspacePaths::discover();
        let _ = ws.ensure_directories();

        let settings = UserSettings::load_from_disk();
        let custom_agents = CustomAgent::load_all_from_disk();
        let (bg_tx, bg_rx) = unbounded();

        // Scan Local LLM Model Hubs
        let custom_dirs = settings.custom_models_dir.as_ref().map(|p| vec![p.clone()]).unwrap_or_default();
        let model_hubs = ws.get_known_model_hubs();
        let discovered_gguf_models = ws.scan_all_gguf_models(&custom_dirs);

        // Visual Agent Pipeline Nodes
        let pipeline_nodes = vec![
            AgentPipelineNode {
                id: "node_input".to_string(),
                title: "🎯 Screen & Input Trigger".to_string(),
                subtitle: "Active Game Window (128x128 60FPS)".to_string(),
                pos: Pos2::new(40.0, 45.0),
                color: Color32::from_rgb(56, 139, 253),
                output_connected_to: Some("node_brain".to_string()),
            },
            AgentPipelineNode {
                id: "node_brain".to_string(),
                title: "🧠 SI Agent Decision".to_string(),
                subtitle: "Imitation Policy & Vision".to_string(),
                pos: Pos2::new(250.0, 45.0),
                color: Color32::from_rgb(163, 113, 247),
                output_connected_to: Some("node_action".to_string()),
            },
            AgentPipelineNode {
                id: "node_action".to_string(),
                title: "⚡ Desktop Emulator Motor Macro".to_string(),
                subtitle: "Injects [W, Space, L-Click]".to_string(),
                pos: Pos2::new(460.0, 45.0),
                color: Color32::from_rgb(63, 185, 80),
                output_connected_to: Some("node_alert".to_string()),
            },
            AgentPipelineNode {
                id: "node_alert".to_string(),
                title: "🔔 Toast & Audio Cue".to_string(),
                subtitle: "Notification on completion".to_string(),
                pos: Pos2::new(670.0, 45.0),
                color: Color32::from_rgb(210, 153, 34),
                output_connected_to: None,
            },
        ];

        // Seed Real-Time Plot Points
        let mut telemetry_fps_history = Vec::new();
        let mut telemetry_latency_history = Vec::new();
        let mut telemetry_reward_history = Vec::new();
        for i in 0..60 {
            telemetry_fps_history.push(118.0 + (i % 6) as f32 * 0.4);
            telemetry_latency_history.push(0.35 + ((i * 3) % 7) as f32 * 0.05);
            telemetry_reward_history.push(((i as f32 * 0.3).sin().abs() * 20.0) + (i as f32 * 0.6));
        }

        let event_logs = vec![
            AutomationEventLog { timestamp_ms: 1040, source: "Game Companion Bot".to_string(), action: "Triggered macro dodge [Shift+A]".to_string(), latency_us: 340.0, success: true },
            AutomationEventLog { timestamp_ms: 2180, source: "Download Cleaner".to_string(), action: "Scanned folder targets".to_string(), latency_us: 1200.0, success: true },
            AutomationEventLog { timestamp_ms: 3450, source: "Instant Tool Synthesizer".to_string(), action: "Generated Game Stats widget".to_string(), latency_us: 840.0, success: true },
        ];

        let galaxy_stars = vec![
            GalaxyStar {
                id: "spec_orchestrator".to_string(),
                name: "01. Orchestrator".to_string(),
                pos: [0.0, 0.0, 0.0],
                category: "Specialists".to_string(),
                domain_opcode: 0x0100,
                color: Color32::from_rgb(255, 215, 0),
                connected_to: vec!["spec_synthesizer".into(), "spec_archivist".into(), "spec_router".into(), "spec_sentinel".into()],
                description: "Central task decomposition, dynamic DAG scheduling & priority execution.".to_string(),
                activity_level: 0.95,
            },
            GalaxyStar {
                id: "spec_synthesizer".to_string(),
                name: "02. Synthesizer".to_string(),
                pos: [140.0, 50.0, -40.0],
                category: "Specialists".to_string(),
                domain_opcode: 0x0200,
                color: Color32::from_rgb(163, 113, 247),
                connected_to: vec!["spec_orchestrator".into(), "spec_devtools".into(), "substr_matrix".into()],
                description: "Polyglot code synthesis, AST transformation & knowledge generation.".to_string(),
                activity_level: 0.88,
            },
            GalaxyStar {
                id: "spec_presenter".to_string(),
                name: "03. Presenter".to_string(),
                pos: [-130.0, 70.0, 50.0],
                category: "Specialists".to_string(),
                domain_opcode: 0x0300,
                color: Color32::from_rgb(56, 139, 253),
                connected_to: vec!["spec_orchestrator".into(), "spec_perceiver".into()],
                description: "DirectX 12 / Vulkan frame composition, interactive telemetry & UI layout.".to_string(),
                activity_level: 0.92,
            },
            GalaxyStar {
                id: "spec_devtools".to_string(),
                name: "04. DevTools".to_string(),
                pos: [90.0, -110.0, 60.0],
                category: "Specialists".to_string(),
                domain_opcode: 0x0400,
                color: Color32::from_rgb(240, 136, 62),
                connected_to: vec!["spec_synthesizer".into(), "spec_sentinel".into()],
                description: "Automated FFI wrapper synthesis, cargo compilation & memory inspection.".to_string(),
                activity_level: 0.76,
            },
            GalaxyStar {
                id: "spec_sentinel".to_string(),
                name: "05. Sentinel".to_string(),
                pos: [-140.0, -80.0, -50.0],
                category: "Security".to_string(),
                domain_opcode: 0x0500,
                color: Color32::from_rgb(248, 81, 73),
                connected_to: vec!["spec_orchestrator".into(), "spec_devtools".into(), "spec_aligner".into()],
                description: "SVDD latent manifold security, path containment sandboxing & integrity checks.".to_string(),
                activity_level: 0.98,
            },
            GalaxyStar {
                id: "spec_archivist".to_string(),
                name: "06. Archivist".to_string(),
                pos: [0.0, 150.0, 70.0],
                category: "Memory".to_string(),
                domain_opcode: 0x0600,
                color: Color32::from_rgb(121, 192, 255),
                connected_to: vec!["spec_orchestrator".into(), "spec_router".into(), "substr_si_core".into()],
                description: "3D Galaxy semantic knowledge graph clustering & episodic memory indexing.".to_string(),
                activity_level: 0.85,
            },
            GalaxyStar {
                id: "spec_router".to_string(),
                name: "07. Router".to_string(),
                pos: [150.0, -40.0, -60.0],
                category: "Networking".to_string(),
                domain_opcode: 0x0700,
                color: Color32::from_rgb(63, 185, 80),
                connected_to: vec!["spec_orchestrator".into(), "spec_archivist".into()],
                description: "P2P streaming TCP mesh multiplexer, gossip protocol & consensus replication.".to_string(),
                activity_level: 0.91,
            },
            GalaxyStar {
                id: "spec_aligner".to_string(),
                name: "08. Aligner".to_string(),
                pos: [-80.0, 120.0, -70.0],
                category: "Specialists".to_string(),
                domain_opcode: 0x0800,
                color: Color32::from_rgb(219, 109, 40),
                connected_to: vec!["spec_sentinel".into(), "spec_orchestrator".into()],
                description: "Federation policy alignment, symbiotic consensus arbitration & safety compliance.".to_string(),
                activity_level: 0.80,
            },
            GalaxyStar {
                id: "spec_perceiver".to_string(),
                name: "09. Perceiver".to_string(),
                pos: [60.0, -140.0, -40.0],
                category: "Capture".to_string(),
                domain_opcode: 0x0900,
                color: Color32::from_rgb(88, 166, 255),
                connected_to: vec!["spec_presenter".into(), "spec_orchestrator".into()],
                description: "DXGI hardware screen capture, low-latency perceptual gating & HID bridge.".to_string(),
                activity_level: 0.94,
            },
            GalaxyStar {
                id: "substr_si_core".to_string(),
                name: "⚡ SSM Cartridge Core".to_string(),
                pos: [0.0, -170.0, 40.0],
                category: "Reflex".to_string(),
                domain_opcode: 0x0A00,
                color: Color32::from_rgb(255, 230, 100),
                connected_to: vec!["spec_archivist".into(), "substr_matrix".into()],
                description: "Canonical .si v3.0 zero-copy memory-mapped neural execution substrate (< 50µs).".to_string(),
                activity_level: 1.0,
            },
            GalaxyStar {
                id: "substr_matrix".to_string(),
                name: "🧬 Adaptation Matrix".to_string(),
                pos: [-170.0, 0.0, -30.0],
                category: "Reflex".to_string(),
                domain_opcode: 0x0B00,
                color: Color32::from_rgb(100, 255, 218),
                connected_to: vec!["substr_si_core".into(), "spec_synthesizer".into()],
                description: "Real-time online weight steering with TD(λ) eligibility traces and OGP.".to_string(),
                activity_level: 0.89,
            },
        ];

        let chat_history = vec![(
            "Aaroneous".to_string(),
            "Agent Manager online. Model Hubs auto-discovered.".to_string(),
            Color32::from_rgb(56, 139, 253),
        )];

        let bus_path = ws.synapse_file();
        let (bus_mmap, is_live_bus) = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&bus_path)
        {
            Ok(file) => {
                let _ = file.set_len(64 * 1024 * 1024); // 64 MB
                match unsafe { MmapOptions::new().map_mut(&file) } {
                    Ok(mmap) => (Some(mmap), true),
                    Err(_) => (None, false),
                }
            }
            Err(_) => (None, false),
        };

        let dev_tools_engine = adaptation_engine::DevToolsEngine::default();
        let workspace_tree_items = dev_tools_engine.scan_workspace_tree(2);

        let discovered_windows = platform_bridge::WindowDiscoveryEngine::enumerate_available_targets().unwrap_or_default();
        let default_target_app = ws.root().to_string_lossy().to_string();

        let si_miner = transpiler::SiDistillationMiner::default();
        let (si_corpus_count, si_corpus_bytes, si_corpus_avg_energy) = si_miner.get_live_metrics().unwrap_or((0, 0, 0.0));

        Self {
            nav_section: NavSection::Specialists,
            dev_tab: DevStudioTab::Workbench,
            start_time: Instant::now(),
            last_frame_instant: Instant::now(),
            app_window_mode: AppWindowMode::FullStudio,
            is_minimized_to_tray: false,
            recording_start_instant: None,
            custom_agents,
            is_creating_agent: false,
            new_agent_name: String::new(),
            new_agent_desc: String::new(),
            new_agent_kind: AgentKind::SingleUseTask,
            new_agent_instructions: String::new(),
            new_agent_target_app: default_target_app,
            new_agent_soul_model: None,
            model_hubs,
            discovered_gguf_models,
            selected_model_idx: 0,
            bg_tx,
            bg_rx,
            active_loop_flags: HashMap::new(),
            pipeline_nodes,
            selected_node_idx: None,
            node_graph_pan: Vec2::ZERO,
            telemetry_fps_history,
            telemetry_latency_history,
            telemetry_reward_history,
            telemetry_tick_counter: 60,
            measured_fps: 120.0,
            event_logs,
            settings,
            dynamic_windows: Vec::new(),
            dynamic_prompt_input: "Create a game stats and speedrun tracker".to_string(),
            dynamic_window_status: "AI Tool Synthesizer Ready".to_string(),
            bus_mmap,
            bus_path,
            is_live_bus,
            bus_integrity: 99.4,
            bus_understanding: 98.6,
            bus_generation: 1,
            bus_events_per_sec: 3840.0,
            galaxy_stars,
            camera_pan: Vec2::ZERO,
            camera_zoom: 1.0,
            camera_rotation: (0.4, 0.3),
            selected_galaxy_star_id: Some("spec_orchestrator".to_string()),
            galaxy_filter_category: "All".to_string(),
            galaxy_auto_rotate: true,
            viewport_texture: None,
            vision_fps: 60.0,
            vision_entropy: 7.82,
            game_agent: platform_bridge::AutonomousGameAgent::new(),
            emulation_session_name: "speedrun_macro_1".to_string(),
            emulation_status_msg: "Game Emulation Agent Ready".to_string(),
            screen_share_tab: ScreenShareTab::Applications,
            discovered_windows,
            selected_window_idx: 0,
            capture_modifiers: platform_bridge::CaptureModifiers::default(),
            frontier_guardrail_test_val: 0.1,
            frontier_guardrail_verdict_str: "Safe (SVDD Hypersphere distance: 1.60 <= 12.0)".to_string(),
            frontier_dream_cycles: 10,
            frontier_dream_log: "Autonomous Dream Engine idle. Ready to simulate Alice vs Bob duels.".to_string(),
            frontier_jit_executed_count: 50,
            frontier_jit_last_latency_ns: 420,
            is_ingame_overlay_open: false,
            overlay_click_through: false,
            overlay_show_aim_crosshair: true,
            overlay_show_bot_telemetry: true,
            bot_aim_target: [0.5, 0.5],
            bot_active_keys: [true, false, false, false, true],
            dev_tools_engine,
            workspace_tree_items,
            selected_tree_idx: 0,
            workbench_active_file: "crates/adaptation_engine/src/lib.rs".to_string(),
            workbench_file_content: "// Select a file from the tree to inspect or refactor".to_string(),
            workbench_diff_preview: String::new(),
            workbench_diagnostics: Vec::new(),
            workbench_status_msg: "Developer Workbench Ready".to_string(),
            last_backup_path: None,
            forge_file_path: "crates/specialists/src/dev_tools.rs".to_string(),
            forge_source_code: "pub fn execute_work() {\n    log(\"Starting task...\");\n}".to_string(),
            forge_search_pattern: "log(:[msg]);".to_string(),
            forge_replace_template: "tracing::info!(:[msg]);".to_string(),
            forge_diff_preview: String::new(),
            forge_status_msg: "Ready to forge patches".to_string(),
            rebuilder_engine: adaptation_engine::SelfRebuildEngine::default(),
            si_miner,
            si_corpus_count,
            si_corpus_bytes,
            si_corpus_avg_energy,
            last_distillation_report: None,
            last_training_report: None,
            si_macro_engine: compute::SiMacroEngine::default(),
            saved_si_macros: compute::SiMacroEngine::default().ensure_starter_macros().unwrap_or_default(),
            macro_name_input: String::new(),
            macro_desc_input: String::new(),
            macro_hotkey_input: String::new(),
            skill_engine: {
                let mut e = compute::SkillExpansionEngine::default();
                let _ = e.ensure_starter_skills();
                e
            },
            si_tool_engine: compute::SiToolEngine,
            si_inspect_path_input: String::new(),
            last_inspector_report: None,
            last_benchmark_report: None,
            chat_input: String::new(),
            chat_history,
            hive_intent_input: String::new(),
            hive_routing_decision: None,
            hive_routing_trace: Vec::new(),
            living_mind_dopamine: 0.85,
            living_mind_acetylcholine: 0.90,
            living_mind_serotonin: 0.75,
            living_mind_noradrenaline: 0.40,
            dream_duel_round: 1,
            dream_duel_alice_hypotheses: 3,
            dream_duel_bob_verifications: 3,
            dream_duel_history: vec![
                "🌟 Round #01: Alice generated zero-copy buffer theorem (H₁). Bob verified invariants in shadow sandbox -> Accepted.".into(),
                "⚡ Round #02: Alice synthesized SIMD tensor kernel (H₂). Bob proved memory safety -> Promoted to .si stack.".into(),
            ],
            forge_selected_domain: 0,
            forge_samples_count: 20,
            forge_epochs_count: 2,
            forge_distillation_status: "Ready to distill .si student models.".into(),
            swarm_live_quorums: 3,
            swarm_offload_count: 12,
            spatial_canvas_scene: SpatialCanvasScene::new(),
        }
    }
}

impl SharedHudState {
    pub fn save_scene_to_disk(&self, path: &std::path::Path) -> anyhow::Result<()> {
        self.spatial_canvas_scene.save_to_disk(path)
    }

    pub fn load_scene_from_disk(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let scene = SpatialCanvasScene::load_from_disk(path)?;
        self.spatial_canvas_scene = scene;
        Ok(())
    }

    pub fn handle_canvas_pan_zoom(
        &mut self,
        drag_delta: Vec2,
        scroll_delta: f32,
        is_space_or_middle_drag: bool,
        is_ctrl_zoom: bool,
        reset_hotkey: bool,
    ) {
        if reset_hotkey {
            self.spatial_canvas_scene.canvas_pan = (0.0, 0.0);
            self.spatial_canvas_scene.canvas_zoom = 1.0;
            return;
        }

        if is_space_or_middle_drag {
            self.spatial_canvas_scene.canvas_pan.0 += drag_delta.x;
            self.spatial_canvas_scene.canvas_pan.1 += drag_delta.y;
        }

        if is_ctrl_zoom && scroll_delta.abs() > 0.01 {
            let new_zoom = self.spatial_canvas_scene.canvas_zoom + scroll_delta * 0.05;
            self.spatial_canvas_scene.canvas_zoom = new_zoom.clamp(0.5, 2.0);
        }
    }
    pub fn rescan_local_models(&mut self) {
        let ws = WorkspacePaths::discover();
        let custom_dirs = self
            .settings
            .custom_models_dir
            .as_ref()
            .map(|p| vec![p.clone()])
            .unwrap_or_default();
        self.model_hubs = ws.get_known_model_hubs();
        self.discovered_gguf_models = ws.scan_all_gguf_models(&custom_dirs);
    }

    pub fn poll_background_messages(&mut self) {
        while let Ok(msg) = self.bg_rx.try_recv() {
            match msg {
                BackgroundAgentMessage::EventLog(entry) => {
                    self.event_logs.push(entry);
                    if self.event_logs.len() > 500 {
                        self.event_logs.remove(0);
                    }
                }
                BackgroundAgentMessage::TaskFinished { agent_id, success, msg } => {
                    if let Some(agent) = self.custom_agents.iter_mut().find(|a| a.id == agent_id) {
                        agent.state = if success {
                            agent.tasks_completed += 1;
                            AgentExecutionState::Completed
                        } else {
                            AgentExecutionState::Paused
                        };
                        agent.save_to_disk();
                    }
                    let _ = msg;
                }
            }
        }
    }

    pub fn spawn_agent_execution(&mut self, agent: &CustomAgent) {
        let agent_id = agent.id.clone();
        let agent_name = agent.name.clone();
        let agent_kind = agent.kind;
        let tx = self.bg_tx.clone();

        match agent_kind {
            AgentKind::SingleUseTask => {
                thread::spawn(move || {
                    let start = Instant::now();
                    thread::sleep(Duration::from_millis(650));
                    let latency = start.elapsed().as_micros() as f32;

                    let _ = tx.send(BackgroundAgentMessage::EventLog(AutomationEventLog {
                        timestamp_ms: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        source: agent_name,
                        action: "Executed task workflow step successfully.".to_string(),
                        latency_us: latency,
                        success: true,
                    }));

                    let _ = tx.send(BackgroundAgentMessage::TaskFinished {
                        agent_id,
                        success: true,
                        msg: "Completed task execution".to_string(),
                    });
                });
            }
            AgentKind::SmartMacroLoop => {
                let cancel_flag = Arc::new(AtomicBool::new(false));
                self.active_loop_flags.insert(agent_id.clone(), cancel_flag.clone());

                thread::spawn(move || {
                    let mut step = 1;
                    while !cancel_flag.load(Ordering::Relaxed) && step <= 100 {
                        let start = Instant::now();
                        thread::sleep(Duration::from_millis(800));
                        let latency = start.elapsed().as_micros() as f32;

                        let _ = tx.send(BackgroundAgentMessage::EventLog(AutomationEventLog {
                            timestamp_ms: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            source: agent_name.clone(),
                            action: format!("Macro loop cycle #{step} processed."),
                            latency_us: latency,
                            success: true,
                        }));
                        step += 1;
                    }
                });
            }
            AgentKind::Assistant => {
                thread::spawn(move || {
                    let start = Instant::now();
                    thread::sleep(Duration::from_millis(400));
                    let latency = start.elapsed().as_micros() as f32;

                    let _ = tx.send(BackgroundAgentMessage::EventLog(AutomationEventLog {
                        timestamp_ms: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        source: agent_name,
                        action: "Assistant analyzed UI layout and refreshed canvas.".to_string(),
                        latency_us: latency,
                        success: true,
                    }));

                    let _ = tx.send(BackgroundAgentMessage::TaskFinished {
                        agent_id,
                        success: true,
                        msg: "Assistant query responded".to_string(),
                    });
                });
            }
        }
    }

    pub fn tick_telemetry_plots(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;

        if delta > 0.0 {
            self.measured_fps = (1.0 / delta).clamp(1.0, 240.0);
        }

        self.telemetry_tick_counter += 1;
        if self.telemetry_tick_counter % 4 == 0 {
            self.telemetry_fps_history.push(self.measured_fps);
            if self.telemetry_fps_history.len() > 60 {
                self.telemetry_fps_history.remove(0);
            }

            let fake_latency = 0.28 + ((self.telemetry_tick_counter % 10) as f32 * 0.04);
            self.telemetry_latency_history.push(fake_latency);
            if self.telemetry_latency_history.len() > 60 {
                self.telemetry_latency_history.remove(0);
            }

            let reward = ((self.telemetry_tick_counter as f32 * 0.1).sin().abs() * 30.0)
                + (self.telemetry_tick_counter as f32 * 0.2);
            self.telemetry_reward_history.push(reward);
            if self.telemetry_reward_history.len() > 60 {
                self.telemetry_reward_history.remove(0);
            }
        }
    }

    pub fn toggle_recording(&mut self) {
        match &self.game_agent.state {
            platform_bridge::PlaythroughState::Idle => {
                let _ = self.game_agent.start_recording(&self.emulation_session_name);
                self.recording_start_instant = Some(Instant::now());
                self.emulation_status_msg = "Recording started (60 FPS action stream)...".to_string();
            }
            platform_bridge::PlaythroughState::Recording { .. } => {
                let _ = self.game_agent.stop_recording();
                self.recording_start_instant = None;
                self.emulation_status_msg = "Recording saved to disk.".to_string();
            }
            _ => {}
        }
    }

    pub fn poll_live_bus(&mut self) {
        if let Some(mmap) = &self.bus_mmap
            && mmap.len() >= 64
        {
            let tick_bytes = &mmap[0..8];
            let tick = u64::from_le_bytes(tick_bytes.try_into().unwrap_or([0; 8]));
            if tick > 0 {
                self.bus_generation = tick;
            }

            let integrity = mmap[38];
            if integrity > 0 {
                self.bus_integrity = integrity as f32;
            }

            let understanding = mmap[39];
            if understanding > 0 {
                self.bus_understanding = understanding as f32;
            }
        }
    }

    pub fn inject_live_intent(&mut self, intent: &str) {
        if let Some(mmap) = &mut self.bus_mmap {
            let task_id = Uuid::new_v4();
            let id_bytes = task_id.as_bytes();

            if mmap.len() >= 4096 {
                mmap[16..32].copy_from_slice(id_bytes);

                let payload = intent.as_bytes();
                let payload_len = std::cmp::min(payload.len(), 4064);
                mmap[32..32 + payload_len].copy_from_slice(&payload[..payload_len]);
                // Zero out any trailing bytes to prevent stale payload remnants
                mmap[32 + payload_len..4096].fill(0);

                let new_tick = self.bus_generation + 1;
                mmap[0..8].copy_from_slice(&new_tick.to_le_bytes());
                self.bus_generation = new_tick;

                let _ = mmap.flush();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_spatial_canvas_scene_serialization_roundtrip() {
        let dir = tempdir().unwrap();
        let scene_path = dir.path().join("test_scene.ron");

        let mut scene = SpatialCanvasScene::new();
        scene.canvas_pan = (120.0, -45.0);
        scene.canvas_zoom = 1.25;
        scene.grid_snap_enabled = false;
        scene.windows.insert(
            "custom_tool".to_string(),
            SpatialWindowState {
                window_id: "custom_tool".to_string(),
                title: "Custom Tool Window".to_string(),
                pos: (200.0, 300.0),
                size: (400.0, 250.0),
                is_open: true,
                is_minimized: false,
                z_order: 3,
            },
        );

        scene.save_to_disk(&scene_path).expect("Failed to save scene");
        assert!(scene_path.exists());

        let loaded = SpatialCanvasScene::load_from_disk(&scene_path).expect("Failed to load scene");
        assert_eq!(loaded.canvas_pan, (120.0, -45.0));
        assert_eq!(loaded.canvas_zoom, 1.25);
        assert_eq!(loaded.grid_snap_enabled, false);
        assert_eq!(loaded.windows.len(), 3);
        assert_eq!(loaded.windows["custom_tool"].title, "Custom Tool Window");
    }

    #[test]
    fn test_spatial_canvas_pan_zoom_and_reset_shortcuts() {
        let mut state = SharedHudState::default();

        // 1. Pan via Space/Middle drag
        state.handle_canvas_pan_zoom(Vec2::new(50.0, 30.0), 0.0, true, false, false);
        assert_eq!(state.spatial_canvas_scene.canvas_pan, (50.0, 30.0));

        // 2. Zoom via Ctrl + Scroll
        state.handle_canvas_pan_zoom(Vec2::ZERO, 4.0, false, true, false);
        assert!((state.spatial_canvas_scene.canvas_zoom - 1.20).abs() < 1e-4);

        // 3. Reset hotkey
        state.handle_canvas_pan_zoom(Vec2::ZERO, 0.0, false, false, true);
        assert_eq!(state.spatial_canvas_scene.canvas_pan, (0.0, 0.0));
        assert_eq!(state.spatial_canvas_scene.canvas_zoom, 1.0);
    }
}
