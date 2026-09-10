# Code-Owner GUI Preferences & Alignment

## 0. Core Courtesy Principles (The Unrestricted Power User Manifesto)
Before any UI component or background service is written, it must adhere to the following strict courtesy bounds to ensure absolute respect for the user's hardware and agency:

1. **Strict Portable Mode (Zero-Footprint)**: The software must be capable of running as a raw `.zip`. It must touch absolutely zero Windows Registry keys, install no background services, and write all data exclusively to its local `./config` directory unless explicitly configured otherwise.
2. **Voluntary Yielding (Eco/Game Mode)**: The system must monitor the Windows Power API and full-screen state. If a heavy AAA game is launched or the laptop is unplugged, Aaroneous must voluntarily suspend its own background tasks (e.g., `SiForge` compilation) to yield CPU threads.
3. **Graceful Signal Handling (Anti-Corruption)**: The engine must trap `SIGINT`/`SIGTERM` (OS shutdowns) and use its final milliseconds to cleanly flush the `.lib` state bank and `hnsw_rs` databases. No data corruption upon forced reboots.
4. **Absolute "Zero Phone-Home" (Airgapped Default)**: The system assumes it is offline. Zero background analytics, zero unrequested update checks. The ecosystem must function at 100% capacity without throwing network errors if the internet is disconnected.

## 0.5. Onboarding & Contributor Standards
To ensure a flawless Developer Experience (DX) and user onboarding sequence, the architecture enforces the following practices:
* **Safe Mode Boot (`Shift` Interrupt)**: Guarantees recovery from broken 3rd-party UI plugins by bypassing `.si` injections and forcing the pristine `Default.ron` layout.
* **Bring Your Own Weights (BYOW)**: The installer remains under 15MB. It automatically soft-links existing LM Studio/Ollama `.gguf` files rather than forcing redundant 40GB downloads.
* **Visual Shortcut Overlays**: A master modifier (e.g., `Ctrl+Alt`) instantly draws hotkeys over every visible UI element, catering strictly to keyboard-heavy workflows.
* **Live "Dev-Mode" Event Sniffing**: A built-in diagnostic pane that exposes the `SignalBridge` IPC traffic, making the architecture entirely self-documenting for plugin developers.


## 1. Code-Owner UI/UX Preferences
Based on the software the code-owner enjoys (AutoCAD, Solidworks, Obsidian, Notepad++, Android, Overwolf, Steam) and actively rejects (macOS, Edge, McAfee), the GUI preferences are defined by the "Unrestricted Power User & Modder" archetype:

* **High Information Density & Utility**: Preference for dense, highly functional interfaces like CAD tools and IDEs. All tools should be accessible; avoid hiding essential features behind minimalist menus.
* **Modular, Dockable, and Customizable**: Like Obsidian's panes or Android's widgets, the interface must allow for repositioning, splitting, and building a personalized workspace.
* **Dark Mode & Technical Aesthetic**: Essential for long sessions (coding, gaming, 3D modeling).
* **Zero Nagware / Absolute Agency**: Software must never demand attention with pop-ups, forced integrations, or unrequested features (rejection of McAfee/Edge bloat).
* **Transparent Overlays / Game Injection**: Based on the usage of Overwolf and heavily modded gaming environments, the software should support click-through, non-intrusive telemetry overlays.
* **Deep System Telemetry**: Like IObit products, the UI should expose raw hardware and system data transparently.

## 2. Review of Available Aaroneous GUIs

The repository currently contains two distinct GUI paradigms:

### A. MaelstromUI (Legacy)
* **Location**: `MaelstromUI/`
* **Architecture**: Web-based (React/Tauri).
* **Status**: Deprecated.
* **Review**: While it served as an initial fascia, a web-based DOM cannot adequately satisfy the zero-latency, raw telemetry, and transparent game hooking required by the code-owner. It inherently introduces overhead and feels like a wrapped web page rather than a native system tool.

### B. Machine-Native Rust Desktop Studio (`a_hud`)
* **Location**: `core/hypervisor/src/hud/`
* **Architecture**: Pure Rust, `egui`, `eframe`, `wgpu`.
* **Features**: Command palette, spatial canvas with pan/zoom (like CAD), floating windows, compact overlays, and toast notifications.
* **Review**: This strongly aligns with the code-owner's preferences. `egui` allows for immediate-mode rendering with extremely low latency. The existing structure (SettingsView, Galaxy3DView, SignalAnalyzerView) fits the "Command Center" aesthetic perfectly.

## 3. Suggestions to Align Current State to Goal

To fully realize the code-owner's preferences in `a_hud`, the following steps should be prioritized:

1. **Purge the Legacy Bloat**: Completely delete the `MaelstromUI` directory. Keeping deprecated web tech in the repository contradicts the "zero bloat" philosophy.
2. **Implement an Advanced Docking System**: While `a_hud` has a spatial canvas (floating windows), implementing a strict docking system (like `egui_tiles` or `egui_dock`) will allow the user to lock panels into place exactly like AutoCAD or Solidworks.
3. **Formalize the Overlay (Overwolf-style)**: Expand the `render_transparent_hud` and `AppWindowMode::CompactRecorderOverlay` to leverage `hudhook` (Horizon 7). This will allow the HUD to directly inject into DirectX/Vulkan games (Skyrim, Black Ops) without window borders.
4. **Expose a Plugin/Mod API for the UI**: Emulate Obsidian and Notepad++ by allowing users to load `.si` cartridges that dynamically add new panels or widgets to the `egui` context at runtime.
5. **Telemetry Dashboards**: Ensure the `SystemThermoView` and `SignalAnalyzerView` prominently display hardware metrics (VRAM, CPU latency, TOPS) satisfying the IObit/tinkerer preference.

