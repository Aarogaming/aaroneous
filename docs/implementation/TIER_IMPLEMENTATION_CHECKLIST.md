# Tier Implementation Checklist: From Foundation to Phygital

This document maps each tier to specific code locations and provides step-by-step implementation guidance.

---

## Current State (Tier 0: Foundation ✅)

**Completed components:**
- Ariel (context weaver): `core/hypervisor/src/`
- Glass (spatial vision): `core/hypervisor/spatial/`
- Sentinel (consensus): `core/hypervisor/raft_consensus/`
- WASM-EBus bridge: `core/hypervisor/wasm_ebus_bridge/`
- HID driver: `core/hypervisor/hid_driver/`
- Event log: `core/hypervisor/event_log/`

**Tier 0 Tests:** 555 passing (Phases 5-6C)

---

## Tier 1: Visionary (Dream-State Buffer)

### Architecture Overview
```
Event Log → Glass Replay Engine → Headless MaelstromUI → Ariel Student Engram → Weight Deltas → DNA Bank
```

### Implementation Checklist

#### Step 1.1: Create Visionary Service (3 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/visionary/mod.rs` (main service orchestrator)
- `core/hypervisor/federation/specialists/visionary/replay_engine.rs` (log playback)
- `core/hypervisor/federation/specialists/visionary/reflection_encoder.rs` (weight delta persistence)

**Location reference:**
- Copy VFD idle detection from `docs/architecture/VFD_INTELLIGENCE_THROTTLING.md`
- Use Event Log interface from `core/hypervisor/event_log/mod.rs`
- Hook into HiveRuntime from `core/hypervisor/src/`

**Pseudocode:**
```rust
pub struct VisionaryService {
    event_log: Arc<EventLog>,      // Read Glass logs
    hive_runtime: Arc<HiveRuntime>, // Access Ariel
    dna_bank: Arc<DnaBank>,        // Write reflections
}

impl VisionaryService {
    pub async fn reflect_during_idle(&self, duty_cycle: f32) -> Result<(), String> {
        if duty_cycle < 0.15 {  // Only during true idle
            // 1. Load recent Glass events
            let events = self.event_log.query_recent(Duration::from_secs(3600)).await?;
            
            // 2. Replay through Ariel
            let reflections = self.replay_events(events).await?;
            
            // 3. Extract weight deltas
            let deltas = self.extract_deltas(&reflections)?;
            
            // 4. Persist as DNA events
            self.dna_bank.append_reflection_event(deltas).await?;
            
            Ok(())
        }
    }
}
```

#### Step 1.2: Implement Replay Engine (2 hours)
**Files:**
- `core/hypervisor/federation/specialists/visionary/replay_engine.rs`

**Key functions:**
```rust
pub fn replay_glass_frames(frames: Vec<GlassFrame>) -> Vec<VisionaryContext> {
    // Convert Glass spatial frames to Ariel context input
    // Each frame becomes a "thought" in the replay
}

pub async fn run_headless_instance(
    contexts: Vec<VisionaryContext>
) -> Result<Vec<WeightDelta>, String> {
    // Start headless MaelstromUI with Ariel running
    // Feed contexts sequentially
    // Capture GGUF weight changes
}
```

**Integration points:**
- Use `GlassFrame` from `core/hypervisor/spatial/`
- Use `HiveRuntime::infer()` from `core/hypervisor/src/`
- Store results in new `DnaBank` (see Step 1.3)

#### Step 1.3: Create DNA Bank (Persistence Layer) (2 hours)
**Files:**
- `core/hypervisor/src/dna_bank/mod.rs` (new module)
- `core/hypervisor/src/dna_bank/event_store.rs` (append-only log)
- `core/hypervisor/src/dna_bank/encoder.rs` (serialize DNA events)

**Key struct:**
```rust
#[derive(Serialize, Deserialize)]
pub struct DnaEvent {
    pub event_id: Uuid,
    pub timestamp_ms: u64,
    pub source_tier: String,  // "visionary"
    pub context_snapshot: Map<String, Value>,
    pub ariel_intent: Option<String>,
    pub outcome: Option<String>,
    pub weight_delta: Option<Vec<f32>>,
}

pub struct DnaBank {
    log_file: File,  // Append-only SSD write
}

impl DnaBank {
    pub async fn append_event(&mut self, event: DnaEvent) -> Result<(), String> {
        // Write JSON line + newline (JSONL format)
        // Ensure atomic writes
    }
}
```

**Usage:**
```rust
let mut dna = DnaBank::new(Path::new("/path/to/dna.log"))?;
dna.append_event(DnaEvent {
    event_id: Uuid::new_v4(),
    timestamp_ms: now_ms(),
    source_tier: "visionary".to_string(),
    context_snapshot: reflection_data,
    weight_delta: Some(extracted_deltas),
    ..Default::default()
}).await?;
```

#### Step 1.4: Integrate with VFD Governor (2 hours)
**Files to modify:**
- `docs/architecture/VFD_INTELLIGENCE_THROTTLING.md` (add callback)

**Change:**
```rust
pub struct VfdGovernor {
    // ... existing fields ...
    visionary_service: Option<Arc<VisionaryService>>,  // NEW
}

impl VfdGovernor {
    pub async fn tick(&mut self, /* params */) {
        let duty_cycle = self.calculate_duty_cycle(/* */);
        
        // NEW: Trigger reflection during idle
        if let Some(visionary) = &self.visionary_service {
            let _ = visionary.reflect_during_idle(duty_cycle).await;
        }
    }
}
```

#### Step 1.5: Write Tests (2 hours)
**Test file:** `core/hypervisor/federation/specialists/visionary/tests.rs`

```rust
#[tokio::test]
async fn test_simple_reflection() {
    let mut dna = DnaBank::new(temp_file()).unwrap();
    
    // Create a simple Glass frame
    let frame = GlassFrame { /* ... */ };
    
    // Run replay
    let deltas = replay_glass_frames(vec![frame]).await.unwrap();
    
    // Store in DNA
    dna.append_event(DnaEvent {
        event_id: Uuid::new_v4(),
        timestamp_ms: now_ms(),
        source_tier: "visionary".to_string(),
        context_snapshot: map! {},
        weight_delta: Some(deltas),
        ..Default::default()
    }).await.unwrap();
    
    // Verify event was written
    let events = dna.query_recent(Duration::from_secs(60)).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_visionary_improves_agent() {
    // Run Visionary service for fake "day"
    // Measure agent decision quality before/after
    // Assert improvement
}
```

**Expected test count:** +8-10 tests (target: 565+ total)

---

## Tier 2: Omnipresent (P2P Sync - Locus Protocol)

### Architecture Overview
```
Desktop (Hub) ←→ [Tailscale Mesh] ←→ Phone/Tablet/AR (Peripherals)
```

### Implementation Checklist

#### Step 2.1: Add Tailscale Integration (3 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/omnipresent/mod.rs`
- `core/hypervisor/federation/specialists/omnipresent/mesh_client.rs`
- `core/hypervisor/federation/specialists/omnipresent/intent_streamer.rs`

**Dependency:**
```toml
[dependencies]
tailscale = "0.1"  # Hypothetical; use tailscale-go SDK via FFI or existing Rust wrapper
```

**Key struct:**
```rust
pub struct LocusProtocol {
    mesh_client: TailscaleMeshClient,
    device_id: String,              // "desktop" or "phone_001"
    discovered_peers: Vec<PeerInfo>,
}

pub struct PeerInfo {
    pub device_id: String,
    pub ip_address: String,
    pub device_type: DeviceType,    // Desktop, Phone, Tablet, AR
    pub last_heartbeat_ms: u64,
}

impl LocusProtocol {
    pub async fn start_discovery(&mut self) -> Result<(), String> {
        // Listen for other Aaroneous instances on mesh
        // Register self
    }
    
    pub async fn stream_intent_to_peer(
        &self,
        peer_id: &str,
        intent: &str,
    ) -> Result<(), String> {
        // Send Intent over TCP/gRPC to peer
    }
}
```

#### Step 2.2: Intent Streaming Protocol (3 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/omnipresent/intent_protocol.rs` (gRPC/protobuf definitions)

**Protocol definition (protobuf):**
```protobuf
syntax = "proto3";

message Intent {
    string intent_id = 1;
    string action = 2;              // e.g., "click", "type", "scroll"
    map<string, string> params = 3; // e.g., {"x": "100", "y": "200"}
    int64 timestamp_ms = 4;
    float confidence = 5;           // 0.0-1.0
}

message IntentStream {
    repeated Intent intents = 1;
    string source_device = 2;
    int32 network_latency_ms = 3;
}

message Feedback {
    string intent_id = 1;
    bool success = 2;
    string reason = 3;             // Optional error message
}

service IntentService {
    rpc StreamIntent(IntentStream) returns (Feedback);
}
```

**Rust wrapper:**
```rust
#[derive(Serialize, Deserialize)]
pub struct Intent {
    pub intent_id: String,
    pub action: String,
    pub params: Map<String, String>,
    pub timestamp_ms: u64,
    pub confidence: f32,
}

pub struct IntentStreamer {
    grpc_server: GrpcServer,
}

impl IntentStreamer {
    pub async fn broadcast_intent(&self, intent: Intent) -> Result<(), String> {
        // Send to all connected peers
    }
}
```

#### Step 2.3: Device Adapter System (2 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/omnipresent/device_adapter.rs`

**Concept:**
```rust
pub trait DeviceAdapter: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn scale_intent(&self, intent: &Intent) -> Intent;
    fn scale_glass(&self, glass: &GlassFrame) -> GlassFrame;
}

pub struct DesktopAdapter;
pub struct PhoneAdapter;
pub struct TabletAdapter;
pub struct ArAdapter;

impl DeviceAdapter for PhoneAdapter {
    fn device_type(&self) -> DeviceType { DeviceType::Phone }
    
    fn scale_intent(&self, intent: &Intent) -> Intent {
        // Simplify intent for small screen
        // Remove complex commands, reduce text length
    }
    
    fn scale_glass(&self, glass: &GlassFrame) -> GlassFrame {
        // Reduce resolution, fewer details
        // Optimize for bandwidth
    }
}
```

#### Step 2.4: Offline Mode (Cache) (2 hours)
**Files to modify:**
- `core/hypervisor/federation/specialists/omnipresent/intent_streamer.rs`

**Addition:**
```rust
pub struct IntentCache {
    buffer: VecDeque<Intent>,
    max_age_ms: u64,
}

impl IntentStreamer {
    pub async fn get_intent_with_fallback(&self, device_id: &str) -> Result<Intent, String> {
        // Try to get Intent from hub
        match self.request_intent(device_id).await {
            Ok(intent) => {
                self.cache.insert(intent.clone());
                Ok(intent)
            }
            Err(_) => {
                // Hub offline: return cached intent
                self.cache.get_latest(Duration::from_secs(300))
            }
        }
    }
}
```

#### Step 2.5: Write Tests (2 hours)
**Test file:** `core/hypervisor/federation/specialists/omnipresent/tests.rs`

```rust
#[tokio::test]
async fn test_intent_delivery_latency() {
    let protocol = LocusProtocol::new("desktop").unwrap();
    
    let intent = Intent {
        intent_id: uuid(),
        action: "click".to_string(),
        params: map! {"x": "100", "y": "200"},
        timestamp_ms: now_ms(),
        confidence: 0.95,
    };
    
    let start = Instant::now();
    protocol.broadcast_intent(&intent).await.unwrap();
    let latency = start.elapsed();
    
    assert!(latency.as_millis() < 100, "Intent delivery must be <100ms");
}

#[tokio::test]
async fn test_offline_cache() {
    let mut streamer = IntentStreamer::new().unwrap();
    
    // Store intent
    let intent = Intent { /* */ };
    streamer.cache.insert(intent.clone());
    
    // Simulate hub offline
    // Should return cached intent
    let retrieved = streamer.get_intent_with_fallback("phone").await;
    assert!(retrieved.is_ok());
}
```

**Expected test count:** +6-8 tests (target: 571+ total)

---

## Tier 3: Symbiotic (Biometric Metadata)

### Architecture Overview
```
BLE Peripherals (Watch, Ring, etc) → Biometric Reader → State Classifier → Intent Scaler → Ariel
```

### Implementation Checklist

#### Step 3.1: BLE Peripheral Polling (2 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/symbiotic/mod.rs`
- `core/hypervisor/federation/specialists/symbiotic/biometric_reader.rs`

**Dependency:**
```toml
[dependencies]
btleplug = "0.10"  # Cross-platform BLE library
```

**Key struct:**
```rust
pub struct BiometricReader {
    peripherals: Vec<BlePeripheral>,
}

#[derive(Serialize)]
pub struct BiometricSample {
    pub heart_rate: Option<u32>,
    pub heart_rate_variability: Option<u32>,
    pub galvanic_skin_response: Option<f32>,
    pub timestamp_ms: u64,
}

impl BiometricReader {
    pub async fn poll_all(&mut self) -> Result<Vec<BiometricSample>, String> {
        let mut samples = Vec::new();
        
        // Poll Apple Watch
        if let Some(watch) = self.find_device("Apple Watch") {
            let sample = self.read_heart_rate_service(&watch).await?;
            samples.push(sample);
        }
        
        // Poll Oura Ring
        if let Some(ring) = self.find_device("Oura") {
            let sample = self.read_ring_services(&ring).await?;
            samples.push(sample);
        }
        
        Ok(samples)
    }
}
```

#### Step 3.2: State Classifier (2 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/symbiotic/state_classifier.rs`

**Key function:**
```rust
#[derive(Serialize)]
pub struct UserState {
    pub stress_level: f32,    // 0.0-1.0
    pub focus_level: f32,     // 0.0-1.0
    pub fatigue_level: f32,   // 0.0-1.0
}

pub fn classify_state(samples: &[BiometricSample]) -> UserState {
    // Simple heuristics (can upgrade to ML later)
    
    let avg_hr = samples.iter().filter_map(|s| s.heart_rate).sum::<u32>() / samples.len() as u32;
    let avg_hrv = samples.iter().filter_map(|s| s.heart_rate_variability).sum::<u32>() / samples.len() as u32;
    
    // High HR + low HRV = stress
    let stress = ((avg_hr as f32 - 60.0) / 40.0).clamp(0.0, 1.0);
    
    // High HRV = recovery (low stress/fatigue)
    let fatigue = (1.0 - (avg_hrv as f32 / 100.0)).clamp(0.0, 1.0);
    
    // TODO: EEG data for focus_level
    let focus = 0.5;
    
    UserState {
        stress_level: stress,
        focus_level: focus,
        fatigue_level: fatigue,
    }
}
```

#### Step 3.3: Intent Scaler (1 hour)
**Files to create:**
- `core/hypervisor/federation/specialists/symbiotic/intent_scaler.rs`

**Key function:**
```rust
pub fn scale_intent(
    original_intent: &Intent,
    user_state: &UserState,
) -> Intent {
    let mut scaled = original_intent.clone();
    
    if user_state.stress_level > 0.7 {
        // Under stress: simplify response
        scaled.params.insert("complexity".to_string(), "low".to_string());
    }
    
    if user_state.fatigue_level > 0.6 {
        // Fatigued: suggest break
        scaled.action = "suggest_break".to_string();
    }
    
    if user_state.focus_level < 0.3 {
        // Unfocused: increase prompting
        scaled.params.insert("prompt_frequency".to_string(), "high".to_string());
    }
    
    scaled
}
```

#### Step 3.4: Integrate with Ariel (1 hour)
**Files to modify:**
- `core/hypervisor/src/`

**Change:**
```rust
pub struct HiveRuntime {
    // ... existing fields ...
    biometric_reader: Option<Arc<BiometricReader>>, // NEW
}

impl HiveRuntime {
    pub async fn infer(&self, obs: &Observation) -> Result<PlayerAction, String> {
        // Get user state if available
        let user_state = if let Some(reader) = &self.biometric_reader {
            let samples = reader.poll_all().await.ok();
            samples.as_ref().map(|s| classify_state(s))
        } else {
            None
        };
        
        // Original inference
        let intent = self.ariel.generate_intent(obs)?;
        
        // Scale intent by user state
        let action = if let Some(state) = user_state {
            scale_intent(&intent, &state)
        } else {
            intent
        };
        
        Ok(action)
    }
}
```

#### Step 3.5: Write Tests (1.5 hours)
**Test file:** `core/hypervisor/federation/specialists/symbiotic/tests.rs`

```rust
#[test]
fn test_stress_detection() {
    let samples = vec![
        BiometricSample {
            heart_rate: Some(110),
            heart_rate_variability: Some(20),  // Low HRV = stress
            ..Default::default()
        },
    ];
    
    let state = classify_state(&samples);
    assert!(state.stress_level > 0.6, "Should detect high stress");
}

#[test]
fn test_intent_scaling_under_stress() {
    let original = Intent {
        action: "complex_task".to_string(),
        ..Default::default()
    };
    
    let user_state = UserState {
        stress_level: 0.8,
        ..Default::default()
    };
    
    let scaled = scale_intent(&original, &user_state);
    assert_eq!(scaled.params.get("complexity"), Some(&"low".to_string()));
}
```

**Expected test count:** +5-7 tests (target: 576+ total)

---

## Tier 4: Phygital (Spatial Anchoring)

### Architecture Overview
```
AR Glasses (passthrough camera + depth) → OpenXR → MaelstromUI → Spatial Anchor Context → Ariel
```

### Implementation Checklist

#### Step 4.1: OpenXR Integration (4 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/phygital/mod.rs`
- `core/hypervisor/federation/specialists/phygital/openxr_adapter.rs`

**Dependency:**
```toml
[dependencies]
openxr = "0.18"  # OpenXR runtime bindings
```

**Key struct:**
```rust
pub struct OpenXrAdapter {
    instance: xr::Instance,
    session: xr::Session<xr::OpenGles>,
    frame_state: xr::FrameState,
}

impl OpenXrAdapter {
    pub fn new() -> Result<Self, String> {
        // Initialize OpenXR instance
        // Create session with desktop/AR platform
        // Setup frame timing
    }
    
    pub fn poll_frame(&mut self) -> Result<FrameData, String> {
        // Wait for frame, get camera pose
        // Return depth mesh + head pose
    }
}

#[derive(Serialize)]
pub struct FrameData {
    pub depth_mesh: Vec<f32>,          // Point cloud
    pub head_pose: Transform6D,        // Position + rotation
    pub timestamp_ms: u64,
}
```

#### Step 4.2: Depth Processing (3 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/phygital/depth_processor.rs`

**Key functions:**
```rust
pub struct PointCloud {
    points: Vec<[f32; 3]>,
}

pub fn depth_to_point_cloud(
    depth_image: &[f32],
    width: u32,
    height: u32,
    intrinsics: &CameraIntrinsics,
) -> PointCloud {
    // Convert depth map to 3D point cloud using camera intrinsics
    let mut points = Vec::new();
    
    for y in 0..height {
        for x in 0..width {
            let depth = depth_image[(y * width + x) as usize];
            if depth > 0.0 && depth < 10.0 {  // Valid depth range
                let pt = intrinsics.unproject(x as f32, y as f32, depth);
                points.push([pt.x, pt.y, pt.z]);
            }
        }
    }
    
    PointCloud { points }
}
```

#### Step 4.3: Landmark Detection (3 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/phygital/landmark_detector.rs`

**Key struct:**
```rust
pub struct Landmark {
    pub name: String,           // "workbench", "desk", "wall"
    pub center: [f32; 3],
    pub bounds: BoundingBox,
    pub confidence: f32,
}

pub fn detect_landmarks(cloud: &PointCloud) -> Vec<Landmark> {
    // RANSAC plane fitting to detect surfaces
    // Cluster analysis to find objects
    // Return significant landmarks
    
    // For MVP: simple heuristics (large horizontal surface = desk)
    let mut landmarks = Vec::new();
    
    // Find floor/desk plane
    if let Some(plane) = fit_plane(cloud) {
        landmarks.push(Landmark {
            name: "work_surface".to_string(),
            center: plane.center,
            bounds: plane.bounds,
            confidence: 0.9,
        });
    }
    
    landmarks
}
```

#### Step 4.4: Hand Tracking (Optional, 2 hours)
**Files to create:**
- `core/hypervisor/federation/specialists/phygital/hand_tracker.rs`

**Key struct:**
```rust
#[derive(Serialize)]
pub struct HandPose {
    pub position: [f32; 3],
    pub joints: Vec<JointPose>,  // 21 hand joints
    pub confidence: f32,
}

impl HandPose {
    pub fn is_pointing(&self) -> bool {
        // Detect pointing gesture
        // Extended index finger + others folded
    }
    
    pub fn is_pinching(&self) -> bool {
        // Detect pinch (thumb + index close)
    }
}
```

#### Step 4.5: Integration with Ariel (2 hours)
**Files to modify:**
- `core/hypervisor/src/`
- `core/hypervisor/spatial/`

**Change:**
```rust
pub struct HiveRuntime {
    // ... existing fields ...
    openxr_adapter: Option<Arc<OpenXrAdapter>>, // NEW
}

impl HiveRuntime {
    pub async fn infer(&self, obs: &Observation) -> Result<PlayerAction, String> {
        // If AR is available, use spatial context
        if let Some(xr) = &self.openxr_adapter {
            let frame = xr.poll_frame()?;
            let cloud = depth_to_point_cloud(&frame.depth_mesh, /* */);
            let landmarks = detect_landmarks(&cloud);
            
            // Add spatial context to observation
            let mut spatial_obs = obs.clone();
            spatial_obs.landmarks = landmarks;
            spatial_obs.head_pose = frame.head_pose;
            
            self.ariel.generate_intent(&spatial_obs)
        } else {
            // Fallback to desktop Glass
            self.ariel.generate_intent(obs)
        }
    }
}
```

#### Step 4.6: Write Tests (2 hours)
**Test file:** `core/hypervisor/federation/specialists/phygital/tests.rs`

```rust
#[test]
fn test_depth_to_point_cloud() {
    let depth = vec![1.0; 1024 * 768];  // 1 meter depth everywhere
    let intrinsics = CameraIntrinsics {
        fx: 500.0,
        fy: 500.0,
        cx: 512.0,
        cy: 384.0,
    };
    
    let cloud = depth_to_point_cloud(&depth, 1024, 768, &intrinsics);
    
    assert!(cloud.points.len() > 0);
    assert!(cloud.points.iter().all(|p| p[2] > 0.99 && p[2] < 1.01));
}

#[test]
fn test_landmark_detection() {
    // Create synthetic point cloud (flat surface)
    let mut points = Vec::new();
    for x in 0..100 {
        for z in 0..100 {
            points.push([x as f32, 0.0, z as f32]);
        }
    }
    
    let cloud = PointCloud { points };
    let landmarks = detect_landmarks(&cloud);
    
    assert!(landmarks.len() > 0);
    assert!(landmarks[0].name.contains("surface"));
}
```

**Expected test count:** +8-10 tests (target: 584+ total)

---

## Summary: Path to Universal Application

| Tier | Status | Files | Tests | Hours | Goal |
|------|--------|-------|-------|-------|------|
| Tier 0 (Foundation) | ✅ Complete | - | 555 | - | Baseline Ariel + Glass + Consensus |
| **Tier 1 (Visionary)** | 🎯 Next | 5-6 | +10 | 11-15 | Agent self-improvement via reflection |
| **Tier 2 (Omnipresent)** | 📋 Planned | 4-5 | +8 | 12-16 | Multi-device sync via P2P mesh |
| **Tier 3 (Symbiotic)** | 📋 Planned | 3-4 | +7 | 6-10 | User-aware adaptation via biometrics |
| **Tier 4 (Phygital)** | 📋 Planned | 4-5 | +10 | 14-18 | Spatial awareness via OpenXR |
| **Total New Code** | | 16-19 | +35 | 43-59 | **One universal agent, all surfaces** |

---

## Next Immediate Step: Begin Tier 1 Implementation

**This week's focus:**
1. Create `core/hypervisor/federation/specialists/visionary/` module structure
2. Implement replay engine and basic DNA Bank
3. Write 10 tests for reflection logic
4. Integrate with VFD idle detection

---

## Post-Integration Roadmaps (Usability Phase)

This section details the critical path to moving from a functional core to a fully managed agentic system.

### 1. Agent Lifecycle Manager (`LifecycleManager`)
- **Objective**: Managed agent lifecycle (spawn/monitor/spool-down).
- **Steps**:
    1. Define `AgentDescriptor` struct (config, status, resource usage).
    2. Implement `LifecycleManager` using `tokio::process`.
    3. Update `orchestration_daemon` to register managed processes.
    4. Implement heartbeats and automatic restarts on failure.

### 2. High-Level Marionette API
- **Objective**: Formalized control for agent physical/system output.
- **Steps**:
    1. Define `MarionetteAction` enum (Mouse, Keyboard, Window, EBus events).
    2. Map enums to `hid_driver` and `ActionExecutor` primitives.
    3. Expose API through a thread-safe `MarionetteBridge` accessible by enzymes.
    4. Verify via a integration test that drives a test window.

### 3. Automated Data Ingestion Pipeline
- **Objective**: Real-time data gathering.
- **Steps**:
    1. Configure `notify` crate to watch target directories.
    2. Hook events into `MetadataIngestor` queue.
    3. Implement async consumer in `AutonomicNervousSystem` to pipe events to `HiveDB`.
    4. Validate embeddings generation for new files.

### 4. MaelstromUI IPC Bridge
- **Objective**: Visualize and control system via GUI.
- **Steps**:
    1. Implement WebSocket server in `mcp_service`.
    2. Serialize `SynapseState` to JSON via Serde.
    3. Implement Tauri IPC commands in `MaelstromUI`.
    4. Create live dashboard view of Constellation nodes and Homeostatic metrics.



