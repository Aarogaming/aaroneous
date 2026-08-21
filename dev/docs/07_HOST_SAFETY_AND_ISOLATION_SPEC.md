# 07: Host Safety & Isolation Specification

## Incident Post-Mortem: Host Computer Instability

### Problem Description
Attempting to run Aaroneous has previously caused **severe operating issues for the host computer**:
- Spontaneous cursor movement, random clicking across the desktop, and accidental window dragging.
- Host PC freezing or stuttering due to unthrottled desktop screen capture in tight loops.
- High memory lockups and orphaned `.synapse` memory-mapped file handles remaining open in Windows `%TEMP%`.
- High GPU/CPU utilization from unconstrained background daemon threads.

---

## 🔬 Root-Cause Technical Dissection

### 1. Unconstrained Hardware Input Injection
- **Problem File**: `core/hypervisor/src/win32_intercept/hid_bridge.rs` and `spatial_kinetic_engine.rs`.
- **Mechanism**: The GPU compute reflex pipeline translates frame luminance activations into `MotorIntent` packets (containing `ACTION_MOUSE_MOVE`, `ACTION_CLICK`, `ACTION_DOUBLE_CLICK`, `ACTION_DRAG_START`).
- **Failure Mode**: `HIDOutputBridge::execute_intent()` immediately invoked Win32 `SendInput()` directly on the host OS desktop at 30–60 FPS. If any noise or untrained shader weights were present, the cursor erraticly jumped and clicked all over the developer's screen, wresting control away from the user.

### 2. Full-Desktop GDI Screengrab Overhead
- **Problem File**: `core/hypervisor/src/win32_intercept/capture.rs`.
- **Mechanism**: Invoked `GetDC(HWND::default())`, `CreateCompatibleBitmap`, and `StretchBlt()` on every frame tick.
- **Failure Mode**: Capturing the entire desktop surface continuously without throttling or backoff caused GDI handle leaks, desktop stutter, and CPU lockups.

### 3. Orphaned Memory-Mapped Handles
- **Problem File**: `core/hypervisor/src/autonomic_loop.rs` (`LegacySharedMemorySynapse`).
- **Mechanism**: Mapped `C:\Users\aarog\AppData\Local\Temp\primary.synapse` using `memmap2`.
- **Failure Mode**: When processes crashed or were forcefully terminated, file locks remained active, causing subsequent runs to fail or read corrupted state.

---

## 🛡️ Mandatory Safety Rules for Developers & Agents

### Rule 1: The Strict Dry-Run / Headless Default
- **`enable_hid_output` MUST BE FALSE BY DEFAULT** in all configuration structs and CLI runners.
- Direct Win32 `SendInput` execution can only be activated by explicitly passing both:
  1. `--enable-live-emulation` flag on the CLI.
  2. Setting environment variable `AARONEOUS_ALLOW_HOST_INPUT=1`.

### Rule 2: Swappable Mock Backends for Dev & CI
- When developing, testing, or running unit tests, Marionette MUST instantiate `MockMarionette`:
  - Visual inputs are served from synthetic in-memory test patterns or pre-recorded image files.
  - Motor outputs are written to an in-memory telemetry log buffer (`MotorIntentLog`) rather than calling OS APIs.

### Rule 3: Watchdog Timeout Enforcement
- The `TICK_WATCHDOG` (default: 10 seconds) must abort or log any single tick that exceeds budget.
- Automatic cooperative shutdown triggers if CPU temperature exceeds 85°C (`ThermalStatus::Critical`).

### Rule 4: Dedicated Cleanup Harness
- Before and after every development session, run `dev/tools/maintenance/purge_stale_synapse.ps1` and `dev/tools/diagnostics/host_safety_monitor.ps1` to ensure no zombie processes or locked `.synapse` files exist.
