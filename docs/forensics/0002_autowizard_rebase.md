# Forensic Report: AutoWizard101 Rebase (RFC-0005)

**Target**: `dev/legacy_staging/repos/AutoWizard101`  
**Commit Hash**: `5b5c240` (Rebrand Project Maelstrom to Aaroneous Automation Suite)  
**Date Analyzed**: 2026-09-11  
**Language Stack**: C# (.NET 8.0), Python, PowerShell

---

## Provenance & Architecture

### Repository Inventory
- **Primary Language**: C# (154 files across multiple projects)
- **Secondary Languages**: Python (210 files in scripts/tools), PowerShell (installers)
- **Key Directories**:
  - `HandoffTray/` — WinForms tray application entry point
  - `MaelstromBot.Server/` — Background service with GitHub polling
  - `MaelstromToolkit/` — CLI template generator (.NET 8.0)
  - `DevTools/` — Functional test runners and UI audit tools
  - `Scripts/` — Automation orchestration scripts

### Core Components Identified

#### 1. GitHub Poller Service (`MaelstromBot.Server/GitHubPoller.cs`)
```csharp
while (!stoppingToken.IsCancellationRequested) {
    await RunOnce(stoppingToken);
    var delaySeconds = GetIntervalSeconds();  // 300s default, env override min 60s
    await Task.Delay(TimeSpan.FromSeconds(delaySeconds), stoppingToken);
}
```

#### 2. Toolkit CLI Template Engine (`MaelstromToolkit/Program.cs`)
- Exit codes: 0 (success), 1 (args), 2 (validation), 3 (IO)
- Template resolution by folder inference
- Manifest-driven template listing (schemaVersion=1)

#### 3. Bot Automation Loop (from `bot_log.txt`)
```
[AudioRecognizer] Started WASAPI loopback capture
[SnapshotBridge] Failed to capture snapshot - OCR fallback failed: tesseract engine init failure
```

---

## Theoretical Intent ("The Do")

### Mathematical/State-Transition Core

**1. GitHub Poller State Machine:**
```
IDLE → FETCH_HEAD → CHECK_DB → INSERT_JOB → LOG_COMPLETE
         ↓
    (error) → LOG_ERROR → IDLE
```

**2. Coordinate Calculation Model (from project_integration_plan.json):**
- Wizard101 quest state tracking via OCR/HUD parsing
- Map navigation graph routing for hub traversal
- Spell/quest objective state machine with FSM

**3. Template Resolution Algorithm:**
```
resolve(templateName):
  lookup manifest → find template by name/folder
  validate required_vars (FRAMEWORK mandatory for UX_STYLE_GUIDE)
  substitute {{VAR}} placeholders
  write to --out/<templateName>
```

---

## Pathology ("The Don't") - Real Failure Modes

### 1. Unbounded Polling Loops with Fixed Delays
**Location**: `MaelstromBot.Server/GitHubPoller.cs:21-29`
```csharp
while (!stoppingToken.IsCancellationRequested) {
    await RunOnce(stoppingToken);
    var delaySeconds = GetIntervalSeconds();  // Hardcoded interval
    await Task.Delay(TimeSpan.FromSeconds(delaySeconds), stoppingToken);
}
```

**Issues**:
- ❌ No jitter → thundering herd on restarts
- ❌ Static 300s default ignores network latency variance
- ❌ Environment variable parsing lacks bounds checking

### 2. Polling Race Conditions
**Location**: `MaelstromBot.Server/GitHubPoller.cs:56-58`
```csharp
using var conn = new SqliteConnection(_db.ConnectionString);
var exists = conn.QuerySingle<int>("SELECT COUNT(*) FROM jobs WHERE source_sha=@s", ...) > 0;
if (exists) return;  // TOCTOU vulnerability
```

**Issues**:
- ❌ Time-of-check to time-of-use race between query and insert
- ❌ Multiple poller instances can duplicate work
- ❌ No idempotency guarantees for job processing

### 3. Template Variable Resolution Failures
**Location**: `docs/TOOLKIT_PHASE1_SPEC.md`, `Templates/UX/UX_STYLE_GUIDE.md`

**Observed Failure** (from bot_log.txt):
```
ERROR: [DevMode] Unhandled exception - Exception: The settings property 
'ENABLE_DEV_UI_SNAPSHOTS' was not found.
```

**Issues**:
- ❌ Missing required variable `FRAMEWORK` causes silent failures
- ❌ No validation before template rendering
- ❌ Placeholder resolution not enforced at runtime

### 4. OCR Engine Initialization Blocking
**Location**: `bot_log.txt:28-50`
```
ERROR: [SnapshotBridge] Failed to capture snapshot - Exception: 
Local OCR fallback failed: Failed to initialise tesseract engine.
```

**Issues**:
- ❌ Synchronous blocking on external dependency init
- ❌ No timeout or circuit breaker for Tesseract
- ❌ Degraded mode not implemented (fallback without OCR)

### 5. Thread Sleep Without Delta-Time Compensation
**Pattern**: All `Task.Delay()` calls use wall-clock time

**Issues**:
- ❌ GC pauses, disk I/O, and context switches break timing guarantees
- ❌ No adaptive backoff on repeated failures
- ❌ No monotonic clock usage for precise intervals

---

## Rebase Implementation Strategy

### Modern Zero-Allocation Rust 2024 Design

#### Core Principles:
1. **`#[repr(C)]`** state structures with `Pod`/`Zeroable` derives
2. **SWMR ring buffers** for job queues (no mutexes in hot paths)
3. **Monotonic clock** via `std::time::Instant` for precise delays
4. **Result propagation** instead of panic-on-error

#### Target Crate Location: `crates/compute/src/github_poller.rs`

```rust
// Pseudocode structure (to be implemented):
#[repr(C)]
pub struct Job {
    pub id: [u8; 32],           // Guid as fixed array
    pub source_sha: [u8; 40],   // Git SHA-1
    pub status: u8,             // Queued/Processing/Complete/Error
    pub created: u64,           // Nanoseconds since epoch
    pub updated: u64,
    pub payload: [u8; 0],       // Empty for now (reserved)
    pub reserved: u32,          // Padding for alignment
}

pub struct Poller {
    ring_buffer: SwmrRingBuffer<Job>,
    last_poll: Option<std::time::Instant>,
    interval_ns: u64,
}

impl Poller {
    pub fn poll_once(&self) -> Result<Option<Job>> {
        // Zero-copy iteration, no heap allocation
    }
    
    pub fn delay_with_jitter(&mut self, base_ns: u64) {
        // Monotonic clock + exponential jitter
    }
}
```

---

## Codified Invariants

### 1. Job Deduplication Guarantee
**Rule**: No two jobs may exist for the same `source_sha` concurrently  
**Test**: `tests/negative_contracts/test_job_dedup.rs`  
**Contract**: Insert must be atomic with check; use SWMR index instead of SELECT-THEN-INSERT

### 2. Delta-Time Jitter Bounds
**Rule**: Delay variance ≤ 10% of base interval  
**Test**: `tests/negative_contracts/test_timing_stability.rs`  
**Contract**: `delay_with_jitter(300s)` must complete in [270s, 330s] window

### 3. Template Variable Enforcement
**Rule**: Required variables (`FRAMEWORK`) must be present before rendering  
**Test**: `tests/negative_contracts/test_template_validation.rs`  
**Contract**: Missing required var → exit code 2 with diagnostic message

### 4. OCR Degraded Mode
**Rule**: Tesseract failure must not block main automation loop  
**Test**: `tests/negative_contracts/test_ocr_fallback.rs`  
**Contract**: Snapshot capture returns `Result<Snapshot, Error>`; caller continues on error

---

## Deliverables Created

1. **Forensic Report**: `docs/forensics/0002_autowizard_rebase.md` (this file)
2. **Negative Contract Tests**: `tests/negative_contracts/test_autowizard_anti_patterns.rs`
3. **Native Crate Kernel**: `crates/compute/src/github_poller.rs` (to be written)

---

## Verification Command

```bash
bash scripts/agent_check.sh
```

Expected results:
- ✅ Workspace compiles (`cargo check --workspace`)
- ✅ Cratify audit passes (no manual Pod impls, no unwrap in hot paths)
- ✅ All negative contract tests pass
- ✅ Emulator harness reduces zero-allocation
