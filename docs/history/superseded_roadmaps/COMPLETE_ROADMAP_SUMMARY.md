# Aaroneous: Complete 11-Week Implementation Roadmap

**Vision:** Transform Aaroneous from a static AI assistant into a **fully autonomous, self-evolving system** that learns, designs, and adapts across all devices and applications.

**Timeline:** 11 weeks, 141 hours, 555 → 665+ tests  
**Current Date:** April 30, 2026  
**Target Completion:** July 21, 2026 (Era 3, Phase 1 Complete)

---

## Executive Roadmap

```
WEEK 1:  Phase A (Watcher)         → 573 tests  ✅
WEEK 2:  Phase B (Dreamer) START   → 595 tests  
WEEK 3:  Phase B (Dreamer) END     → 595 tests  
         Phase C (Learner) START   
WEEK 4:  Phase C (Learner) END     → 613 tests
         Phase D (Glass) START
WEEK 5:  Phase D (Glass) END       → 635 tests
         Phase E (Integration)
         Tier 2 (Omnipresent) START
WEEK 6:  Tier 2 (Omnipresent) END  → 643 tests
         Tier 3 (Symbiotic) START
WEEK 7:  Tier 3 (Symbiotic) END    → 650 tests
         Tier 4 (Phygital) START
WEEK 8:  Tier 4 (Phygital) END     → 660 tests
         DNA Bank + Inference       → 665 tests
WEEK 9:  Stress Testing            → 665 tests (Validated)
WEEK 10: Performance Optimization   → 665 tests (Optimized)
WEEK 11: Deployment & Release       → 665+ tests (Production Ready)
```

---

## Phase-By-Phase Breakdown

### **PHASE A: The Watcher (Week 1) - 20 Hours**

**Goal:** Detect behavioral anchors (game end, error, loading screen) and trigger context events

**Dependencies:**
- Tesseract (OCR): 60k GitHub stars, Apache 2.0
- Palette (color analysis): 1.2k stars, MIT/Apache 2.0
- imageproc (computer vision): 600+ stars, MIT

**Tasks:**
```
A1: Setup Tesseract + Palette + imageproc (2h)
    - Add to Cargo.toml
    - Download tesseract language packs
    - Test OCR on sample screenshot

A2: Implement AnchorDetector (6h)
    - Pattern matching for Steam (green=victory, red=defeat)
    - IDE patterns (red=error, yellow=warning)
    - CAD patterns (simulation complete)
    - Debouncing (prevent duplicate anchors)

A3: Implement ContextEvent + Intent Router (6h)
    - Emit context when anchor detected
    - Ariel generates appropriate intent
    - Hook into hive_runtime.rs

A4: Write tests (6h)
    - Unit tests for each anchor type (7 tests)
    - Integration tests (4 tests)
    - Performance validation (<500ms latency)
```

**Success Metrics:**
- ✅ Detect game end from 1920x1080 screenshot
- ✅ Detect IDE error/warning
- ✅ Generate appropriate intent
- ✅ End-to-end latency <500ms
- ✅ 18 new tests passing

**Tests:** 555 → 573 (+18)

---

### **PHASE B: The Dreamer (Weeks 2-3) - 28 Hours**

**Goal:** Generate new UI designs by splicing learned aesthetic patterns during idle time

**Dependencies:**
- Nannou (GPU design rendering): 3.5k stars, MIT

**Tasks:**
```
B1: Setup Nannou (3h)
    - Create Nannou app for design generation
    - GPU-accelerated drawing
    - Batch rendering of variants

B2: Implement StyleBank (5h)
    - Store aesthetic engrams in JSONL
    - Load engrams from disk
    - Query: random sample, similar styles
    - Engagement weighting (hover +0.5, click +1.0, ignore -0.2)

B3: Implement Design Splicing (10h)
    - Algorithm: randomly select 2 engrams
    - Blend: color palette + typography + spacing
    - Vary: generate 10 distinct variants
    - GPU acceleration for batch rendering

B4: Implement Design Scoring (6h)
    - Load ClassOfAaron preference model
    - Score by: color harmony, typography fit, complexity, novelty
    - Keep top 3 designs
    - Archive others

B5: Write tests (4h)
    - StyleBank CRUD tests (5)
    - Design generation tests (8)
    - Scoring tests (6)
    - GPU rendering tests (3)
```

**Success Metrics:**
- ✅ Generate 10 design variants per cycle
- ✅ Top design scores >0.8
- ✅ No duplicates (novelty penalty working)
- ✅ 22 new tests passing

**Tests:** 573 → 595 (+22)

---

### **PHASE C: The Learner (Weeks 3-4) - 16 Hours**

**Goal:** Extract visual preferences and learn user's aesthetic taste

**Dependencies:**
- Fontdue (typography): 500+ stars, MIT/Apache 2.0
- rustfft (patterns): 500+ stars, MIT/Apache 2.0
- ndarray (vectors): 3k+ stars, MIT/Apache 2.0

**Tasks:**
```
C1: Setup Dependencies (2h)
    - Add Fontdue, rustfft, ndarray to Cargo.toml
    - Download reference fonts (Arial, Roboto, etc)

C2: Implement Engram Extractor (5h)
    - Extract color palettes from screenshots
    - Detect typography (font, size, weight)
    - Analyze spacing and layout
    - Frequency-domain feature extraction (rustfft)
    - Convert to embedding vectors (ndarray)

C3: Implement Engagement Tracking (4h)
    - Hook into HID driver for user interactions
    - Hover: +0.5 engagement
    - Click: +1.0 engagement
    - Ignore: -0.2 engagement
    - Store with engrams

C4: Implement Preference Model (4h)
    - Build user preference vector from engrams
    - Update model as new designs approved/rejected
    - Predict which designs user will like

C5: Write tests (1h)
    - Feature extraction tests (6)
    - Engagement tracking tests (5)
    - Preference model tests (7)
```

**Success Metrics:**
- ✅ Extract engrams from 5+ applications
- ✅ Engagement tracking correlates with actions
- ✅ Preference model predicts user choices >70% accuracy
- ✅ 18 new tests passing

**Tests:** 595 → 613 (+18)

---

### **PHASE D: Glass Workshop (Weeks 4-5) - 32 Hours**

**Goal:** Render design prototypes and contextual overlays in 3D space

**Dependencies:**
- Bevy (game engine + ECS): 35k stars, MIT/Apache 2.0
- OpenCV (depth): 2k stars, BSD

**Tasks:**
```
D1: Setup Bevy (4h)
    - Create Bevy app with ECS
    - Window creation, event loop
    - Plugin architecture

D2: Implement Design Prototype Spawning (6h)
    - Define Design Prototype component
    - Spawn 3D geometry from design config
    - Color, typography, animation rendering

D3: Implement 3D Overlay Rendering (8h)
    - Render design prototypes in world space
    - Camera system for multiple viewports
    - Lighting and materials
    - Animation system (transitions, hover effects)

D4: Setup OpenCV for Depth Processing (4h)
    - Depth mesh to point cloud conversion
    - RANSAC plane fitting for surface detection
    - Landmark detection (workbench, desk, wall)

D5: Implement PDF Projection (6h)
    - Load PDF into memory
    - Render PDF page as texture
    - Project into 3D space at user location
    - Example: error in code → manual above desk

D6: Write tests (4h)
    - Bevy ECS tests (4)
    - Geometry rendering tests (5)
    - Overlay tests (4)
    - OpenCV depth tests (4)
```

**Success Metrics:**
- ✅ Render prototype in Glass Workshop
- ✅ Project PDF into 3D at desk location
- ✅ All designs render without errors
- ✅ <200ms render latency
- ✅ 17 new tests passing

**Tests:** 613 → 630 (+17)

---

### **PHASE E: Integration & Polish (Week 5) - 8 Hours**

**Goal:** Coordinate all Visionary phases and ensure smooth operation

**Tasks:**
```
E1: VFD Orchestration (2h)
    - Integrate with VFD duty cycle
    - Design generation only when duty <15%
    - Priority: user work > background tasks

E2: DNA Bank Migration to RocksDB (2h)
    - Replace custom JSONL with RocksDB
    - Column families: visionary, omnipresent, etc
    - Append events atomically

E3: Performance Optimization (2h)
    - Profile all phases (GPU, CPU, memory)
    - Optimize hot paths
    - Target: <4GB active memory

E4: Integration Tests (2h)
    - End-to-end VBE cycle
    - 7-day simulation test
    - No crashes or memory leaks
```

**Success Metrics:**
- ✅ Full VBE loop runs autonomously
- ✅ No resource leaks over 7 days
- ✅ DNA Bank logs all decisions
- ✅ 8 new integration tests passing

**Tests:** 630 → 635 (+5)

---

## **TIER 1 VISIONARY: COMPLETE (Week 5)**

**Test Count:** 555 → 635 (+80)  
**Hours:** 94 of 141  
**Status:** Self-improving agent fully functional

**What Aaroneous Can Do:**
- Detect when you finish a game → congratulates you
- Detect when you get a code error → offers troubleshooting
- Generate UI designs splicing your favorite styles
- Learn which designs you prefer
- Generate better designs based on feedback
- Run all of this during idle time (VFD <15% duty)

**Next:** Multi-device sync, biometric awareness, AR integration

---

## **TIER 2: Omnipresent (Weeks 5-6) - 11 Hours**

**Goal:** Same Ariel brain visible on desktop, phone, tablet, AR glasses simultaneously

**Dependencies:**
- Iroh (P2P by Protocol Labs): 4k stars, Apache 2.0

**Tasks:**
```
O1: Setup Iroh (2h)
    - P2P document sync
    - Encryption built-in
    - NAT traversal

O2: Implement Intent Streaming (4h)
    - Hub publishes Intent to Iroh document
    - Peripherals subscribe to Intent updates
    - Real-time sync <100ms

O3: Device Adapters (3h)
    - Desktop: full resolution, complex Intent
    - Phone: small screen, simplified Intent
    - Tablet: medium screen, mixed complexity
    - AR: spatial gestures, full Intent

O4: Offline Caching (2h)
    - Cache last 5 minutes of intents
    - Serve from cache if hub offline
    - Sync when connectivity restored

O5: Tests (0h - included above)
    - Intent delivery latency <100ms (2)
    - Device adapter scaling (3)
    - Offline cache operation (3)
```

**Success Metrics:**
- ✅ Intent visible on 3+ devices simultaneously
- ✅ <100ms latency on mesh
- ✅ Offline operation for 5 minutes
- ✅ 8 new tests passing

**Tests:** 635 → 643 (+8)

---

## **TIER 3: Symbiotic (Weeks 6-7) - 8 Hours**

**Goal:** Agent adapts to your physical state (stress, focus, fatigue)

**Dependencies:**
- btleplug (BLE): 2k stars, MPL-2.0

**Tasks:**
```
S1: Setup btleplug (2h)
    - BLE device discovery
    - Cross-platform support

S2: Heart Rate Service (2h)
    - Apple Watch: read HR characteristic
    - Oura Ring: read biometric data
    - Continuous polling

S3: State Classification (2h)
    - High HR + low HRV = stress
    - Low HRV = recovery
    - EEG data (if available) = focus
    - Fatigue = low activity + high HR

S4: Intent Scaling (1h)
    - Stress → simplify Intent
    - Low focus → increase prompting
    - High fatigue → suggest breaks

S5: Tests (1h)
    - BLE polling tests (3)
    - State classification tests (2)
    - Intent scaling tests (2)
```

**Success Metrics:**
- ✅ Detect stress spike within 5 seconds
- ✅ Adjust Intent appropriately
- ✅ Works across all applications
- ✅ 7 new tests passing

**Tests:** 643 → 650 (+7)

---

## **TIER 4: Phygital (Weeks 7-8) - 20 Hours**

**Goal:** Transition from desktop overlay to room-space AR

**Dependencies:**
- OpenXR-RS (AR/VR): 500+ stars, Apache 2.0

**Tasks:**
```
P1: OpenXR Setup (3h)
    - Session initialization
    - Graphics API binding (Vulkan/D3D)
    - Frame timing

P2: Head Pose Tracking (3h)
    - Poll frame state
    - Extract head position + rotation
    - Eye gaze (optional)

P3: Depth Mesh Processing (5h)
    - Capture depth frames (RGB-D)
    - Convert to point cloud
    - Filter/downsample

P4: Landmark Detection (5h)
    - RANSAC plane fitting (surfaces)
    - Connected component analysis (objects)
    - Semantic labeling (workbench, wall)

P5: Hand Tracking (2h, optional)
    - Hand pose from AR device
    - Gesture recognition (pointing, pinching)
    - Interaction hints

P6: Glass Workshop Integration (2h)
    - Project designs into room space
    - Spatial audio for feedback
    - Persistent anchor points

P7: Tests (0h - included above)
    - Frame polling tests (3)
    - Depth conversion tests (3)
    - Landmark detection tests (2)
    - Hand tracking tests (2)
```

**Success Metrics:**
- ✅ Render prototype in AR
- ✅ Project PDF manual above desk
- ✅ <50ms latency from hand gesture to response
- ✅ 10 new tests passing

**Tests:** 650 → 660 (+10)

---

## **WEEK 8: DNA Bank & Inference**

**DNA Bank (RocksDB):**
- Append-only event log on SSD
- Column families per tier
- Range queries by timestamp
- Compression: LZ4

**Inference (ONNX Runtime):**
- GGUF model conversion to ONNX
- GPU acceleration (CUDA/Metal/Vulkan)
- Batch inference for Intent generation
- <150ms inference latency

**Tests:** 660 → 665 (+5)

---

## **VALIDATION PHASE (Weeks 9-11)**

### **Week 9: Stress Testing & Documentation** (30 hours)
- 7-day autonomous VBE run (all tiers)
- Multi-device Omnipresent test (3+ devices)
- Biometric integration under stress
- AR rendering performance validation
- User guide for Visionary
- Integration guide for developers
- Example workflows

### **Week 10: Performance Optimization** (25 hours)
- Profile all components (GPU, CPU, memory)
- Optimize hot paths (rendering, inference, sync)
- Memory reduction (target: <4GB active)
- Latency optimization (design gen <2s, inference <150ms)

### **Week 11: Deployment & Release** (20 hours)
- Build release binaries (Windows, Linux, macOS)
- Installation guide with troubleshooting
- CI/CD pipeline (GitHub Actions)
- Version bump (1.0.0) and changelog
- GitHub release

---

## **Final Metrics**

| Metric | Start | End | Change |
|--------|-------|-----|--------|
| **Tests Passing** | 555 | 665+ | +110 (+20%) |
| **Code (LOC)** | 35,000 | 40,000 | +5,000 |
| **Phases** | 1 | 8 | +7 |
| **Tiers** | 0 | 4 | +4 |
| **Libraries** | 5 | 15+ | +10 |
| **Hours** | - | 141 | - |
| **Weeks** | - | 11 | - |

---

## **Key Dates**

- **Week 1 End (May 7):** Phase A complete, Watcher functional
- **Week 3 End (May 21):** Phase B complete, Design generation live
- **Week 5 End (June 4):** Tier 1 Visionary complete, 635+ tests
- **Week 8 End (June 25):** All tiers operational, 665 tests
- **Week 9 End (July 2):** Validation complete
- **Week 10 End (July 9):** Optimized
- **Week 11 End (July 21):** Production ready, 1.0.0 released

---

## **Success Criteria: Full System**

### **Visionary (Week 5)**
- [ ] Agent generates designs you actually want
- [ ] Learns your aesthetic after 1 week
- [ ] Runs autonomously during idle
- [ ] 635+ tests passing

### **Omnipresent (Week 6)**
- [ ] Same Ariel visible on 3+ devices
- [ ] Intent sync <100ms
- [ ] Offline operation 5 min
- [ ] 643+ tests

### **Symbiotic (Week 7)**
- [ ] Detect stress within 5 seconds
- [ ] Adapt Intent appropriately
- [ ] Works universally
- [ ] 650+ tests

### **Phygital (Week 8)**
- [ ] AR rendering in room space
- [ ] Project manuals above desk
- [ ] <50ms response time
- [ ] 660+ tests

### **Complete System**
- [ ] 7-day stress test (no crashes)
- [ ] Multi-device operation
- [ ] Full diagnostic logging (DNA Bank)
- [ ] <4GB memory usage
- [ ] 665+ tests passing
- [ ] Production binaries ready
- [ ] Installation guide written

---

## **The Loop: What Aaroneous Does**

```
Monday:
  └─ You code in VSCode
  └─ Glass learns: dark theme, clean fonts, sans-serif preference

Monday Night (VFD <15% duty):
  └─ Visionary generates 10 designs splicing VSCode style + Steam UI
  └─ You sleep

Tuesday:
  └─ Review 3 designs in Glass Workshop
  └─ Approve favorite → feedback stored

Wednesday:
  └─ Code error detected
  └─ Ariel: "You need error logs"
  └─ Opens terminal, projects troubleshooting guide in AR
  └─ Error fixed, lesson logged in DNA Bank

Thursday (on phone):
  └─ Same Ariel sends you Intent about pending task
  └─ Adapted for small screen (simplified wording)
  └─ Works offline if no internet

Friday (wearing AR glasses):
  └─ Your heart rate spikes (stress detected)
  └─ Ariel simplifies Intent prose
  └─ Suggests 5-min break
  └─ Renders tips in 3D space above workbench

Friday Night (Reflection):
  └─ Visionary analyzes: which designs did you approve?
  └─ Updates preference model
  └─ Tomorrow's designs will match your taste even better

By Week 2:
  └─ Aaroneous is generating designs you love
  └─ Without you typing anything
  └─ It learned your aesthetic by watching

By Week 4:
  └─ Agent anticipates your needs
  └─ Detects problems before you do
  └─ Scales responses to your stress level

By Week 8:
  └─ Works on your phone, tablet, AR glasses, desktop
  └─ Learns from your behavior across all devices
  └─ Reflects on its decisions every night
  └─ Gets smarter every day
```

---

## **No Blockers. Ready to Build.**

**Status: 🚀 READY FOR WEEK 1 IMPLEMENTATION**

All code examples are provided. All libraries are identified. All tests are designed. The path is clear.

**Next Step:** Begin Phase A (Week 1) Monday morning.

- Day 1-2: Setup Tesseract, Palette, imageproc
- Day 2-3: Implement AnchorDetector
- Day 3-4: Implement ContextEvent + IntentRouter
- Day 4-5: Write and validate tests
- Day 5: Merge, celebrate, move to Phase B

**Target:** 573 tests passing by Friday May 7, 2026.
