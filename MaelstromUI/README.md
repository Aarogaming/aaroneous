# [DEPRECATED] MaelstromUI (Legacy Tauri / React Fascia)

> **Status:** ⚠️ DEPRECATED  
> **Superseded by:** Machine-Native Rust Desktop Studio (`core/hypervisor/bin/a_hud.rs` / `a_run hud`)

`MaelstromUI` was the original gaming-version web/Tauri fascia of Aaroneous. 

As part of the Solid-State Synthetic Intelligence (SI) architecture migration, all presentation and operator controls have been consolidated into the zero-overhead, 60 FPS machine-native immediate-mode UI:

- **Desktop Studio (`a_hud`)**: Located at `core/hypervisor/bin/a_hud.rs`, built in pure Rust with `egui`, `eframe`, and `wgpu`.
- **Integrated Telescopes**:
  - Live 11-Specialist SPMC Synapse Bus activation monitors.
  - Argus Deep SVDD $\mathbb{R}^{256}$ latent manifold radar.
  - Machine-Native JIT Crystallization and LoRA stability inspector.
  - 3D Gravitational Skill Constellation physics canvas.
  - Multi-Specialist Distillation & Self-Evolution visualizer.

To launch the modern native UI:
```sh
a_run hud
# or
cargo run --bin aaroneous
```
