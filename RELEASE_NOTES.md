# Aaroneous OS v1.2.0 - The Sovereign Core Update

This is the largest architectural update in the history of the Aaroneous Desktop Hypervisor. The engine has been completely decoupled from the cloud, hardware-accelerated, and transformed into a true local-first AI Operating System.

## 🔴 Priority 1: The Core Intercom & Modularity
*   **Dynamic UI Plugin Hot-Loading:** Integrated libloading for zero-restart C-ABI UI cartridge swapping.
*   **Native Scratchpad Plugin:** Built a standalone DLL intercom with native Markdown rendering and syntax highlighting via egui_commonmark.
*   **Global Summoning Hook:** Zero-latency Win32 global hotkey (Ctrl+Alt+Space) for instant overlay access.
*   **Live State Persistence:** Switched to .ron for deterministic layout saves.
*   **Deep Clipboard Integration:** Native OS thread-safe read/write via rboard.
*   **Watchdog Daemon:** Headless auto-recovery mechanism isolates UI crashes from core workflows.
*   **Local Brain Bridge:** Direct LLM integration into LM Studio (http://localhost:1234/v1).

## 🟠 Priority 2: Dashboard Magic & Semantic Routing
*   **Dashboard Tag Filtering:** Command Palette now natively filters active semantic contexts (#dev, #gaming).
*   **Dynamic Projection Router:** Hot-swaps .si AI models based on active dashboard tag resolution.
*   **Edge-Compute Tagger:** Heuristically analyzes foreground process memory/titles to auto-tag workflows (0 token cost).
*   **Episodic Memory Pipeline:** Direct ingestion vector pipeline for the HNSW R^256 fabric.
*   **Background Watchers & Crawlers:** Added 
otify daemon for file changes and eqwest for headless URL scraping.
*   **Multi-Monitor Detachment:** Enabled native eframe multiple viewports for tear-away developer panels.
*   **Offline Data Sync:** CalDAV & IMAP pipeline integrated into vector memory for schedule context.

## ⚡ Priority 3: The Power-User Flex
*   **DirectX Game Overlay Injection:** Built DLL payload injection for native game rendering over SwapChains.
*   **Hardware Telemetry:** Direct 
vml-wrapper integration for GPU temperatures and VRAM allocation.
*   **OpenTelemetry:** OTLP exports for Jaeger/Prometheus visualization of AI thought pipelines.
*   **NDI & Physical Hooks:** Zero-latency OBS streaming and OSC/MIDI hook listeners for Elgato hardware.
*   **Process Management:** Autonomous RAM/CPU anomaly detection via sysinfo with task-killer capabilities.
*   **Hardware RGB:** Hooks into OpenRGB to shift case lighting based on AI system load.

## 🔵 Priority 4: Ecosystem Polish & Extensibility
*   **DAP Server:** Enabled VSCode / Neovim debugger attachment to the internal AI Semantic Router.
*   **BYOW Installer:** Refactored engine to <15MB, fetching GGUF weights asynchronously on launch.
*   **Safe Mode Boot:** Implemented Shift key bypass for UI un-bricking.
*   **Zero-Warning CI/CD:** Hardened GitHub Actions pipeline enforcing strict Clippy compliance.
*   **P2P Swarm Fleet:** Decentralized Iroh-based QUIC mesh node integration.
*   **Dynamic Compiler:** Built a real-time cargo build macro exporter for AI self-modification.

All roadmap dependencies have been successfully cleared. The substrate is stable.