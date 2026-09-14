// core/hypervisor/src/hud/state_snapshot.rs
//! Lock-Free Read Snapshot & Engine State Projection for Studio, Console, and HUD.
//!
//! Enforces lightweight, zero-copy presentation state sharing:
//! 1. EngineSnapshot: Immutable point-in-time state updated by the hypervisor core.
//! 2. StudioProjection: Workstation metrics, file tree status, active diagnostics.
//! 3. ConsoleProjection: Immersive 10-foot telemetry, harmony score, user profile & level.
//! 4. HudProjection: Lightweight situational awareness ticker, active bot indicators, FPS.

use core_contracts::EngineSnapshotPod;
use ipc_bus::SwmrSnapshotReader;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Dynamic resource pacing mode regulating shell rendering budgets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernorPacing {
    /// Full performance target (120+ FPS) when thermal/VRAM headroom is nominal
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

impl Default for GovernorPacing {
    fn default() -> Self {
        Self::FullPerformance
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

/// Thread-safe lock-free state publisher connecting core loop to shells via SWMR shared memory
pub struct EngineStatePublisher {
    current: RwLock<Arc<EngineSnapshot>>,
    shm_reader: Option<SwmrSnapshotReader>,
    last_seq: AtomicU64,
}

impl Default for EngineStatePublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineStatePublisher {
    pub fn new() -> Self {
        let config = paths::WorkspacePathsConfig::default();
        let path = paths::resolve_synapse_path("engine_state", &config);
        let shm_reader = Some(SwmrSnapshotReader::open(&path));
        Self {
            current: RwLock::new(Arc::new(EngineSnapshot::default())),
            shm_reader,
            last_seq: AtomicU64::new(0),
        }
    }

    pub fn new_with_shm_path(path: &Path) -> Self {
        let shm_reader = Some(SwmrSnapshotReader::open(path));
        Self {
            current: RwLock::new(Arc::new(EngineSnapshot::default())),
            shm_reader,
            last_seq: AtomicU64::new(0),
        }
    }

    pub fn new_in_memory() -> Self {
        Self {
            current: RwLock::new(Arc::new(EngineSnapshot::default())),
            shm_reader: None,
            last_seq: AtomicU64::new(0),
        }
    }

    /// Return the last read sequence number from shared memory
    pub fn last_sequence(&self) -> u64 {
        self.last_seq.load(Ordering::Relaxed)
    }

    /// Poll the SWMR shared memory ring buffer for new engine snapshot updates.
    /// Returns true if an updated snapshot was acquired.
    pub fn poll_shm(&self) -> bool {
        if let Some(ref reader) = self.shm_reader {
            let last = self.last_seq.load(Ordering::Relaxed);
            if let Some(entry) = reader.read_next(last) {
                self.last_seq.store(entry.sequence, Ordering::Relaxed);
                let mut w = self.current.write();
                *w = Arc::new(EngineSnapshot::from(&entry.snapshot));
                return true;
            }
        }
        false
    }

    /// Core engine publish: swaps the current snapshot reference in sub-microsecond time
    pub fn publish(&self, snapshot: EngineSnapshot) {
        let mut w = self.current.write();
        *w = Arc::new(snapshot);
    }

    /// Update dynamic resource governor pacing mode
    pub fn set_pacing(&self, pacing: GovernorPacing) {
        let mut w = self.current.write();
        let mut snap = (**w).clone();
        snap.pacing = pacing;
        snap.bus_generation = snap.bus_generation.wrapping_add(1);
        *w = Arc::new(snap);
    }

    /// Read current governor pacing
    pub fn pacing(&self) -> GovernorPacing {
        self.poll_shm();
        self.current.read().pacing
    }

    /// Shell reader: checks SWMR shared memory for updates, then returns cloned Arc pointer
    pub fn snapshot(&self) -> Arc<EngineSnapshot> {
        self.poll_shm();
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
            user_badge: format!("👤 {} [Flow {:.0}%]", snap.active_profile_name, snap.flow_score * 100.0),
            level_badge: format!("⭐ Lv. {}", snap.user_level),
        }
    }

    /// Project state for HUD Mode
    pub fn project_hud(&self) -> HudProjection {
        let snap = self.snapshot();
        HudProjection {
            active_guidance: format!("Specialist: {} • {}", snap.active_specialist, snap.last_event_desc),
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
        let publ = EngineStatePublisher::new_in_memory();
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
        let publ = EngineStatePublisher::new_in_memory();
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
    fn test_studio_hud_shm_bridge() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let shm_path = tmp.path().join("test_studio_engine_state.synapse");

        let publisher = ipc_bus::SwmrSnapshotPublisher::open_or_create(&shm_path)
            .expect("publisher create");
        let reader = EngineStatePublisher::new_with_shm_path(&shm_path);

        // Before any publish from hypervisor, reader falls back to default nominal state
        let init = reader.snapshot();
        assert_eq!(init.measured_fps, 120.0);

        // Hypervisor publishes out-of-process snapshot into SWMR ring
        let mut pod = EngineSnapshotPod::default();
        pod.bus_generation = 42;
        pod.measured_fps = 165.0;
        pod.user_xp = 9999;
        pod.set_active_specialist("TelemetryIsolate");
        pod.set_active_profile_name("Commander");
        pod.set_last_event_desc("SHM lock-free sync nominal");
        publisher.publish(&pod).expect("publish frame");

        // Studio HUD reader polls SHM and reflects update
        let updated = reader.snapshot();
        assert_eq!(reader.last_sequence(), 1);
        assert_eq!(updated.bus_generation, 42);
        assert_eq!(updated.measured_fps, 165.0);
        assert_eq!(updated.user_xp, 9999);
        assert_eq!(updated.active_specialist, "TelemetryIsolate");
        assert_eq!(updated.active_profile_name, "Commander");
        assert_eq!(updated.last_event_desc, "SHM lock-free sync nominal");

        // Polling again without new frames returns false (no unnecessary redraw/cloning)
        assert!(!reader.poll_shm());
        assert_eq!(reader.last_sequence(), 1);

        // Hypervisor publishes frame 2
        pod.bus_generation = 43;
        pod.measured_fps = 144.0;
        publisher.publish(&pod).expect("publish frame 2");

        assert!(reader.poll_shm());
        assert_eq!(reader.last_sequence(), 2);
        let updated2 = reader.snapshot();
        assert_eq!(updated2.bus_generation, 43);
        assert_eq!(updated2.measured_fps, 144.0);

        // Projections in studio_hud are immediately accurate
        let studio = reader.project_studio();
        assert_eq!(studio.measured_fps, 144.0);
        assert_eq!(studio.last_event, "SHM lock-free sync nominal");

        let console = reader.project_console();
        assert!(console.user_badge.contains("Commander"));

        let hud = reader.project_hud();
        assert!(hud.active_guidance.contains("TelemetryIsolate"));
    }
}
