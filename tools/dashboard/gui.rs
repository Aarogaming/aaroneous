// Aaroneous Graphical Dashboard
// Native 3D visualization using wgpu embedded in egui via PaintCallback

use eframe::egui;
use ratatui::{Terminal, backend::Backend};
use std::time::Instant;
use crate::tui_framework::{TuiApp, draw, Page};
use crate::constellation_ui::ConstellationCanvas;
use crate::constellation_3d::Constellation3D;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;

pub struct EguiRatatuiBridge {
    pub app_state: TuiApp,
    last_tick: Instant,
    constellation_2d: ConstellationCanvas,
    constellation_3d: Constellation3D,
    use_3d: bool,
    wgpu_ready: bool,
}

impl EguiRatatuiBridge {
    pub fn new() -> Self {
        Self {
            app_state: TuiApp::default(),
            last_tick: Instant::now(),
            constellation_2d: ConstellationCanvas::new(),
            constellation_3d: Constellation3D::new(),
            use_3d: true,
            wgpu_ready: false,
        }
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.heading("⚡ Aaroneous Omni Dashboard");
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                if ui.button("🏠 Home").clicked() { self.app_state.page = Page::Home; }
                if ui.button("🌌 Constellation").clicked() { self.app_state.page = Page::Metabolic; }
                if ui.button("🤖 Specialists").clicked() { self.app_state.page = Page::Specialists; }
                if ui.button("📜 Log").clicked() { self.app_state.page = Page::EventLog; }
                
                ui.separator();
                if ui.button(if self.use_3d { "3D" } else { "2D" }).clicked() {
                    self.use_3d = !self.use_3d;
                }
            });

            ui.add_space(8.0);
            ui.separator();

            if self.app_state.page == Page::Metabolic {
                if self.use_3d && self.wgpu_ready {
                    // Render 3D constellation via wgpu
                    self.render_3d_constellation(ui);
                } else {
                    // Fallback to 2D
                    self.constellation_2d.ui(ui);
                }
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

    fn render_3d_constellation(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        // Handle mouse interaction for camera rotation
        if response.dragged() {
            let delta = response.drag_delta();
            self.constellation_3d.rotation.0 += delta.x * 0.01;
            self.constellation_3d.rotation.1 += delta.y * 0.01;
            self.constellation_3d.rotation.1 = self.constellation_3d.rotation.1.clamp(-1.5, 1.5);
        }

        // Scroll to zoom
        if response.hovered() {
            let scroll = ui.input(|i| i.scroll_delta.y);
            if scroll != 0.0 {
                self.constellation_3d.camera_distance = (self.constellation_3d.camera_distance - scroll).max(50.0).min(2000.0);
            }
        }

        // Use PaintCallback to embed wgpu rendering
        ui.painter().add(egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(egui_wgpu::Callback::new_paint_callback(
                rect,
                self.constellation_3d.clone(),
            )),
        });
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_tick.elapsed() >= std::time::Duration::from_millis(250) {
            self.last_tick = Instant::now();
        }

        // Use a Frame with a clear background to prevent system ghosting
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(15, 15, 15)))
            .show(ctx, |ui| {
                self.draw_ui(ui, _frame);
            });

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

pub fn run_dashboard() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 900.0])
            .with_title("Aaroneous Omni Dashboard"),
        wgpu_options: egui_wgpu::WgpuConfiguration::default(),
        ..Default::default()
    };
    
    eframe::run_native(
        "Aaroneous Dashboard",
        options,
        Box::new(|cc| {
            // Initialize wgpu if available
            let mut bridge = EguiRatatuiBridge::new();
            if let Some(wgpu_render_state) = &cc.wgpu_render_state {
                // wgpu is available
                bridge.wgpu_ready = true;
            }
            Ok(Box::new(bridge))
        }),
    )
}
