//! Aaroneous Uninstaller GUI
//! Professional graphical uninstaller desktop application for Aaroneous Sovereign Hypervisor & Studio.

#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UninstallPage {
    Confirm,
    Uninstalling,
    Completed,
    Error,
}

#[derive(Clone)]
struct UninstallProgress {
    percentage: f32,
    status_text: String,
    logs: Vec<String>,
    is_done: bool,
    error_message: Option<String>,
}

pub struct AaroneousUninstallApp {
    current_page: UninstallPage,
    purge_user_data: bool,
    install_dir: PathBuf,
    progress: Arc<Mutex<UninstallProgress>>,
}

impl AaroneousUninstallApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let install_dir = if let Ok(current_exe) = std::env::current_exe() {
            if let Some(parent) = current_exe.parent() {
                parent.to_path_buf()
            } else {
                dirs::data_local_dir()
                    .map(|p| p.join("Programs").join("Aaroneous"))
                    .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\Aaroneous"))
            }
        } else {
            dirs::data_local_dir()
                .map(|p| p.join("Programs").join("Aaroneous"))
                .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\Aaroneous"))
        };

        Self {
            current_page: UninstallPage::Confirm,
            purge_user_data: false,
            install_dir,
            progress: Arc::new(Mutex::new(UninstallProgress {
                percentage: 0.0,
                status_text: "Ready to uninstall.".into(),
                logs: Vec::new(),
                is_done: false,
                error_message: None,
            })),
        }
    }

    fn start_uninstallation(&mut self) {
        self.current_page = UninstallPage::Uninstalling;
        let progress_arc = self.progress.clone();
        let target_dir = self.install_dir.clone();
        let purge_data = self.purge_user_data;

        thread::spawn(move || {
            let log = |text: &str, pct: f32| {
                if let Ok(mut p) = progress_arc.lock() {
                    p.percentage = pct;
                    p.status_text = text.to_string();
                    p.logs.push(text.to_string());
                }
                thread::sleep(std::time::Duration::from_millis(70));
            };

            // Step 1: Remove Shortcuts
            log("Removing Desktop and Start Menu shortcuts...", 0.20);
            let desktop_path = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from(r"C:\Users\Public\Desktop"));
            let start_menu_path = dirs::data_dir()
                .map(|p| p.join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
                .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"));

            let _ = std::fs::remove_file(desktop_path.join("Aaroneous.lnk"));
            let _ = std::fs::remove_file(desktop_path.join("Aaroneous Sovereign HUD.lnk"));
            let _ = std::fs::remove_file(start_menu_path.join("Aaroneous.lnk"));
            let _ = std::fs::remove_dir_all(start_menu_path.join("Aaroneous"));

            // Step 2: Remove from User PATH
            log("Removing Aaroneous from Windows User PATH...", 0.45);
            let bin_dir_str = target_dir.join("bin").to_string_lossy().to_string();
            let ps_path_script = format!(
                r#"$uPath = [Environment]::GetEnvironmentVariable('Path', [EnvironmentVariableTarget]::User)
if ($uPath -like '*{bin_dir_str}*') {{
    $clean = ($uPath -split ';' | Where-Object {{ $_ -ne '{bin_dir_str}' -and $_ -ne '' }}) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $clean, [EnvironmentVariableTarget]::User)
}}
"#
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_path_script])
                .output();

            // Step 3: Remove Registry Entry
            log("Removing Windows Add/Remove Programs registry entry...", 0.70);
            let ps_reg_script = r#"$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Aaroneous'
if (Test-Path $key) { Remove-Item -Recurse -Force $key }
"#;
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", ps_reg_script])
                .output();

            // Step 4: Remove Files
            log("Deleting installed files and directories...", 0.88);
            let _ = std::fs::remove_dir_all(target_dir.join("bin"));
            let _ = std::fs::remove_dir_all(target_dir.join("config"));
            let _ = std::fs::remove_dir_all(target_dir.join("shaders"));
            let _ = std::fs::remove_dir_all(target_dir.join("deploy"));

            if purge_data {
                log("Purging model weights and persistent database...", 0.95);
                let _ = std::fs::remove_dir_all(target_dir.join("data"));
                let _ = std::fs::remove_dir_all(target_dir.join("models"));
                let _ = std::fs::remove_file(target_dir.join("hive.db"));
            }

            log("Aaroneous successfully uninstalled.", 1.0);
            if let Ok(mut p) = progress_arc.lock() {
                p.is_done = true;
            }
        });
    }
}

impl eframe::App for AaroneousUninstallApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let is_uninstalling = self.current_page == UninstallPage::Uninstalling;
        if is_uninstalling {
            ui.ctx().request_repaint();
            if let Ok(p) = self.progress.lock() {
                if p.is_done {
                    self.current_page = UninstallPage::Completed;
                } else if p.error_message.is_some() {
                    self.current_page = UninstallPage::Error;
                }
            }
        }

        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(230, 240, 255));
        visuals.panel_fill = egui::Color32::from_rgb(15, 18, 24);
        visuals.window_fill = egui::Color32::from_rgb(15, 18, 24);
        ui.ctx().set_visuals(visuals);

        ui.add_space(12.0);

        // Header Banner
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🗑️").size(26.0));
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Aaroneous Uninstaller")
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 120, 120)),
                );
                ui.label(
                    egui::RichText::new("Remove Aaroneous Sovereign Hypervisor & Studio")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(160, 175, 195)),
                );
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(12.0);

        match self.current_page {
            UninstallPage::Confirm => {
                ui.heading("Uninstall Confirmation");
                ui.add_space(8.0);
                ui.label("Are you sure you want to completely remove Aaroneous and all its components from this computer?");
                ui.add_space(6.0);
                ui.label(format!("Installation Directory: {}", self.install_dir.display()));

                ui.add_space(14.0);
                ui.group(|ui| {
                    ui.checkbox(&mut self.purge_user_data, "Also remove user data, models, and databases ('data/', 'hive.db')");
                });

                ui.add_space(20.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    if ui.button(egui::RichText::new("Uninstall 🗑️").size(14.0).strong().color(egui::Color32::from_rgb(255, 100, 100))).clicked() {
                        self.start_uninstallation();
                    }
                    if ui.button("Cancel").clicked() {
                        std::process::exit(0);
                    }
                });
            }

            UninstallPage::Uninstalling => {
                ui.heading("Uninstalling Aaroneous...");
                ui.add_space(10.0);

                let (pct, status, logs) = if let Ok(p) = self.progress.lock() {
                    (p.percentage, p.status_text.clone(), p.logs.clone())
                } else {
                    (0.5, "Processing...".into(), vec![])
                };

                ui.add(egui::ProgressBar::new(pct).show_percentage().animate(true));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&status).color(egui::Color32::from_rgb(255, 180, 180)));

                ui.add_space(10.0);
                egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    for line in &logs {
                        ui.label(egui::RichText::new(format!("> {}", line)).size(11.0).color(egui::Color32::from_rgb(140, 155, 175)));
                    }
                });
            }

            UninstallPage::Completed => {
                ui.heading("✅ Uninstallation Complete");
                ui.add_space(10.0);
                ui.label("Aaroneous Sovereign Hypervisor was successfully removed from your computer.");
                ui.add_space(16.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    if ui.button(egui::RichText::new("Close").size(14.0).strong()).clicked() {
                        std::process::exit(0);
                    }
                });
            }

            UninstallPage::Error => {
                ui.heading("❌ Uninstallation Error");
                ui.add_space(10.0);
                let err = if let Ok(p) = self.progress.lock() {
                    p.error_message.clone().unwrap_or_else(|| "Unknown error occurred.".into())
                } else {
                    "Lock error".into()
                };
                ui.label(egui::RichText::new(err).color(egui::Color32::from_rgb(255, 100, 100)));
                ui.add_space(16.0);
                if ui.button("Close").clicked() {
                    std::process::exit(1);
                }
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([580.0, 380.0])
            .with_resizable(false)
            .with_title("Aaroneous Uninstaller"),
        ..Default::default()
    };

    eframe::run_native(
        "Aaroneous Uninstaller",
        native_options,
        Box::new(|cc| Ok(Box::new(AaroneousUninstallApp::new(cc)))),
    )
}
