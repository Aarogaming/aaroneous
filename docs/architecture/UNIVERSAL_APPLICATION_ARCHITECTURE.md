# Universal Application Architecture: The Aaroneous Bridge Pattern

**Vision:** Aaroneous is not a "game AI" or a "biometric wearable app" or a "cloud agent." It is a **Universal I/O Platform** where all capabilities—dream-state reflection, peer-to-peer sync, biometric awareness, spatial anchoring—are **standardized plugins** feeding the same core intelligence.

---

## The Bridge Pattern: Three Sacred Rules

Every new capability (Visionary, Omnipresent, Symbiotic, Phygital) must obey:

### 1. **Standardized Input: Key-Value Context Format**
- All relics (Glass, Biometrics, P2P state) output **JSON-serializable Key-Value pairs**
- Ariel's GGUF reads these as immutable **Context Tags**
- Format: `{ "source": "relic_type", "key": value, "timestamp_ms": u64 }`
- Examples:
  - Glass: `{ "source": "glass", "cursor_pos": [1024, 768], "visible_window": "VSCode" }`
  - Biometrics: `{ "source": "symbiotic", "heart_rate": 92, "stress_level": 0.7 }`
  - P2P: `{ "source": "omnipresent", "device_id": "tablet_001", "screen_size": [1920, 1080] }`
  - Dream-Replay: `{ "source": "visionary", "replay_offset_ms": 45000, "intent": "reflect_patterns" }`

### 2. **Binary Portability: WASM Modules**
- All logic changes are WASM modules, not native code
- A WASM module running on desktop MaelstromUI should run on mobile WASM runtime **without code changes**
- Module registry stored in **SSD DNA Bank** (immutable, versioned)
- Modules are composable: a "stress-response" module + "game-context" module = dynamic behavior

### 3. **SSD-First Persistence: The DNA Bank**
- Every decision, every context input, every learned pattern → **SSD DNA Bank**
- Format: **Append-only event log** (like Raft log)
- Immutable: Once written, never deleted (only archived)
- Enables: Dream-state replay, family inheritance, agent resurrection

---

## Tier Evolution: Smooth, Non-Breaking Integration

### **Tier 0: Foundation (Era 2 ✅ Complete)**
**What exists now:**
- Ariel (context weaver) + Glass (spatial vision) + Sentinel (consensus)
- WASM-EBus bridge for I/O
- HID driver for marionette control
- Event log for persistence

**Architecture:** Single-node, single-device, single-instance

---

### **Tier 1: Visionary (Dream-State Buffer)**

**Goal:** Improve agent intelligence without adding hardware

**Key Insight:** Use "Low Duty Cycle" idle time to replay Glass logs through Ariel in headless mode

**Implementation Path:**

```
┌─────────────────────────────────────────────────┐
│ VFD Governor (Duty Cycle Controller)            │
└────────────────────┬────────────────────────────┘
                     │ (10% duty = background tasks available)
                     ▼
┌─────────────────────────────────────────────────┐
│ Visionary Service (Runs during idle)            │
│ ┌─────────────────────────────────────────────┐ │
│ │ 1. Load Glass replay logs from SSD DNA Bank │ │
│ │ 2. Start headless MaelstromUI instance            │ │
│ │ 3. Feed logs to Ariel (Student Engram)     │ │
│ │ 4. Collect GGUF weight deltas              │ │
│ │ 5. Store as "Reflection Events" in DNA     │ │
│ └─────────────────────────────────────────────┘ │
└────────────────────┬────────────────────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │ SSD DNA Bank           │
        │ (Append: reflection    │
        │  events with deltas)   │
        └────────────────────────┘
```

**Standardized I/O Plugin:**

```rust
// Visionary Input Format
#[derive(Serialize)]
pub struct VisionaryContext {
    pub source: "visionary",
    pub replay_offset_ms: u64,           // Which moment in the day
    pub glass_frames: Vec<GlassFrame>,   // Spatial memories
    pub intent: "reflect_patterns",      // Always this for now
    pub timestamp_ms: u64,
}

// Ariel processes this like any other context input
// Output: GGUF weight deltas → DNA Bank
```

**WASM Module:**
- `visionary_replay.wasm`: Orchestrates headless replay
- `reflection_encoder.wasm`: Converts weight deltas to immutable DNA events
- Both composed at runtime based on VFD duty cycle

**Success Criteria:**
- Agent demonstrates improved decision quality after 7 days of reflection
- Reflection runs transparently during idle (no user-perceived lag)
- Deltas reproducible: same logs → same deltas (deterministic GGUF)

**Timeline:** 8-12 hours
- Log replay: 2-3 hours
- Headless MaelstromUI integration: 2-3 hours
- Weight delta persistence: 2-3 hours
- Testing: 2-3 hours

---

### **Tier 2: Omnipresent (Locus Protocol - P2P Sync)**

**Goal:** Same agent brain, visible on any device (desktop, phone, tablet)

**Key Insight:** Keep Ariel GGUF on main SSD (Hive), stream only "Intent" and "Glass" to secondary devices

**Implementation Path:**

```
┌──────────────────────────────────────┐
│ Desktop (Hive)                       │
│ ┌──────────────────────────────────┐ │
│ │ Ariel GGUF (8GB+)                │ │
│ │ DNA Bank (immutable log)         │ │
│ └────────────┬─────────────────────┘ │
└─────────────┼────────────────────────┘
              │
              │ P2P Mesh (Tailscale/WireGuard)
              │
    ┌─────────┴─────────┬────────────┐
    ▼                   ▼            ▼
Phone            Tablet         AR Glasses
(Intent Stream)  (Intent        (Glass Stream)
                  + Glass)
```

**Architecture:**

1. **Central Hub (Desktop/Hive):**
   - Runs full Aaroneous stack
   - Ariel generates "Intent" (next action)
   - Streams Intent + metadata to mesh peers

2. **Peripheral Nodes (Phone/Tablet/AR):**
   - Lightweight WASM runtime (wasmtime or Wasmer)
   - Local Glass module (camera → spatial data)
   - Displays Intent, collects user feedback
   - Streams back to Hub

3. **P2P Mesh:**
   - Tailscale for firewall traversal
   - Mesh VPN creates virtual LAN
   - Low-latency (<50ms) intent delivery
   - Automatic failover if Hub goes offline

**Standardized I/O Plugin:**

```rust
#[derive(Serialize)]
pub struct OmnipresentContext {
    pub source: "omnipresent",
    pub device_id: String,              // "desktop", "phone_001", "tablet"
    pub screen_size: [u32; 2],          // Resolution
    pub capabilities: Vec<String>,      // ["glass", "touch", "voice"]
    pub network_latency_ms: u32,        // For latency-aware decisions
    pub timestamp_ms: u64,
}

// Ariel adjusts Intent complexity based on device
// High latency → simpler Intent, smaller Glass frames
// Low latency → full Intent, full Glass payload
```

**WASM Modules:**
- `locus_protocol.wasm`: P2P mesh negotiation and intent streaming
- `device_adapter.wasm`: Scale Intent/Glass for device capabilities
- `feedback_aggregator.wasm`: Collect user feedback from all devices

**Success Criteria:**
- Same Ariel instance controls actions on 3+ devices
- Intent delivery <100ms p99 over mesh
- Offline mode: peripheral nodes cache recent intents (5min buffer)
- No recompilation for new device types

**Timeline:** 12-16 hours
- P2P mesh setup: 2-3 hours (integrate Tailscale SDK)
- Intent streaming protocol: 3-4 hours
- Device adapters: 3-4 hours
- Testing + failover: 4-5 hours

---

### **Tier 3: Symbiotic (Biometric Metadata Layer)**

**Goal:** Agent adjusts behavior based on user's physical state (stress, focus, fatigue)

**Key Insight:** Don't model emotions—just tag every context with `[User_State: X]`

**Implementation Path:**

```
┌──────────────────────────────────────┐
│ Biometric Sources                    │
│ (Bluetooth/ANT+/HID)                 │
│ • Apple Watch (heart rate)           │
│ • Oura Ring (sleep, readiness)       │
│ • Fitbit (activity level)            │
│ • EEG headset (focus)                │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│ Symbiotic WASM Module                │
│ (Polls BLE peripherals)              │
│ ┌──────────────────────────────────┐ │
│ │ 1. Read heart rate → stress %    │ │
│ │ 2. Read HRV → recovery status    │ │
│ │ 3. Compute composite state tag   │ │
│ │ 4. Emit context to Ariel         │ │
│ └──────────────────────────────────┘ │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│ Ariel Context Input                  │
│ { "source": "symbiotic",             │
│   "stress_level": 0.72,              │
│   "focus_level": 0.89,               │
│   "fatigue_level": 0.34 }            │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│ Ariel Decision Modulation            │
│ High stress → Simplify intent        │
│ Low focus → Increase prompting       │
│ High fatigue → Suggest breaks        │
└──────────────────────────────────────┘
```

**Standardized I/O Plugin:**

```rust
#[derive(Serialize)]
pub struct SymbioticContext {
    pub source: "symbiotic",
    pub heart_rate: Option<u32>,        // BPM
    pub heart_rate_variability: Option<u32>, // ms
    pub galvanic_skin_response: Option<f32>, // μS
    pub computed_stress: f32,           // 0.0-1.0
    pub computed_focus: f32,            // 0.0-1.0
    pub computed_fatigue: f32,          // 0.0-1.0
    pub timestamp_ms: u64,
}

// Ariel applies this as a "Scaling Factor" to her decision-making
// No "emotional" model; just frequency adjustment
```

**WASM Modules:**
- `symbiotic_reader.wasm`: Polls Bluetooth peripherals, reads HID biometric data
- `state_classifier.wasm`: Maps raw biometric → stress/focus/fatigue scores
- `intent_scaler.wasm`: Adjusts Ariel's output based on user state

**Success Criteria:**
- Agent adapts within 5 seconds of stress spike
- Works across all applications (not just game-specific)
- No privacy leak: biometric data stays local (not sent to cloud)
- Graceful degradation: agent works without biometric input

**Timeline:** 6-10 hours
- BLE peripheral polling: 2-3 hours
- State classifier (simple ML): 2-3 hours
- Intent scaling: 1-2 hours
- Testing: 2 hours

---

### **Tier 4: Phygital (Spatial Anchor API)**

**Goal:** Transition MaelstromUI from "desktop overlay" to "room-space anchor"

**Key Insight:** Treat physical workbench/room as a 3D map; use AR passthrough instead of desktop scraper

**Implementation Path:**

```
┌──────────────────────────────────────┐
│ Tier 3 Devices                       │
│ • AR Glasses with passthrough camera │
│ • Depth sensor (RGB-D)               │
│ • 6DOF head tracking                 │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│ MaelstromUI OpenXR Integration              │
│ (Camera feed → Point Cloud mesh)     │
│ ┌──────────────────────────────────┐ │
│ │ 1. Ingest passthrough video      │ │
│ │ 2. Depth sensor → 3D mesh        │ │
│ │ 3. Register room landmarks       │ │
│ │ 4. Map Glass to physical coords  │ │
│ └──────────────────────────────────┘ │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│ Spatial Anchor Context               │
│ { "source": "phygital",              │
│   "camera_feed": <depth_mesh>,       │
│   "head_pose": [x,y,z,rx,ry,rz],   │
│   "world_origin": [0,0,0] }          │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│ Ariel's View (unchanged)             │
│ "I see a workbench at (1,0,1) with   │
│  a component at (1.2,0,1.5)"         │
│ (Works the same way whether data     │
│  came from desktop screenshot or     │
│  AR passthrough)                     │
└──────────────────────────────────────┘
```

**Standardized I/O Plugin:**

```rust
#[derive(Serialize)]
pub struct PhygitalContext {
    pub source: "phygital",
    pub depth_mesh: Vec<f32>,           // Point cloud (flattened)
    pub camera_intrinsics: [f32; 9],    // Pinhole camera model
    pub head_pose: Transform6D,         // Position + rotation
    pub hand_pose: Option<HandPose>,    // If tracked
    pub room_landmarks: Vec<Landmark>,  // Detected objects (workbench, etc)
    pub timestamp_ms: u64,
}

// Ariel uses this to understand "room context" instead of "desktop context"
// Same intent generation logic; different camera feed
```

**WASM Modules:**
- `openxr_adapter.wasm`: Bridge MaelstromUI to OpenXR runtime
- `depth_processor.wasm`: Convert RGB-D → point cloud mesh
- `landmark_detector.wasm`: Identify noteworthy objects in room
- `hand_tracking.wasm`: Optional: track user's hands for interaction hints

**Success Criteria:**
- Same Ariel instance works with both desktop Glass and AR passthrough
- <50ms latency from hand gesture to response
- Works with multiple AR platforms (Meta Quest, Apple Vision Pro, etc.)
- Graceful fallback to desktop if AR unavailable

**Timeline:** 14-18 hours
- OpenXR integration: 4-5 hours
- Depth mesh processing: 3-4 hours
- Landmark detection: 3-4 hours
- Hand tracking: 2-3 hours
- Testing: 2-3 hours

---

## The Unified Core: How It All Works Together

```
┌──────────────────────────────────────────────────────┐
│ Ariel GGUF (8GB, on SSD Hive)                        │
│ • Reads all context tags simultaneously              │
│ • Generates unified "Intent" output                  │
│ • No conditional logic for "which tier"              │
└────────────────────┬─────────────────────────────────┘
                     │
      ┌──────────────┼──────────────┬───────────┐
      │              │              │           │
      ▼              ▼              ▼           ▼
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐
│Visionary │  │Omnipresent│ │Symbiotic │  │ Phygital   │
│(Dream    │  │(P2P       │ │(Biometric)  │(AR         │
│ Replay)  │  │ Sync)     │ │            │ Spatial)   │
└──────────┘  └──────────┘  └──────────┘  └────────────┘
      │              │              │           │
      └──────────────┼──────────────┴───────────┘
                     │
                     ▼
         ┌─────────────────────────┐
         │ DNA Bank (SSD Log)      │
         │ All events, all tiers   │
         │ Immutable, versioned    │
         └─────────────────────────┘
```

**Key Properties:**

1. **Composability:** All tiers emit Key-Value context. Ariel doesn't care which tiers are active.
2. **Backwards Compatibility:** Tier 1 works without Tier 2-4. Adding tiers doesn't break existing logic.
3. **Modular:** Each tier is a WASM module. Disable one, restart, move on.
4. **Universal:** Same Ariel GGUF works across all tiers and all devices.

---

## Persistence: The DNA Bank Event Format

Every decision, every reflection, every biometric tag gets written to the **SSD DNA Bank** as an immutable event:

```rust
#[derive(Serialize)]
pub struct DnaEvent {
    pub event_id: Uuid,                    // Unique identifier
    pub timestamp_ms: u64,                 // When it happened
    pub source_tier: String,               // "visionary", "omnipresent", etc
    pub context_snapshot: Map<String, Value>, // All context inputs
    pub ariel_intent: String,              // What Ariel decided
    pub outcome: Option<String>,           // What actually happened
    pub weight_delta: Option<Vec<f32>>,    // GGUF changes (if from reflection)
}
```

**Usage:**

- **Audit Trail:** Replay any decision and understand why Ariel chose it
- **Transfer Learning:** Copy DNA Bank to a new device; agent inherits all knowledge
- **Family Inheritance:** Child instances start with parent's DNA Bank
- **Deterministic Reflection:** Same logs always produce same weight deltas

---

## Deployment: The Roadmap

### **Month 1: Foundation + Visionary**
- Week 1-2: Complete Phase 6D.2 (HID driver, policy engine)
- Week 3-4: Implement Visionary (dream-state reflection)
- Deliverable: Agent improves itself during idle time

### **Month 2: Omnipresent**
- Week 1-2: P2P mesh integration (Tailscale)
- Week 3-4: Device adapters for phone/tablet
- Deliverable: Same agent visible on multiple devices

### **Month 3: Symbiotic**
- Week 1-2: Biometric polling (BLE, ANT+)
- Week 3-4: Intent scaling based on user state
- Deliverable: Agent adapts to user's stress/focus level

### **Month 4: Phygital**
- Week 1-2: OpenXR integration
- Week 3-4: Spatial anchoring in room-space
- Deliverable: Agent works in AR as well as desktop

---

## Success Metrics

| Tier | Goal | Metric |
|------|------|--------|
| **Visionary** | Improve intelligence | Agent win-rate +15% after 7 days reflection |
| **Omnipresent** | Multi-device control | Same intent on 3+ devices, <100ms latency |
| **Symbiotic** | User-aware adaptation | Stress spike → intent simplification within 5s |
| **Phygital** | Spatial awareness | Room-space anchor registration <500ms, 50ms response |

---

## The Core Insight

You're not building a "game AI" that happens to work on AR. You're building a **Universal I/O Platform** where Ariel is at the center, and every capability (dream, sync, biometric, spatial) is just another standardized context feed.

This is how you avoid "Specialist Hell"—by making the rule: **"If it can be converted to Key-Value context, it plugs into Ariel. If it can be compiled to WASM, it runs everywhere."**

The result: One agent, infinite surfaces.

