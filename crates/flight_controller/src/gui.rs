// crates/flight_controller/src/gui.rs
use crate::config::FlightConfig;
use crate::queue::QueueManager;
use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Debug, Clone)]
pub enum FlightEvent {
    CycleStarted(usize, usize),
    PhaseChanged(&'static str),
    Log(String),
    HardwareUpdated(u32, String),
    TaskUpdated(Vec<String>),
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum FlightCommand {
    Start,
    Stop,
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
    pub tx_cmd: Sender<FlightCommand>,
    pub rx_event: Receiver<FlightEvent>,
}

impl FlightControllerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: FlightConfig) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgb(18, 20, 26);
        visuals.panel_fill = egui::Color32::from_rgb(14, 16, 22);
        cc.egui_ctx.set_visuals(visuals);

        let repo_root = config
            .repo_root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

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
            tx_cmd,
            rx_event,
        };

        // Spawn background worker thread
        let worker_root = repo_root.clone();
        let worker_config = config;
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

                while let Ok(cmd) = rx_cmd.recv() {
                    match cmd {
                        FlightCommand::Start => {
                            let _ = tx_event.send(FlightEvent::Log(
                                "[AFC Engine] Starting flight cycle...".to_string(),
                            ));
                            let engine =
                                match crate::engine::FlightEngine::new(worker_config.clone()) {
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

                            // Refresh tasks
                            if let Ok(tasks) = QueueManager::find_pending_tasks(&queue_path).await {
                                let _ = tx_event.send(FlightEvent::TaskUpdated(tasks));
                            }
                        }
                        FlightCommand::Stop => {
                            let _ = tx_event
                                .send(FlightEvent::Log("[AFC Engine] Stopped.".to_string()));
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
                    if self.logs.len() >= 200 {
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
    }
}

impl eframe::App for FlightControllerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_events();
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));

        // Top Control Bar
        egui::Panel::top("flight_control_bar").show_inside(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("✈ Aaroneous Flight Controller");
                ui.label(egui::RichText::new("v0.3.2").color(egui::Color32::GRAY));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status_color = if self.running {
                        egui::Color32::from_rgb(0, 220, 130)
                    } else if self.active_phase == "Finished" {
                        egui::Color32::from_rgb(80, 180, 255)
                    } else {
                        egui::Color32::from_rgb(255, 180, 0)
                    };

                    ui.label(
                        egui::RichText::new(format!("● {}", self.active_phase))
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

            // Mission Control Buttons
            ui.horizontal(|ui| {
                if !self.running {
                    if ui
                        .button(
                            egui::RichText::new("▶ Launch Autonomous Flight")
                                .color(egui::Color32::GREEN)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.running = true;
                        self.active_phase = "Launching...";
                        let _ = self.tx_cmd.send(FlightCommand::Start);
                    }
                } else {
                    if ui
                        .button(
                            egui::RichText::new("⏹ Abort Flight")
                                .color(egui::Color32::RED)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.running = false;
                        self.active_phase = "Aborted";
                        let _ = self.tx_cmd.send(FlightCommand::Stop);
                    }
                }

                ui.separator();

                ui.checkbox(&mut self.config.clippy_gate, "Clippy Gate");
                ui.checkbox(&mut self.config.run_tests, "Test Gate");
                ui.checkbox(&mut self.config.run_security, "Security Gate");
                ui.checkbox(&mut self.config.enforce_format, "Fmt Gate");
                ui.checkbox(&mut self.config.auto_rollback, "Auto Rollback");
                ui.checkbox(&mut self.config.build_artifacts, "Release Package");
            });
            ui.add_space(4.0);
        });

        // Left Panel: Queue & Gates
        egui::Panel::left("queue_panel")
            .resizable(true)
            .default_size(320.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📋 Active Audit Queue").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{} pending", self.pending_tasks.len()));
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.pending_tasks.is_empty() {
                        ui.label(
                            egui::RichText::new("✨ Queue is empty! No pending defects.")
                                .color(egui::Color32::GRAY),
                        );
                    } else {
                        for task in &self.pending_tasks {
                            ui.horizontal(|ui| {
                                ui.label("⏳");
                                ui.label(egui::RichText::new(task).small());
                            });
                        }
                    }
                });
            });

        // Central Area: Live Logs & Telemetry
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📡 Live Mission Telemetry").strong());
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
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Aaroneous Flight Controller",
        native_options,
        Box::new(|cc| Ok(Box::new(FlightControllerApp::new(cc, config)))),
    )
}
