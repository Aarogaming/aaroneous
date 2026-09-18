// core/hypervisor/src/hud/state_snapshot.rs
//! Lock-Free Read Snapshot & Engine State Projection for Studio, Console, and HUD.
//!
//! Enforces lightweight, zero-copy presentation state sharing:
//! 1. EngineSnapshot: Immutable point-in-time state updated by the hypervisor core.
//! 2. StudioProjection: Workstation metrics, file tree status, active diagnostics.
//! 3. ConsoleProjection: Immersive 10-foot telemetry, harmony score, user profile & level.
//! 4. HudProjection: Lightweight situational awareness ticker, active bot indicators, FPS.

use core_contracts::EngineSnapshotPod;
use ipc_bus::SwmrSnapshotPublisher;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Dynamic resource pacing mode regulating shell rendering budgets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GovernorPacing {
    /// Full performance target (120+ FPS) when thermal/VRAM headroom is nominal
    #[default]
    FullPerformance,
    /// Balanced frame pacing (60 FPS) under moderate thermal load or active background inference
    ThermalThrottled,
    /// Critical power/VRAM preservation (30 FPS, idle at 4 FPS) during heavy local LLM generation
    CriticalVramSave,
}

impl GovernorPacing {
    /// Target frame duration in milliseconds
    pub fn target_frame_ms(&self) -> u64 {
        match self {
            Self::FullPerformance => 8,   // ~120 FPS
            Self::ThermalThrottled => 16, // ~60 FPS
            Self::CriticalVramSave => 33, // ~30 FPS
        }
    }

    /// Target FPS numeric cap
    pub fn target_fps(&self) -> f32 {
        match self {
            Self::FullPerformance => 120.0,
            Self::ThermalThrottled => 60.0,
            Self::CriticalVramSave => 30.0,
        }
    }
}

/// Compute-driven metrics for constellation / star graph nodes
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeMetrics {
    pub entropy: f64,    // Shannon entropy of node state
    pub confidence: f64, // Bayesian confidence score (0.0-1.0)
    pub load_risk: f64,  // Monte Carlo predicted risk (0.0-1.0)
    pub centrality: f64, // Graph centrality score
    pub mdp_value: f64,  // MDP state value estimate
}

/// Headless point-in-time constellation canvas state for the hypervisor engine
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpatialCanvasState {
    pub node_metrics: std::collections::HashMap<String, NodeMetrics>,
}

impl SpatialCanvasState {
    pub fn new() -> Self {
        Self {
            node_metrics: std::collections::HashMap::new(),
        }
    }

    pub fn update_node_metrics(&mut self, node_id: impl Into<String>, metrics: NodeMetrics) {
        self.node_metrics.insert(node_id.into(), metrics);
    }
}

/// Point-in-time snapshot of the hypervisor core engine state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSnapshot {
    pub timestamp_ms: u64,
    pub measured_fps: f32,
    pub bus_integrity: f32,
    pub bus_understanding: f32,
    pub bus_generation: u64,
    pub active_specialist: String,
    pub active_companions_count: usize,
    pub running_macros_count: usize,
    pub last_event_desc: String,
    pub user_level: u32,
    pub user_xp: u64,
    pub flow_score: f32,
    pub active_profile_name: String,
    pub pacing: GovernorPacing,
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            measured_fps: 120.0,
            bus_integrity: 99.4,
            bus_understanding: 98.6,
            bus_generation: 1,
            active_specialist: "Orchestrator".to_string(),
            active_companions_count: 0,
            running_macros_count: 0,
            last_event_desc: "Core initialized and nominal".to_string(),
            user_level: 1,
            user_xp: 0,
            flow_score: 0.85,
            active_profile_name: "Default Operator".to_string(),
            pacing: GovernorPacing::FullPerformance,
        }
    }
}

/// Specialized view projection for Studio Mode (Interactive Workstation)
#[derive(Debug, Clone)]
pub struct StudioProjection {
    pub measured_fps: f32,
    pub bus_integrity: f32,
    pub active_companions_count: usize,
    pub last_event: String,
}

/// Specialized view projection for Console Mode (10-Foot Immersive Launcher)
#[derive(Debug, Clone)]
pub struct ConsoleProjection {
    pub display_badge: String,
    pub harmony_label: String,
    pub user_badge: String,
    pub level_badge: String,
}

/// Specialized view projection for HUD Mode (Transparent Overlay & Intercom)
#[derive(Debug, Clone)]
pub struct HudProjection {
    pub active_guidance: String,
    pub measured_fps: f32,
    pub is_nominal: bool,
}

impl From<&EngineSnapshot> for EngineSnapshotPod {
    fn from(snap: &EngineSnapshot) -> Self {
        let mut pod = Self {
            timestamp_ms: snap.timestamp_ms,
            bus_generation: snap.bus_generation,
            user_xp: snap.user_xp,
            measured_fps: snap.measured_fps,
            bus_integrity: snap.bus_integrity,
            bus_understanding: snap.bus_understanding,
            flow_score: snap.flow_score,
            user_level: snap.user_level,
            active_companions_count: snap.active_companions_count as u32,
            running_macros_count: snap.running_macros_count as u32,
            pacing: match snap.pacing {
                GovernorPacing::FullPerformance => 0,
                GovernorPacing::ThermalThrottled => 1,
                GovernorPacing::CriticalVramSave => 2,
            },
            active_specialist: [0; 32],
            active_profile_name: [0; 32],
            last_event_desc: [0; 128],
        };
        pod.set_active_specialist(&snap.active_specialist);
        pod.set_active_profile_name(&snap.active_profile_name);
        pod.set_last_event_desc(&snap.last_event_desc);
        pod
    }
}

impl From<&EngineSnapshotPod> for EngineSnapshot {
    fn from(pod: &EngineSnapshotPod) -> Self {
        Self {
            timestamp_ms: pod.timestamp_ms,
            measured_fps: pod.measured_fps,
            bus_integrity: pod.bus_integrity,
            bus_understanding: pod.bus_understanding,
            bus_generation: pod.bus_generation,
            active_specialist: pod.active_specialist_str().to_string(),
            active_companions_count: pod.active_companions_count as usize,
            running_macros_count: pod.running_macros_count as usize,
            last_event_desc: pod.last_event_desc_str().to_string(),
            user_level: pod.user_level,
            user_xp: pod.user_xp,
            flow_score: pod.flow_score,
            active_profile_name: pod.active_profile_name_str().to_string(),
            pacing: match pod.pacing {
                0 => GovernorPacing::FullPerformance,
                1 => GovernorPacing::ThermalThrottled,
                _ => GovernorPacing::CriticalVramSave,
            },
        }
    }
}

/// Thread-safe lock-free state publisher connecting core loop to shells
pub struct EngineStatePublisher {
    current: RwLock<Arc<EngineSnapshot>>,
    shm_publisher: Option<SwmrSnapshotPublisher>,
}

impl Default for EngineStatePublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineStatePublisher {
    pub fn new() -> Self {
        let config = paths::WorkspacePathsConfig::default();
        let path = paths::resolve_synapse_path("engine_state_v4", &config);
        let shm_publisher = Self::open_shm_publisher(&path);
        Self {
            current: RwLock::new(Arc::new(EngineSnapshot::default())),
            shm_publisher,
        }
    }

    pub fn new_with_shm_path(path: &std::path::Path) -> Self {
        let shm_publisher = Self::open_shm_publisher(path);
        Self {
            current: RwLock::new(Arc::new(EngineSnapshot::default())),
            shm_publisher,
        }
    }

    /// `SwmrSnapshotPublisher::open_or_create` fails closed on a genuinely
    /// incompatible segment (an old-version file left over from before an
    /// upgrade, or corrupt geometry) — that's correct, but discarding the
    /// error via `.ok()` with no logging left the shared-memory bridge
    /// silently disabled with no signal that it ever happened. This still
    /// disables it (there's no way to safely reinitialize someone else's
    /// segment), but now says why.
    fn open_shm_publisher(path: &std::path::Path) -> Option<SwmrSnapshotPublisher> {
        match SwmrSnapshotPublisher::open_or_create(path) {
            Ok(publisher) => Some(publisher),
            Err(error) => {
                tracing::warn!(
                    %error,
                    path = %path.display(),
                    "failed to open shared-memory snapshot publisher; the shm bridge is disabled for this process (studio/console/HUD readers will see no live snapshot)"
                );
                None
            }
        }
    }

    pub fn new_in_memory() -> Self {
        Self {
            current: RwLock::new(Arc::new(EngineSnapshot::default())),
            shm_publisher: None,
        }
    }

    /// Core engine publish: swaps the current snapshot reference in sub-microsecond time
    /// and emits the POD frame to the shared memory ring buffer without blocking.
    pub fn publish(&self, snapshot: EngineSnapshot) {
        if let Some(ref shm) = self.shm_publisher {
            let pod = EngineSnapshotPod::from(&snapshot);
            let _ = shm.publish(&pod);
        }
        let mut w = self.current.write();
        *w = Arc::new(snapshot);
    }

    /// Pure zero-allocation monotonic point-in-time state emission into shared memory.
    pub fn publish_pod(&self, pod: &EngineSnapshotPod) {
        if let Some(ref shm) = self.shm_publisher {
            let _ = shm.publish(pod);
        }
        let snapshot = EngineSnapshot::from(pod);
        let mut w = self.current.write();
        *w = Arc::new(snapshot);
    }

    /// Update dynamic resource governor pacing mode
    pub fn set_pacing(&self, pacing: GovernorPacing) {
        let mut w = self.current.write();
        let mut snap = (**w).clone();
        snap.pacing = pacing;
        snap.bus_generation = snap.bus_generation.wrapping_add(1);
        if let Some(ref shm) = self.shm_publisher {
            let pod = EngineSnapshotPod::from(&snap);
            let _ = shm.publish(&pod);
        }
        *w = Arc::new(snap);
    }

    /// Read current governor pacing
    pub fn pacing(&self) -> GovernorPacing {
        self.current.read().pacing
    }

    /// Shell reader: gets a cloned Arc pointer with zero lock contention
    pub fn snapshot(&self) -> Arc<EngineSnapshot> {
        self.current.read().clone()
    }

    /// Project state for Studio Mode
    pub fn project_studio(&self) -> StudioProjection {
        let snap = self.snapshot();
        StudioProjection {
            measured_fps: snap.measured_fps,
            bus_integrity: snap.bus_integrity,
            active_companions_count: snap.active_companions_count,
            last_event: snap.last_event_desc.clone(),
        }
    }

    /// Project state for Console Mode
    pub fn project_console(&self) -> ConsoleProjection {
        let snap = self.snapshot();
        ConsoleProjection {
            display_badge: format!("{:.0} FPS", snap.measured_fps),
            harmony_label: format!("Harmony {:.0}%", snap.bus_integrity),
            user_badge: format!(
                "👤 {} [Flow {:.0}%]",
                snap.active_profile_name,
                snap.flow_score * 100.0
            ),
            level_badge: format!("⭐ Lv. {}", snap.user_level),
        }
    }

    /// Project state for HUD Mode
    pub fn project_hud(&self) -> HudProjection {
        let snap = self.snapshot();
        HudProjection {
            active_guidance: format!(
                "Specialist: {} • {}",
                snap.active_specialist, snap.last_event_desc
            ),
            measured_fps: snap.measured_fps,
            is_nominal: snap.bus_integrity >= 90.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_state_publisher_projections() {
        let publ = EngineStatePublisher::new();
        let init = publ.snapshot();
        assert_eq!(init.measured_fps, 120.0);

        publ.publish(EngineSnapshot {
            timestamp_ms: 1000,
            measured_fps: 144.0,
            bus_integrity: 99.8,
            bus_understanding: 99.1,
            bus_generation: 42,
            active_specialist: "Synthesizer".to_string(),
            active_companions_count: 3,
            running_macros_count: 1,
            last_event_desc: "Associative cluster converged".to_string(),
            user_level: 5,
            user_xp: 1250,
            flow_score: 0.95,
            active_profile_name: "Aaron".to_string(),
            pacing: GovernorPacing::FullPerformance,
        });

        let studio = publ.project_studio();
        assert_eq!(studio.measured_fps, 144.0);
        assert_eq!(studio.active_companions_count, 3);

        let console = publ.project_console();
        assert_eq!(console.level_badge, "⭐ Lv. 5");
        assert!(console.user_badge.contains("Aaron"));

        let hud = publ.project_hud();
        assert!(hud.active_guidance.contains("Synthesizer"));
        assert!(hud.is_nominal);
    }

    #[test]
    fn test_governor_pacing_transitions() {
        let publ = EngineStatePublisher::new();
        assert_eq!(publ.pacing(), GovernorPacing::FullPerformance);

        publ.set_pacing(GovernorPacing::CriticalVramSave);
        assert_eq!(publ.pacing(), GovernorPacing::CriticalVramSave);
        assert_eq!(publ.pacing().target_frame_ms(), 33);
        assert_eq!(publ.pacing().target_fps(), 30.0);

        publ.set_pacing(GovernorPacing::ThermalThrottled);
        assert_eq!(publ.pacing(), GovernorPacing::ThermalThrottled);
        assert_eq!(publ.pacing().target_frame_ms(), 16);
    }

    #[test]
    fn test_engine_state_publisher_shm_bridge() {
        let dir = tempfile::tempdir().unwrap();
        let shm_path = dir.path().join("test_engine_state.synapse");
        let publ = EngineStatePublisher::new_with_shm_path(&shm_path);

        let mut pod = EngineSnapshotPod::default();
        pod.timestamp_ms = 999;
        pod.bus_generation = 7;
        pod.measured_fps = 165.0;
        pod.set_active_specialist("Synthesizer");
        pod.set_last_event_desc("SHM published");
        publ.publish_pod(&pod);

        let reader = ipc_bus::SwmrSnapshotReader::open(&shm_path);
        let read = reader.read_latest().expect("read latest pod");
        assert_eq!(read.timestamp_ms, 999);
        assert_eq!(read.bus_generation, 7);
        assert_eq!(read.measured_fps, 165.0);
        assert_eq!(read.active_specialist_str(), "Synthesizer");
        assert_eq!(read.last_event_desc_str(), "SHM published");
    }
}
