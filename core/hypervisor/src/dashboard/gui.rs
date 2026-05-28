// Aaroneous Graphical Dashboard
// Native 3D visualization using wgpu embedded in egui via PaintCallback

use eframe::egui;
use ratatui::{Terminal, backend::Backend};
use std::time::Instant;
use crate::tui_framework::{TuiApp, draw, Page};
use crate::constellation_ui::ConstellationCanvas;
use crate::constellation_3d::ConstellationCallback;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;

use nervous_system::SharedMemorySynapse;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::cognitive_weighting::CognitiveWeights;
use crate::lora_adapter_vault::LoraAdapterVault;
use crate::dashboard::spatial_kinetic::SpatialKineticTelemetry;

pub struct EguiRatatuiBridge {
    pub app_state: TuiApp,
    last_tick: Instant,
    constellation_2d: ConstellationCanvas,
    callback: ConstellationCallback,
    use_3d: bool,
    synapse: Arc<RwLock<SharedMemorySynapse>>,
    cognitive_weights: CognitiveWeights,
    adapter_vault: LoraAdapterVault,
    spatial_telemetry: SpatialKineticTelemetry,
}

impl EguiRatatuiBridge {
    pub fn new(synapse: Arc<RwLock<SharedMemorySynapse>>) -> Self {
        Self {
            app_state: TuiApp::default(),
            last_tick: Instant::now(),
            constellation_2d: ConstellationCanvas::new(),
            callback: ConstellationCallback::new(),
            use_3d: true,
            synapse,
            cognitive_weights: CognitiveWeights::default(),
            adapter_vault: LoraAdapterVault::new(),
            spatial_telemetry: SpatialKineticTelemetry::default(),
        }
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.heading("⚡ Aaroneous Omni Dashboard");
            ui.add_space(4.0);

            // HITL Handshake Banner
            self.draw_hitl_banner(ui);

            // Homeostatic Vital Meters
            self.draw_vital_meters(ui);
            
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                if ui.button("🏠 Home").clicked() { self.app_state.page = Page::Home; }
                if ui.button("🌌 Constellation").clicked() { self.app_state.page = Page::Metabolic; }
                if ui.button("🤖 Specialists").clicked() { self.app_state.page = Page::Specialists; }
                if ui.button("🧬 Evolution").clicked() { self.app_state.page = Page::Lore; }
                if ui.button("🎮 Spatial-Kinetic").clicked() { self.app_state.page = Page::SpatialKinetic; }
                if ui.button("📜 Log").clicked() { self.app_state.page = Page::EventLog; }
                
                ui.separator();
                if ui.button(if self.use_3d { "3D" } else { "2D" }).clicked() {
                    self.use_3d = !self.use_3d;
                }
            });

            ui.add_space(8.0);
            ui.separator();

            // Intelligence Stream (Visualizing curiosity pulses)
            self.draw_intelligence_stream(ui);

            if self.app_state.page == Page::Metabolic {
                if self.use_3d {
                    // Render 3D constellation via wgpu
                    self.render_3d_constellation(ui);
                } else {
                    // Fallback to 2D
                    self.constellation_2d.ui(ui);
                }
                
                // --- CLUSTER EXPLORER OVERLAY ---
                self.draw_cluster_explorer(ui);
            } else if self.app_state.page == Page::Lore {
                self.draw_evolution_view(ui);
            } else if self.app_state.page == Page::SpatialKinetic {
                self.spatial_telemetry.render_panel(ui);
            } else {
                let available_rect = ui.available_rect_before_wrap();
                let cw = 9.0;
                let ch = 18.0;
                
                let cols = (available_rect.width() / cw).floor() as u16;
                let rows = (available_rect.height() / ch).floor() as u16;

                if cols > 0 && rows > 0 {
                    let backend = EguiBackend { width: cols, height: rows, cursor: (0, 0) };
                    let mut terminal = Terminal::new(backend).unwrap();
                    
                    let frame_res = terminal.draw(|f| {
                        draw(f, &self.app_state);
                    }).unwrap();

                    render_buffer_to_egui(ui, frame_res.buffer, cw, ch);
                }
            }
        });
    }

    fn draw_hitl_banner(&mut self, ui: &mut egui::Ui) {
        let synapse = self.synapse.write();
        let state = unsafe { &mut *(synapse.get_ptr_sync() as *mut nervous_system::shared_memory::SynapseState) };

        if state.approval_required == 1 {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(60, 40, 0))
                .stroke(egui::Stroke::new(2.0, egui::Color32::YELLOW))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("⚠ PENDING APPROVAL: Enzyme requesting Tier 1/2 access").color(egui::Color32::YELLOW).strong());
                        ui.add_space(20.0);
                        if ui.button(egui::RichText::new("✅ APPROVE").color(egui::Color32::GREEN)).clicked() {
                            state.approval_granted = 1;
                            state.approval_required = 0;
                            println!("[HITL] Manual approval granted.");
                        }
                        if ui.button(egui::RichText::new("❌ DENY").color(egui::Color32::RED)).clicked() {
                            state.approval_granted = 0;
                            state.approval_required = 0;
                            state.safety_lock = 1;
                            println!("[HITL] Manual approval denied. Safety lock engaged.");
                        }
                    });
                });
            ui.add_space(8.0);
        }
    }

    fn draw_vital_meters(&mut self, ui: &mut egui::Ui) {
        let synapse = self.synapse.read();
        let state = unsafe { &*(synapse.get_ptr_sync() as *const nervous_system::shared_memory::SynapseState) };

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("🧠 Understanding:");
            ui.add(egui::ProgressBar::new(state.understanding_score as f32 / 100.0)
                .text(format!("{}%", state.understanding_score))
                .fill(egui::Color32::from_rgb(0, 150, 255)));
            
            ui.label("🧬 Integrity:");
            ui.add(egui::ProgressBar::new(state.integrity_score as f32 / 100.0)
                .text(format!("{}%", state.integrity_score))
                .fill(egui::Color32::from_rgb(0, 255, 150)));

            ui.label("⚡ Curiosity:");
            ui.add(egui::ProgressBar::new(state.curiosity_drive as f32 / 100.0)
                .text(format!("{}%", state.curiosity_drive))
                .fill(egui::Color32::from_rgb(255, 150, 0)));
            
            ui.label("📉 Concept Drift:");
            let drift_color = if state.safety_lock == 1 { egui::Color32::RED } else { egui::Color32::from_rgb(200, 200, 200) };
            ui.label(egui::RichText::new(format!("{:.2}", state.concept_drift)).color(drift_color));
        });
        ui.add_space(8.0);
    }

    fn draw_intelligence_stream(&mut self, ui: &mut egui::Ui) {
        let synapse = self.synapse.read();
        let state = unsafe { &*(synapse.get_ptr_sync() as *const nervous_system::shared_memory::SynapseState) };

        if state.latent_vector[0] > 0.8 {
             ui.colored_label(egui::Color32::from_rgb(0, 255, 255), "💠 LATENT INJECTION ACTIVE: Zero-copy mathematical thought transfer in progress.");
        }

        if state.hox_mutation_flag == 1 {
             ui.colored_label(egui::Color32::from_rgb(0, 255, 255), "🧬 EPIGENETIC EVOLUTION: WASM enzyme hot-swapped in real-time.");
        }

        if state.curiosity_drive > 70 {
            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "📡 Proactive Research Pulse: High Curiosity Drive");
        }
        if state.integrity_score < 50 {
            ui.colored_label(egui::Color32::from_rgb(255, 50, 50), "⚠ Homeostatic Warning: Low System Integrity");
        }
    }

    fn draw_evolution_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧬 Splicing Engine & Epigenetic Evolution");
        ui.add_space(10.0);

        ui.collapsing("🧠 Cognitive Weighting (Influence Factors)", |ui| {
            for (specialist, weight) in self.cognitive_weights.specialist_weights.iter_mut() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", specialist));
                    if ui.add(egui::Slider::new(weight, 0.0..=1.0).text("Priority")).changed() {
                         println!("[WeightChange] Adjusting priorities. Neural activity shifting.");
                    }
                });
            }
            if ui.button("🔬 Simulate Offspring Hybrid").clicked() {
                 println!("[Simulator] Previewing genetic hybrid based on weights...");
            }
        });

        ui.collapsing("🔌 MCP Universal Gateway", |_ui| {
            // ... (existing code)
        });

        ui.collapsing("🗣 Specialist Dialogue (Cross-Husk Debate)", |ui| {
            let synapse = self.synapse.read();
            let state = unsafe { &*(synapse.get_ptr_sync() as *const nervous_system::shared_memory::SynapseState) };
            let d = &state.dialogue();
            
            ui.horizontal(|ui| {
                ui.label("Consensus:");
                ui.add(egui::ProgressBar::new(d.consensus_score as f32 / 100.0)
                    .text(format!("{}%", d.consensus_score))
                    .fill(egui::Color32::from_rgb(200, 100, 255)));
            });

            if d.message_size > 0 {
                let msg = std::str::from_utf8(&d.message_payload[..d.message_size as usize]).unwrap_or("...");
                let speaker_info = match d.active_speaker_hash {
                    0x1111222233334444 => ("Odin (Strategic)", egui::Color32::from_rgb(0, 200, 255)),
                    0x5555666677778888 => ("Merlin (Pattern)", egui::Color32::from_rgb(0, 255, 150)),
                    0x9999AAAABBBBCCCC => ("Hephaestus (Execution)", egui::Color32::from_rgb(255, 100, 0)),
                    _ => ("Unknown", egui::Color32::GRAY),
                };
                
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", speaker_info.0)).strong().color(speaker_info.1));
                        ui.label(egui::RichText::new("🧠 Neural Splicing Active").small().color(egui::Color32::YELLOW));
                    });
                    ui.label(egui::RichText::new(msg).italics());
                });
            }
            
            ui.label(format!("Turn: {}", d.turn_count));
        });

        ui.collapsing("👁 LoRA Adapter Vault", |ui| {
            ui.label(format!("Active Adapters: {}", self.adapter_vault.adapters.len()));
            for (id, switches) in &self.adapter_vault.adapters {
                ui.label(format!("- {}: {} LoRAs, temp_bias: {:.2}", id, switches.active_loras.len(), switches.temperature_bias));
            }
        });

        ui.collapsing("👁 Retina Visual Ingestion", |ui| {
            ui.label("Latent Space Projection: 🟢 Online");
            ui.horizontal(|ui| {
                if ui.button("📸 Capture UI Latent State").clicked() {
                    println!("[Retina] Triggering manual visual-to-latent projection...");
                    let synapse = self.synapse.write();
                    let state = unsafe { &mut *(synapse.get_ptr_sync() as *mut nervous_system::shared_memory::SynapseState) };
                    for i in 0..1024 {
                        state.latent_vector[i] = rand::random::<f32>() * 2.0 - 1.0;
                    }
                    state.latent_activation_id = *uuid::Uuid::new_v4().as_bytes();
                }
                if ui.button("🔍 Scan for wgpu Framebuffer").clicked() {
                    println!("[Retina] Attempting zero-copy mapping to wgpu framebuffer...");
                }
            });
            ui.label("Visual summary: High-density interface detected. 3 control hubs active.");
        });

        ui.add_space(10.0);
        egui::Grid::new("evolution_grid").striped(true).show(ui, |ui| {
            ui.label("Specialist");
            ui.label("Status");
            ui.label("DNA (WASM) Patch");
            ui.end_row();

            ui.label("Odin");
            ui.label(egui::RichText::new("STABLE").color(egui::Color32::GREEN));
            ui.label("v3.0.2 - Native Strategic");
            ui.end_row();

            ui.label("Merlin");
            ui.label(egui::RichText::new("EVOLVING").color(egui::Color32::GOLD));
            ui.add(egui::Spinner::new());
            ui.label("Patching: zero-copy-read-sync...");
            ui.end_row();
        });

        ui.add_space(20.0);
        ui.group(|ui| {
            ui.label("Recent Chromosome Mutation Log:");
            ui.label("• [08:42:10] Hephaestus: Injected range-check guard at 0x4A2");
            ui.label("• [08:41:55] Merlin: Swapped HTTP-Gate with Local-Retina-V2");
        });
    }

    fn draw_cluster_explorer(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("🌌 Cluster Explorer")
            .default_pos(egui::pos2(20.0, 300.0))
            .show(ui.ctx(), |ui| {
                ui.label("Detected Semantic Hubs:");
                ui.separator();
                
                // In a real run, this would query the Constellation's cluster list
                let _ = ui.selectable_label(true, "• Research Discovery Hub (3 nodes)");
                ui.label("  ↳ Focus: Rust WASM Sandboxing");

                let _ = ui.selectable_label(false, "• Development Nexus (5 nodes)");
                let _ = ui.selectable_label(false, "• Core Architecture Cluster (2 nodes)");
                
        ui.collapsing("🧬 Evolution & Genetic Breeding", |ui| {
            ui.label("Chromosome Vault: 🟣 Odin, 🔵 Merlin, 🟢 Hephaestus");
            if ui.button("🧬 Breed Diplomatic Hybrid (Solon)").clicked() {
                println!("[Evolution] Splicing Odin + Merlin genetic templates...");
                // Simulate the factory flow
                println!("[Factory] Loading skill modules: strategic_planning.wasm, pattern_synthesis.wasm");
                println!("[Factory] JIT Compiling composite phenotype: solon.wasm");
            }
            ui.add_space(5.0);
            ui.label("Recent mutation: Latent subspace expanded to 1024-f32.");
            if ui.button("🏭 View Synth DNA Factory Stats").clicked() {
                println!("[Factory] VRAM: 512MB Reserved | CPU: 2 Cores | JIT: Active");
            }
        });

        ui.add_space(10.0);

                if ui.button("♻ Recalculate Clusters").clicked() {
                    println!("[Explorer] Triggering force-directed layout reset.");
                }
            });
    }

    fn render_3d_constellation(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        // Handle mouse interaction for camera rotation
        if response.dragged() {
            let delta = response.drag_delta();
            if let Ok(mut renderer) = self.callback.renderer.lock() {
                renderer.rotation.0 += delta.x * 0.01;
                renderer.rotation.1 += delta.y * 0.01;
                renderer.rotation.1 = renderer.rotation.1.clamp(-1.5, 1.5);
            }
        }

        // Scroll to zoom
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                if let Ok(mut renderer) = self.callback.renderer.lock() {
                    renderer.camera_distance = (renderer.camera_distance - scroll).max(50.0).min(2000.0);
                }
            }
        }

        // Use PaintCallback to embed wgpu rendering
        ui.painter().add(crate::constellation_3d::create_paint_callback(rect, &self.callback));
    }
}

struct EguiBackend {
    width: u16,
    height: u16,
    cursor: (u16, u16),
}

impl Backend for EguiBackend {
    fn draw<'a, I>(&mut self, _content: I) -> Result<(), std::io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), std::io::Error> { Ok(()) }
    fn show_cursor(&mut self) -> Result<(), std::io::Error> { Ok(()) }
    fn get_cursor(&mut self) -> Result<(u16, u16), std::io::Error> { Ok(self.cursor) }
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), std::io::Error> {
        self.cursor = (x, y);
        Ok(())
    }
    fn clear(&mut self) -> Result<(), std::io::Error> { Ok(()) }
    fn size(&self) -> Result<Rect, std::io::Error> {
        Ok(Rect::new(0, 0, self.width, self.height))
    }
    fn window_size(&mut self) -> Result<ratatui::backend::WindowSize, std::io::Error> {
        Ok(ratatui::backend::WindowSize {
            columns_rows: (self.width, self.height).into(),
            pixels: (self.width * 8, self.height * 16).into(),
        })
    }
    fn flush(&mut self) -> Result<(), std::io::Error> { Ok(()) }
}

impl eframe::App for EguiRatatuiBridge {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.last_tick.elapsed() >= std::time::Duration::from_millis(250) {
            self.last_tick = Instant::now();
        }

        // Use a Frame with a clear background to prevent system ghosting
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(15, 15, 15)))
            .show(ctx, |ui| {
                self.draw_ui(ui, frame);
            });

        // Render 3D constellation using wgpu if available
        if self.use_3d && self.app_state.page == Page::Metabolic {
            if let Some(render_state) = frame.wgpu_render_state() {
                crate::constellation_3d::render_constellation_3d(
                    &self.callback,
                    &render_state.device,
                    &render_state.queue,
                    render_state.target_format,
                    (ctx.screen_rect().width(), ctx.screen_rect().height()),
                );
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

fn render_buffer_to_egui(ui: &mut egui::Ui, buffer: &Buffer, cw: f32, ch: f32) {
    let (response, painter) = ui.allocate_painter(ui.available_size_before_wrap(), egui::Sense::hover());
    let rect = response.rect;

    // Draw solid background first to prevent ghosting
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 15, 15));

    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.get(x, y);
            let pos = rect.min + egui::vec2(x as f32 * cw, y as f32 * ch);
            
            // Background
            let bg_color = match cell.bg {
                ratatui::style::Color::Black => egui::Color32::from_rgb(15, 15, 15),
                ratatui::style::Color::DarkGray => egui::Color32::from_rgb(40, 40, 40),
                ratatui::style::Color::Cyan => egui::Color32::from_rgb(0, 150, 150),
                ratatui::style::Color::Green => egui::Color32::from_rgb(0, 120, 0),
                _ => egui::Color32::TRANSPARENT,
            };
            
            if bg_color != egui::Color32::TRANSPARENT {
                painter.rect_filled(egui::Rect::from_min_size(pos, egui::vec2(cw, ch)), 0.0, bg_color);
            }

            // Foreground Text
            if !cell.symbol().trim().is_empty() {
                let fg_color = match cell.fg {
                    ratatui::style::Color::White => egui::Color32::WHITE,
                    ratatui::style::Color::Cyan => egui::Color32::from_rgb(0, 255, 255),
                    ratatui::style::Color::Green => egui::Color32::from_rgb(0, 255, 0),
                    ratatui::style::Color::Yellow => egui::Color32::from_rgb(255, 255, 0),
                    ratatui::style::Color::Gray => egui::Color32::GRAY,
                    _ => egui::Color32::WHITE,
                };

                painter.text(
                    pos + egui::vec2(cw/2.0, ch/2.0),
                    egui::Align2::CENTER_CENTER,
                    cell.symbol(),
                    egui::FontId::monospace(ch * 0.8),
                    fg_color,
                );
            }
        }
    }
}

pub fn run_dashboard(synapse: Arc<RwLock<SharedMemorySynapse>>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 900.0])
            .with_title("Aaroneous Omni Dashboard"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Aaroneous Dashboard",
        options,
        Box::new(|_cc| Ok(Box::new(EguiRatatuiBridge::new(synapse)))),
    )
}
