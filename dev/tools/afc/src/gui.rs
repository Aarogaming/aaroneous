// dev/tools/afc/src/gui.rs
use crate::config::FlightConfig;
use crate::model_probe::{ModelEndpointStatus, ModelProbe};
use crate::queue::QueueManager;
use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum FlightEvent {
    CycleStarted(usize, usize),
    PhaseChanged(&'static str),
    Log(String),
    HardwareUpdated(u32, String),
    TaskUpdated(Vec<String>),
    ModelProbeUpdated(ModelEndpointStatus),
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum FlightCommand {
    Start(FlightConfig),
    Stop,
    ProbeModel,
}

pub struct FlightControllerApp {
    pub config: FlightConfig,
    pub repo_root: PathBuf,
    pub running: bool,
    pub current_cycle: usize,
    pub total_cycles: usize,
    pub active_phase: &'static str,
    pub logs: VecDeque<String>,
    pub pending_tasks: Vec<String>,
    pub gpu_temp: u32,
    pub vram_str: String,
    pub model_status: ModelEndpointStatus,
    pub last_probe_time: Instant,
    pub tx_cmd: Sender<FlightCommand>,
    pub rx_event: Receiver<FlightEvent>,
}

impl FlightControllerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: FlightConfig) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgb(18, 20, 26);
        visuals.panel_fill = egui::Color32::from_rgb(14, 16, 22);
        cc.egui_ctx.set_visuals(visuals);

        let repo_root = config.resolve_repo_root();

        let (tx_event, rx_event) = channel::<FlightEvent>();
        let (tx_cmd, rx_cmd) = channel::<FlightCommand>();

        let app = Self {
            total_cycles: config.auto_cycles,
            config: config.clone(),
            repo_root: repo_root.clone(),
            running: false,
            current_cycle: 0,
            active_phase: "Idle",
            logs: VecDeque::new(),
            pending_tasks: Vec::new(),
            gpu_temp: 0,
            vram_str: "0 MB".to_string(),
            model_status: ModelEndpointStatus::Unconfigured,
            last_probe_time: Instant::now(),
            tx_cmd,
            rx_event,
        };

        // Spawn background worker thread
        let worker_root = repo_root.clone();
        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx_event.send(FlightEvent::Error(format!("Runtime error: {e}")));
                    return;
                }
            };

            rt.block_on(async move {
                let queue_path = worker_root
                    .join("dev")
                    .join("docs")
                    .join("audits")
                    .join("active")
                    .join("ACTIVE_AUDIT_QUEUE.md");

                // Initial task queue poll
                if let Ok(tasks) = QueueManager::find_pending_tasks(&queue_path).await {
                    let _ = tx_event.send(FlightEvent::TaskUpdated(tasks));
                }

                // Initial model probe
                let status = ModelProbe::check_endpoint(&worker_root).await;
                let _ = tx_event.send(FlightEvent::ModelProbeUpdated(status));

                while let Ok(cmd) = rx_cmd.recv() {
                    match cmd {
                        FlightCommand::Start(active_cfg) => {
                            let _ = tx_event.send(FlightEvent::Log(
                                "[AFC Engine] Starting autonomous flight cycle...".to_string(),
                            ));
                            let engine = match crate::engine::FlightEngine::new(active_cfg) {
                                Ok(eng) => eng,
                                Err(e) => {
                                    let _ = tx_event.send(FlightEvent::Error(format!("{e}")));
                                    continue;
                                }
                            };

                            let _ = tx_event.send(FlightEvent::PhaseChanged("Active Flight"));
                            if let Err(e) = engine.run().await {
                                let _ =
                                    tx_event.send(FlightEvent::Error(format!("Flight error: {e}")));
                            } else {
                                let _ = tx_event.send(FlightEvent::Completed);
                            }

                            if let Ok(tasks) = QueueManager::find_pending_tasks(&queue_path).await {
                                let _ = tx_event.send(FlightEvent::TaskUpdated(tasks));
                            }
                        }
                        FlightCommand::Stop => {
                            let _ = tx_event
                                .send(FlightEvent::Log("[AFC Engine] Stopped.".to_string()));
                        }
                        FlightCommand::ProbeModel => {
                            let status = ModelProbe::check_endpoint(&worker_root).await;
                            let _ = tx_event.send(FlightEvent::ModelProbeUpdated(status));
                        }
                    }
                }
            });
        });

        app
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.rx_event.try_recv() {
            match event {
                FlightEvent::CycleStarted(c, total) => {
                    self.current_cycle = c;
                    self.total_cycles = total;
                    self.running = true;
                }
                FlightEvent::PhaseChanged(phase) => {
                    self.active_phase = phase;
                }
                FlightEvent::Log(msg) => {
                    if self.logs.len() >= 250 {
                        self.logs.pop_front();
                    }
                    self.logs.push_back(msg);
                }
                FlightEvent::HardwareUpdated(temp, vram) => {
                    self.gpu_temp = temp;
                    self.vram_str = vram;
                }
                FlightEvent::TaskUpdated(tasks) => {
                    self.pending_tasks = tasks;
                }
                FlightEvent::ModelProbeUpdated(status) => {
                    self.model_status = status;
                }
                FlightEvent::Completed => {
                    self.running = false;
                    self.active_phase = "Finished";
                    self.logs.push_back(
                        "[AFC Engine] Autonomous Flight cycle finished successfully!".to_string(),
                    );
                }
                FlightEvent::Error(err) => {
                    self.running = false;
                    self.active_phase = "Error";
                    self.logs.push_back(format!("[ERROR] {err}"));
                }
            }
        }

        // Auto probe model endpoint every 10 seconds
        if self.last_probe_time.elapsed() > Duration::from_secs(10) {
            self.last_probe_time = Instant::now();
            let _ = self.tx_cmd.send(FlightCommand::ProbeModel);
        }
    }
}

impl eframe::App for FlightControllerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_events();
        ui.ctx().request_repaint_after(Duration::from_millis(100));

        // ── Top Bar: Mission Control & Provider Recognition ───────────────────
        egui::Panel::top("flight_top_bar").show_inside(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("Aaroneous Flight Controller");
                ui.label(egui::RichText::new("v0.1.0").color(egui::Color32::GRAY));

                ui.separator();

                // Provider Recognition Badge
                match &self.model_status {
                    ModelEndpointStatus::Connected {
                        provider,
                        configured_model,
                        discovered_models,
                    } => {
                        let badge = ui.label(
                            egui::RichText::new(format!("[Online] {provider}: {configured_model}"))
                                .color(egui::Color32::from_rgb(80, 230, 130))
                                .strong(),
                        );
                        badge.on_hover_ui(|ui| {
                            ui.label(format!("Provider: {provider}"));
                            ui.label(format!("Active Model: {configured_model}"));
                            ui.separator();
                            ui.label("Discovered Endpoint Models:");
                            for m in discovered_models {
                                ui.label(format!("  - {m}"));
                            }
                        });
                    }
                    ModelEndpointStatus::Disconnected {
                        provider,
                        target_endpoint,
                        reason,
                    } => {
                        let badge = ui.label(
                            egui::RichText::new(format!("[Offline] {provider}"))
                                .color(egui::Color32::from_rgb(255, 90, 90))
                                .strong(),
                        );
                        badge.on_hover_ui(|ui| {
                            ui.label(format!("Endpoint: {target_endpoint}"));
                            ui.label(format!("Status: {reason}"));
                        });
                    }
                    ModelEndpointStatus::Unconfigured => {
                        ui.label(
                            egui::RichText::new("[Checking] AI Provider...")
                                .color(egui::Color32::GRAY),
                        );
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status_color = if self.running {
                        egui::Color32::from_rgb(0, 220, 130)
                    } else if self.active_phase == "Finished" {
                        egui::Color32::from_rgb(80, 180, 255)
                    } else {
                        egui::Color32::from_rgb(255, 180, 0)
                    };

                    ui.label(
                        egui::RichText::new(format!("[{}]", self.active_phase))
                            .color(status_color)
                            .strong(),
                    );

                    if self.gpu_temp > 0 {
                        let temp_color = if self.gpu_temp > 75 {
                            egui::Color32::RED
                        } else {
                            egui::Color32::LIGHT_GREEN
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "GPU: {}°C | {}",
                                self.gpu_temp, self.vram_str
                            ))
                            .color(temp_color),
                        );
                    }
                });
            });
            ui.add_space(4.0);

            // Primary Control Row
            ui.horizontal(|ui| {
                if !self.running {
                    if ui
                        .button(
                            egui::RichText::new("Launch Autonomous Flight")
                                .color(egui::Color32::GREEN)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.running = true;
                        self.active_phase = "Launching...";
                        let _ = self.tx_cmd.send(FlightCommand::Start(self.config.clone()));
                    }
                } else if ui
                    .button(
                        egui::RichText::new("Abort Flight")
                            .color(egui::Color32::RED)
                            .strong(),
                    )
                    .clicked()
                {
                    self.running = false;
                    self.active_phase = "Aborted";
                    let _ = self.tx_cmd.send(FlightCommand::Stop);
                }

                if ui.button("Probe Model").clicked() {
                    let _ = self.tx_cmd.send(FlightCommand::ProbeModel);
                }

                ui.separator();
                ui.label("Cycles:");
                ui.add(egui::DragValue::new(&mut self.config.auto_cycles).range(1..=50));
            });
            ui.add_space(4.0);
        });

        // ── Multiselect Phase & Audit Control Panels ──────────────────────────
        egui::Panel::top("multiselect_panel").show_inside(ui, |ui| {
            ui.add_space(2.0);
            ui.collapsing("Pipeline Phase Selection (Multiselect)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.config.phase_plan, "1. Plan (Architect)");
                    ui.checkbox(&mut self.config.phase_audit, "2. Audit (Forensics)");
                    ui.checkbox(&mut self.config.phase_fix, "3. Fix (Remediate)");
                    ui.checkbox(&mut self.config.phase_sweep, "4. Sweep (Archive)");
                    ui.checkbox(&mut self.config.phase_verify, "5. Verify (Gates)");
                    ui.checkbox(&mut self.config.phase_commit, "6. Commit (Git)");
                    ui.checkbox(&mut self.config.phase_deliver, "7. Deliver (Release)");
                });
            });

            ui.collapsing("Forensic Audit Types (Multiselect)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.config.audit_security, "Security & CVEs");
                    ui.checkbox(&mut self.config.audit_panics, "Panic & Unwrap Removal");
                    ui.checkbox(
                        &mut self.config.audit_concurrency,
                        "Concurrency & Lock Safety",
                    );
                    ui.checkbox(&mut self.config.audit_dead_code, "Dead Code & Stubs");
                    ui.checkbox(&mut self.config.audit_health, "SystemsHealthAuditor");
                    ui.checkbox(
                        &mut self.config.audit_resilience,
                        "AdvancedResilienceAuditor",
                    );
                });
            });

            ui.collapsing("CI/CD Quality Gatekeeper Toggles", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.config.clippy_gate, "Clippy Gate");
                    ui.checkbox(&mut self.config.run_tests, "Test Gate");
                    ui.checkbox(&mut self.config.run_security, "Cargo Audit Gate");
                    ui.checkbox(&mut self.config.enforce_format, "Auto Format Gate");
                    ui.checkbox(&mut self.config.auto_rollback, "Auto Rollback on Error");
                    ui.checkbox(&mut self.config.build_artifacts, "Release Package Zip");
                });
            });
            ui.add_space(2.0);
        });

        // ── Left Panel: Active Queue Checklist ────────────────────────────────
        egui::Panel::left("queue_panel")
            .resizable(true)
            .default_size(320.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Active Audit Queue").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{} pending", self.pending_tasks.len()));
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.pending_tasks.is_empty() {
                        ui.label(
                            egui::RichText::new("Queue is empty. No pending defects.")
                                .color(egui::Color32::GRAY),
                        );
                    } else {
                        for task in &self.pending_tasks {
                            ui.horizontal(|ui| {
                                ui.label("-");
                                ui.label(egui::RichText::new(task).small());
                            });
                        }
                    }
                });
            });

        // ── Central Area: Live Mission Telemetry & Output ─────────────────────
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Live Mission Telemetry").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear Logs").clicked() {
                        self.logs.clear();
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &self.logs {
                        let text_color = if log.contains("[ERROR]") || log.contains("failed") {
                            egui::Color32::from_rgb(255, 90, 90)
                        } else if log.contains("WARN") || log.contains("warning") {
                            egui::Color32::from_rgb(255, 200, 80)
                        } else if log.contains("validated")
                            || log.contains("cleanly")
                            || log.contains("passed")
                        {
                            egui::Color32::from_rgb(80, 230, 130)
                        } else {
                            egui::Color32::from_rgb(200, 210, 220)
                        };

                        ui.label(
                            egui::RichText::new(log)
                                .color(text_color)
                                .monospace()
                                .small(),
                        );
                    }
                });
        });
    }
}

pub fn launch_gui(config: FlightConfig) -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Aaroneous Flight Controller")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Aaroneous Flight Controller",
        native_options,
        Box::new(|cc| Ok(Box::new(FlightControllerApp::new(cc, config)))),
    )
}
