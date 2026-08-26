//! Aaroneous Setup GUI
//! Professional graphical installer desktop application for Aaroneous Sovereign Hypervisor & Studio.

// #![windows_subsystem = "windows"]  // temporarily disabled to debug launch

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
enum WizardPage {
    Welcome,
    Configuration,
    Installing,
    Completed,
    Error,
}

struct InstallOptions {
    install_dir: String,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    add_to_user_path: bool,
    launch_after_install: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        let default_dir = dirs::data_local_dir()
            .map(|p| p.join("Programs").join("Aaroneous"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\Aaroneous"))
            .to_string_lossy()
            .to_string();

        Self {
            install_dir: default_dir,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            add_to_user_path: true,
            launch_after_install: true,
        }
    }
}

#[derive(Clone)]
struct InstallProgress {
    percentage: f32,
    status_text: String,
    logs: Vec<String>,
    is_done: bool,
    error_message: Option<String>,
}

pub struct AaroneousSetupApp {
    current_page: WizardPage,
    options: InstallOptions,
    progress: Arc<Mutex<InstallProgress>>,
}

impl AaroneousSetupApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            current_page: WizardPage::Welcome,
            options: InstallOptions::default(),
            progress: Arc::new(Mutex::new(InstallProgress {
                percentage: 0.0,
                status_text: "Ready to install.".into(),
                logs: Vec::new(),
                is_done: false,
                error_message: None,
            })),
        }
    }

    fn start_installation(&mut self) {
        self.current_page = WizardPage::Installing;
        let progress_arc = self.progress.clone();
        let target_dir = PathBuf::from(self.options.install_dir.clone());
        let add_path = self.options.add_to_user_path;
        let create_desktop = self.options.create_desktop_shortcut;
        let create_start_menu = self.options.create_start_menu_shortcut;

        thread::spawn(move || {
            let log = |text: &str, pct: f32| {
                if let Ok(mut p) = progress_arc.lock() {
                    p.percentage = pct;
                    p.status_text = text.to_string();
                    p.logs.push(text.to_string());
                }
                thread::sleep(std::time::Duration::from_millis(60));
            };

            let discover_source_root = || -> PathBuf {
                if let Ok(current_exe) = std::env::current_exe()
                    && let Some(parent) = current_exe.parent()
                {
                    if parent.join("aaroneous.exe").exists() {
                        return parent.to_path_buf();
                    }
                    if let Some(grandparent) = parent.parent()
                        && (grandparent.join("target").exists() || grandparent.join("Cargo.toml").exists())
                    {
                        return grandparent.to_path_buf();
                    }
                }
                aaroneous_paths::WorkspacePaths::discover().root().to_path_buf()
            };

            let source_root = discover_source_root();
            log(&format!("Discovered source assets at: {}", source_root.display()), 0.05);

            // Step 1: Create Directories
            log("Creating target directories...", 0.15);
            let bin_dir = target_dir.join("bin");
            let config_dir = target_dir.join("config");
            let shaders_dir = target_dir.join("shaders");
            let mcp_dir = target_dir.join("deploy").join("mcp_clients");

            if let Err(e) = std::fs::create_dir_all(&bin_dir) {
                if let Ok(mut p) = progress_arc.lock() {
                    p.error_message = Some(format!("Failed to create bin dir: {}", e));
                }
                return;
            }
            let _ = std::fs::create_dir_all(&config_dir);
            let _ = std::fs::create_dir_all(&shaders_dir);
            let _ = std::fs::create_dir_all(&mcp_dir);

            // Step 2: Copy Executables
            log("Installing binary executables (aaroneous.exe, a_run.exe)...", 0.35);
            let possible_bins = vec![
                source_root.join("target").join("release"),
                source_root.join("bin"),
                source_root.clone(),
            ];

            let copy_if_found = |name: &str, dest: &Path| -> bool {
                for candidate in &possible_bins {
                    let src = candidate.join(name);
                    if src.exists() && std::fs::copy(&src, dest).is_ok() {
                        return true;
                    }
                }
                false
            };

            copy_if_found("aaroneous.exe", &bin_dir.join("aaroneous.exe"));
            copy_if_found("a_run.exe", &bin_dir.join("a_run.exe"));
            copy_if_found("aaroneous-uninstall.exe", &target_dir.join("uninstall.exe"));

            // Step 3: Copy Assets & Configs
            log("Deploying configurations, shaders, and MCP profiles...", 0.55);
            let copy_dir_contents = |src: &Path, dst: &Path| {
                if let Ok(entries) = std::fs::read_dir(src) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file()
                            && let Some(fname) = path.file_name()
                        {
                            let _ = std::fs::copy(&path, dst.join(fname));
                        }
                    }
                }
            };

            copy_dir_contents(&source_root.join("config"), &config_dir);
            copy_dir_contents(&source_root.join("shaders"), &shaders_dir);
            copy_dir_contents(&source_root.join("deploy").join("mcp_clients"), &mcp_dir);

            // Step 4: Windows Registry Registration
            log("Registering Aaroneous in Windows Add/Remove Programs...", 0.70);
            let install_dir_str = target_dir.to_string_lossy().to_string();
            let main_exe_str = bin_dir.join("aaroneous.exe").to_string_lossy().to_string();
            let uninstall_exe_str = target_dir.join("uninstall.exe").to_string_lossy().to_string();

            let ps_reg_script = format!(
                r#"$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Aaroneous'
if (-not (Test-Path $key)) {{ New-Item -Path $key -Force | Out-Null }}
Set-ItemProperty -Path $key -Name 'DisplayName' -Value 'Aaroneous Sovereign Hypervisor' -Force
Set-ItemProperty -Path $key -Name 'DisplayVersion' -Value '0.3.0' -Force
Set-ItemProperty -Path $key -Name 'Publisher' -Value 'Aaroneous Team' -Force
Set-ItemProperty -Path $key -Name 'InstallLocation' -Value '{install_dir_str}' -Force
Set-ItemProperty -Path $key -Name 'DisplayIcon' -Value '{main_exe_str}' -Force
Set-ItemProperty -Path $key -Name 'UninstallString' -Value '{uninstall_exe_str}' -Force
"#
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_reg_script])
                .output();

            // Step 5: Shortcuts Creation
            if create_desktop || create_start_menu {
                log("Creating desktop and Start menu shortcuts...", 0.85);
                let desktop_path = dirs::desktop_dir().unwrap_or_else(|| PathBuf::from(r"C:\Users\Public\Desktop"));
                let start_menu_path = dirs::data_dir()
                    .map(|p| p.join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
                    .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"));

                let mut ps_shortcut_script = String::new();
                ps_shortcut_script.push_str("$ws = New-Object -ComObject WScript.Shell\n");

                if create_desktop {
                    let d_link = desktop_path.join("Aaroneous.lnk").to_string_lossy().to_string();
                    ps_shortcut_script.push_str(&format!(
                        "$s = $ws.CreateShortcut('{d_link}'); $s.TargetPath = '{main_exe_str}'; $s.WorkingDirectory = '{install_dir_str}'; $s.Save()\n"
                    ));
                }

                if create_start_menu {
                    let s_link = start_menu_path.join("Aaroneous.lnk").to_string_lossy().to_string();
                    ps_shortcut_script.push_str(&format!(
                        "$s = $ws.CreateShortcut('{s_link}'); $s.TargetPath = '{main_exe_str}'; $s.WorkingDirectory = '{install_dir_str}'; $s.Save()\n"
                    ));
                }

                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &ps_shortcut_script])
                    .output();
            }

            // Step 6: User PATH Registration
            if add_path {
                log("Adding Aaroneous to Windows User PATH...", 0.95);
                let bin_str = bin_dir.to_string_lossy().to_string();
                let ps_path_script = format!(
                    r#"$uPath = [Environment]::GetEnvironmentVariable('Path', [EnvironmentVariableTarget]::User)
if ($uPath -notlike '*{bin_str}*') {{
    $nPath = "$uPath;$bin_str"
    [Environment]::SetEnvironmentVariable('Path', $nPath, [EnvironmentVariableTarget]::User)
}}
"#
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &ps_path_script])
                    .output();
            }

            log("Aaroneous successfully installed!", 1.0);
            if let Ok(mut p) = progress_arc.lock() {
                p.is_done = true;
            }
        });
    }
}

impl eframe::App for AaroneousSetupApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let is_installing = self.current_page == WizardPage::Installing;
        if is_installing {
            ui.ctx().request_repaint();
            if let Ok(p) = self.progress.lock() {
                if p.is_done {
                    self.current_page = WizardPage::Completed;
                } else if p.error_message.is_some() {
                    self.current_page = WizardPage::Error;
                }
            }
        }

        // Custom Cyberpunk / Sovereign Dark Theme styling
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(230, 240, 255));
        visuals.panel_fill = egui::Color32::from_rgb(13, 17, 23);
        visuals.window_fill = egui::Color32::from_rgb(13, 17, 23);
        ui.ctx().set_visuals(visuals);

        ui.add_space(12.0);

        // Header Banner
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🌌").size(28.0));
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Aaroneous Sovereign Hypervisor")
                        .size(19.0)
                        .strong()
                        .color(egui::Color32::from_rgb(0, 255, 204)),
                );
                ui.label(
                    egui::RichText::new("Machine-Native AI Runtime & Desktop Studio v0.3.0")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 165, 190)),
                );
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(12.0);

        match self.current_page {
            WizardPage::Welcome => {
                ui.heading("Welcome to the Aaroneous Setup Wizard");
                ui.add_space(8.0);
                ui.label(
                    "This wizard will install Aaroneous Sovereign Hypervisor, Studio Telemetry HUD, \
                     and local MCP integration tools on your computer.",
                );
                ui.add_space(14.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("✨ Included Sovereign Subsystems:").strong().color(egui::Color32::from_rgb(180, 220, 255)));
                    ui.add_space(4.0);
                    ui.label("• 🪐 3D Omni Galaxy Viewport & N-body Gravitational Physics");
                    ui.label("• ⚡ 128-byte Aligned SPMC Synapse Bus & Argus SVDD Guardrail");
                    ui.label("• 🏛️ 9 Olympian Sovereign Domain Specialists (Odin, Merlin, Ariel, etc.)");
                    ui.label("• 🔌 Model Context Protocol (MCP) Server for Claude Desktop & Cursor");
                    ui.label("• 🌐 Caduceus Multi-Hive P2P LAN Gossip Mesh & Task Offloading");
                });

                ui.add_space(20.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    if ui.button(egui::RichText::new("Next >").size(14.0).strong()).clicked() {
                        self.current_page = WizardPage::Configuration;
                    }
                });
            }

            WizardPage::Configuration => {
                ui.heading("Installation Options");
                ui.add_space(8.0);

                ui.label("Select destination directory:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.options.install_dir);
                    if ui.button("Browse...").clicked()
                        && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    {
                        self.options.install_dir = folder.to_string_lossy().to_string();
                    }
                });

                ui.add_space(14.0);
                ui.group(|ui| {
                    ui.label(egui::RichText::new("System Integrations:").strong());
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.options.create_desktop_shortcut, "Create Desktop Shortcut ('Aaroneous')");
                    ui.checkbox(&mut self.options.create_start_menu_shortcut, "Create Start Menu Entry");
                    ui.checkbox(&mut self.options.add_to_user_path, "Add Aaroneous 'bin/' to Windows User PATH (for 'a_run' & 'aaroneous' CLI)");
                    ui.checkbox(&mut self.options.launch_after_install, "Launch Aaroneous Desktop Studio upon completion");
                });

                ui.add_space(20.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    if ui.button(egui::RichText::new("Install ⚡").size(14.0).strong().color(egui::Color32::from_rgb(0, 255, 204))).clicked() {
                        self.start_installation();
                    }
                    if ui.button("< Back").clicked() {
                        self.current_page = WizardPage::Welcome;
                    }
                });
            }

            WizardPage::Installing => {
                ui.heading("Installing Aaroneous...");
                ui.add_space(10.0);

                let (pct, status, logs) = if let Ok(p) = self.progress.lock() {
                    (p.percentage, p.status_text.clone(), p.logs.clone())
                } else {
                    (0.5, "Processing...".into(), vec![])
                };

                ui.add(egui::ProgressBar::new(pct).show_percentage().animate(true));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&status).color(egui::Color32::from_rgb(180, 220, 255)));

                ui.add_space(10.0);
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    for line in &logs {
                        ui.label(egui::RichText::new(format!("> {}", line)).size(11.0).color(egui::Color32::from_rgb(140, 155, 175)));
                    }
                });
            }

            WizardPage::Completed => {
                ui.heading("🎉 Installation Completed Successfully!");
                ui.add_space(12.0);
                ui.label("Aaroneous Sovereign Hypervisor has been installed to your computer.");
                ui.add_space(6.0);
                ui.label(format!("Location: {}", self.options.install_dir));
                ui.add_space(14.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("Quick Start:").strong().color(egui::Color32::from_rgb(0, 255, 204)));
                    ui.label("• Launch 'Aaroneous' from your Desktop or Start Menu.");
                    ui.label("• Run 'a_run flagship' in any terminal to benchmark 500 cycles/sec.");
                    ui.label("• Run 'a_run mcp' to start the local Claude Desktop & Cursor tool server.");
                });

                ui.add_space(20.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    if ui.button(egui::RichText::new("Finish").size(14.0).strong().color(egui::Color32::from_rgb(0, 255, 204))).clicked() {
                        if self.options.launch_after_install {
                            let main_exe = PathBuf::from(&self.options.install_dir).join("bin").join("aaroneous.exe");
                            if main_exe.exists() {
                                let _ = std::process::Command::new(main_exe).spawn();
                            }
                        }
                        std::process::exit(0);
                    }
                });
            }

            WizardPage::Error => {
                ui.heading("❌ Installation Error");
                ui.add_space(10.0);
                let err = if let Ok(p) = self.progress.lock() {
                    p.error_message.clone().unwrap_or_else(|| "Unknown error occurred.".into())
                } else {
                    "Lock error".into()
                };
                ui.label(egui::RichText::new(err).color(egui::Color32::from_rgb(255, 100, 100)));
                ui.add_space(20.0);
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
            .with_inner_size([640.0, 440.0])
            .with_resizable(false)
            .with_title("Aaroneous Setup — Sovereign Hypervisor"),
        ..Default::default()
    };

    eframe::run_native(
        "Aaroneous Setup",
        native_options,
        Box::new(|cc| Ok(Box::new(AaroneousSetupApp::new(cc)))),
    )
}
