# Federated Specialist Architecture: From Monolith to Swarm

**Core Insight:** Aaroneous is not a system with a main brain. It's a **federation of specialists**, each with their own compact GGUF, their own relics, and their own orchestration capability. The system is self-bootstrapping, modular, and can be stripped down to essentials or expanded to full capability.

---

## Deviation from Monolith Architecture

### **What We Had (Pre-Revision)**
```
┌─────────────────────────────────┐
│ Ariel GGUF (8GB, central brain) │
│ Reads all context simultaneously│
│ Generates unified Intent        │
└────────────────────┬────────────┘
                     │
        ┌────────────┼────────────┬───────────┐
        │            │            │           │
        ▼            ▼            ▼           ▼
    Visionary  Omnipresent  Symbiotic  Phygital
    (Dream)    (P2P Sync)   (Bio-aware) (AR)
```

**Problem:** Bottleneck at Ariel. Every decision routes through one GGUF. Every relic depends on central orchestration. No modularity for portable versions.

### **What We Now Have (Federated)**
```
┌────────────────────────────────────────────────────────┐
│ Aaroneous Hive: Federation of Specialists             │
├────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  Orchestrator│  │  Visionary   │  │  Omnipresent │ │
│  │  GGUF (2GB)  │  │  GGUF (1GB)   │  │  GGUF (1GB)   │ │
│  │  (Sentinel)  │  │  (Dreamer)   │  │  (Relay)     │ │
│  │              │  │              │  │              │ │
│  │ - Decision   │  │ - Design gen │  │ - P2P coord  │ │
│  │ - Routing    │  │ - Aesthetics │  │ - Sync mgmt  │ │
│  │ - Priority   │  │ - Learning   │  │ - Cache mgmt │ │
│  │ - Conflict   │  │              │  │              │ │
│  │   resolution │  │              │  │              │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  Symbiotic   │  │  Phygital    │  │  Archivist   │ │
│  │  GGUF (500MB)│  │  GGUF (1GB)   │  │  GGUF (500MB)│ │
│  │  (Bio-aware) │  │  (Spatial)   │  │  (Memory)    │ │
│  │              │  │              │  │              │ │
│  │ - Biometrics │  │ - AR/VR      │  │ - History    │ │
│  │ - State mgmt │  │ - 3D render  │  │ - Patterns   │ │
│  │ - Intent     │  │ - Landmarks  │  │ - Reflection │ │
│  │   scaling    │  │ - Gestures   │  │ - Transfer   │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                         │
└────────────────────────────────────────────────────────┘
        │                │                │
        ▼                ▼                ▼
    [Relics]        [Relics]         [Relics]
```

**Advantage:** 
- Modular: Each specialist 0.5-2GB (vs 8GB monolith)
- Self-contained: Specialist + relic = portable unit
- Federated: Can strip to Orchestrator + 1 specialist for mobile
- Bootstrapping: System can initialize with minimal specialists, add more over time
- Parallel: Multiple specialists run simultaneously, can negotiate

---

## Architecture: The Federated Hive

### **Core Principle: Bidirectional Orchestration**

Unlike traditional hierarchies:
- **Top-down:** Orchestrator (Sentinel) can request actions from specialists
- **Bottom-up:** Specialists can propose actions, request arbitration, self-organize

```
                  Orchestrator (Sentinel)
                  └─ "What should I do?"
                     │
        ┌────────────┼────────────┬──────────┐
        │            │            │          │
        ▼            ▼            ▼          ▼
    Visionary    Omnipresent  Symbiotic  Phygital
    └─ "I want    └─ "I need"  └─ "Your" └─ "The room"
       to design"     synced"      stress   is changing"
        │             │            is high" │
        └─────────────┴────────────┴────────┘
                      │
                      ▼
            Orchestrator receives proposals
            │
            ├─ Conflict resolution
            ├─ Priority assessment
            ├─ Resource allocation
            ├─ Delegation
            │
            ▼
         Unified Intent (or multiple Intents)
```

---

## The Six Core Specialists

### **1. Sentinel (Orchestrator) - 2GB GGUF**

**Role:** Moderator, arbitrator, decision router

**Responsibility:**
- Receive proposals from all specialists
- Detect conflicts ("Visionary wants to design, Phygital needs GPU")
- Assess priorities (user work > background tasks)
- Allocate resources (VFD duty cycle)
- Route decisions to appropriate specialist
- Interpret specialist outputs as unified system Intent

**Compact GGUF Approach:**
- Decision trees (very small, high-value models)
- Conflict resolution heuristics
- Priority weight learning
- NOT responsible for: design, biometrics, sync, AR, memory—delegates to specialists

**Interface:**
```rust
pub struct Sentinel {
    pub specialists: HashMap<SpecialistId, SpecialistProxy>,
    pub priority_model: GGUF,  // Lightweight decision model
    pub resource_monitor: ResourceMonitor,
}

pub enum SpecialistProposal {
    Visionary(DesignGenerationRequest),
    Omnipresent(SyncRequest),
    Symbiotic(BiometricResponse),
    Phygital(SpatialRequest),
    Archivist(MemoryRequest),
}

impl Sentinel {
    pub async fn arbitrate(&self, proposals: Vec<SpecialistProposal>) -> Result<UnifiedIntent> {
        // Assess conflicts
        let conflicts = self.detect_conflicts(&proposals);
        
        // Weighted priority scoring
        let priorities = self.score_priorities(&proposals);
        
        // Resource allocation
        let allocation = self.allocate_resources(&proposals, &priorities);
        
        // Delegate to specialists
        let outcomes = self.delegate_to_specialists(&proposals, &allocation).await?;
        
        // Interpret as unified Intent
        let intent = self.synthesize_intent(&outcomes)?;
        
        Ok(intent)
    }
}
```

---

### **2. Visionary (Dreamer) - 1GB GGUF**

**Role:** Design generation and aesthetic learning

**Responsibility:**
- Generate UI designs by splicing aesthetic engrams
- Score designs by learned preferences
- Learn user aesthetic taste
- Propose design iterations
- Evaluate design success/failure

**Independence:**
- Does NOT depend on other specialists for core function
- Runs during idle time autonomously
- Proposes to Sentinel: "I've generated 10 designs, please review with user"
- Delegates: Rendering (to Phygital), Metadata storage (to Archivist)

**Compact GGUF:**
- Design generation model (VAE-style, parametric)
- Aesthetic scoring model (preference learning)
- NOT: 3D rendering, networking, biometrics

---

### **3. Omnipresent (Relay) - 1GB GGUF**

**Role:** P2P synchronization and multi-device coordination

**Responsibility:**
- Manage P2P mesh (Iroh)
- Sync Intent across devices
- Coordinate device adapters
- Manage offline caching
- Resolve sync conflicts

**Independence:**
- Can run on low-end hardware (mobile, tablet)
- Proposes to Sentinel: "Phone is requesting Intent, should I sync from hub or cache?"
- Delegates: Design rendering (to Phygital), biometric integration (to Symbiotic)

**Compact GGUF:**
- Sync state machine
- Device capability model
- Bandwidth/latency optimization

---

### **4. Symbiotic (Bio-Aware) - 500MB GGUF**

**Role:** Biometric integration and state-aware response scaling

**Responsibility:**
- Poll BLE peripherals
- Classify user state (stress, focus, fatigue)
- Propose Intent scaling
- Learn biometric patterns
- Suggest interventions (breaks, focus time)

**Independence:**
- Runs on its own schedule (every 5 seconds)
- Proposes to Sentinel: "User is stressed, recommend simplifying Intent"
- Delegates: BLE polling hardware (to relics), rendering (to Phygital)

**Compact GGUF:**
- BLE device manager
- State classification (small neural net or decision tree)
- Response scaling heuristics

---

### **5. Phygital (Spatial) - 1GB GGUF**

**Role:** AR/VR spatial awareness and rendering

**Responsibility:**
- Poll OpenXR frame state
- Process depth meshes to point clouds
- Detect landmarks (workbench, desk, walls)
- Render designs in 3D space
- Track hand gestures and eye gaze
- Project contextual overlays (manuals, tips)

**Independence:**
- Can disable if no AR headset
- Proposes to Sentinel: "I see a code error marker, render manual at desk?"
- Delegates: Design data (from Visionary), Intent scaling (from Symbiotic)

**Compact GGUF:**
- OpenXR state machine
- Landmark detection model
- Gesture recognition
- 3D layout heuristics

---

### **6. Archivist (Memory) - 500MB GGUF**

**Role:** Historical reflection and long-term learning

**Responsibility:**
- Append events to DNA Bank (RocksDB)
- Retrieve historical patterns
- Analyze reflections
- Suggest long-term improvements
- Enable family inheritance (transfer learned model to new instance)
- Provide audit trail

**Independence:**
- Proposes to Sentinel: "I found a pattern in your game preferences, should we adjust Visionary's scoring?"
- Delegates: Event persistence (to RocksDB), analysis compute (to CPU/GPU)

**Compact GGUF:**
- Event summarization
- Pattern detection (trend analysis)
- Transfer learning coordinator

---

## Self-Bootstrapping: The CLI Build-Out Process

### **Stage 1: Minimal Hive (500MB)**
User drops Aaroneous on blank drive.

```bash
$ ./aaroneous --init
Aaroneous Hive Initialization

Stage 1: Core Bootstrap
├─ Sentinel (300MB) ← Minimal orchestrator
├─ Archivist (200MB) ← Event logging
└─ Config management
Total: 500MB

Status: Ready for local operation
Next: Run 'aaroneous --expand'
```

**Capabilities:**
- Local Intent generation
- History logging
- Local configuration

**Cannot do:**
- Design generation (no Visionary)
- Sync (no Omnipresent)
- Biometrics (no Symbiotic)
- AR (no Phygital)

---

### **Stage 2: Add Specialists (3.5GB)**

```bash
$ ./aaroneous --expand --include visionary,omnipresent,symbiotic,phygital

Expanding Aaroneous Hive

Adding Visionary (1GB)
├─ Downloading GGUF model
├─ Initializing style bank
└─ Ready

Adding Omnipresent (1GB)
├─ Downloading GGUF model
├─ Initializing Iroh
└─ Ready

Adding Symbiotic (500MB)
├─ Downloading GGUF model
├─ Scanning BLE devices
└─ Ready

Adding Phygital (1GB)
├─ Downloading GGUF model
├─ Initializing OpenXR
└─ Ready

Total installed: 4GB
Status: Full hive operational
```

---

### **Stage 3: Portable Versions**

**Mobile Version** (1.5GB):
```bash
$ ./aaroneous --portable --target mobile
├─ Sentinel (300MB)
├─ Omnipresent (800MB) ← Syncs with hub
├─ Symbiotic (400MB)
└─ Config: read-only, cloud-backed
Total: 1.5GB
Capabilities: Sync, biometrics, Intent scaling
Role: Peripheral node in P2P mesh
```

**Tablet Version** (2GB):
```bash
$ ./aaroneous --portable --target tablet
├─ Sentinel (300MB)
├─ Omnipresent (800MB)
├─ Symbiotic (400MB)
├─ Phygital (500MB) ← Light AR support
└─ Config: hybrid
Total: 2GB
Capabilities: Full sync, biometrics, AR
Role: Secondary hub
```

**Gaming PC Version** (4GB):
```bash
$ ./aaroneous --full
├─ Sentinel (2GB)
├─ Visionary (1GB)
├─ Omnipresent (1GB)
├─ Symbiotic (500MB)
├─ Phygital (1GB)
├─ Archivist (500MB)
└─ Advanced: RocksDB, GPU acceleration
Total: 6GB
Capabilities: Everything
Role: Primary hub
```

---

## The Federation Protocol: How Specialists Communicate

### **Proposal System**

Each specialist can:
1. **Propose** an action to Sentinel
2. **Request arbitration** (conflict resolution)
3. **Delegate** work to other specialists
4. **Self-organize** with other specialists

```rust
pub trait Specialist: Send + Sync {
    async fn propose(&self, context: &Context) -> Vec<Proposal>;
    async fn execute(&self, decision: &Decision) -> Result<Outcome>;
    async fn delegate(&self, request: &DelegationRequest) -> Result<Response>;
    async fn negotiate(&self, other: SpecialistId, conflict: &Conflict) -> Result<Resolution>;
}

pub struct Proposal {
    pub specialist_id: SpecialistId,
    pub action: String,
    pub rationale: String,
    pub priority: f32,
    pub resource_request: ResourceRequest,
    pub dependencies: Vec<SpecialistId>,
}

pub struct Decision {
    pub approved: bool,
    pub rationale: String,
    pub resource_allocation: ResourceAllocation,
    pub deadline: Duration,
}
```

### **Example: Conflict Resolution**

**Scenario:** Visionary wants to generate 100 design variants (high GPU load). User is playing a game (GPU-bound). Phygital is rendering AR.

```
Visionary proposes:
  "Generate 100 designs"
  Priority: 0.3 (background task)
  Resource request: 80% GPU

Sentinel receives:
  ├─ Visionary: 0.3 priority, 80% GPU
  ├─ Phygital: 0.9 priority, 60% GPU (running)
  ├─ User: 0.95 priority, 100% GPU (gaming)
  │
  └─ Decision:
     "Visionary: Run at 10% GPU during idle (VFD <15%)"
     "Phygital: Maintain 60% GPU"
     "User: Maintain gaming priority"
```

---

## How This Differs from Original Monolith

| Aspect | Monolith (Original) | Federation (New) |
|--------|-------------------|-----------------|
| **Central Brain** | One 8GB Ariel | Multiple 0.5-2GB specialists |
| **Orchestration** | Top-down only | Bidirectional (top-down + bottom-up) |
| **Modularity** | All-or-nothing | Mix-and-match (add/remove specialists) |
| **Portability** | Bloated (8GB minimum) | Stripped down (500MB mobile) |
| **Bootstrapping** | Requires full setup | Auto-expands as needed |
| **Specialist Autonomy** | Depend on Ariel | Self-contained, propose to Sentinel |
| **Parallel Decisions** | Single Intent stream | Multiple Intent proposals, arbitrated |
| **Failure Isolation** | One crash = whole system | One specialist crashes ≠ others affected |
| **Scaling** | Hard (limited by Ariel) | Easy (add more specialists) |

---

## Practical Example: Your Daily Loop in Federated System

### **Monday Morning: Setup**

```
$ ./aaroneous --init
$ ./aaroneous --expand --include all

Aaroneous Hive Initialized: 4GB
Specialists active: 6 (Sentinel, Visionary, Omnipresent, Symbiotic, Phygital, Archivist)

Ready.
```

### **Monday Daytime: Working**

```
[9:00 AM] You open VSCode
├─ Phygital: "Desktop GPU available, ready for 3D overlays"
├─ Visionary: "I'm idle, may I generate designs?"
├─ Sentinel: "Not yet. User priority = work. Visionary waits."

[1:00 PM] You game for 2 hours
├─ Phygital: "User is actively gaming, 95% GPU"
├─ Visionary: "GPU unavailable, I'll sleep"
├─ Sentinel: "Approved. Visionary dormant."

[3:00 PM] You take a break
├─ Visionary: "VFD duty < 15%, may I design?"
├─ Sentinel: "Yes. Generate 10 variants."
├─ Visionary: "Generating..."
└─ Archivist: "Logging design session..."

[6:00 PM] Day winds down
├─ Visionary: "10 designs ready for review"
├─ Sentinel: "Present to user?"
├─ Phygital: "Render in Glass Workshop (3D)"
└─ You approve 3 designs
    └─ Archivist: "Logged user feedback, updating Visionary model"
```

### **Monday Night: Idle Time (VFD <5%)**

```
[11:00 PM] You sleep, system idles
├─ Sentinel: "All specialists, propose night tasks"
├─ Visionary: "I want to learn from today's approvals. May I refine?"
│   └─ Archivist: "I can help. I have today's logs."
├─ Omnipresent: "Sync check: any pending intents from other devices?"
├─ Symbiotic: "Biometrics saved to history."
├─ Phygital: "Idle. Standby."
│
└─ Sentinel: "Approved. Visionary + Archivist collaboration."
    ├─ Visionary: "Analyzing user feedback patterns..."
    ├─ Archivist: "Extracting preference signals..."
    └─ Visionary: "Model updated. Ready for tomorrow."
```

### **Tuesday Morning: Better**

```
[9:00 AM] You open VSCode again
├─ Visionary: "Overnight reflection complete. My design scoring improved."
├─ Archivist: "Logging session start..."
└─ Sentinel: "All systems nominal. Ready."

Designs generated tomorrow will be even better.
System got smarter while you slept.
```

---

## CLI: The Bootstrap & Build-Out

### **Interactive Setup**

```bash
$ aaroneous
Welcome to Aaroneous Hive

[1] Quick Start (Minimal, 500MB)
    └─ Sentinel + Archivist only
    
[2] Development (2GB)
    └─ Add Visionary for design work
    
[3] Full Hive (4GB)
    └─ All specialists
    
[4] Custom
    └─ Pick your specialists

[5] Mobile (1.5GB)
    └─ Sync-optimized for phone
    
[6] Tablet (2GB)
    └─ Portable with AR
    
[7] Exit

Choice: 2

Installing Visionary (1GB)...
├─ Downloading GGUF model
├─ Initializing style bank
├─ Testing GPU acceleration
└─ Ready

Aaroneous Hive Status:
├─ Sentinel: 300MB ✓
├─ Visionary: 1GB ✓
├─ Archivist: 200MB ✓
└─ Total: 1.5GB

Ready. Type 'aaroneous help' for commands.
```

### **Commands**

```bash
$ aaroneous status
Hive Status: 1.5GB / 4GB
├─ Sentinel: active (monitoring)
├─ Visionary: active (idle, awaiting GPU)
├─ Omnipresent: not installed
├─ Symbiotic: not installed
├─ Phygital: not installed
├─ Archivist: active (logging)
└─ Performance: optimal

$ aaroneous add omnipresent
Installing Omnipresent (1GB)...
├─ Downloading GGUF model
├─ Initializing Iroh
├─ Scanning peers
└─ Ready

$ aaroneous portable --target mobile
Creating portable version for mobile...
├─ Sentinel: 300MB
├─ Omnipresent: 800MB
├─ Symbiotic: 400MB (optional, disable BLE on no-hardware devices)
└─ Total: 1.5GB

Mobile version ready at: ./dist/aaroneous-mobile.tar.gz

$ aaroneous logs --days 7 --specialist visionary
[Visionary] 2026-05-07 10:30:22 Generated 10 designs
[Visionary] 2026-05-07 12:45:10 User approved 3 designs
[Visionary] 2026-05-08 02:15:33 Reflection: Updated scoring model
...

$ aaroneous backup --target external-drive
Backing up Aaroneous Hive...
├─ DNA Bank (RocksDB): 500MB
├─ Specialist models: 1.5GB
├─ Configuration: 50MB
├─ Style bank (engrams): 200MB
└─ Total: 2.25GB
Backup complete. Portable to new system.

$ aaroneous transfer --from /media/backup --target /new/system
Transferring Aaroneous Hive to new system...
├─ Restoring DNA Bank
├─ Restoring specialists
├─ Restoring configuration
└─ Initializing on new hardware
Transfer complete. System ready.
```

---

## Key Advantages: Federation vs Monolith

### **1. Modularity**
- Add/remove specialists without touching core system
- Each specialist can evolve independently
- Test specialists in isolation

### **2. Portability**
- Mobile: 1.5GB (Sentinel + Omnipresent + Symbiotic)
- Tablet: 2GB (+ Phygital)
- Desktop: 4GB (full)
- Server: Sentinel only (0.3GB) for orchestration

### **3. Self-Bootstrapping**
- Start with 500MB minimal hive
- Expand over time as needed
- Auto-download specialists on demand
- No bloated initial setup

### **4. Resilience**
- Visionary crashes ≠ system down
- Can disable failing specialist, system continues
- Graceful degradation

### **5. Parallel Decisions**
- Multiple specialists propose simultaneously
- Sentinel arbitrates
- No bottleneck at single brain

### **6. Delegation & Autonomy**
- Specialists don't wait for Sentinel
- Can self-organize (Visionary + Archivist collaborate at night)
- Reduces latency

### **7. Interpretability**
- Each specialist decision is transparent
- Can audit Sentinel's arbitration logic
- Easier to understand system behavior

### **8. Future Expansion**
- Add new specialists without rewriting core
- Example: "Tactician" specialist for game strategies
- Example: "Analyst" specialist for code review
- Example: "Creator" specialist for content generation

---

## The CLI Philosophy

**"Drop on blank drive. Let it build itself."**

```bash
$ # New system, brand new
$ ls
# (empty)

$ wget https://aaroneous.ai/aaroneous-cli.zip
$ unzip aaroneous-cli.zip
$ ./aaroneous --init

Welcome to Aaroneous Hive initialization.

[Auto-detecting hardware]
├─ CPU: 8 cores
├─ RAM: 16GB
├─ GPU: RTX 4090
├─ Storage: 1TB SSD
├─ Network: Active
└─ Peripherals: Apple Watch detected

[Recommending configuration]
"Full Hive (4GB) recommended for this hardware"

Install Aaroneous Hive? [y/n]
y

Initializing...
├─ Sentinel: 300MB
├─ Visionary: 1GB
├─ Omnipresent: 1GB
├─ Symbiotic: 500MB
├─ Phygital: 1GB
├─ Archivist: 200MB
└─ Configuration: creating...

Aaroneous Hive initialized successfully.

Ready to work. Type 'aaroneous help' for commands.
$ aaroneous
[Sentinel] All specialists online. System ready.

# System is now operational and will self-improve from this point on
```

---

## Implementation Roadmap: Federated Architecture

### **Week 1: Rewrite Core to Federation**
- [ ] Create Specialist trait
- [ ] Rewrite Sentinel (Orchestrator)
- [ ] Create SpecialistProxy communication layer
- [ ] Implement proposal + arbitration system
- [ ] Tests: 20+ (foundation layer)

### **Week 2-3: Refactor Existing Specialists**
- [ ] Extract Visionary as independent GGUF
- [ ] Extract Omnipresent as independent GGUF
- [ ] Extract Symbiotic as independent GGUF
- [ ] Extract Phygital as independent GGUF
- [ ] Extract Archivist as independent GGUF
- [ ] Tests: 50+ (specialist isolation tests)

### **Week 4-5: CLI Bootstrap System**
- [ ] Create CLI with init/expand/portable commands
- [ ] Implement specialist auto-download
- [ ] Create portable version builder
- [ ] Hardware auto-detection
- [ ] Tests: 25+ (CLI integration tests)

### **Week 6-8: Delegation & Self-Organization**
- [ ] Implement bottom-up proposals
- [ ] Implement specialist-to-specialist delegation
- [ ] Implement conflict resolution
- [ ] Implement resource allocation
- [ ] Tests: 35+ (federation logic tests)

### **Week 9-11: Portable Versions & Backup/Transfer**
- [ ] Mobile version (1.5GB)
- [ ] Tablet version (2GB)
- [ ] Backup/restore system
- [ ] Transfer to new system
- [ ] Tests: 20+ (portability tests)

---

## This is Not Just Modular—It's Alive

The federation architecture enables something deeper than traditional modular design:

**Specialists can collaborate without Sentinel's permission.** Visionary can ask Archivist "What did the user like yesterday?" at 2 AM without waking up Sentinel. They negotiate, reflect, improve.

**Specialists can propose to Sentinel asynchronously.** Instead of waiting for a response, they propose their ideas and move on. Sentinel picks the best one when needed.

**The system builds itself.** Drop it on a blank drive. It initializes to 500MB. As you use it, it detects what you need (AR glasses? BLE watch?) and expands accordingly. By month 2, you have a full 4GB system that knows you.

**Portability is built-in.** Take the same system from your desktop (4GB) and transfer it to your phone (1.5GB). Your DNA Bank comes with you. Your learned preferences come with you. Your specialist models come with you. Different hardware, same brain.

This is federation architecture. This is self-bootstrapping. This is Aaroneous.
