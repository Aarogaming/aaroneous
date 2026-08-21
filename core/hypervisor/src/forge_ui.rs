//! core/hypervisor/src/forge_ui.rs
//! Aaroneous Desktop Studio: Forge Studio Module.
//!
//! Features:
//! 1. Decouples heavy model distillation & packaging from egui's 60Hz rendering loop
//!    via background worker threads and mpsc message passing channels.
//! 2. Real-time progress bar streaming (Distilling -> Packing -> Complete).
//! 3. Monospace live log viewer auto-scrolling to recent output.

use eframe::egui::{self, Color32, ProgressBar, RichText, ScrollArea, TextStyle, Ui};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use compute::si_forge::SiForge;
use compute::si_packer::SiTierFlags;

/// Status of the background model birthing process
#[derive(Clone, PartialEq, Debug)]
pub enum ForgeStatus {
    Idle,
    Distilling(f32), // Progress (0.0 to 1.0)
    Packing,
    Complete(PathBuf),
    Error(String),
}

/// The Forge Studio UI State
pub struct ForgeStudio {
    // Input parameters
    pub model_name: String,
    pub selected_tier: SiTierFlags,
    pub dataset_path: String,
    pub epochs: usize,
    pub samples: usize,

    // Concurrency & reactive state
    pub current_status: ForgeStatus,
    pub status_rx: Option<Receiver<ForgeStatus>>,
    pub log_buffer: String,
    pub log_rx: Option<Receiver<String>>,
}

impl Default for ForgeStudio {
    fn default() -> Self {
        Self {
            model_name: "chimera_ast_v1".to_string(),
            selected_tier: SiTierFlags::TIER_3_REFLEX,
            dataset_path: "rosetta_code.si".to_string(),
            epochs: 5,
            samples: 50,
            current_status: ForgeStatus::Idle,
            status_rx: None,
            log_buffer: String::new(),
            log_rx: None,
        }
    }
}

impl ForgeStudio {
    pub fn new() -> Self {
        Self::default()
    }

    /// Dispatches the birthing pipeline on a dedicated background thread
    pub fn dispatch_birthing_thread(&mut self) {
        let (status_tx, status_rx) = channel();
        let (log_tx, log_rx) = channel();

        self.status_rx = Some(status_rx);
        self.log_rx = Some(log_rx);
        self.current_status = ForgeStatus::Distilling(0.1);
        self.log_buffer.clear();

        let name = self.model_name.clone();
        let tier = self.selected_tier;
        let dataset_str = self.dataset_path.clone();
        let epochs = self.epochs;
        let samples = self.samples;

        thread::spawn(move || {
            let _ = log_tx.send(format!("🔥 [SiForge] Initializing birthing process for '{}' ({})", name, tier.label()));
            let _ = log_tx.send(format!("   -> Target Architecture: {}", tier.label()));
            let _ = log_tx.send("   -> Step 1: Synthesizing / loading teacher trajectories...".into());
            let _ = status_tx.send(ForgeStatus::Distilling(0.3));

            let paths = aaroneous_paths::WorkspacePaths::discover();
            let out_dir = paths.data().join("models");
            let dataset_path = if dataset_str.is_empty() {
                paths.data().join("datasets").join("rosetta_stone.si")
            } else {
                PathBuf::from(&dataset_str)
            };

            let forge = SiForge::new(&name)
                .with_tier(tier)
                .with_training_data(dataset_path)
                .with_training_params(epochs, 16, 0.001, samples);

            let _ = status_tx.send(ForgeStatus::Distilling(0.7));
            let _ = log_tx.send("   -> Step 2: Training 2-layer GeLU bridge with CKA + InfoNCE loss...".into());

            let _ = status_tx.send(ForgeStatus::Packing);
            let _ = log_tx.send("   -> Step 3: Packing 64-byte aligned solid-state container...".into());

            match forge.birth(&out_dir) {
                Ok(path) => {
                    let _ = log_tx.send(format!("✅ Successfully birthed and verified at {:?}", path));
                    let _ = status_tx.send(ForgeStatus::Complete(path));
                }
                Err(e) => {
                    let _ = log_tx.send(format!("❌ Forge Error: {}", e));
                    let _ = status_tx.send(ForgeStatus::Error(e.to_string()));
                }
            }
        });
    }

    /// Update function called at 60Hz inside egui rendering loop
    pub fn update_ui(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        // 1. Drain background message queues non-blockingly
        if let Some(rx) = &self.status_rx {
            while let Ok(new_status) = rx.try_recv() {
                self.current_status = new_status;
            }
        }
        if let Some(rx) = &self.log_rx {
            while let Ok(msg) = rx.try_recv() {
                self.log_buffer.push_str(&format!("> {}\n", msg));
            }
        }

        ui.heading("⚒️ Machine-Native SiForge Studio");
        ui.label("Configure, distill, and pack solid-state .si containers with 64-byte SIMD alignment.");
        ui.add_space(8.0);

        // 2. Configuration Form
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Model Identifier:");
                ui.text_edit_singleline(&mut self.model_name);
            });

            ui.horizontal(|ui| {
                ui.label("Dataset Path:");
                ui.text_edit_singleline(&mut self.dataset_path);
            });

            ui.horizontal(|ui| {
                ui.label("Epochs:");
                ui.add(egui::Slider::new(&mut self.epochs, 1..=20).text("epochs"));
                ui.label("Samples:");
                ui.add(egui::Slider::new(&mut self.samples, 10..=500).text("tasks"));
            });

            ui.add_space(4.0);
            ui.label(RichText::new("Architectural Tier Designation:").strong());
            ui.horizontal(|ui| {
                if ui.selectable_label(self.selected_tier.is_cortex(), "Tier 1: Cortex (R^4096)").clicked() {
                    self.selected_tier = SiTierFlags::TIER_1_CORTEX;
                }
                if ui.selectable_label(self.selected_tier.is_router(), "Tier 2: Router (R^256)").clicked() {
                    self.selected_tier = SiTierFlags::TIER_2_ROUTER;
                }
                if ui.selectable_label(self.selected_tier.is_reflex(), "Tier 3: Reflex (R^256)").clicked() {
                    self.selected_tier = SiTierFlags::TIER_3_REFLEX;
                }
            });
        });

        ui.add_space(8.0);

        // 3. Action Controls & Real-Time Status Progress
        ui.group(|ui| {
            match &self.current_status {
                ForgeStatus::Idle => {
                    if ui.button(RichText::new("🔥 Birth .si Container").size(15.0).color(Color32::WHITE)).clicked() {
                        self.dispatch_birthing_thread();
                    }
                }
                ForgeStatus::Distilling(progress) => {
                    ui.label(RichText::new("⚙️ Distilling Latent Topological Manifold...").color(Color32::YELLOW));
                    ui.add(ProgressBar::new(*progress).animate(true).show_percentage());
                }
                ForgeStatus::Packing => {
                    ui.label(RichText::new("📦 Packing 64-Byte Aligned Solid-State Memory Map...").color(Color32::LIGHT_BLUE));
                    ui.add(ProgressBar::new(1.0).animate(true));
                }
                ForgeStatus::Complete(path) => {
                    ui.colored_label(Color32::GREEN, format!("✅ Deployed to {:?}", path));
                    if ui.button("Forge Another Model").clicked() {
                        self.current_status = ForgeStatus::Idle;
                    }
                }
                ForgeStatus::Error(err) => {
                    ui.colored_label(Color32::RED, format!("❌ Failed: {}", err));
                    if ui.button("Retry").clicked() {
                        self.current_status = ForgeStatus::Idle;
                    }
                }
            }
        });

        ui.add_space(8.0);

        // 4. Live Log Scroll Area
        ui.label(RichText::new("Forge Execution Logs:").strong());
        ScrollArea::vertical().stick_to_bottom(true).max_height(160.0).show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), 140.0],
                egui::TextEdit::multiline(&mut self.log_buffer)
                    .font(TextStyle::Monospace)
                    .interactive(false),
            );
        });

        if self.current_status != ForgeStatus::Idle {
            ctx.request_repaint(); // Keep responsive 60Hz telemetry
        }
    }
}
